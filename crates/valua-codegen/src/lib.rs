//! Lua 5.1 code emitter — walks a transformed AST and produces formatted source.

use valua_ast::Block;

pub use error::CodeGenError;

mod error;

// ── Configuration ─────────────────────────────────────────────────────────────

/// Target Lua runtime; controls which built-ins are assumed to be available.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LuaTarget {
    /// Plain Lua 5.1 (PUC reference implementation).
    #[default]
    Lua51,
    /// LuaJIT 2.x — has `bit` library and some 5.1 extensions.
    LuaJIT,
}

/// Options that control how the emitted Lua source is formatted.
#[derive(Debug, Clone)]
pub struct EmitOptions {
    /// String used for each indentation level (default: four spaces).
    pub indent: String,
    /// Target runtime.
    pub target: LuaTarget,
    /// Emit a header comment with the valua version.
    pub emit_header_comment: bool,
}

impl Default for EmitOptions {
    fn default() -> Self {
        Self {
            indent: "    ".to_string(),
            target: LuaTarget::default(),
            emit_header_comment: true,
        }
    }
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// A code-generation backend that can emit a `Block` to a `String`.
pub trait CodeGen {
    /// Emit the entire `block` and return the formatted Lua source.
    ///
    /// # Errors
    /// Returns `CodeGenError` if the AST contains a node that cannot be
    /// represented in the target Lua version.
    fn emit(&self, block: &Block) -> Result<String, CodeGenError>;
}

// ── Emitter ───────────────────────────────────────────────────────────────────

/// Concrete Lua 5.1 emitter.
pub struct LuaEmitter {
    pub options: EmitOptions,
}

impl LuaEmitter {
    /// Create an emitter with the given options.
    pub fn new(options: EmitOptions) -> Self {
        Self { options }
    }

    /// Create an emitter with default options targeting Lua 5.1.
    pub fn lua51() -> Self {
        Self::new(EmitOptions { target: LuaTarget::Lua51, ..EmitOptions::default() })
    }

    /// Create an emitter with default options targeting LuaJIT.
    pub fn luajit() -> Self {
        Self::new(EmitOptions { target: LuaTarget::LuaJIT, ..EmitOptions::default() })
    }
}

impl CodeGen for LuaEmitter {
    fn emit(&self, block: &Block) -> Result<String, CodeGenError> {
        let mut ctx = EmitContext::new(&self.options);
        ctx.emit_block(block)?;
        Ok(ctx.finish())
    }
}

// ── Emit context ──────────────────────────────────────────────────────────────

/// Internal state threaded through the recursive emitter.
pub(crate) struct EmitContext<'opts> {
    options: &'opts EmitOptions,
    buf: String,
    depth: usize,
}

impl<'opts> EmitContext<'opts> {
    pub(crate) fn new(options: &'opts EmitOptions) -> Self {
        Self { options, buf: String::new(), depth: 0 }
    }

    pub(crate) fn finish(self) -> String {
        self.buf
    }

    pub(crate) fn emit_block(&mut self, block: &valua_ast::Block) -> Result<(), CodeGenError> {
        // TODO: handle indentation and block-level constructs
        for stmt in &block.stmts {
            self.emit_statement(stmt)?;
        }
        Ok(())
    }

    pub(crate) fn emit_statement(
        &mut self,
        stmt: &valua_ast::Statement,
    ) -> Result<(), CodeGenError> {
        // TODO: match on Statement variant and emit corresponding Lua 5.1 syntax
        self.push_line("");
        if let valua_ast::Statement::ExprStmt(expr) = stmt {
            self.emit_expression(expr)?;
        }
        todo!("emit all Statement variants")
    }

    pub(crate) fn emit_expression(
        &mut self,
        _expr: &valua_ast::Expression,
    ) -> Result<(), CodeGenError> {
        // TODO: match on Expression variant and emit corresponding Lua syntax
        self.push("");
        todo!("emit all Expression variants")
    }

    fn indent(&self) -> String {
        self.options.indent.repeat(self.depth)
    }

    fn push(&mut self, s: &str) {
        self.buf.push_str(s);
    }

    fn push_line(&mut self, s: &str) {
        self.buf.push_str(&self.indent());
        self.buf.push_str(s);
        self.buf.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: test_emit_options_default — indent is 4 spaces, target is Lua51"]
    fn test_emit_options_default() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_emit_empty_block — empty Block emits empty string"]
    fn test_emit_empty_block() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_luajit_target_assumed — LuaJIT emitter assumes bit library"]
    fn test_luajit_target_assumed() {
        todo!()
    }
}
