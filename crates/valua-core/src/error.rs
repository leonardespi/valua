//! Top-level compile error that wraps errors from all pipeline stages.

use thiserror::Error;
use valua_codegen::CodeGenError;
use valua_parser::ParseError;
use valua_transformer::TransformError;

/// A compile error from any stage of the valua transpile pipeline.
#[derive(Debug, Error)]
pub enum CompileError {
    #[error("parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("transform error: {0}")]
    Transform(#[from] TransformError),

    #[error("code generation error: {0}")]
    CodeGen(#[from] CodeGenError),
}
