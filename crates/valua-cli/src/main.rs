use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use tracing::info;
use valua_core::{CompileOptions, Compiler, Target};
use valua_diagnostics::{ConsoleReporter, Reporter};
use valua_lint::LintPipeline;

/// valua — Lua 5.5 → 5.1 / LuaJIT transpiler.
#[derive(Debug, Parser)]
#[command(name = "valua", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Transpile a Lua 5.5 source file to Lua 5.1 or LuaJIT.
    Build {
        /// Input Lua 5.5 source file.
        input: PathBuf,

        /// Output file path (defaults to stdout when omitted).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target Lua runtime.
        #[arg(long, default_value = "lua51")]
        target: TargetArg,
    },

    /// Validate a Lua 5.5 source file without producing output.
    Check {
        /// Input Lua 5.5 source file.
        input: PathBuf,
    },

    /// Run static analysis on a Lua 5.5 source file.
    Lint {
        /// Input Lua 5.5 source file.
        input: PathBuf,

        /// Target Lua runtime for target-specific checks.
        #[arg(long, default_value = "luajit")]
        target: TargetArg,
    },

    /// Print version information.
    Version,
}

/// CLI representation of the target Lua runtime.
#[derive(Debug, Clone, clap::ValueEnum)]
enum TargetArg {
    Lua51,
    Luajit,
}

impl From<TargetArg> for Target {
    fn from(t: TargetArg) -> Self {
        match t {
            TargetArg::Lua51 => Target::Lua51,
            TargetArg::Luajit => Target::LuaJIT,
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Command::Build {
            input,
            output,
            target,
        } => cmd_build(input, output, target),
        Command::Check { input } => cmd_check(input),
        Command::Lint { input, target } => cmd_lint(input, target),
        Command::Version => cmd_version(),
    }
}

fn cmd_build(input: PathBuf, output: Option<PathBuf>, target: TargetArg) -> Result<()> {
    use std::io::Write as _;

    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("failed to read '{}'", input.display()))?;

    info!(input = %input.display(), target = ?target, "transpiling");

    let opts = CompileOptions {
        target: target.into(),
        ..CompileOptions::default()
    };

    let lua = Compiler::compile(&source, opts)
        .with_context(|| format!("compilation failed: '{}'", input.display()))?;

    match output {
        Some(ref path) => {
            let bytes = lua.len();
            atomic_write(path, &lua)?;
            println!("{}: {bytes} bytes written", path.display());
        }
        None => {
            // No output path — write transpiled source to stdout for pipeline use.
            let stdout = std::io::stdout();
            let mut out = std::io::BufWriter::new(stdout.lock());
            out.write_all(lua.as_bytes()).context("failed to write to stdout")?;
        }
    }

    Ok(())
}

/// Write `content` to `path` atomically by staging through a sibling temp file.
///
/// Creates `.{stem}.{pid}.tmp` in the same directory as `path`, writes
/// `content` via a buffered writer, flushes, then renames onto `path`.
/// `rename(2)` is atomic on POSIX when src and dst are on the same filesystem;
/// on Windows (Vista+) it also overwrites atomically.
/// The temp file is removed on write failure to avoid leaving stale files.
fn atomic_write(path: &Path, content: &str) -> Result<()> {
    use std::io::Write as _;

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "_".to_string());
    let tmp = path.with_file_name(format!(".{stem}.{pid}.tmp", pid = std::process::id()));

    let write_result = (|| -> std::io::Result<()> {
        let mut w = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
        w.write_all(content.as_bytes())?;
        w.flush()
    })();

    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp); // best-effort; ignore secondary error
        return Err(e).with_context(|| format!("failed to write '{}'", path.display()));
    }

    std::fs::rename(&tmp, path)
        .with_context(|| format!("failed to finalize '{}'", path.display()))
}

fn cmd_check(input: PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("failed to read '{}'", input.display()))?;

    info!(input = %input.display(), "checking syntax");

    // TODO: call Compiler::parse_only and report diagnostics
    let _block = Compiler::parse_only(&source);
    todo!("report diagnostics and exit with appropriate code")
}

fn cmd_lint(input: PathBuf, target: TargetArg) -> Result<()> {
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("failed to read '{}'", input.display()))?;

    info!(input = %input.display(), target = ?target, "linting");

    let block = Compiler::parse_only(&source)
        .with_context(|| format!("parse failed: '{}'", input.display()))?;

    let pipeline = LintPipeline::default_for(target.into());
    let diags = pipeline.run(&block);

    let filename = input.to_string_lossy();
    let mut reporter = ConsoleReporter::stderr();
    for diag in &diags {
        reporter.report(diag, &source, &filename);
    }

    if reporter.has_errors() {
        std::process::exit(1);
    }
    Ok(())
}

fn cmd_version() -> Result<()> {
    println!("valua {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
