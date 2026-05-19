//! AST mutation passes that lower Lua 5.5 features to Lua 5.1 equivalents.
//!
//! # Dependency boundary — `valua-transformer` → `valua-parser`
//!
//! [`PolyfillInjector`] calls `valua_parser::parse()` at runtime to convert
//! embedded polyfill source strings (from `valua-polyfills`) into [`Block`]
//! nodes that are prepended to the user's AST.  This establishes the **only**
//! deliberate directed edge from this crate into `valua-parser`.
//!
//! ## Invariant that must never be violated
//!
//! `valua-parser` must remain completely unaware of this crate.  It must not
//! import — directly or transitively — any of the following:
//!
//! - The [`Transform`] trait
//! - Any concrete pass struct (`BitwiseOpTransform`, `PolyfillInjector`, …)
//! - [`TransformPipeline`] or [`TransformError`]
//!
//! Violating this invariant produces a **Cargo circular-dependency error** that
//! prevents the entire workspace from compiling:
//!
//! ```text
//! error: cyclic package dependency: package `valua-parser` depends on itself.
//!   Cycle: valua-parser → valua-transformer → valua-parser
//! ```
//!
//! ## Where to put shared logic
//!
//! If you find logic that both the parser and the transformer need, the correct
//! home is one of the two crates that sit *below* both in the graph:
//!
//! | Need | Correct crate |
//! |---|---|
//! | Shared AST node types | `valua-ast` |
//! | Span / diagnostic types | `valua-diagnostics` |
//!
//! Neither of those crates depends on the parser or the transformer, so adding
//! to them cannot introduce a cycle.

use valua_ast::{Attribute, BinaryOp, Block, Call, Expression, Statement, TableField, UnaryOp};
use valua_diagnostics::Span;
use valua_polyfills::FeatureSet;

pub use error::TransformError;

mod error;

// ── Core trait ───────────────────────────────────────────────────────────────

/// A single, composable transformation pass over the AST.
pub trait Transform {
    /// Mutate `block` in-place.
    ///
    /// # Errors
    /// Returns `TransformError` if the pass cannot complete.
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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pass(&mut self, pass: impl Transform + 'static) -> &mut Self {
        self.passes.push(Box::new(pass));
        self
    }

    /// Execute all passes in insertion order. Stops on the first error.
    pub fn run(&mut self, block: &mut Block) -> Result<(), TransformError> {
        for pass in &mut self.passes {
            pass.transform(block)?;
        }
        Ok(())
    }
}

// ── Feature detection ─────────────────────────────────────────────────────────

/// Walks the AST and collects which Lua 5.5 features are actually used.
///
/// This pass is read-only — it does not mutate the AST. After `transform()`,
/// the result is stored in `self.feature_set` and also available via `detect()`.
#[derive(Debug, Default, Clone)]
pub struct FeatureDetector {
    pub feature_set: FeatureSet,
}

impl FeatureDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Walk `block` and return the set of Lua 5.5 features used.
    ///
    /// Pure read-only traversal — does not allocate beyond the returned `FeatureSet`.
    pub fn detect(block: &Block) -> FeatureSet {
        let mut fs = FeatureSet::default();
        detect_block(block, &mut fs);
        fs
    }
}

impl Transform for FeatureDetector {
    fn name(&self) -> &'static str {
        "FeatureDetector"
    }

    fn transform(&mut self, block: &mut Block) -> Result<(), TransformError> {
        self.feature_set = Self::detect(block);
        Ok(())
    }
}

fn detect_block(block: &Block, fs: &mut FeatureSet) {
    for stmt in &block.stmts {
        detect_stmt(stmt, fs);
    }
}

fn detect_stmt(stmt: &Statement, fs: &mut FeatureSet) {
    match stmt {
        Statement::LocalDecl(d) => {
            for name in &d.names {
                if name.attribute == Some(Attribute::Close) {
                    fs.close_attribute = true;
                }
            }
            for val in &d.values {
                detect_expr(val, fs);
            }
        }
        Statement::Assign(a) => {
            for target in &a.targets {
                detect_expr(target, fs);
            }
            for val in &a.values {
                detect_expr(val, fs);
            }
        }
        Statement::Return(r) => {
            for val in &r.values {
                detect_expr(val, fs);
            }
        }
        Statement::ExprStmt(e) => detect_expr(e, fs),
        Statement::Do(b) => detect_block(&b.body, fs),
        Statement::While(w) => {
            detect_expr(&w.condition, fs);
            detect_block(&w.body, fs);
        }
        Statement::Repeat(r) => {
            detect_block(&r.body, fs);
            detect_expr(&r.condition, fs);
        }
        Statement::If(i) => {
            detect_expr(&i.condition, fs);
            detect_block(&i.then_block, fs);
            for elseif in &i.elseif_clauses {
                detect_expr(&elseif.condition, fs);
                detect_block(&elseif.body, fs);
            }
            if let Some(else_block) = &i.else_block {
                detect_block(else_block, fs);
            }
        }
        Statement::NumericFor(f) => {
            detect_expr(&f.start, fs);
            detect_expr(&f.limit, fs);
            if let Some(step) = &f.step {
                detect_expr(step, fs);
            }
            detect_block(&f.body, fs);
        }
        Statement::GenericFor(f) => {
            for iter in &f.iterators {
                detect_expr(iter, fs);
            }
            detect_block(&f.body, fs);
        }
        Statement::FunctionDecl(f) => detect_block(&f.func.body, fs),
        Statement::LocalFunctionDecl(f) => detect_block(&f.func.body, fs),
        _ => {}
    }
}

fn detect_expr(expr: &Expression, fs: &mut FeatureSet) {
    match expr {
        Expression::BinOp(lhs, op, rhs, _) => {
            match op {
                BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::Shl
                | BinaryOp::Shr => {
                    fs.bitwise_ops = true;
                }
                BinaryOp::IDiv => {
                    fs.integer_div = true;
                }
                _ => {}
            }
            detect_expr(lhs, fs);
            detect_expr(rhs, fs);
        }
        Expression::UnOp(op, operand, _) => {
            if *op == UnaryOp::BitwiseNot {
                fs.bitwise_ops = true;
            }
            detect_expr(operand, fs);
        }
        Expression::Call(call) => match call {
            Call::Call { func, args, .. } => {
                detect_expr(func, fs);
                for arg in args {
                    detect_expr(arg, fs);
                }
            }
            Call::MethodCall { obj, args, .. } => {
                detect_expr(obj, fs);
                for arg in args {
                    detect_expr(arg, fs);
                }
            }
        },
        Expression::Table(t) => {
            for field in &t.fields {
                match field {
                    TableField::ExprKey { key, value, .. } => {
                        detect_expr(key, fs);
                        detect_expr(value, fs);
                    }
                    TableField::NameKey { value, .. } => detect_expr(value, fs),
                    TableField::Positional(value) => detect_expr(value, fs),
                }
            }
        }
        Expression::Function(f) => detect_block(&f.body, fs),
        Expression::Index(base, _, _) => detect_expr(base, fs),
        Expression::IndexExpr(base, key, _) => {
            detect_expr(base, fs);
            detect_expr(key, fs);
        }
        _ => {}
    }
}

// ── Bitwise lowering helpers ──────────────────────────────────────────────────

fn rewrite_block(block: &mut Block) {
    for stmt in &mut block.stmts {
        rewrite_stmt(stmt);
    }
}

fn rewrite_stmt(stmt: &mut Statement) {
    match stmt {
        Statement::LocalDecl(d) => {
            for val in &mut d.values {
                rewrite_expr(val);
            }
        }
        Statement::Assign(a) => {
            for target in &mut a.targets {
                rewrite_expr(target);
            }
            for val in &mut a.values {
                rewrite_expr(val);
            }
        }
        Statement::Return(r) => {
            for val in &mut r.values {
                rewrite_expr(val);
            }
        }
        Statement::ExprStmt(e) => rewrite_expr(e),
        Statement::Do(b) => rewrite_block(&mut b.body),
        Statement::While(w) => {
            rewrite_expr(&mut w.condition);
            rewrite_block(&mut w.body);
        }
        Statement::Repeat(r) => {
            rewrite_block(&mut r.body);
            rewrite_expr(&mut r.condition);
        }
        Statement::If(i) => {
            rewrite_expr(&mut i.condition);
            rewrite_block(&mut i.then_block);
            for elseif in &mut i.elseif_clauses {
                rewrite_expr(&mut elseif.condition);
                rewrite_block(&mut elseif.body);
            }
            if let Some(else_block) = &mut i.else_block {
                rewrite_block(else_block);
            }
        }
        Statement::NumericFor(f) => {
            rewrite_expr(&mut f.start);
            rewrite_expr(&mut f.limit);
            if let Some(step) = &mut f.step {
                rewrite_expr(step);
            }
            rewrite_block(&mut f.body);
        }
        Statement::GenericFor(f) => {
            for iter in &mut f.iterators {
                rewrite_expr(iter);
            }
            rewrite_block(&mut f.body);
        }
        Statement::FunctionDecl(f) => rewrite_block(&mut f.func.body),
        Statement::LocalFunctionDecl(f) => rewrite_block(&mut f.func.body),
        _ => {}
    }
}

fn rewrite_expr(expr: &mut Expression) {
    // Post-order: recurse into children before replacing this node,
    // so nested bitwise ops (e.g. `(a & b) | c`) lower inside-out.
    match expr {
        Expression::BinOp(lhs, _, rhs, _) => {
            rewrite_expr(lhs);
            rewrite_expr(rhs);
        }
        Expression::UnOp(_, operand, _) => {
            rewrite_expr(operand);
        }
        Expression::Call(call) => match call {
            Call::Call { func, args, .. } => {
                rewrite_expr(func);
                for arg in args {
                    rewrite_expr(arg);
                }
            }
            Call::MethodCall { obj, args, .. } => {
                rewrite_expr(obj);
                for arg in args {
                    rewrite_expr(arg);
                }
            }
        },
        Expression::Table(t) => {
            for field in &mut t.fields {
                match field {
                    TableField::ExprKey { key, value, .. } => {
                        rewrite_expr(key);
                        rewrite_expr(value);
                    }
                    TableField::NameKey { value, .. } => rewrite_expr(value),
                    TableField::Positional(value) => rewrite_expr(value),
                }
            }
        }
        Expression::Function(f) => rewrite_block(&mut f.body),
        Expression::Index(base, _, _) => rewrite_expr(base),
        Expression::IndexExpr(base, key, _) => {
            rewrite_expr(base);
            rewrite_expr(key);
        }
        _ => {}
    }

    let needs = match expr {
        Expression::BinOp(_, op, _, _) => matches!(
            op,
            BinaryOp::BitwiseAnd
                | BinaryOp::BitwiseOr
                | BinaryOp::BitwiseXor
                | BinaryOp::Shl
                | BinaryOp::Shr
        ),
        Expression::UnOp(op, _, _) => *op == UnaryOp::BitwiseNot,
        _ => false,
    };

    if needs {
        let old = std::mem::replace(expr, Expression::Nil(Span::dummy()));
        *expr = lower_bitwise(old);
    }
}

fn lower_bitwise(expr: Expression) -> Expression {
    match expr {
        Expression::BinOp(lhs, op, rhs, span) => {
            let method = match op {
                BinaryOp::BitwiseAnd => "band",
                BinaryOp::BitwiseOr => "bor",
                BinaryOp::BitwiseXor => "bxor",
                BinaryOp::Shl => "lshift",
                BinaryOp::Shr => "rshift",
                _ => unreachable!(),
            };
            make_bit_call(method, vec![*lhs, *rhs], span)
        }
        Expression::UnOp(UnaryOp::BitwiseNot, operand, span) => {
            make_bit_call("bnot", vec![*operand], span)
        }
        _ => unreachable!(),
    }
}

fn make_bit_call(method: &str, args: Vec<Expression>, span: Span) -> Expression {
    Expression::Call(Call::Call {
        func: Box::new(Expression::Index(
            Box::new(Expression::Name("bit".to_string(), span)),
            method.to_string(),
            span,
        )),
        args,
        span,
    })
}

// ── Concrete passes ───────────────────────────────────────────────────────────

/// Rewrites `a & b`, `a | b`, `a ~ b`, `~a`, `a << b`, `a >> b` to `bit.*` calls.
#[derive(Debug, Default)]
pub struct BitwiseOpTransform;

impl Transform for BitwiseOpTransform {
    fn name(&self) -> &'static str {
        "BitwiseOpTransform"
    }

    fn transform(&mut self, block: &mut Block) -> Result<(), TransformError> {
        rewrite_block(block);
        Ok(())
    }
}

/// Rewrites `a // b` (integer division) to `math.floor(a / b)`.
///
/// # Invariant
///
/// Must run after [`BitwiseOpTransform`] and before [`CloseAttributeTransform`].
/// See [`default_pipeline`] for the canonical pass order.
///
/// # AST expansion (PRD §5.1, §6.2)
///
/// Source node:
/// ```text
/// Expression::BinOp(lhs, BinaryOp::IDiv, rhs, span)
/// ```
///
/// Target node:
/// ```text
/// Expression::Call(Call::Call {
///     func: Box(Expression::Index(
///         Box(Expression::Name("math", span)),
///         "floor",
///         span,
///     )),
///     args: [Expression::BinOp(lhs, BinaryOp::Div, rhs, span)],
///     span,
/// })
/// ```
///
/// In Lua surface syntax: `a // b` → `math.floor(a / b)`.
///
/// # Why `math.floor(a / b)` and not an integer truncation or cast
///
/// LuaJIT JIT-compiles `math.floor()` on float register paths to a single
/// `ROUNDSD` machine instruction; the preceding float division (`/`) is
/// likewise JIT-compiled.  The combined pattern is therefore a two-instruction
/// native trace — identical throughput to a native `//` operator, with no
/// runtime dispatch overhead.  (PRD §5.1 "Rendimiento nativo", §8.2.)
///
/// # Traversal
///
/// Post-order recursive walk mirroring [`BitwiseOpTransform`]:
/// `rewrite_idiv_block` → `rewrite_idiv_stmt` → `rewrite_idiv_expr`.
/// The `needs` check is `BinaryOp::IDiv`; all other `BinaryOp` variants pass
/// through unmodified.  Consider sharing the walk helpers with
/// `BitwiseOpTransform` via a strategy closure once there are ≥ 4 similar
/// operator-rewrite passes (per CLAUDE.md "three similar lines" rule).
///
/// # Span preservation
///
/// Copy the original `IDiv` span to both the outer `Call` node and the inner
/// `Div` node.  Diagnostics must remain anchored to the original `//` token in
/// the source, not to a synthetic zero-offset location.
///
/// # Constant folding
///
/// Per PRD §3.3 ("valua is not a partial evaluator"), even `4 // 2` emits
/// `math.floor(4 / 2)`.  Constant folding is out of scope and must not be
/// added here without a PRD amendment.
#[derive(Debug, Default)]
pub struct IntegerDivisionTransform;

impl Transform for IntegerDivisionTransform {
    fn name(&self) -> &'static str {
        "IntegerDivisionTransform"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        todo!("rewrite BinaryOp::IDiv nodes to math.floor(a / b) call expressions")
    }
}

/// Validates that `<const>` locals are never mutated after initialisation.
#[derive(Debug, Default)]
pub struct ConstAttributeValidator;

impl Transform for ConstAttributeValidator {
    fn name(&self) -> &'static str {
        "ConstAttributeValidator"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        todo!("validate that <const> locals are never reassigned")
    }
}

/// Desugars `<close>` locals into a `pcall`-based cleanup wrapper.
///
/// # Why `pcall` (PRD §2.3, §6.3, Anexo A — Lema 3)
///
/// Preserving `<close>` semantics natively in LuaJIT requires emulating the
/// to-be-closed mechanism, which LuaJIT's VM does not implement.  The `pcall`
/// wrapper is the correct emulation because:
///
/// - The slowdown factor `c ∈ [20, 100]` proven in Lema 3 applies **only to
///   arithmetic operations in hot loops**, not to scope-exit overhead.
/// - `<close>` guards wrap I/O-bound resources (files, sockets, DB handles)
///   by convention.  The cost of `pcall` is negligible compared to any real
///   I/O operation that the guarded code performs.
/// - The rule is **fixed, not heuristic**: every `<close>` declaration is
///   wrapped, unconditionally.  This avoids the Caso C trap of the
///   impossibility theorem (PRD §2.3, §6.3).
///
/// # Transformation algorithm
///
/// Given a block with a `<close>` local at statement index `i`:
///
/// ```text
/// -- Source block:
/// [stmts 0 .. i-1]
/// local f <close> = <init_expr>   ←  index i;  Attribute::Close present
/// [tail: stmts i+1 .. n]          ←  runs while f is in scope
/// ```
///
/// Becomes:
///
/// ```text
/// -- Transformed block:
/// [stmts 0 .. i-1]                         ←  prefix unchanged
/// local f = <init_expr>                    ←  Attribute::Close stripped
/// local _ok, _err = pcall(function()       ←  tail wrapped in anonymous closure
///     [tail: stmts i+1 .. n]
/// end)
/// local _close_mt = getmetatable(f)        ←  safe metamethod lookup
/// if _close_mt and _close_mt.__close then
///     _close_mt.__close(f, _err)           ←  pass error as second arg (Lua 5.4 §3.3.8)
/// end
/// if not _ok then error(_err, 0) end       ←  re-raise; level 0 omits extra frame
/// ```
///
/// # Step-by-step AST mutation
///
/// 1. Walk `block.stmts` forward.  On each `Statement::LocalDecl`, check
///    whether any `LocalName` has `attribute == Some(Attribute::Close)`.
///
/// 2. On match at index `i`:
///
///    a. Drain `block.stmts.drain(i+1..)` into `Vec<Statement>` named `tail`.
///
///    b. Strip `Attribute::Close` from the matching `LocalName` (set
///       `attribute = None`).  The `<init_expr>` is unchanged.
///
///    c. Build **`pcall_stmt`**:
///       ```text
///       Statement::LocalDecl {
///           names: [LocalName { name: "_ok",  attribute: None, span: dummy },
///                   LocalName { name: "_err", attribute: None, span: dummy }],
///           values: [Expression::Call(Call::Call {
///               func: Box(Expression::Name("pcall", span)),
///               args: [Expression::Function(FunctionBody {
///                   params: [],
///                   is_vararg: false,
///                   body: Block { stmts: tail, span: dummy },
///                   span: dummy,
///               })],
///               span,
///           })],
///           span: dummy,
///       }
///       ```
///
///    d. Build **`mt_stmt`** (safe metamethod retrieval):
///       ```text
///       Statement::LocalDecl {
///           names: [LocalName { name: "_close_mt", attribute: None, span: dummy }],
///           values: [Expression::Call(Call::Call {
///               func: Box(Expression::Name("getmetatable", span)),
///               args: [Expression::Name(close_var_name.clone(), span)],
///               span,
///           })],
///           span: dummy,
///       }
///       ```
///
///    e. Build **`close_call_stmt`** (conditional `__close` invocation):
///       ```text
///       Statement::If {
///           condition: Box(Expression::BinOp(
///               Box(Expression::Name("_close_mt", span)),
///               BinaryOp::And,
///               Box(Expression::Index(
///                   Box(Expression::Name("_close_mt", span)),
///                   "__close",
///                   span,
///               )),
///               span,
///           )),
///           then_block: Block {
///               stmts: [Statement::ExprStmt(Expression::Call(Call::Call {
///                   func: Box(Expression::Index(
///                       Box(Expression::Name("_close_mt", span)),
///                       "__close",
///                       span,
///                   )),
///                   args: [Expression::Name(close_var_name.clone(), span),
///                          Expression::Name("_err", span)],
///                   span,
///               }))],
///               span: dummy,
///           },
///           elseif_clauses: [],
///           else_block: None,
///           span: dummy,
///       }
///       ```
///
///    f. Build **`reraise_stmt`** (propagate error after cleanup):
///       ```text
///       Statement::If {
///           condition: Box(Expression::UnOp(
///               UnaryOp::Not,
///               Box(Expression::Name("_ok", span)),
///               span,
///           )),
///           then_block: Block {
///               stmts: [Statement::ExprStmt(Expression::Call(Call::Call {
///                   func: Box(Expression::Name("error", span)),
///                   args: [Expression::Name("_err", span),
///                          Expression::Integer(0, span)],
///                   span,
///               }))],
///               span: dummy,
///           },
///           elseif_clauses: [],
///           else_block: None,
///           span: dummy,
///       }
///       ```
///
///    g. Append `pcall_stmt`, `mt_stmt`, `close_call_stmt`, `reraise_stmt`
///       to `block.stmts` in that order.
///
///    h. **Stop forward scanning** — the tail has been consumed.  Recursion
///       (step 3) processes the newly constructed inner blocks.
///
/// 3. Recurse into **all sub-blocks**: function bodies, `do`/`while`/
///    `repeat`/`if`/`for` bodies.  This ensures `<close>` declarations nested
///    inside inner scopes are also transformed.
///
/// # Multiple `<close>` locals in the same scope
///
/// Lua 5.4 closes variables in **reverse declaration order** (last in, first
/// closed).  Implement by scanning `block.stmts` from the **end** toward the
/// front, or by building nested wrappers such that the innermost pcall
/// corresponds to the last-declared `<close>`.  This ordering invariant must
/// hold or resource cleanup will execute in the wrong sequence.
///
/// # MVP limitations (PRD §11)
///
/// - **Coroutine yield across `pcall`**: `pcall` does not propagate
///   `coroutine.yield()`.  Tail code that yields will silently fail to yield.
///   Detect statically with a lint (`E04xx`) and document; do not attempt to
///   work around at transform time.
/// - **Traceback fidelity**: `error(_err, 0)` re-raises at level 0 (no extra
///   stack frame added), but the original traceback string stored in `_err` is
///   already the formatted message from the pcall boundary, not the raw error
///   object.  Acceptable for I/O-bound use cases.
/// - **`__close` vs. `close` convention**: this pass always invokes `__close`
///   via the metatable (strict Lua 5.4 §3.3.8 semantics).  Direct `:close()`
///   method calling patterns (older Lua conventions) are not emulated.
#[derive(Debug, Default)]
pub struct CloseAttributeTransform;

impl Transform for CloseAttributeTransform {
    fn name(&self) -> &'static str {
        "CloseAttributeTransform"
    }

    fn transform(&mut self, _block: &mut Block) -> Result<(), TransformError> {
        todo!("desugar <close> locals into pcall-based cleanup wrappers")
    }
}

/// Injects polyfill preamble code for features detected in the AST.
///
/// If `feature_set` is pre-populated via `with_features`, uses it directly;
/// otherwise calls `FeatureDetector::detect()` on the block.
///
/// **Surgical injection guarantee:** returns immediately without touching the
/// block when `feature_set.is_empty()`, preventing dead string allocation and
/// any AST mutation for clean inputs.
///
/// # Dependency boundary
///
/// This struct is the **sole site** in `valua-transformer` that is permitted to
/// call into `valua-parser`.  The dependency flows in exactly one direction:
///
/// ```text
/// valua-transformer  (this crate)
///     └── valua-parser::parse()   ← one-way utility call; maps &str → Block
///           ├── valua-lexer       ← tokenisation only
///           └── valua-ast         ← shared node types; never transformer types
/// ```
///
/// **`valua-parser` must never import anything from `valua-transformer`.**
/// The moment a `use valua_transformer::…` line appears in any file under
/// `crates/valua-parser/`, `cargo build` will fail with a cyclic-dependency
/// error for the entire workspace.  There is no workaround short of breaking
/// the cycle.
///
/// If you believe shared logic is needed between the parser and this pass,
/// extract it into `valua-ast` (types) or `valua-diagnostics` (spans/errors).
/// Both crates are below both `valua-parser` and `valua-transformer` in the
/// dependency graph and are safe common ground.  See the crate-level
/// documentation for the full boundary rationale.
#[derive(Debug, Default)]
pub struct PolyfillInjector {
    feature_set: Option<FeatureSet>,
}

impl PolyfillInjector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-supply a `FeatureSet` computed by a prior `FeatureDetector` pass.
    pub fn with_features(fs: FeatureSet) -> Self {
        Self {
            feature_set: Some(fs),
        }
    }
}

impl Transform for PolyfillInjector {
    fn name(&self) -> &'static str {
        "PolyfillInjector"
    }

    fn transform(&mut self, block: &mut Block) -> Result<(), TransformError> {
        let features = self
            .feature_set
            .clone()
            .unwrap_or_else(|| FeatureDetector::detect(block));

        // Surgical early-return: no polyfills needed, block is untouched.
        if features.is_empty() {
            return Ok(());
        }

        // Phase 3 body: parse the polyfill chunk and prepend its statements
        // to `block.stmts`. Blocked on: non-empty polyfill constants in
        // valua-polyfills and valua-parser as a runtime dependency here.
        //
        // Shape of the future implementation:
        //   let src = valua_polyfills::polyfills_for(&features);
        //   if src.is_empty() { return Ok(()); }
        //   let polyfill_block = valua_parser::parse(&src)
        //       .map_err(|e| TransformError::Internal { pass: self.name(), detail: e.to_string() })?;
        //   let tail = std::mem::replace(&mut block.stmts, polyfill_block.stmts);
        //   block.stmts.extend(tail);
        Ok(())
    }
}

// ── Default pipeline ──────────────────────────────────────────────────────────

/// Build the standard Lua 5.5 → 5.1 transform pipeline.
///
/// Pass order:
/// 1. `ConstAttributeValidator`    — static validation before any rewrite
/// 2. `FeatureDetector`            — populate `FeatureSet` for downstream passes
/// 3. `BitwiseOpTransform`         — `&`, `|`, `~`, `<<`, `>>` → `bit.*` calls
/// 4. `IntegerDivisionTransform`   — `//` → `math.floor(a / b)`
/// 5. `CloseAttributeTransform`    — `<close>` locals → `pcall` wrappers
/// 6. `PolyfillInjector`           — prepend polyfill preamble last
pub fn default_pipeline() -> TransformPipeline {
    let mut pipeline = TransformPipeline::new();
    pipeline
        .add_pass(ConstAttributeValidator)
        .add_pass(FeatureDetector::new())
        .add_pass(BitwiseOpTransform)
        .add_pass(IntegerDivisionTransform)
        .add_pass(CloseAttributeTransform)
        .add_pass(PolyfillInjector::new());
    pipeline
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> valua_ast::Block {
        valua_parser::parse(src).expect("parse failed")
    }

    // ── FeatureDetector::detect — no false positives ──────────────────────────

    #[test]
    fn detect_empty_for_pure_arithmetic() {
        assert!(
            FeatureDetector::detect(&parse("local x = 1 + 2")).is_empty(),
            "pure arithmetic must not trigger any feature flag"
        );
    }

    #[test]
    fn detect_empty_for_string_concat() {
        assert!(FeatureDetector::detect(&parse("local x = 'a' .. 'b'")).is_empty());
    }

    #[test]
    fn detect_empty_for_const_attribute() {
        assert!(
            FeatureDetector::detect(&parse("local x <const> = 42")).is_empty(),
            "<const> must not set any feature flag"
        );
    }

    #[test]
    fn detect_empty_for_comparison_ops() {
        assert!(FeatureDetector::detect(&parse("return a == b and c ~= d")).is_empty());
    }

    #[test]
    fn detect_empty_for_regular_div() {
        // `/` is float division — present in all targets, no flag needed
        assert!(FeatureDetector::detect(&parse("local x = a / b")).is_empty());
    }

    // ── FeatureDetector::detect — bitwise ops ────────────────────────────────

    #[test]
    fn detect_bitwise_and() {
        let fs = FeatureDetector::detect(&parse("local x = a & b"));
        assert!(fs.bitwise_ops);
        assert!(!fs.integer_div);
    }

    #[test]
    fn detect_bitwise_or() {
        assert!(FeatureDetector::detect(&parse("local x = a | b")).bitwise_ops);
    }

    #[test]
    fn detect_bitwise_xor_binary() {
        assert!(FeatureDetector::detect(&parse("local x = a ~ b")).bitwise_ops);
    }

    #[test]
    fn detect_bitwise_shl() {
        assert!(FeatureDetector::detect(&parse("local x = a << 2")).bitwise_ops);
    }

    #[test]
    fn detect_bitwise_shr() {
        assert!(FeatureDetector::detect(&parse("local x = a >> 2")).bitwise_ops);
    }

    #[test]
    fn detect_unary_bitwise_not() {
        assert!(FeatureDetector::detect(&parse("local x = ~a")).bitwise_ops);
    }

    // ── FeatureDetector::detect — other flags ────────────────────────────────

    #[test]
    fn detect_integer_div() {
        let fs = FeatureDetector::detect(&parse("local x = a // b"));
        assert!(fs.integer_div);
        assert!(!fs.bitwise_ops, "IDiv must not set bitwise_ops");
    }

    #[test]
    fn detect_close_attribute() {
        let fs = FeatureDetector::detect(&parse("local f <close> = io.open('x')"));
        assert!(fs.close_attribute);
        assert!(!fs.bitwise_ops);
        assert!(!fs.integer_div);
    }

    // ── FeatureDetector::detect — nested contexts ────────────────────────────

    #[test]
    fn detect_bitwise_in_nested_function_body() {
        assert!(
            FeatureDetector::detect(&parse("function foo() return a & b end")).bitwise_ops
        );
    }

    #[test]
    fn detect_bitwise_in_do_block() {
        assert!(FeatureDetector::detect(&parse("do\n  local x = a | b\nend")).bitwise_ops);
    }

    #[test]
    fn detect_bitwise_in_while_condition() {
        // `(flags & mask) ~= 0`: `&` has higher precedence than `~=` → BinOp(BitwiseAnd) found
        assert!(
            FeatureDetector::detect(&parse("while (flags & mask) ~= 0 do end")).bitwise_ops
        );
    }

    #[test]
    fn detect_bitwise_in_if_condition() {
        assert!(FeatureDetector::detect(&parse("if a & 1 == 1 then end")).bitwise_ops);
    }

    #[test]
    fn detect_bitwise_in_call_argument() {
        assert!(FeatureDetector::detect(&parse("foo(a & b)")).bitwise_ops);
    }

    #[test]
    fn detect_bitwise_in_table_field_value() {
        assert!(
            FeatureDetector::detect(&parse("local t = { flags = a | b }")).bitwise_ops
        );
    }

    #[test]
    fn detect_bitwise_in_anonymous_closure_value() {
        // Closure is an expression; its body must be traversed
        assert!(
            FeatureDetector::detect(&parse("local f = function() return a & b end")).bitwise_ops
        );
    }

    #[test]
    fn detect_bitwise_in_deeply_nested_block() {
        let src = "do\n  do\n    do\n      local x = a >> 1\n    end\n  end\nend";
        assert!(FeatureDetector::detect(&parse(src)).bitwise_ops);
    }

    // ── FeatureDetector as Transform ─────────────────────────────────────────

    #[test]
    fn feature_detector_transform_populates_feature_set() {
        let mut block = parse("local x = a & b");
        let mut detector = FeatureDetector::new();
        detector.transform(&mut block).expect("must not fail");
        assert!(detector.feature_set.bitwise_ops);
    }

    #[test]
    fn feature_detector_transform_does_not_mutate_block() {
        let mut block = parse("local x = a & b");
        let original_len = block.stmts.len();
        let mut detector = FeatureDetector::new();
        detector.transform(&mut block).expect("must not fail");
        assert_eq!(
            block.stmts.len(),
            original_len,
            "FeatureDetector must not modify the AST"
        );
    }

    #[test]
    fn feature_detector_transform_clean_code_empty_feature_set() {
        let mut block = parse("local x = 1 + 2");
        let mut detector = FeatureDetector::new();
        detector.transform(&mut block).expect("must not fail");
        assert!(
            detector.feature_set.is_empty(),
            "pure arithmetic block must produce empty FeatureSet"
        );
    }

    // ── PolyfillInjector — surgical early return ──────────────────────────────

    #[test]
    fn polyfill_injector_no_op_for_pure_arithmetic() {
        let mut block = parse("local x = 1 + 2\nreturn x");
        let original_len = block.stmts.len();
        PolyfillInjector::new().transform(&mut block).unwrap();
        assert_eq!(
            block.stmts.len(),
            original_len,
            "pure arithmetic: no polyfill statements must be prepended"
        );
    }

    #[test]
    fn polyfill_injector_no_op_for_string_ops() {
        let mut block = parse("local x = 'hello' .. ' world'\nreturn x");
        let original_len = block.stmts.len();
        PolyfillInjector::new().transform(&mut block).unwrap();
        assert_eq!(block.stmts.len(), original_len);
    }

    #[test]
    fn polyfill_injector_no_op_for_comparison_only() {
        let mut block = parse("return a == b or c ~= d");
        let original_len = block.stmts.len();
        PolyfillInjector::new().transform(&mut block).unwrap();
        assert_eq!(block.stmts.len(), original_len);
    }

    #[test]
    fn polyfill_injector_with_empty_feature_set_suppresses_injection() {
        // Even when block has bitwise ops, a forced empty FeatureSet skips injection.
        let mut block = parse("local x = a & b");
        let original_len = block.stmts.len();
        PolyfillInjector::with_features(FeatureSet::default())
            .transform(&mut block)
            .unwrap();
        assert_eq!(
            block.stmts.len(),
            original_len,
            "forced-empty FeatureSet must suppress injection"
        );
    }

    #[test]
    fn polyfill_injector_does_not_panic_for_bitwise_block() {
        // Injection path is reachable; must not panic even with empty BITWISE_FALLBACK.
        let mut block = parse("local x = a & b");
        PolyfillInjector::new().transform(&mut block).unwrap();
    }

    // ── Two-pass mini-pipeline ────────────────────────────────────────────────

    #[test]
    fn pipeline_feature_detector_then_injector_clean_code() {
        let mut block = parse("local x = 1 + 2");
        let original_len = block.stmts.len();
        let mut pipeline = TransformPipeline::new();
        pipeline
            .add_pass(FeatureDetector::new())
            .add_pass(PolyfillInjector::new());
        pipeline.run(&mut block).expect("pipeline must succeed");
        assert_eq!(
            block.stmts.len(),
            original_len,
            "clean block must be unchanged after feature-detect + inject pipeline"
        );
    }

    #[test]
    fn pipeline_feature_detector_then_injector_bitwise_block() {
        // Both passes must complete without panic when bitwise ops are present.
        let mut block = parse("local x = a & b");
        let mut pipeline = TransformPipeline::new();
        pipeline
            .add_pass(FeatureDetector::new())
            .add_pass(PolyfillInjector::new());
        pipeline.run(&mut block).expect("pipeline must succeed");
    }

    // ── BitwiseOpTransform ────────────────────────────────────────────────────

    fn no_bitwise(src: &str) {
        let mut block = parse(src);
        BitwiseOpTransform.transform(&mut block).unwrap();
        assert!(
            !FeatureDetector::detect(&block).bitwise_ops,
            "bitwise ops must be gone after transform: {src}"
        );
    }

    #[test]
    fn bitwise_transform_band() {
        no_bitwise("local x = a & b");
    }

    #[test]
    fn bitwise_transform_bor() {
        no_bitwise("local x = a | b");
    }

    #[test]
    fn bitwise_transform_bxor() {
        no_bitwise("local x = a ~ b");
    }

    #[test]
    fn bitwise_transform_lshift() {
        no_bitwise("local x = a << 2");
    }

    #[test]
    fn bitwise_transform_rshift() {
        no_bitwise("local x = a >> 2");
    }

    #[test]
    fn bitwise_transform_bnot() {
        no_bitwise("local x = ~a");
    }

    #[test]
    fn bitwise_transform_nested() {
        // (a & b) | c  →  bit.bor(bit.band(a, b), c)
        no_bitwise("local x = (a & b) | c");
    }

    #[test]
    fn bitwise_transform_in_function_body() {
        no_bitwise("function foo() return a & b end");
    }

    #[test]
    fn bitwise_transform_in_while_condition() {
        no_bitwise("while (flags & mask) ~= 0 do end");
    }

    #[test]
    fn bitwise_transform_in_table_field() {
        no_bitwise("local t = { flags = a | b }");
    }

    #[test]
    fn bitwise_transform_in_closure() {
        no_bitwise("local f = function() return a & b end");
    }

    #[test]
    fn bitwise_transform_leaves_arithmetic_intact() {
        let mut block = parse("local x = a + b * c - d");
        BitwiseOpTransform.transform(&mut block).unwrap();
        // No bitwise ops to detect, and arithmetic structure is untouched.
        assert!(FeatureDetector::detect(&block).is_empty());
    }

    #[test]
    fn bitwise_transform_call_site_is_bit_dot_band() {
        let mut block = parse("local x = a & b");
        BitwiseOpTransform.transform(&mut block).unwrap();
        // Verify AST structure: LocalDecl → first value is Call(Index(Name("bit"), "band"))
        let Statement::LocalDecl(ref d) = block.stmts[0] else {
            panic!("expected LocalDecl");
        };
        let Expression::Call(Call::Call { ref func, ref args, .. }) = d.values[0] else {
            panic!("expected Call");
        };
        let Expression::Index(ref base, ref field, _) = **func else {
            panic!("expected Index");
        };
        let Expression::Name(ref name, _) = **base else {
            panic!("expected Name");
        };
        assert_eq!(name, "bit");
        assert_eq!(field, "band");
        assert_eq!(args.len(), 2);
    }

    #[test]
    #[ignore = "TODO: ConstAttributeValidator not yet implemented"]
    fn test_const_violation() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: CloseAttributeTransform not yet implemented"]
    fn test_close_desugaring() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: verify passes run in declared order via observable side-effects"]
    fn test_pipeline_order() {
        todo!()
    }
}
