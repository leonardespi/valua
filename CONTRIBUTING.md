# Contributing to valua

Thanks for your interest. This document covers everything needed to submit a
correct, reviewable contribution.

---

## Prerequisites

| Tool | Version | How to install |
|---|---|---|
| Rust toolchain | ≥ 1.75 (pinned via `rust-toolchain.toml`) | `rustup` auto-installs the pinned version |
| `just` | any recent | `cargo install just` |
| `rustfmt` + `clippy` | bundled by rustup | installed automatically |

Optional but useful:

- `cargo-audit` — `cargo install cargo-audit` — used by `just audit`
- `cargo-llvm-cov` — `cargo install cargo-llvm-cov` — used by `just coverage`

---

## Development setup

```sh
git clone https://github.com/leonardespi/valua
cd valua
just install-hooks   # installs pre-commit hook that runs just check-fast
cargo build          # verify the workspace compiles
cargo test           # run the test suite
```

---

## The one rule before every commit

```sh
just check
```

This runs `fmt-check`, `clippy`, `test`, and `build` in order. If it does not
pass, the commit is not ready. The pre-commit hook installed by
`just install-hooks` enforces this automatically.

During active development `just check-fast` (skips the release build) is
faster for inner-loop iteration.

---

## Opening a pull request

1. Fork the repository and create a branch from `main`.
2. Make your changes. Keep each PR to a single concern.
3. Run `just check` — it must pass cleanly.
4. Open the PR against `main`. Fill in the description with *why*, not *what*.
5. Reference PRD sections in the description when relevant (e.g. `Refs PRD §6.2`).

PRs that fail `just check` will not be reviewed.

---

## Commit message format

```
<area>: <short imperative summary>

<optional body explaining why, not what>

<optional footer, e.g. "Refs PRD §6.2">
```

Valid areas: `parser`, `lexer`, `lint`, `transformer`, `codegen`, `cli`,
`core`, `ast`, `diagnostics`, `polyfills`, `docs`, `ci`, `tests`.

Examples:

```
transformer: implement IntegerDivisionTransform

Rewrites BinaryOp::IDiv to math.floor(a / b). LuaJIT JIT-compiles this
to a ROUNDSD + float-div pair — native throughput, no wrappers.

Refs PRD §5.1
```

```
lint: add E0101 for math.type reflection

Detects calls to math.type and emits E0101 with a remediation suggestion.
Per the impossibility proof (Anexo A), this is fundamentally outside the
native target.

Refs PRD §5.2, §6.2
```

---

## Adding a transform pass

Follow this checklist in order. Work test-first.

1. **Write the fixture.** Create `tests/fixtures/<feature_name>/input.lua` and
   `tests/fixtures/<feature_name>/expected.lua`. The expected file is what the
   transpiler must emit.
2. **Wire the integration test.** Add a test case in `tests/integration_tests.rs`
   that loads the fixture pair, runs `Compiler::compile`, and asserts equality.
3. **Confirm the test fails** with `just test-one <feature_name>`. If it passes
   already, the fixture tests nothing.
4. **Implement the pass** in `crates/valua-transformer/src/lib.rs` (or a
   submodule). The pass must implement the `Transform` trait.
5. **Register the pass** in `TransformPipeline::default()` at the correct
   position. Pass order matters: detection before rewriting, rewriting before
   injection.
6. **Confirm the test passes** with `just test-one <feature_name>`.
7. **Run `just check`** to verify nothing else broke.

Do not skip step 1. Defining the expected output first forces a precise
definition of "correct" before any code is written.

---

## Adding an error code

Error codes are stable API contracts once released. Adding one is deliberate.

1. Pick a code in the correct category:

   | Prefix | Category |
   |---|---|
   | `E00xx` | Lexer errors |
   | `E01xx` | Domain errors (program outside `L_5.5^native`) |
   | `E02xx` | Parse errors |
   | `E03xx` | Static validation errors (`<const>` reassignment, etc.) |
   | `E04xx` | Recognised syntax from an unsupported Lua version |
   | `W01xx` | Portability warnings (non-blocking) |

2. Add a fixture under `tests/fixtures/errors/<code>/input.lua` and an
   `expected.txt` with the rendered diagnostic output.
3. Implement the detection, usually in `valua-lint`.
4. Document the code in `docs/errors/<code>.md`: description, triggering
   example, justification, remediation.

Every error code in the codebase must have a fixture. Codes without fixtures
will be rejected in review.

---

## Code style

These rules are enforced by tooling (`cargo fmt`, `cargo clippy`) or by review:

- **No `unwrap()` or `expect()` in library code.** Use `?` with a typed error.
  `unwrap()` is acceptable only in tests and behind `unreachable!()`.
- **No `panic!()` on user input.** A panic on valid Lua source is a P0 bug.
  Errors are values; return them.
- **Every AST node carries its `Span`.** Do not lose spans during
  transformation — diagnostics depend on them.
- **Errors are typed enums, not strings.** Use `thiserror` in library crates
  and `anyhow` in `valua-cli` only.
- **No global state.** No `static mut`, no mutable `lazy_static`. The compiler
  must be reentrant.
- **Determinism is required.** Same input + same version = byte-identical
  output across runs. No `HashMap` iteration in output-producing code.
- **Comments answer "why", not "what".** Write a comment only when the
  constraint, invariant, or workaround would surprise a future reader.
  Never describe what the code already says.
- **`pub(crate)` by default.** Promote to `pub` only for intentional API surface.

---

## Running specific tests

```sh
# Full suite
just test

# Single crate
just test-crate valua-parser

# Single test by name
just test-one bitwise_and

# Integration tests only
just test-integration

# With output (debugging)
just test-verbose
```

---

## Where decisions live

| Source | Use for |
|---|---|
| `docs/PRD.md` | Product and architecture decisions. Cite section numbers in PRs. |
| `docs/decisions/` | Architecture Decision Records for choices not in the PRD. Add a new ADR before making a non-trivial architectural change. |
| `CLAUDE.md` | Workflow and style rules for this repository. |
| `crates/valua-*/src/lib.rs` | Crate-level rustdoc explains each module's invariants. |

When something is undocumented: decide, implement, and add an ADR. The next
contributor should not face the same ambiguity twice.
