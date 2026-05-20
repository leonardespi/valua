

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

<p align="center">
  <a href="https://github.com/leonardespi/valua/actions/workflows/ci.yml">
    <img src="https://github.com/leonardespi/valua/actions/workflows/ci.yml/badge.svg" alt="CI">
  </a>
  <a href="https://crates.io/crates/valua-cli">
    <img src="https://img.shields.io/crates/v/valua-cli?label=valua-cli&color=%23e9573f" alt="valua-cli on crates.io">
  </a>
  <a href="https://crates.io/crates/valua-lint">
    <img src="https://img.shields.io/crates/v/valua-lint?label=valua-lint&color=%23e9573f" alt="valua-lint on crates.io">
  </a>
  <a href="https://docs.rs/valua-lint">
    <img src="https://img.shields.io/docsrs/valua-lint?label=docs.rs" alt="valua-lint docs">
  </a>
  <a href="https://deps.rs/repo/github/leonardespi/valua">
    <img src="https://deps.rs/repo/github/leonardespi/valua/status.svg" alt="dependency status">
  </a>
  <a href="https://github.com/leonardespi/valua/blob/main/LICENSE">
    <img src="https://img.shields.io/github/license/leonardespi/valua?color=blue" alt="License">
  </a>
  <a href="https://www.lua.org/manual/5.5/">
    <img src="https://img.shields.io/badge/lua-5.5-002040?logo=lua&logoColor=white" alt="Lua 5.5 source">
  </a>
  <a href="https://luajit.org/">
    <img src="https://img.shields.io/badge/target-LuaJIT-208888" alt="LuaJIT target">
  </a>
</p>

---

**Documentation**: [https://github.com/leonardespi/valua](https://github.com/leonardespi/valua)

**Source Code**: [https://github.com/leonardespi/valua](https://github.com/leonardespi/valua)

---

**valua** is a professional, fast transpiler engineered to resolve runtime fragmentation within the Lua ecosystem. It translates modern **Lua 5.5** source code into compatible **Lua 5.1** streams, designed for native high-velocity execution under **LuaJIT**.

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

### Translation Example

```lua
-- Input: Lua 5.5
local function process(mask, value)
    local result = value & mask    -- bitwise AND
    local half   = result // 2     -- integer floor division
    return half | 0xFF00           -- bitwise OR
end
```

```lua
-- Output: Lua 5.1 / LuaJIT  (valua build input.lua --target luajit)
local function process(mask, value)
    local result = bit.band(value, mask)
    local half = math.floor(result / 2)
    return bit.bor(half, 65280)
end
```

No polyfill injection when targeting LuaJIT — `bit.*` functions are native intrinsics compiled to single-cycle machine instructions. For the `--target lua51` target, a pure-Lua `bit` compatibility table is prepended automatically.

## Performance

Validated by [`trilateral-perf`](#run-the-benchmark) — a process-level harness that runs the same bitwise-intensive compute kernel across three environments and verifies JIT trace parity.

> **N = 100,000,000 iterations** · LuaJIT 2.1 · Lua 5.5.0 · 7 measured runs, 2 warmup

```
 A  Lua 5.1 native  (LuaJIT)            99 ms  ██
 B  Lua 5.5 → valua → LuaJIT            93 ms  ██  ← write modern Lua, pay nothing extra
 C  Lua 5.5 reference interpreter     1,085 ms  ██████████████████████
                                                    (1 block ≈ 50 ms)
```

valua-transpiled code runs **within measurement noise of handwritten Lua 5.1** and is **≈ 11× faster** than the Lua 5.5 reference interpreter on bitwise-heavy numeric paths. The transpiled source is byte-for-byte identical to what an expert would write by hand — because valua emits the same `bit.*` intrinsics that LuaJIT compiles to single-cycle machine instructions.

### JIT Trace Parity

The guarantee that matters: transpiled code must not trigger `NYI` bailouts that silently fall off the JIT fast path.

| Target | Traces Compiled | NYI Aborts | JIT Status |
|--------|----------------|------------|------------|
| A — Lua 5.1 native | 1 | 0 | ✓ Full JIT |
| B — valua transpiled | 1 | **0** | ✓ Full JIT |
| C — Lua 5.5 native | — | — | No JIT |

Zero NYI aborts. Identical trace count. The transpiled hot loop compiles to the exact same native machine code path as hand-written Lua 5.1.

### Run the Benchmark

```sh
# Quick smoke test (N = 10M, ~15 seconds total)
cargo run -p valua-bench --release --bin trilateral-perf -- --lua55 /path/to/lua5.5

# Full production run (stable ratios, ~3 minutes)
cargo run -p valua-bench --release --bin trilateral-perf -- \
  -n 100000000 -r 10 -w 3 \
  --lua55 /path/to/lua5.5 \
  -o ./bench_reports

# Emits: bench_reports/trilateral_perf_report.md
#        bench_reports/trilateral_perf_report.json
```

---

## Deterministic Rejection Criteria

Programs relying on features that break target execution invariants are strictly blocked during the validation phase. `valua` guarantees compile-time panics over silent semantic deviations.

| Error Code | Rejection Trigger | Remediation / Context |
|:---|:---|:---|
| `E0101` | Invocations of `math.type()` | Type reflection cannot differentiate integers from floats natively under LuaJIT's NaN-boxing strategy. |
| `E0102` | Explicit 64-bit integer overflow dependencies | Strict mathematical wrapping semantics cannot be replicated without heavy boxing overhead. |
| `E0301` | Mutation of a `<const>` binding | Detected modification of a compile-time immutable reference. |
| `E04xx` | Unrecognized downstream syntax constructs | Token sequences belonging to experimental features or newer specifications. |

### Diagnostic Output

Every rejection emits a precise span-annotated error. No silent failures, no stack traces — just the line, the token, and the fix.

```
$ valua check input.lua

error[E0101]: math.type observes the integer/float distinction absent in LuaJIT
  ┌─ input.lua:1:11
  │
1 │ local t = math.type(1)
  │           ^^^^^^^^^ math.type observes the integer/float distinction absent in LuaJIT
  │
  = note: math.type() has no equivalent in LuaJIT; all numbers are IEEE 754 doubles
  = help: Remove type discrimination or track numeric kind explicitly in user data
```

Run `valua explain <code>` (e.g. `valua explain E0101`) for full documentation, examples, and remediation guidance on any error code.

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

# Print full documentation for a diagnostic error code (case-insensitive)
valua explain E0101

```

> **Note:** `valua build` operates on single source files. Directory-wide compilation is not yet supported. For large codebases, invoke valua per-file via shell scripting:
> ```sh
> find src -name '*.lua' | xargs -I{} valua build {} --target luajit -o dist/{}.out
> ```

---

## Toolchain Development

### Prerequisites

* Rust Toolchain (v1.75+)
* `just` automation runner

```sh
# Install development task runner
cargo install just

# Install the pre-commit hook into .git/hooks/pre-commit
just install-hooks
# The hook runs `just check-fast` on every commit:
#   1. cargo fmt --all -- --check   (formatting, must be exact)
#   2. cargo clippy -- -D warnings  (all lints are errors)
#   3. cargo test --workspace       (full test suite)

# Run the complete check suite manually (fmt + clippy + test + release build)
just check

# Execute test suite targets exclusively
just test

# Target an individual verification test fixture
just test-one bitwise_and

```

---

## Ecosystem Positioning & Future-Proofing

**Q: What happens to `valua` if LuaJIT Remake (LJR) reaches production with Lua 5.4 support?**

**A:** `valua` remains useful. The design targets the structural reality of Lua's ecosystem fragmentation, not a single version gap:

1. **`valua-lint` stays relevant regardless of runtime:** Verifying cross-runtime portability becomes *more* important with more runtime options, not less. CI pipelines that catch LuaJIT-incompatible syntax at commit time are useful whether LJR exists or not.
2. **Lua 5.5 is already out; fragmentation continues:** LJR targeting Lua 5.4 doesn't unify the ecosystem. The parser is built to handle Lua 5.5+ source and future grammar extensions without core rewrites.
3. **Modular codegen enables new targets:** Adding Luau, Teal, or an LJR-native target is a new backend pass, not an architectural change. `valua` is not a bet on the 5.4↔5.1 gap specifically — it is infrastructure for Lua's permanently fragmented runtime landscape.

---

## License

Distributed under the terms of the MIT License. Review [LICENSE](./LICENSE) for absolute legal terms.


