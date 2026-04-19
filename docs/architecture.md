# LJSP Macro Compiler: System Architecture (v11 Conforming)

This document describes the architecture of the Rust-based macro compiler for JavaScript-syntax macros targeting the LJSP runtime.

## 1. High-Level Pipeline

The compiler operates as a multi-stage transformation pipeline:

```txt
Source JS (.js) 
  -> SWC Parser (AST + Spans)
  -> Macro Discovery (Identify *.macro.js imports)
  -> Sandbox Initialization (Deno/V8 + ctx API)
  -> Recursive Expander (Fixed-point loop, Outside-in)
  -> Analysis Pass (v11.2 Safety Rules: recur, try/finally)
  -> Lowering Pass (recur -> while(true) loop)
  -> Runtime Injection (Auto-import map, filter from 'ljsp')
  -> Emitter (SWC Codegen + Source Maps)
```

## 2. Core Components

### 2.1 Parser (`src/parser.rs`)
Uses `swc_ecma_parser` to generate a high-fidelity Abstract Syntax Tree (AST). It enables precise source tracking with `Span` metadata, which is critical for the mandatory source map requirement.

### 2.2 Macro Sandbox (`src/sandbox.rs`)
Powered by `deno_core` and the V8 engine.
- **Isolation**: Macro modules execute in a deterministic environment.
- **ctx API**: Implements the required builder suite (`ctx.gensym`, `ctx.syntax`, `ctx.block`, etc.).
- **Transpilation**: Performs on-the-fly ESM-to-CJS shimming to support `export` and official `@ljsp/macro-runtime` imports within the sandbox.

### 2.3 The Expander (`src/expander.rs`)
A `swc_ecma_visit::Fold` implementation that performs recursive expansion.
- **Outside-In Order**: Processes macro calls at the parent level before descending into children.
- **Fixed-Point**: Iterates until no more macro calls are detected.
- **Scope Tracking**: Implements a lexical scope stack to respect variable shadowing (e.g., local variables shadowing macro names).
- **Namespace Support**: Supports both named (`{ unless }`) and namespace (`* as pkg`) macro imports.

### 2.4 Recur System (`src/recur.rs`)
Handles the special `recur` control transfer.
- **Validation**: Enforces v11.1 (arrow function restriction) and v11.2 (conservative `finally` restriction).
- **Arity Check**: Validates argument counts against function parameters.
- **Transformation**: Lowers valid tail-position `recur` calls into optimized `while(true)` loops with collision-safe state updates (`__nextN`).

### 2.5 AST Marshalling (`src/marshalling.rs` & `src/template.rs`)
The bridge between Rust (SWC AST) and JavaScript (ESTree-like objects).
- **Bidirectional**: Converts complex Rust nodes to JSON for the sandbox and reconstructs Rust nodes from macro return values.
- **Syntax Templates**: Implements fragment parsing and surgical interpolation, allowing macro authors to use tagged templates for code generation.

### 2.6 Runtime Manager (`src/runtime.rs`)
Analyzes the final AST for free variables (e.g., `map`, `filter`). It automatically injects the necessary LJSP runtime imports, ensuring the output is ready for execution without manual dependency management.

## 3. Mandatory Specification Compliance

- **v11**: Implements valid-JS syntax, outside-in expansion, and mandatory source maps.
- **v11.1**: Implements `ctx.block` semantics and arrow function expression body restrictions.
- **v11.2**: Implements conservative `try/catch/finally` safety rules for `recur`.

## 4. Usage

```bash
cargo run -- <input>.js
```
- **Output**: `<input>.out.js` (Transformed runtime code)
- **Source Map**: `<input>.out.js.map` (Mapping back to original source)
