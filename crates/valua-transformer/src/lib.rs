//! AST mutation passes that lower Lua 5.4 features to Lua 5.1 equivalents.

use valua_ast::Block;

pub use error::TransformError;

mod error;

// ── Core trait ───────────────────────────────────────────────────────────────

/// A single, composable transformation pass over the AST.
pub trait Transform {
    /// Mutate `block` in-place. May emit diagnostics via `reporter`.
    ///
    /// # Errors
    /// Returns `TransformError` if the pass cannot complete (e.g., an
    /// unsupported feature combination or an internal invariant violation).
    fn transform(&mut self, block: &mut Block) -> Result<(), TransformError>;

    /// Human-readable name of this pass, used in error messages.
    fn name(&self) -> &'static str;
}

// ── Pipeline ─────────────────────────────────────────────────────────────────

/// Runs a sequence of `Transform` passes in order over a single `Block`.
#[derive(Default)]
pub struct TransformPipeline {
    passes: Vec<Box<dyn Transform>>,
}

impl TransformPipeline {
    /// Create an empty pipeline.
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a pass to the end of the pipeline.
    pub fn add_pass(&mut self, pass: impl Transform + 'static) -> &mut Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// Execute all passes in insertion order.
    ///
    /// Stops on the first error.
    pub fn run(&mut self, block: &mut Block) -> Result<(), TransformError> {
        for pass in &mut self.passes {
            pass.transform(block)?;
        }
        Ok(())
    }
}

// ── Concrete passes ───────────────────────────────────────────────────────────

/// Rewrites `a & b`, `a | b`, `a ~ b`, `~a`, `a << b`, `a >> b` to `bit.*` calls.
///
/// Targets LuaJIT's `bit` library or `bit32` (Lua 5.2+). The specific module
/// name is chosen based on `LuaTarget`.
#[derive(Debug, Default)]
pub struct BitwiseOpTransform;

impl Transform for BitwiseOpTransform {
    fn name(&self) -> &'static str {
        "BitwiseOpTransform"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        // TODO: walk AST and replace BinaryOp::{BitwiseAnd,BitwiseOr,BitwiseXor,Shl,Shr}
        //       and UnaryOp::BitwiseNot with equivalent bit.band / bit.bor / etc. calls
        todo!("rewrite bitwise operators to bit.* function calls")
    }
}

/// Validates that `<const>` locals are never mutated after initialisation.
///
/// This is a static analysis pass; it does not rewrite the AST but will emit
/// `TransformError::ConstViolation` when a const binding is reassigned.
#[derive(Debug, Default)]
pub struct ConstAttributeValidator;

impl Transform for ConstAttributeValidator {
    fn name(&self) -> &'static str {
        "ConstAttributeValidator"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        // TODO: build a scope map of const names, then scan assignments for violations
        todo!("validate that <const> locals are never reassigned")
    }
}

/// Desugars `<close>` locals into a `pcall`-based cleanup wrapper.
///
/// Wraps the enclosing block in a deferred-call pattern that calls `__close`
/// on the to-be-closed value when the scope exits (normally or via error).
#[derive(Debug, Default)]
pub struct CloseAttributeTransform;

impl Transform for CloseAttributeTransform {
    fn name(&self) -> &'static str {
        "CloseAttributeTransform"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        // TODO: locate LocalDecl nodes with Attribute::Close and rewrite surrounding
        //       block to call __close metamethod via pcall on scope exit
        todo!("desugar <close> locals into pcall-based cleanup wrappers")
    }
}

/// Injects polyfill preamble code for features detected in the AST.
///
/// Must run *after* feature detection but *before* code generation. Prepends
/// the required polyfill strings (from `valua-polyfills`) as `LocalDecl`
/// statements at the top of the root `Block`.
#[derive(Debug, Default)]
pub struct PolyfillInjector;

impl Transform for PolyfillInjector {
    fn name(&self) -> &'static str {
        "PolyfillInjector"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        // TODO: scan block for used features, compute FeatureSet, prepend polyfill chunk
        //       as a parsed sub-block at the top of `block`
        todo!("detect used features and inject polyfill preamble")
    }
}

/// Walks the AST and collects which Lua 5.4 features are actually used.
///
/// Produces a `valua_polyfills::FeatureSet` for `PolyfillInjector`.
#[derive(Debug, Default)]
pub struct FeatureDetector {
    pub uses_bitwise: bool,
    pub uses_close: bool,
    pub uses_string_extensions: bool,
    pub uses_math_extensions: bool,
}

impl Transform for FeatureDetector {
    fn name(&self) -> &'static str {
        "FeatureDetector"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        // TODO: walk block, set boolean flags for detected 5.4 features
        todo!("walk AST and set feature flags")
    }
}

// ── Default pipeline ──────────────────────────────────────────────────────────

/// Build the standard Lua 5.4 → 5.1 transform pipeline.
///
/// Pass order matters:
/// 1. `ConstAttributeValidator` — validate before rewriting
/// 2. `FeatureDetector`         — scan before injection
/// 3. `BitwiseOpTransform`      — rewrite operators
/// 4. `CloseAttributeTransform` — desugar `<close>`
/// 5. `PolyfillInjector`        — prepend polyfills last
pub fn default_pipeline() -> TransformPipeline {
    let mut pipeline = TransformPipeline::new();
    pipeline
        .add_pass(ConstAttributeValidator)
        .add_pass(FeatureDetector::default())
        .add_pass(BitwiseOpTransform)
        .add_pass(CloseAttributeTransform)
        .add_pass(PolyfillInjector);
    pipeline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: test_transforms_bitwise_and — BinaryOp::BitwiseAnd rewrites to bit.band call"]
    fn test_transforms_bitwise_and() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_const_violation — reassigning <const> local emits error"]
    fn test_const_violation() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_close_desugaring — <close> local wraps scope in pcall cleanup"]
    fn test_close_desugaring() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: test_pipeline_order — passes run in declared order"]
    fn test_pipeline_order() {
        todo!()
    }
}
