use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;
use valua_core::{CompileOptions, Compiler, Target};

/// valua — Lua 5.4 → 5.1 / LuaJIT transpiler.
#[derive(Debug, Parser)]
#[command(name = "valua", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Transpile a Lua 5.4 source file to Lua 5.1 or LuaJIT.
    Build {
        /// Input Lua 5.4 source file.
        input: PathBuf,

        /// Output file path (defaults to stdout when omitted).
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target Lua runtime.
        #[arg(long, default_value = "lua51")]
        target: TargetArg,
    },

    /// Validate a Lua 5.4 source file without producing output.
    Check {
        /// Input Lua 5.4 source file.
        input: PathBuf,
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
        Command::Build { input, output, target } => cmd_build(input, output, target),
        Command::Check { input } => cmd_check(input),
        Command::Version => cmd_version(),
    }
}

fn cmd_build(input: PathBuf, output: Option<PathBuf>, target: TargetArg) -> Result<()> {
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("failed to read '{}'", input.display()))?;

    info!(input = %input.display(), target = ?target, "transpiling");

    let opts = CompileOptions {
        target: target.into(),
        ..CompileOptions::default()
    };

    // TODO: remove this placeholder once Compiler::compile is implemented
    let _result = Compiler::compile(&source, opts);
    todo!("handle compile result and write to output or stdout")
}

fn cmd_check(input: PathBuf) -> Result<()> {
    let source = std::fs::read_to_string(&input)
        .with_context(|| format!("failed to read '{}'", input.display()))?;

    info!(input = %input.display(), "checking syntax");

    // TODO: call Compiler::parse_only and report diagnostics
    let _block = Compiler::parse_only(&source);
    todo!("report diagnostics and exit with appropriate code")
}

fn cmd_version() -> Result<()> {
    println!("valua {}", env!("CARGO_PKG_VERSION"));
    Ok(())
}
