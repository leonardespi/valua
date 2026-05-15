# valua 🚀
**A modern, high-performance Lua 5.4 to Lua 5.1/LuaJIT transcompiler written in Rust.**

---

## 🌌 Overview

Lua is a beautiful, minimalist language, but its ecosystem suffers from a deep version fragmentation. While **Lua 5.4** introduces modern features like `<close>` attributes, `<const>` variables, and native bitwise operators, the industry remains largely anchored to **Lua 5.1** due to the incredible performance of **LuaJIT** (used in Neovim, OpenResty, Kong, and major game engines).

**valua** bridges this generational gap. It acts as the "Babel" for the Lua ecosystem, allowing developers to write clean, modern, and safe Lua 5.4+ code, and compile it down to highly optimized, JIT-friendly Lua 5.1-compatible source code.

---

## ✨ Features & Compilation Mapping

valua parses Lua 5.4 source code into an Abstract Syntax Tree (AST) using Rust, transforms modern syntactical constructs, and emits vanilla Lua 5.1 code.

| Feature (Lua 5.4) | Transpilation Strategy / Target (Lua 5.1) | Status |
| :--- | :--- | :--- |
| **Bitwise Operators** (`&`, `|`, `<<`, etc.) | Transformed into `bit` operations (e.g., `bit.lshift(a, 2)`) utilizing LuaJIT's built-in bit library. | 🛠️ Planned |
| **`<const>` Attributes** | Validated via **Static Analysis** during compilation. Emits standard local variables but throws a compile-time error if mutated. | 🛠️ Planned |
| **`<close>` Attributes** | Emulated by wrapping the execution scope into a protected call (`pcall`) mechanics to guarantee deterministic resource cleanup (mimicking `__close` metamethods). | 🛠️ Planned |
| **String & Math Utilities** | Injects high-performance, JIT-friendly pure-Lua polyfills for modern standard library extensions. | 🛠️ Planned |

---

## 🏗️ Architecture

The project is structured as a standard compiler pipeline leveraging Rust's safety and pattern-matching capabilities:

1. **Lexer & Parser:** Tokenizes and parses Lua 5.4 grammar into a strongly-typed AST.
2. **Transformer (AST Mutator):** Walks the tree and rewrites Lua 5.4 specific nodes into equivalent Lua 5.1 structures. This is where static analysis (like checking `<const>` mutations) happens.
3. **Code Generator:** Emits formatted, production-ready Lua 5.1 code.

---

## 🛠️ Getting Started (Development)

### Prerequisites
* Rust (MSRV 1.75+)
* Cargo

### Installation
Clone the repository and build the binary:

```bash
git clone [https://github.com/leonardespi/valua.git](https://github.com/leonardespi/valua.git)
cd valua
cargo build --release
