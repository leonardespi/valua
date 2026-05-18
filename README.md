# valua

**A Lua 5.5 to Lua 5.1/LuaJIT transpiler written in Rust.**

Lua 5.4 is a fully supported subset of the source language.

---

## Overview

The Lua ecosystem is split between two incompatible eras. Lua 5.5 introduced modern syntax — `<const>` and `<close>` variable attributes, native bitwise operators, compact array literals, and global declarations — but the dominant production runtimes remain anchored to Lua 5.1. LuaJIT, which powers Neovim, OpenResty, Kong, and most embedded game engines, delivers performance one to two orders of magnitude beyond PUC-Rio Lua on numeric and loop-heavy code, and it has not tracked the syntax changes introduced in Lua 5.2 through 5.5.

valua closes that gap. It takes Lua 5.5 source and emits semantically equivalent Lua 5.1 source that runs correctly under LuaJIT. The result is JIT-compilable, free of heap wrappers, and byte-identical across repeated runs for the same input and compiler version.

This is not a total translation. A formal proof (see [the impossibility theorem](docs/demostracion_transpilacion_lua.md)) establishes that no transpiler can simultaneously be total, semantically correct, and performance-preserving for all Lua 5.4+ programs targeting LuaJIT. valua adopts that impossibility as a design principle: it defines a well-specified domain (`L_{5.5}^native`) and rejects programs outside it with explicit, actionable compiler errors rather than emitting silently incorrect code.

---

## What valua translates

| Source construct | Output strategy | Performance |
|---|---|---|
| Bitwise operators (`&`, `\|`, `~`, `<<`, `>>`) | `bit.band`, `bit.bor`, `bit.bxor`, `bit.lshift`, `bit.rshift` calls | Native — LuaJIT JIT-compiles `bit.*` to single instructions |
| Integer division (`//`) | `math.floor(a / b)` or equivalent JIT-friendly pattern | Native |
| `<const>` attribute | Static validation at compile time; emits plain `local` | Native |
| `<close>` attribute | `pcall` wrapper with `__close` metamethod invocation on scope exit | Negligible overhead for I/O-bound usage |
| Compact array literals | Transparent — identical syntax in 5.5 and 5.1/LuaJIT | Native |
| `<global>` declarations | Rewritten to `_G` assignment | Native |

## What valua rejects

Programs that observe the integer/float type distinction (`math.type`), depend on exact 64-bit integer overflow semantics, or use other constructs outside the native domain produce a compiler error with a stable error code, source span, and remediation suggestion. valua never emits semantically questionable code silently.

| Error code | Trigger |
|---|---|
| `E0101` | Call to `math.type()` — numeric type reflection is not representable in LuaJIT |
| `E0102` | Detected dependency on exact 64-bit integer overflow semantics |
| `E0301` | Assignment to a `<const>` variable |
| `E04xx` | Syntactically recognized construct from a Lua version not yet supported as source |

---

## Why not a total transpiler

The short answer: it is mathematically impossible. LuaJIT collapses Lua's integer/float distinction into a single IEEE 754 double type. Preserving the distinction requires heap wrappers that degrade arithmetic performance by a factor of 20 to 100. Deciding statically which programs need wrappers and which do not reduces to the halting problem.

valua's response to this is fixed, transparent rules: what you write is what you get, with no hidden emulation and no performance surprises. The full argument is in [docs/demostracion_transpilacion_lua.md](docs/demostracion_transpilacion_lua.md).

---

## Architecture

valua is a Cargo workspace. Each compilation stage is an independent crate.

```
source text
  valua-lexer        text -> tokens
  valua-parser       tokens -> AST
  valua-lint         AST -> diagnostics        (usable independently)
  valua-transformer  AST -> AST (rewritten)
  valua-codegen      AST -> text (via Backend)
output text
```

The pipeline is orchestrated by `valua-core` and exposed as a binary through `valua-cli`. Two crates are public API with semver stability from 1.0:

- **`valua-lint`** — static analysis without requiring transpilation. Useful for pre-commit hooks, editor integrations, and validating portability across Lua versions without committing to a full build pipeline.
- **`valua-core`** — the complete transpilation API: `Compiler::compile(source, opts) -> Result<String, CompileError>`.

---

## Installation

*valua is under active development and has not yet reached 1.0. The instructions below apply once a release is published.*

```sh
cargo install valua-cli
```

Pre-compiled binaries for Linux x86\_64, Linux aarch64, macOS aarch64, macOS x86\_64, and Windows x86\_64 will be available on the GitHub releases page.

---

## Usage

```sh
# Transpile a file to LuaJIT-compatible Lua 5.1
valua build input.lua -o output.lua --target luajit

# Transpile to plain Lua 5.1 (includes bit polyfill)
valua build input.lua -o output.lua --target lua51

# Validate without emitting output (useful in CI)
valua check input.lua

# Run static analysis only (no transpilation required)
valua lint input.lua --target luajit

# Print version
valua version
```

---

## Development

Requirements: Rust 1.75 or later, `just`.

```sh
# Install just
cargo install just

# Wire the pre-commit hook
just install-hooks

# Run all checks (format, lint, test, build) — must pass before every commit
just check

# Run only tests
just test

# Run a specific fixture test
just test-one bitwise_and
```

---

## Q: What happens to valua if LuaJIT Remake reaches production on Lua 5.4?

`valua-lint` remains useful regardless: validating portability across runtimes is more valuable with more runtime options, not less. Beyond that, Lua 5.5 shipped in December 2025, and the version fragmentation will continue as PUC-Rio publishes new releases. The parser is designed to accept the syntactic union of Lua 5.1 through 5.5 with deliberate tolerance for future constructs, and new source versions are supported by adding transformer passes rather than rewriting the parser. valua targets the structural reality of Lua fragmentation, not a specific version gap.

---

## License

MIT. See [LICENSE](LICENSE).
