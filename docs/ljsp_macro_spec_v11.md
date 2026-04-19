# Specification: JS-Syntax Macro System for LJSP Targets (v11, Direction A)

## 1. Overview

This document specifies a **JavaScript-syntax macro system** that targets standard JavaScript and is intended to pair well with LJSP as a runtime library.

This is **not** a Lisp-syntax language. It is a **compile-time macro system over JavaScript source**.

v11 replaces the invalid-syntax approach from v10 with a design that uses:

- **valid JavaScript source syntax**
- **compile-time macro modules**
- **AST transforms plus syntax templates**
- **explicit source metadata and source map requirements**
- **formal macro expansion order**
- **a defined compile-time execution model**
- **formal `recur` lowering rules**

---

## 2. Language boundary and naming

This system is a **JS macro system targeting LJSP-compatible runtime code**.

It is not LJSP itself and does not use Lisp reader syntax.

Implementations SHOULD present it as a name distinct from bare LJSP, for example:

- “LJSP Macros for JS”
- “LJSP MacroJS”
- another implementation-defined product name

This avoids confusion with Lisp-syntax expectations such as `defmacro` and s-expressions.

---

## 3. Surface syntax

All user source files are valid JavaScript modules.

There are two kinds of modules:

- **runtime modules**: ordinary `.js`, `.mjs`, or implementation-configured JS files
- **macro modules**: ordinary valid JS files identified by file extension or import position, for example `.macro.js`

No new parser-level keyword such as `macro` is introduced.

---

## 4. Execution model

Pipeline:

```txt
JS source
-> Parse (ESTree-compatible AST with source spans)
-> Load macro modules
-> Execute macro module top-level code in compile-time sandbox
-> Expand macro calls to fixed point
-> Analyze scopes / validate recur / collect imports
-> Lower recur and other special transforms
-> Optimize
-> Emit JavaScript + source maps
-> Execute emitted JS with LJSP/runtime libraries
```

Macros exist only at compile time and are fully erased before runtime.

---

## 5. Valid JS macro definition syntax

Macros are defined in macro modules using ordinary JS exports and a compile-time helper called `defineMacro`.

Example:

```js
import { defineMacro } from "@ljsp/macro-runtime";

export const unless = defineMacro((ctx, test, ...body) => {
  return ctx.syntax.statement`
    if (!(${test})) {
      ${ctx.block(body)}
    }
  `;
});
```

This is valid JavaScript syntax.

### 5.1 Requirements

- `defineMacro` may only appear in macro modules
- the function passed to `defineMacro` is executed only in the compile-time environment
- macro definitions must be top-level exports
- macro functions receive AST/syntax values, not runtime JS values

---

## 6. Valid JS macro import syntax

Macro imports use ordinary JS syntax but MUST come from macro modules.

Example:

```js
import { unless, whenLet } from "./control.macro.js";
```

Rules:

- imports from `.macro.js` are compile-time-only imports
- such imports are not emitted into runtime JavaScript
- ordinary imports from non-macro modules are runtime imports

This removes ambiguity between macro and runtime values without inventing new JS syntax.

### 6.1 Portability rule

A conforming implementation MUST support at least one explicit macro-module identification mechanism, and that mechanism MUST be documented.

The default portable convention in this spec is:

```txt
*.macro.js
```

---

## 7. Compile-time execution model

Macro modules execute in a **compile-time sandbox**.

### 7.1 Sandbox semantics

The sandbox MUST support:

- deterministic JS evaluation of macro module top-level code
- deterministic execution of macro functions
- imports from other macro modules
- access to the compile-time macro API
- access to documented deterministic helpers only

The sandbox MUST NOT provide unrestricted access to:

- filesystem IO, unless explicitly enabled by implementation configuration
- network IO
- wall-clock time
- process mutation
- ambient runtime application state

### 7.2 Purity and side effects

Macros SHOULD be deterministic. Implementations MAY enforce stricter determinism.

At minimum:

- macro expansion output must depend only on source input, macro module code, and configured compiler options
- observable side effects outside the compile-time sandbox are prohibited by default

### 7.3 Evaluation limits

The compiler MUST enforce configurable limits:

- `maxExpansionDepth`: default `256`
- `maxMacroSteps`: default `100000`
- `maxMacroCallDepth`: default `1024`

Exceeding a limit MUST produce a compile-time error with source location and expansion stack.

---

## 8. Macro values and invocation

A macro is a compile-time function value created by `defineMacro`.

A macro is invoked when the expander encounters a `CallExpression` whose callee resolves to a macro binding.

Examples:

```js
unless(x > 10, log("low"));
```

or

```js
controls.unless(x > 10, log("low"));
```

if `controls` is a namespace object imported from a macro module and the implementation supports namespace macro imports.

### 8.1 Expansion result

A macro MUST return one of:

- a single AST node
- a syntax-template result that compiles to a single AST node
- a statement list only in positions where statement-list expansion is explicitly allowed by the context API

Returning arbitrary runtime JS values is invalid.

---

## 9. Macro expansion order

Expansion proceeds to a fixed point.

### 9.1 Algorithm

For each node:

1. inspect the current node
2. if it is a macro invocation, invoke the macro
3. replace the node with the macro result
4. restart expansion on the replacement node
5. otherwise recursively expand child nodes in source order

### 9.2 Nested macro expansions

If a macro expands to code containing further macro calls, those macro calls MUST be expanded by the same fixed-point algorithm.

### 9.3 Expansion order guarantee

Expansion is **outside-in first**, then recursive on replacements, then recursive through non-macro child nodes in source order.

This guarantee is normative.

### 9.4 Shadowing rule

Runtime-local variables shadow unqualified macro bindings.

Example:

```js
import { unless } from "./control.macro.js";

function f(unless) {
  return unless(x);
}
```

Inside `f`, `unless(...)` is **not** a macro invocation, because the local parameter shadows the imported macro binding.

Qualified macro references, if supported by the implementation, are not shadowed by unqualified locals.

---

## 10. Source metadata and source maps

Every parsed AST node MUST carry source span metadata sufficient for:

- file
- start line / column / offset
- end line / column / offset

Macro-produced nodes MUST also carry origin metadata such that source maps can attribute output code to:

- original call site
- macro definition site, optionally as auxiliary metadata
- expanded node spans

A conforming compiler MUST emit source maps.

This is required, not optional.

---

## 11. AST representation

The implementation MUST use an ESTree-compatible AST or a documented equivalent with a lossless mapping.

Macro authors do not need to manipulate raw AST objects directly unless they choose to.

The macro API MUST expose either:

- stable builder helpers, or
- syntax-template values, or
- both

v11 requires both.

---

## 12. Macro authoring API

Each macro function receives a first argument `ctx`, the compile-time macro context.

Example:

```js
export const unless = defineMacro((ctx, test, ...body) => {
  return ctx.syntax.statement`
    if (!(${test})) {
      ${ctx.block(body)}
    }
  `;
});
```

### 12.1 Required `ctx` API

A conforming implementation MUST provide:

- `ctx.syntax.expression` tagged template
- `ctx.syntax.statement` tagged template
- `ctx.syntax.program` tagged template
- `ctx.ident(name)`
- `ctx.gensym(prefix?)`
- `ctx.block(statements)`
- `ctx.call(callee, args)`
- `ctx.member(object, property, { computed? })`
- `ctx.literal(value)`
- `ctx.return(expr)`
- `ctx.var(kind, id, init)`
- `ctx.function(params, body, options?)`
- `ctx.clone(node)`
- `ctx.error(message, node?)`

### 12.2 Builder result shapes

Builder helpers MUST return ESTree-compatible nodes.

Examples:

- `ctx.ident("x")` -> `Identifier`
- `ctx.literal(1)` -> `Literal`
- `ctx.call(callee, args)` -> `CallExpression`
- `ctx.member(obj, prop, { computed: false })` -> `MemberExpression`

### 12.3 Template interpolation semantics

Template interpolations may contain:

- AST nodes
- arrays of statement nodes in statement-list positions
- identifiers produced by `ctx.gensym`
- implementation-defined syntax fragments if documented

Invalid interpolation shape MUST raise a compile-time error.

---

## 13. Syntax templates

Because raw AST construction is verbose, syntax templates are required.

Example:

```js
export const debug = defineMacro((ctx, expr) => {
  return ctx.syntax.statement`
    console.log(${expr});
  `;
});
```

### 13.1 Expression vs statement templates

- `ctx.syntax.expression` MUST produce an expression node
- `ctx.syntax.statement` MUST produce a statement node
- `ctx.syntax.program` MUST produce a program or statement-list fragment, as documented by the implementation

### 13.2 Hygiene in templates

Interpolated identifiers created with `ctx.gensym()` MUST preserve generated identity through template insertion and later lowering.

---

## 14. Hygiene model

v11 is **partially hygienic**:

- generated identifiers from `ctx.gensym()` are hygienic and cannot collide with user-defined bindings
- user-written identifiers interpolated into templates preserve their original binding identity
- plain textual identifiers written literally inside templates are not automatically hygienic and obey ordinary JS lexical rules

Example of safe binding:

```js
export const once = defineMacro((ctx, expr) => {
  const tmp = ctx.gensym("tmp");
  return ctx.syntax.expression`
    (() => {
      const ${tmp} = ${expr};
      return ${tmp};
    })()
  `;
});
```

This MUST not collide with user code.

### 14.1 Explicit capture rule

If a macro author writes a literal identifier inside a template, such as `tmp`, and does not use `ctx.gensym()`, that identifier is an ordinary lexical name and may capture or be captured.

This is intentional and must be documented to macro authors.

---

## 15. Compile-time helper API

The compile-time sandbox MUST provide:

- `defineMacro`
- the `ctx` API described above
- deterministic helper functions for macro modules:
  - `String`, `Array`, `Object`, `Map`, `Set`
  - standard arithmetic and comparison operators through ordinary JS
  - `JSON` if desired by implementation
- implementation-defined additional helpers only if documented

This system does **not** require a separate Lisp-like compile-time interpreter. Macro modules are written in JavaScript and execute in the compile-time JS sandbox.

---

## 16. `recur` syntax and semantics

`recur` is not an ordinary runtime function. It is a recognized compile-time special form represented in JS syntax as a call expression:

```js
recur(nextN, nextAcc)
```

### 16.1 Validity rules

`recur(...)` is valid only when:

- it appears inside a function eligible for recur lowering
- it is in tail position of that function
- its arity matches the function parameter count

Otherwise compilation fails.

### 16.2 Eligible functions

By default, recur lowering applies to:

- function declarations
- function expressions
- arrow functions with block bodies

Implementations MAY additionally support named macro-generated loop abstractions if documented.

### 16.3 Tail-position rules

A `recur(...)` call is in tail position only when it is the final result of the current function body, recursively through:

- the final statement of a block
- both branches of an `if` in tail position
- the final branch of nested block structures
- the expression directly returned by `return recur(...)`

It is not valid:

- in argument position
- in non-final statements
- inside nested inner functions targeting the outer function
- in conditional expressions unless the implementation documents exact support and validates both branches

### 16.4 Lowering model

The compiler MUST lower valid `recur` to an internal loop transformation.

Reference lowering:

```js
function fact(n, acc) {
  while (true) {
    if (n === 0) return acc;
    const __next0 = n - 1;
    const __next1 = acc * n;
    n = __next0;
    acc = __next1;
    continue;
  }
}
```

### 16.5 Conditional recur

For tail-position recur in branches, the compiler MUST preserve branch semantics.

Example source:

```js
function fact(n, acc) {
  if (n === 0) return acc;
  return recur(n - 1, acc * n);
}
```

Example lowered shape:

```js
function fact(n, acc) {
  while (true) {
    if (n === 0) return acc;
    const __next0 = n - 1;
    const __next1 = acc * n;
    n = __next0;
    acc = __next1;
    continue;
  }
}
```

If no direct structural lowering is possible, the compiler MAY use an internal sentinel protocol, but that protocol MUST remain inside compiler-generated control scaffolding and must never leak into normal emitted expression contexts.

---

## 17. Scope and binding analysis

The compiler MUST perform lexical scope analysis before and during macro expansion sufficient to determine:

- whether a callee resolves to a macro binding
- whether that binding is shadowed by a local runtime binding
- whether identifiers introduced by macros preserve intended hygiene
- whether `recur` targets the correct enclosing function

This analysis is required for correctness.

---

## 18. Runtime integration and LJSP

The emitted JavaScript may target LJSP runtime functions and helpers.

Example:

```js
import { map, filter } from "ljsp";
```

### 18.1 Post-expansion import analysis

Import discovery MUST run after macro expansion.

If a macro introduces runtime references such as `map`, `filter`, or helper shims, those references MUST be included in final import analysis.

### 18.2 Macro runtime shims

If macros expand to runtime helper calls not otherwise available, the compiler MUST either:

- inject documented runtime shims, or
- emit a compile-time error

This behavior MUST be configurable.

---

## 19. Macro modules and namespaces

### 19.1 Module loading

Macro modules are loaded and executed before expanding the modules that import them.

### 19.2 Module caching

Implementations MAY cache compiled/evaluated macro modules between builds. Caching must not change semantics.

### 19.3 Namespace import shape

At minimum, a conforming implementation MUST support named imports from macro modules:

```js
import { unless, once } from "./control.macro.js";
```

Implementations MAY support namespace imports:

```js
import * as controls from "./control.macro.js";
```

If namespace macro imports are supported, expansion rules for `controls.unless(...)` MUST be documented.

---

## 20. Error model

The compiler MUST distinguish:

- `ParseError`
- `MacroLoadError`
- `ExpansionError`
- `AnalysisError`
- `LoweringError`
- `EmitError`

Every compiler-stage error MUST include:

- stage
- message
- file
- line / column
- expansion stack where relevant
- macro definition location where relevant

---

## 21. Optimization

The compiler MAY perform:

- flattening of macro-generated nested blocks
- dead code elimination if semantics are preserved
- inlining of certain helper scaffolds
- optimized direct recur lowering

Any optimization must preserve observable semantics and source map quality.

---

## 22. Examples

### 22.1 `unless`

Macro module:

```js
import { defineMacro } from "@ljsp/macro-runtime";

export const unless = defineMacro((ctx, test, ...body) => {
  return ctx.syntax.statement`
    if (!(${test})) {
      ${ctx.block(body)}
    }
  `;
});
```

Use site:

```js
import { unless } from "./control.macro.js";

unless(x > 10, console.log("low"));
```

Output:

```js
if (!(x > 10)) {
  console.log("low");
}
```

### 22.2 `once`

Macro module:

```js
import { defineMacro } from "@ljsp/macro-runtime";

export const once = defineMacro((ctx, expr) => {
  const tmp = ctx.gensym("tmp");
  return ctx.syntax.expression`
    (() => {
      const ${tmp} = ${expr};
      return ${tmp};
    })()
  `;
});
```

Use site:

```js
import { once } from "./util.macro.js";

const value = once(expensiveCall());
```

Possible output:

```js
const value = (() => {
  const __m_tmp_1 = expensiveCall();
  return __m_tmp_1;
})();
```

with source maps preserving the original macro call location.

---

## 23. Summary

v11 defines a **valid-JS macro system** that:

- uses ordinary JS module syntax
- identifies macro modules by file convention
- executes macros in a compile-time sandbox
- provides both syntax templates and AST builders
- specifies macro expansion order
- requires source metadata and source maps
- provides a formal `recur` transform model
- keeps compile-time and runtime semantics separate
- targets JavaScript suitable for LJSP-backed runtime code
