# Specification: JS-Syntax Macro System for LJSP Targets (v11.1)

## 5.1 `defineMacro` import path

The default import path for macro definition is:

```js
import { defineMacro } from "@ljsp/macro-runtime";
```

Implementations MAY allow configuration but MUST document it.

---

## 12.1.1 `ctx.block` semantics

`ctx.block(nodes)` returns a `BlockStatement` node.

- `nodes` must be an array of statement nodes
- empty array produces `{ body: [] }`

---

## 12.1.2 AST mutability

All AST nodes returned by `ctx` builders are **immutable**.

- Mutation is forbidden
- `ctx.clone(node)` exists but SHOULD be avoided in normal macro authoring

---

## 16.2.1 Arrow function expression bodies

`recur` is NOT allowed in arrow functions with expression bodies.

Invalid:

```js
(n, acc) => recur(n - 1, acc * n)
```

Valid:

```js
(n, acc) => {
  if (n === 0) return acc;
  return recur(n - 1, acc * n);
}
```

---

## 19.3 Namespace imports

For portability:

- Macro authors SHOULD use named imports

```js
import { unless } from "./control.macro.js";
```

If namespace imports are supported:

```js
import * as control from "./control.macro.js";
```

Then implementations MUST document how:

```js
control.unless(...)
```

is resolved during macro expansion.

---

## Summary

v11.1 clarifies:

- macro runtime import path
- AST immutability guarantees
- block construction rules
- recur limitations in arrow functions
- macro import portability rules
