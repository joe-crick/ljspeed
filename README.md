# LJSP Macro Compiler

A high-performance Rust-based compiler for JavaScript macros targeting the LJSP runtime. This compiler conforms to the **v11.x specification**, providing recursive macro expansion, high-fidelity source maps, and optimized tail-call lowering.

## Features

- **Recursive Macro Expansion**: Implements an outside-in, fixed-point expansion loop that respects lexical scope and variable shadowing.
- **Isolated V8 Sandbox**: Macros are executed in a deterministic Deno-powered V8 environment, ensuring safety and isolation.
- **Tail-Call Optimization (`recur`)**: Supports the `recur` keyword for self-recursion, lowering it to optimized `while` loops with collision-safe state updates.
- **Rich `ctx` Builder API**:
  - `ctx.syntax`: Fragment parsing via tagged templates (`expression`, `statement`, `program`).
  - `ctx.gensym`: Conflict-free identifier generation.
  - `ctx.block`, `ctx.call`, `ctx.ident`, etc.: Direct AST construction helpers.
- **Automatic Runtime Injection**: Automatically detects free variables (like `map`, `filter`) and injects necessary LJSP runtime imports.
- **Source Map Support**: Full end-to-end source mapping from the generated runtime code back to the original macro-heavy source.

## Specification Compliance

This compiler strictly adheres to the following specification versions:
- **v11**: Core macro syntax and recursive expansion.
- **v11.1**: `ctx.block` semantics and arrow function restrictions.
- **v11.2**: Conservative `try/catch/finally` safety rules for `recur`.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition supported)

### Installation

```bash
git clone https://github.com/your-repo/ljsp-macro-compiler-rs.git
cd ljsp-macro-compiler-rs
cargo build --release
```

### Usage

To compile a JavaScript file containing macro imports:

```bash
./target/release/ljsp-macro-compiler-rs input.js
```

**Output:**
- `input.out.js`: The expanded and transformed JavaScript.
- `input.out.js.map`: Source map for debugging.

## Example

**Input File (`app.js`)**
```javascript
import { unless } from "./control.macro.js";

unless(user.isAdmin, () => {
  console.log("Access Denied");
});
```

**Macro Definition (`control.macro.js`)**
```javascript
import { defineMacro } from "@ljsp/macro-runtime";

export const unless = defineMacro((ctx, condition, body) => {
  return ctx.syntax.statement`if (!(${condition})) { (${body})(); }`;
});
```

## Architecture

The compiler operates as a multi-stage transformation pipeline:
1. **Parser**: Uses SWC to generate a high-fidelity AST.
2. **Sandbox**: Initializes a V8 environment for macro execution.
3. **Expander**: Performs recursive, outside-in macro expansion.
4. **Analysis & Lowering**: Validates and transforms `recur` calls into loops.
5. **Runtime Injection**: Injects required `@ljsp/runtime` imports.
6. **Codegen**: Emitters the final JavaScript and source maps.

For more details, see [Architecture Documentation](docs/architecture.md).
