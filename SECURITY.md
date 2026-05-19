# Security Policy

## Supported versions

Only the latest release on the `main` branch receives security fixes.
No backport policy exists before version 1.0.

| Version | Supported |
|---|---|
| `main` (pre-1.0) | ✓ |
| older branches | ✗ |

---

## What counts as a security issue in a compiler

valua is a source-to-source compiler, not a network service. The relevant
threat surface is different from web applications:

**In scope — report privately:**

| Class | Description |
|---|---|
| **Panic on untrusted input** | Any `lua` source file that causes `valua` to panic instead of emitting a `Result::Err`. In CI pipelines, an attacker-controlled `.lua` file could crash the build runner. All panics on user input are treated as P0 bugs. |
| **Silent incorrect code generation** | The compiler emits Lua 5.1 / LuaJIT output that is semantically different from the Lua 5.5 input, without producing a diagnostic. Silently wrong output that runs in a security-sensitive LuaJIT context is a correctness-security issue. |
| **Dependency vulnerabilities** | CVEs in crates that valua depends on (`logos`, `clap`, `codespan-reporting`, etc.) that affect the compilation pipeline or the compiled binary. |
| **Path traversal or arbitrary file write** | If a future CLI flag or API can be made to write output to an unintended path via crafted input. |

**Out of scope — open a public issue instead:**

- Wrong output that is also flagged by a diagnostic (the diagnostic is working as intended).
- Performance regressions or compilation slowdowns.
- Theoretical issues with no practical exploit path.
- Issues in LuaJIT itself or in the Lua 5.5 runtime.
- Lua programs that produce incorrect results at *runtime* due to intentional
  out-of-domain constructs (those are documented limitations, not bugs).

---

## Reporting a vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Send a private report to:

**Leonardo Espinosa** — [touchmelenny@gmail.com](mailto:touchmelenny@gmail.com)

Include in your report:

1. A minimal reproducer: the smallest `.lua` input that triggers the issue.
2. The valua version or commit SHA you tested against (`valua version` or `git rev-parse HEAD`).
3. The observed behavior (panic message, wrong output, etc.).
4. The expected behavior.
5. Your assessment of impact and exploitability.

PGP encryption is not required but is accepted if you need it — request the
public key in your first email.

---

## Response timeline

| Step | Target time |
|---|---|
| Acknowledgement of report | 48 hours |
| Initial triage (in-scope / out-of-scope) | 5 business days |
| Fix committed to a private branch | Depends on severity; P0 panics within 7 days |
| Public disclosure | After fix is released, coordinated with the reporter |

Pre-1.0 the team is small. Timelines are best-effort.

---

## Severity classification

| Label | Criteria |
|---|---|
| **P0** | Panic on any well-formed Lua 5.5 input; silent wrong code generation in security-sensitive context |
| **P1** | Panic on pathological but constructible input; dependency CVE with CVSS ≥ 7.0 |
| **P2** | Dependency CVE with CVSS < 7.0; non-panicking incorrect output with diagnostic present |

---

## Dependency auditing

Run `just audit` (requires `cargo-audit`) to check all dependencies against
the RustSec advisory database. This is part of the recommended pre-release
checklist (`just release-check`).

```sh
just audit
```
