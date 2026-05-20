//! Trilateral performance benchmarking harness.
//!
//! Compares execution latency and JIT trace quality across three configurations:
//!
//! - **Target A** — Handwritten idiomatic Lua 5.1, run via `luajit`.
//! - **Target B** — Lua 5.5 transpiled by `valua` → Lua 5.1, run via `luajit`.
//! - **Target C** — Raw Lua 5.5, run via the reference `lua5.5` interpreter.
//!
//! # Usage
//!
//! ```text
//! cargo run -p valua-bench --bin trilateral-perf -- [OPTIONS]
//!
//! Options:
//!   -n, --iterations N   Hot-loop iterations       [default: 10_000_000]
//!   -r, --runs R         Timed measurement runs    [default: 7]
//!   -w, --warmup W       Warmup runs (discarded)   [default: 2]
//!   --luajit PATH        LuaJIT interpreter        [default: luajit]
//!   --lua55 PATH         Lua 5.5 interpreter       [default: lua5.5]
//!   -o, --output DIR     Report output directory   [default: .]
//!   -h, --help
//! ```

#![allow(clippy::cast_precision_loss)]

use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

use valua_core::{CompileOptions, Compiler};

// ── Constants ─────────────────────────────────────────────────────────────────

const DEFAULT_N: u64 = 10_000_000;
const DEFAULT_WARMUP: usize = 2;
const DEFAULT_RUNS: usize = 7;

// ── Lua source generators ─────────────────────────────────────────────────────

/// Target A: handwritten idiomatic Lua 5.1 for LuaJIT.
///
/// Uses `bit.*` globals (built-in to LuaJIT 2.x, no `require` needed) and
/// `math.floor` for integer division — the same form valua emits for the
/// LuaJIT target.  This is deliberately identical to what valua produces so
/// that Target B's transpiled output can be compared byte-for-byte.
fn lua51_source(n: u64) -> String {
    format!(
        "local N = {n}\n\
         local acc = 0\n\
         for i = 1, N do\n\
         \x20   acc = bit.band(bit.bxor(acc, bit.band(i, 255)), 2147483647)\n\
         \x20   acc = bit.band(bit.bor(acc, math.floor(i / 3)), 2147483647)\n\
         end\n\
         print(acc)\n"
    )
}

/// Target B/C source: Lua 5.5 with native bitwise operators and floor division.
///
/// This is the input fed to `Compiler::compile` to produce Target B, and also
/// the script handed directly to `lua5.5` for Target C.
fn lua55_source(n: u64) -> String {
    format!(
        "local N = {n}\n\
         local acc = 0\n\
         for i = 1, N do\n\
         \x20   acc = (acc ~ (i & 0xFF)) & 0x7FFFFFFF\n\
         \x20   acc = (acc | (i // 3)) & 0x7FFFFFFF\n\
         end\n\
         print(acc)\n"
    )
}

// ── Process execution ─────────────────────────────────────────────────────────

struct RunResult {
    elapsed_us: u64,
    stdout: String,
    stderr: String,
    success: bool,
}

fn run_script(interpreter: &str, script: &Path, extra_flags: &[&str]) -> RunResult {
    let start = Instant::now();
    let output = Command::new(interpreter)
        .args(extra_flags)
        .arg(script)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn `{interpreter}`: {e}"));
    let elapsed_us = start.elapsed().as_micros() as u64;

    RunResult {
        elapsed_us,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        success: output.status.success(),
    }
}

// ── Measurement ───────────────────────────────────────────────────────────────

struct TargetResult {
    id: char,
    label: String,
    interpreter: String,
    samples_us: Vec<u64>,
    stdout_sample: String,
    jit_traces: Option<usize>,
    nyi_count: Option<usize>,
}

impl TargetResult {
    fn median_us(&self) -> u64 {
        let mut s = self.samples_us.clone();
        s.sort_unstable();
        s[s.len() / 2]
    }

    fn p95_us(&self) -> u64 {
        let mut s = self.samples_us.clone();
        s.sort_unstable();
        let idx = ((s.len() * 95) / 100).min(s.len() - 1);
        s[idx]
    }

    fn mean_us(&self) -> f64 {
        self.samples_us.iter().sum::<u64>() as f64 / self.samples_us.len() as f64
    }
}

fn measure(
    id: char,
    label: &str,
    interpreter: &str,
    script: &Path,
    warmup: usize,
    runs: usize,
    jit_verbose: bool,
) -> TargetResult {
    let interp_name = interpreter.rsplit('/').next().unwrap_or(interpreter);

    eprintln!("[{id}] Warmup ({warmup} run(s))…");
    for _ in 0..warmup {
        run_script(interpreter, script, &[]);
    }

    eprintln!("[{id}] Measuring ({runs} run(s))…");
    let mut samples_us = Vec::with_capacity(runs);
    let mut last_stdout = String::new();

    for r in 1..=runs {
        let res = run_script(interpreter, script, &[]);
        if !res.success {
            eprintln!("[{id}] Warning: run {r} exited non-zero");
            eprintln!("      stderr: {}", res.stderr.trim());
        }
        samples_us.push(res.elapsed_us);
        last_stdout = res.stdout;
        eprintln!("[{id}] Run {r}/{runs}: {} µs", res.elapsed_us);
    }

    let (jit_traces, nyi_count) = if jit_verbose {
        eprintln!("[{id}] JIT trace analysis (-jv)…");
        let jv = run_script(interpreter, script, &["-jv"]);
        let traces = jv.stderr.lines().filter(|l| l.contains("[TRACE")).count();
        let nyis = jv.stderr.lines().filter(|l| l.contains("NYI")).count();
        eprintln!("[{id}] Traces: {traces}, NYI aborts: {nyis}");
        (Some(traces), Some(nyis))
    } else {
        (None, None)
    };

    TargetResult {
        id,
        label: label.to_string(),
        interpreter: interp_name.to_string(),
        samples_us,
        stdout_sample: last_stdout.trim().to_string(),
        jit_traces,
        nyi_count,
    }
}

// ── Formatting helpers ────────────────────────────────────────────────────────

fn fmt_int(n: u64) -> String {
    let s = n.to_string();
    let mut rev = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            rev.push(',');
        }
        rev.push(c);
    }
    rev.chars().rev().collect()
}

fn speedup(slower_us: u64, faster_us: u64) -> f64 {
    if faster_us == 0 {
        return 0.0;
    }
    slower_us as f64 / faster_us as f64
}

fn pct_diff(baseline_us: u64, measured_us: u64) -> f64 {
    if baseline_us == 0 {
        return 0.0;
    }
    (measured_us as f64 - baseline_us as f64) / baseline_us as f64 * 100.0
}

fn current_datetime() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string())
}

fn opt_str(v: Option<usize>) -> String {
    v.map_or_else(|| "N/A".to_string(), |n| n.to_string())
}

// ── Markdown report ───────────────────────────────────────────────────────────

fn generate_markdown(
    results: &[TargetResult],
    iterations: u64,
    warmup: usize,
    runs: usize,
    transpiled: &str,
) -> String {
    let a = &results[0];
    let b = &results[1];
    let c = &results[2];

    let a_med = a.median_us();
    let b_med = b.median_us();
    let c_med = c.median_us();

    let b_vs_a_pct = pct_diff(a_med, b_med);
    let a_vs_c = speedup(c_med, a_med);
    let b_vs_c = speedup(c_med, b_med);

    let within_2pct = b_vs_a_pct.abs() <= 2.0;
    let ac_20x = a_vs_c >= 20.0;
    let bc_20x = b_vs_c >= 20.0;

    let jit_ok = match (a.nyi_count, b.nyi_count) {
        (Some(a_nyi), Some(b_nyi)) => b_nyi <= a_nyi,
        _ => false,
    };

    let output_agree =
        a.stdout_sample == b.stdout_sample && b.stdout_sample == c.stdout_sample;

    let mut md = String::new();

    writeln!(md, "# Valua Trilateral Performance Report").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "| Property | Value |").unwrap();
    writeln!(md, "|----------|-------|").unwrap();
    writeln!(md, "| Date | {} |", current_datetime()).unwrap();
    writeln!(md, "| Iterations (N) | {} |", fmt_int(iterations)).unwrap();
    writeln!(md, "| Warmup runs | {warmup} |").unwrap();
    writeln!(md, "| Measurement runs | {runs} |").unwrap();
    writeln!(md).unwrap();

    writeln!(md, "## Targets").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "| ID | Description | Interpreter |").unwrap();
    writeln!(md, "|----|-------------|-------------|").unwrap();
    for r in results {
        writeln!(md, "| {} | {} | `{}` |", r.id, r.label, r.interpreter).unwrap();
    }
    writeln!(md).unwrap();

    writeln!(md, "## Timing Results").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "| Target | Median (µs) | P95 (µs) | Mean (µs) | Samples (µs) |").unwrap();
    writeln!(md, "|--------|-------------|----------|-----------|--------------|").unwrap();
    for r in results {
        let samples: Vec<String> = r.samples_us.iter().map(|v| v.to_string()).collect();
        writeln!(
            md,
            "| {} | {} | {} | {:.0} | {} |",
            r.id,
            fmt_int(r.median_us()),
            fmt_int(r.p95_us()),
            r.mean_us(),
            samples.join(", ")
        )
        .unwrap();
    }
    writeln!(md).unwrap();

    writeln!(md, "## JIT Trace Analysis").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "| Target | Traces Compiled | NYI Aborts |").unwrap();
    writeln!(md, "|--------|----------------|------------|").unwrap();
    for r in results {
        writeln!(
            md,
            "| {} | {} | {} |",
            r.id,
            opt_str(r.jit_traces),
            opt_str(r.nyi_count)
        )
        .unwrap();
    }
    writeln!(md).unwrap();

    writeln!(md, "## Analysis").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "| Metric | Value | Threshold | Status |").unwrap();
    writeln!(md, "|--------|-------|-----------|--------|").unwrap();
    writeln!(
        md,
        "| B vs A latency delta | {b_vs_a_pct:+.2}% | ≤ ±2% | {} |",
        if within_2pct { "PASS" } else { "FAIL" }
    )
    .unwrap();
    writeln!(
        md,
        "| A vs C speedup | {a_vs_c:.1}x | ≥ 20x | {} |",
        if ac_20x { "PASS" } else { "FAIL" }
    )
    .unwrap();
    writeln!(
        md,
        "| B vs C speedup | {b_vs_c:.1}x | ≥ 20x | {} |",
        if bc_20x { "PASS" } else { "FAIL" }
    )
    .unwrap();
    writeln!(
        md,
        "| JIT parity (B NYI ≤ A NYI) | A={}, B={} | 0 regressions | {} |",
        opt_str(a.nyi_count),
        opt_str(b.nyi_count),
        if jit_ok { "PASS" } else { "FAIL" }
    )
    .unwrap();
    writeln!(md).unwrap();

    writeln!(md, "## Output Verification").unwrap();
    writeln!(md).unwrap();
    if output_agree {
        writeln!(md, "All three targets agree: `{}`", a.stdout_sample).unwrap();
    } else {
        writeln!(md, "**DIVERGENT OUTPUTS**").unwrap();
        for r in results {
            writeln!(md, "- {}: `{}`", r.id, r.stdout_sample).unwrap();
        }
    }
    writeln!(md).unwrap();

    writeln!(md, "## Transpiled Source (Target B)").unwrap();
    writeln!(md).unwrap();
    writeln!(md, "```lua").unwrap();
    writeln!(md, "{}", transpiled.trim()).unwrap();
    writeln!(md, "```").unwrap();

    md
}

// ── JSON report ───────────────────────────────────────────────────────────────

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn generate_json(
    results: &[TargetResult],
    iterations: u64,
    warmup: usize,
    runs: usize,
    transpiled: &str,
) -> String {
    let a = &results[0];
    let b = &results[1];
    let c = &results[2];

    let a_med = a.median_us();
    let b_med = b.median_us();
    let c_med = c.median_us();

    let mut j = String::new();
    writeln!(j, "{{").unwrap();
    writeln!(j, r#"  "generated": "{}","#, current_datetime()).unwrap();
    writeln!(j, r#"  "config": {{"#).unwrap();
    writeln!(j, r#"    "iterations": {iterations},"#).unwrap();
    writeln!(j, r#"    "warmup_runs": {warmup},"#).unwrap();
    writeln!(j, r#"    "measurement_runs": {runs}"#).unwrap();
    writeln!(j, r#"  }},"#).unwrap();
    writeln!(j, r#"  "targets": ["#).unwrap();

    for (i, r) in results.iter().enumerate() {
        let comma = if i + 1 < results.len() { "," } else { "" };
        let samples: Vec<String> = r.samples_us.iter().map(|v| v.to_string()).collect();
        writeln!(j, "    {{").unwrap();
        writeln!(j, r#"      "id": "{}","#, r.id).unwrap();
        writeln!(j, r#"      "label": "{}","#, json_escape(&r.label)).unwrap();
        writeln!(j, r#"      "interpreter": "{}","#, r.interpreter).unwrap();
        writeln!(j, r#"      "median_us": {},"#, r.median_us()).unwrap();
        writeln!(j, r#"      "p95_us": {},"#, r.p95_us()).unwrap();
        writeln!(j, r#"      "mean_us": {:.2},"#, r.mean_us()).unwrap();
        writeln!(j, r#"      "samples_us": [{}],"#, samples.join(", ")).unwrap();
        writeln!(j, r#"      "output": "{}","#, json_escape(&r.stdout_sample)).unwrap();
        let traces = r.jit_traces.map_or("null".to_string(), |v| v.to_string());
        let nyis = r.nyi_count.map_or("null".to_string(), |v| v.to_string());
        writeln!(j, r#"      "jit_traces": {traces},"#).unwrap();
        writeln!(j, r#"      "nyi_count": {nyis}"#).unwrap();
        writeln!(j, "    }}{comma}").unwrap();
    }

    writeln!(j, "  ],").unwrap();
    writeln!(j, r#"  "analysis": {{"#).unwrap();
    writeln!(
        j,
        r#"    "b_vs_a_pct_diff": {:.4},"#,
        pct_diff(a_med, b_med)
    )
    .unwrap();
    writeln!(j, r#"    "a_vs_c_speedup": {:.2},"#, speedup(c_med, a_med)).unwrap();
    writeln!(j, r#"    "b_vs_c_speedup": {:.2},"#, speedup(c_med, b_med)).unwrap();
    writeln!(j, r#"    "a_nyi": {},"#, opt_str(a.nyi_count)).unwrap();
    writeln!(j, r#"    "b_nyi": {}"#, opt_str(b.nyi_count)).unwrap();
    writeln!(j, r#"  }},"#).unwrap();
    writeln!(
        j,
        r#"  "transpiled_source": "{}""#,
        json_escape(transpiled.trim())
    )
    .unwrap();
    writeln!(j, "}}").unwrap();

    j
}

// ── CLI ───────────────────────────────────────────────────────────────────────

struct Config {
    iterations: u64,
    warmup: usize,
    runs: usize,
    luajit: String,
    lua55: String,
    output_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            iterations: DEFAULT_N,
            warmup: DEFAULT_WARMUP,
            runs: DEFAULT_RUNS,
            luajit: "luajit".to_string(),
            lua55: probe_lua55(),
            output_dir: PathBuf::from("."),
        }
    }
}

fn probe_lua55() -> String {
    for candidate in &["lua5.5", "lua-5.5"] {
        if Command::new("which")
            .arg(candidate)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return (*candidate).to_string();
        }
    }
    "lua5.5".to_string()
}

fn parse_args() -> Config {
    let mut cfg = Config::default();
    let args: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-n" | "--iterations" => {
                i += 1;
                cfg.iterations = args[i].parse().expect("--iterations: expected integer");
            }
            "-r" | "--runs" => {
                i += 1;
                cfg.runs = args[i].parse().expect("--runs: expected integer");
            }
            "-w" | "--warmup" => {
                i += 1;
                cfg.warmup = args[i].parse().expect("--warmup: expected integer");
            }
            "--luajit" => {
                i += 1;
                cfg.luajit = args[i].clone();
            }
            "--lua55" => {
                i += 1;
                cfg.lua55 = args[i].clone();
            }
            "-o" | "--output" => {
                i += 1;
                cfg.output_dir = PathBuf::from(&args[i]);
            }
            "-h" | "--help" => {
                print!(
                    "trilateral-perf — valua benchmark harness\n\
                     \n\
                     USAGE: trilateral-perf [OPTIONS]\n\
                     \n\
                     OPTIONS:\n\
                     \x20 -n, --iterations N   Hot-loop iterations     [default: {DEFAULT_N}]\n\
                     \x20 -r, --runs R         Measurement runs        [default: {DEFAULT_RUNS}]\n\
                     \x20 -w, --warmup W       Warmup runs (discarded) [default: {DEFAULT_WARMUP}]\n\
                     \x20     --luajit PATH    LuaJIT interpreter      [default: luajit]\n\
                     \x20     --lua55  PATH    Lua 5.5 interpreter     [default: lua5.5]\n\
                     \x20 -o, --output DIR     Output directory        [default: .]\n\
                     \x20 -h, --help\n"
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("Unknown argument: {other}");
                std::process::exit(1);
            }
        }
        i += 1;
    }
    cfg
}

fn check_interpreter(interp: &str, label: &str) {
    let ok = Command::new(interp)
        .arg("-v")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        eprintln!("Error: {label} interpreter `{interp}` not found or failed.");
        eprintln!("       Pass a valid path with --luajit / --lua55.");
        std::process::exit(1);
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() {
    let cfg = parse_args();

    eprintln!("=== Valua Trilateral Performance Benchmark ===");
    eprintln!("Iterations  : {}", fmt_int(cfg.iterations));
    eprintln!("Warmup runs : {}", cfg.warmup);
    eprintln!("Measure runs: {}", cfg.runs);
    eprintln!("LuaJIT      : {}", cfg.luajit);
    eprintln!("Lua 5.5     : {}", cfg.lua55);
    eprintln!();

    check_interpreter(&cfg.luajit, "LuaJIT");
    check_interpreter(&cfg.lua55, "Lua 5.5");

    // Generate sources
    let src_a = lua51_source(cfg.iterations);
    let src_bc = lua55_source(cfg.iterations);

    eprintln!("Transpiling Lua 5.5 source via valua (→ LuaJIT target)…");
    let src_b = Compiler::compile(&src_bc, CompileOptions::luajit())
        .unwrap_or_else(|e| panic!("valua compile failed: {e:?}"));
    eprintln!("Transpilation OK ({} bytes output).", src_b.len());
    eprintln!();

    // Write temp files
    let tmp = std::env::temp_dir();
    let path_a = tmp.join(format!("valua_bench_a_{}.lua", std::process::id()));
    let path_b = tmp.join(format!("valua_bench_b_{}.lua", std::process::id()));
    let path_c = tmp.join(format!("valua_bench_c_{}.lua", std::process::id()));

    fs::write(&path_a, &src_a).expect("write target A temp file");
    fs::write(&path_b, &src_b).expect("write target B temp file");
    fs::write(&path_c, &src_bc).expect("write target C temp file");

    // Run benchmarks
    eprintln!("--- Target A: Lua 5.1 native (LuaJIT) ---");
    let result_a = measure(
        'A',
        "Lua 5.1 native — handwritten bit.* calls via LuaJIT",
        &cfg.luajit,
        &path_a,
        cfg.warmup,
        cfg.runs,
        true,
    );

    eprintln!();
    eprintln!("--- Target B: valua-transpiled Lua 5.5 → 5.1 (LuaJIT) ---");
    let result_b = measure(
        'B',
        "Lua 5.5 transpiled by valua → Lua 5.1 via LuaJIT",
        &cfg.luajit,
        &path_b,
        cfg.warmup,
        cfg.runs,
        true,
    );

    eprintln!();
    eprintln!("--- Target C: Lua 5.5 native (reference interpreter) ---");
    let result_c = measure(
        'C',
        "Lua 5.5 native — reference interpreter (no JIT)",
        &cfg.lua55,
        &path_c,
        cfg.warmup,
        cfg.runs,
        false,
    );

    // Cleanup
    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);
    let _ = fs::remove_file(&path_c);

    // Generate reports
    let results = vec![result_a, result_b, result_c];
    let markdown = generate_markdown(&results, cfg.iterations, cfg.warmup, cfg.runs, &src_b);
    let json = generate_json(&results, cfg.iterations, cfg.warmup, cfg.runs, &src_b);

    fs::create_dir_all(&cfg.output_dir).expect("create output directory");
    let md_path = cfg.output_dir.join("trilateral_perf_report.md");
    let json_path = cfg.output_dir.join("trilateral_perf_report.json");
    fs::write(&md_path, &markdown).expect("write markdown report");
    fs::write(&json_path, &json).expect("write JSON report");

    // Print summary
    println!("{markdown}");
    eprintln!("Reports written:");
    eprintln!("  {}", md_path.display());
    eprintln!("  {}", json_path.display());
}
