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

#### Building from Source
```bash
git clone https://github.com/your-repo/ljsp-macro-compiler-rs.git
cd ljsp-macro-compiler-rs
cargo build --release
```
The binary will be available at `./target/release/ljsp-macro-compiler-rs`.

#### Installing Globally
To use the compiler from anywhere in your system, you can install it using `cargo install`:
```bash
cargo install --path .
```
Ensure that your Cargo bin directory (usually `~/.cargo/bin`) is in your system's `PATH`.

#### Dependency Requirements
- **Rust Toolchain**: 1.80+ (2024 Edition)
- **V8 Environment**: Managed automatically via `deno_core`. No external V8 installation is required.

## Usage

To compile a JavaScript file containing macro imports:

```bash
./target/release/ljsp-macro-compiler-rs input.js
```

**Output:**
- `input.out.js`: The expanded and transformed JavaScript.
- `input.out.js.map`: Source map for debugging.

## Integration

### Node.js Integration
The easiest way to integrate the compiler into a Node.js project is via a build script in `package.json`.

**package.json**
```json
{
  "scripts": {
    "build:macros": "ljsp-macro-compiler-rs src/index.js",
    "start": "npm run build:macros && node src/index.out.js"
  }
}
```

### Vue/Vite Integration
For modern frontend workflows using Vite, you can use a simple local plugin to transform your files during development and build.

**vite.config.js**
```javascript
import { defineConfig } from 'vite';
import { execSync } from 'child_process';

const ljspMacroPlugin = () => ({
  name: 'vite-plugin-ljsp-macros',
  transform(code, id) {
    if (id.endsWith('.js') && !id.includes('.out.js')) {
      // Call the compiler CLI
      execSync(`ljsp-macro-compiler-rs ${id}`);
      // Return the generated code and map
      const fs = require('fs');
      return {
        code: fs.readFileSync(`${id.replace('.js', '.out.js')}`, 'utf-8'),
        map: fs.readFileSync(`${id.replace('.js', '.out.js.map')}`, 'utf-8')
      };
    }
  }
});

export default defineConfig({
  plugins: [ljspMacroPlugin()]
});
```

## Practical Examples


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

## Practical Examples

The LJSP macro system can be used to optimize or simplify code across different JavaScript environments.

### 1. Node.js: Environment-Specific Configuration
Optimize your production bundles by resolving configuration or environment checks at compile time.

**Macro (`config.macro.js`)**
```javascript
import { defineMacro } from "@ljsp/macro-runtime";

export const getEnv = defineMacro((ctx, key) => {
  const value = process.env[key.value] || null;
  return ctx.literal(value);
});
```

**Usage (`server.js`)**
```javascript
import { getEnv } from "./config.macro.js";

const apiKey = getEnv("API_KEY");
// Compiles directly to: const apiKey = "secret-key-from-env";
```

### 2. React: Conditional Rendering Macro
Simplify complex conditional logic in JSX-like environments without the overhead of additional component layers.

**Macro (`react-utils.macro.js`)**
```javascript
import { defineMacro } from "@ljsp/macro-runtime";

export const renderIf = defineMacro((ctx, condition, element) => {
  return ctx.syntax.expression`${condition} ? ${element} : null`;
});
```

**Usage (`Component.js`)**
```javascript
import { renderIf } from "./react-utils.macro.js";

function Profile({ user }) {
  return (
    <div>
      <h1>{user.name}</h1>
      {renderIf(user.isAdmin, <AdminPanel />)}
    </div>
  );
}
```

### 3. Vue: Composition API Boilerplate
Reduce repetitive boilerplate when defining reactive properties in Vue 3.

**Macro (`vue-utils.macro.js`)**
```javascript
import { defineMacro } from "@ljsp/macro-runtime";

export const quickRef = defineMacro((ctx, name, initialValue) => {
  return ctx.syntax.statement`const ${name} = ref(${initialValue})`;
});
```

**Usage (`App.vue`)**
```javascript
import { quickRef } from "./vue-utils.macro.js";

// Inside setup()
quickRef(count, 0);
quickRef(name, "Vue User");
// Compiles to:
// const count = ref(0);
// const name = ref("Vue User");
```

For more details, see [Architecture Documentation](docs/architecture.md).
