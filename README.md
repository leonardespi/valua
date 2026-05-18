

<p align="center">
  <a href="https://github.com/leonardespi/valua">
    <picture>
      <img alt="valua logo" src="https://raw.githubusercontent.com/leonardespi/valua/main/docs/logos/valua_logo.png" width="450">
    </picture>
  </a>
</p>

<p align="center">
    <i>A modern, high-performance Lua 5.5 to LuaJIT compiler written in Rust.</i>
</p>

<a href="https://crates.io/crates/valua">
    <img src="https://img.shields.io/crates/v/valua?color=%23e9573f&label=crates.io" alt="Crates.io version">
</a>

<!--
<p align="center">
<a href="https://github.com/leonardespi/valua/actions/workflows/test.yml">
    <img src="https://github.com/leonardespi/valua/actions/workflows/test.yml/badge.svg" alt="Test Status">
</a>
-->
<a href="https://github.com/leonardespi/valua/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/leonardespi/valua?color=blue" alt="License">
</a>

<a href="https://www.lua.org/manual/5.5/">
    <img src="https://img.shields.io/badge/lua-5.5-002040?logo=lua&logoColor=white" alt="Supported Lua Source Version">
</a>
<a href="https://luajit.org/">
    <img src="https://img.shields.io/badge/target-LuaJIT-208888" alt="Compatible with LuaJIT">
</a>

</p>

---

**Documentation**:

**Source Code**: [https://github.com/leonardespi/valua](https://github.com/leonardespi/valua)

---

**valua** is a professionalanf fast transpiler engineered to resolve runtime fragmentation within the Lua ecosystem. It translates modern **Lua 5.5** source code into compatible **Lua 5.1** streams, designed for native high-velocity execution under **LuaJIT**.

### Key Architecture Features

* **Zero-Cost Performance Mapping**: Translates target-agnostic Abstract Syntax Trees (AST) directly into LuaJIT-friendly execution patterns, ensuring maximum JIT-compiler throughput and zero runtime overhead.
  
* **Memory-Safe Rust Core**: Uses Rust's strict memory safety guarantees and thread-safe parsing routines for a highly predictable and deterministic compilation pipeline.
* **Modern Syntax Bridging**: Delivers complete baseline support for Lua 5.5 semantics—including bitwise operators, compact array literals, and attributes—to runtimes historically restricted to 5.1 constraints.
* **Fail-Fast Semantic Guarantees**: Eliminates silent runtime compilation issues by implementing a strict ahead-of-time validation pass that rejects borderline lexical ambiguities with explicit error diagnostics.
* **Decoupled Static Analysis**: Ships with `valua-lint` as an autonomous public crate, enabling plug-and-play integration with continuous integration (CI) workflows and editor language servers without committing to full code emission.

---

## Translation Matrix

`valua` maps modern syntax primitives into optimized, standard LuaJIT idioms:

| Source Construct (Lua 5.5) | Compilation Output Strategy | Performance Profile |
|:---|:---|:---|
| Bitwise Operators (`&`, `\|`, `~`, `<<`, `>>`) | Emits explicit `bit.band`, `bit.bor`, `bit.bxor`, `bit.lshift`, `bit.rshift` calls | **Native** — LuaJIT compiles `bit.*` functions directly into single-cycle machine instructions. |
| Integer Division (`//`) | Lowered to a JIT-optimized `math.floor(a / b)` pattern | **Native** |
| `<const>` Attribute | Validated statically at compile time; emitted as a standard `local` variable | **Native** |
| `<close>` Attribute | Scoped `pcall` wrapper invoking the `__close` metamethod on deterministic exit | **Negligible** — Minor structural overhead optimized for I/O-bound resource lifecycles. |
| Compact Array Literals | Passthrough optimization — semantic translation is uniform between versions | **Native** |
| `<global>` Declarations | Compiled directly into explicit global context `_G` dictionary assignments | **Native** |

## Deterministic Rejection Criteria

Programs relying on features that break target execution invariants are strictly blocked during the validation phase. `valua` guarantees compile-time panics over silent semantic deviations.

| Error Code | Rejection Trigger | Remediation / Context |
|:---|:---|:---|
| `E0101` | Invocations of `math.type()` | Type reflection cannot differentiate integers from floats natively under LuaJIT's NaN-boxing strategy. |
| `E0102` | Explicit 64-bit integer overflow dependencies | Strict mathematical wrapping semantics cannot be replicated without heavy boxing overhead. |
| `E0301` | Mutation of a `<const>` binding | Detected modification of a compile-time immutable reference. |
| `E04xx` | Unrecognized downstream syntax constructs | Token sequences belonging to experimental features or newer specifications. |

---

## Technical Constraints & Design Philosophy

### The Impossibility of Total Transpilation

A recurring challenge when bridging modern Lua specifications down to LuaJIT is the underlying data structure layout: LuaJIT unifies all numbers under standard IEEE 754 double-precision floats, whereas Lua 5.3+ introduces an explicit, separate 64-bit integer type subtype. 

Enforcing perfect runtime arithmetic mirroring across this barrier requires wrapping every mathematical operator in a heap-allocated emulation layer. Benchmarks indicate this degrades raw performance by factors between **20x and 100x**, fundamentally defeating the purpose of targeting LuaJIT. Statically identifying whether a script genuinely depends on strict integer overflow properties reduces directly to the Halting Problem.

**valua's architectural response is deterministic boundary definition:** What you compile is exactly what executes. By shifting verification from a complex runtime emulation layer to a predictable static compiler barrier, you receive consistent execution speeds with zero performance penalties.

---

## Installation

*Note: valua is under active pre-production development. The stable binary installation steps below apply to versions 1.0.0 and above.*

```sh
cargo install valua-cli

```

Pre-compiled binary distributions targeting standard platform triplets (`linux-x86_64`, `linux-aarch64`, `macos-arm64`, `windows-x86_64`) are distributed continuously via GitHub Releases.

---

## Command Line Interface

```sh
# Transpile a source file to optimized LuaJIT-compatible Lua 5.1 code
valua build input.lua -o output.lua --target luajit

# Transpile to vanilla Lua 5.1 (includes standard bitwise polyfills)
valua build input.lua -o output.lua --target lua51

# Validate syntax structure across the pipeline without writing to disk
valua check input.lua

# Execute static analysis rules exclusively (zero generation overhead)
valua lint input.lua --target luajit

# Output current version descriptor
valua version

```

---

## Toolchain Development

### Prerequisites

* Rust Toolchain (v1.75+)
* `just` automation runner

```sh
# Install development task runner
cargo install just

# Configure continuous integration pre-commit verification hooks
just install-hooks

# Run the complete test and formatting suite (Mandatory before opening PRs)
just check

# Execute test suite targets exclusively
just test

# Target an individual verification test fixture
just test-one bitwise_and

```

---

## Ecosystem Positioning & Future-Proofing

A common architectural inquiry is how `valua` positions itself if alternative downstream runtime initiatives reach production feature parity with modern versions of the upstream PUC-Rio language specification.

The design of `valua` targets the structural reality of ecosystem fragmentation rather than a static version gap:

1. **Runtimes Diversify, Fragmentation Remains:** Even as alternative runtimes advance, version drift remains an inherent trait of the ecosystem. `valua` decouples development velocity from targeted execution platforms.
2. **Value of Isolated Static Analysis:** The modular design of `valua-lint` remains highly valuable regardless of runtime environment configurations; verifying deterministic portability and enforcing syntactic consistency inside CI infrastructures is critical to enterprise scale.
3. **Extensible Pipeline Layout:** The architecture is explicitly non-monolithic. The parser engine accepts the broad syntactic union of modern versions (up to Lua 5.5+) with engineered tolerance for future grammar adjustments. Introducing support for incoming language syntax variations is achieved by layering downstream AST transformer passes rather than engineering core parser rewrites.

---

## License

Distributed under the terms of the MIT License. Review [LICENSE](https://www.google.com/search?q=LICENSE) for absolute legal terms.


