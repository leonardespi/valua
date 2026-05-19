use codespan_reporting::term::termcolor::{ColorChoice, StandardStream, WriteColor};

#[cfg(feature = "serde")]
use serde::Serialize;

// TODO(Phase 5 - Refactor UTF-8 Column Alignment):
//
// ARCHITECTURAL DEBT — byte-indexed column tracking
//
// `Span::col` is populated by the lexer by counting raw bytes since the last
// newline, NOT logical Unicode scalar values. For the current codebase this is
// safe only because every integration-test fixture is 100% ASCII (1 byte per
// char). The moment a source file contains multi-byte sequences (accented
// letters, CJK, emoji, etc.) `col` will report a byte offset instead of a
// visual column, causing every diagnostic that prints "line:col" coordinates
// to point at the wrong position.
//
// Scope of the problem:
//   • `Span::col` — stored byte column, not char column.
//   • `Span::Display` — emits `line:col`; col is byte-based.
//   • The lexer (`valua-lexer`) — increments its column counter with `+=
//     token_bytes` instead of `+= token.chars().count()`.
//   • `render_to_writer` below — passes `span.start..span.end` byte ranges to
//     `codespan_reporting`. That library does re-derive column from the byte
//     range, so caret rendering may survive for valid UTF-8 boundaries, but
//     `col` stored in `Span` itself will still be wrong for multi-byte input.
//
// Resolution path for Phase 5:
//   Option A (minimal): remap the lexer to walk chars (`str::chars()`) and
//     count code-points; update `col` semantics to mean "1-based Unicode scalar
//     column".
//   Option B (full): replace the hand-rolled rendering path entirely with
//     `codespan-reporting`'s own line/column resolution (it already does this
//     correctly from byte offsets), drop `Span::col` from the public API, and
//     keep only `start`/`end` byte offsets. `codespan_reporting::files::
//     SimpleFiles::location()` handles UTF-8 transparently.
//   Option C (display width): if terminal display-width accuracy matters (e.g.
//     CJK double-width glyphs), additionally integrate the `unicode-width` crate
//     to compute display columns separately from scalar counts.
//
// Until Phase 5 this is guarded by ASCII-only test fixtures. Do NOT extend test
// fixtures or production input with non-ASCII source text before this is fixed.
/// Byte-range + line/column position within a source file.
#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// Byte offset of the first character (inclusive).
    pub start: usize,
    /// Byte offset past the last character (exclusive).
    pub end: usize,
    /// 1-based line number.
    pub line: u32,
    /// 1-based column number (byte-based; see UTF-8 TODO above).
    pub col: u32,
}

impl std::fmt::Display for Span {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

impl Span {
    /// Create a new span from raw fields.
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }

    /// Placeholder span for generated nodes that have no source location.
    pub fn dummy() -> Self {
        Self {
            start: 0,
            end: 0,
            line: 0,
            col: 0,
        }
    }

    /// Merge two spans into one that covers both.
    ///
    /// The column of the merged span comes from whichever span starts first
    /// in the source (smaller `start` offset).
    pub fn merge(self, other: Self) -> Self {
        let first = if self.start <= other.start {
            self
        } else {
            other
        };
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            col: first.col,
        }
    }
}

/// Severity level of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// A hard error that prevents transpilation.
    Error,
    /// A non-fatal issue; transpilation may continue.
    Warning,
    /// Informational annotation attached to another diagnostic.
    Note,
}

/// A structured diagnostic message with optional error code, fix suggestion,
/// and secondary source spans.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    /// Short error code, e.g. `"E0001"`.
    pub code: Option<&'static str>,
    /// Human-readable fix hint shown below the message.
    pub suggestion: Option<String>,
    /// Additional labeled spans rendered alongside the primary span.
    /// Each entry is `(span, label_message)`.
    pub secondary_labels: Vec<(Span, String)>,
}

impl Diagnostic {
    /// Build an error-level diagnostic.
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span,
            code: None,
            suggestion: None,
            secondary_labels: Vec::new(),
        }
    }

    /// Build a warning-level diagnostic.
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span,
            code: None,
            suggestion: None,
            secondary_labels: Vec::new(),
        }
    }

    /// Build a note-level diagnostic.
    pub fn note(message: impl Into<String>, span: Span) -> Self {
        Self {
            severity: Severity::Note,
            message: message.into(),
            span,
            code: None,
            suggestion: None,
            secondary_labels: Vec::new(),
        }
    }

    /// Attach a short error code.
    pub fn with_code(mut self, code: &'static str) -> Self {
        self.code = Some(code);
        self
    }

    /// Attach a fix suggestion shown to the user.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Attach an additional labeled span shown alongside the primary span.
    /// Used for two-location diagnostics such as E0301 (declaration + mutation site).
    pub fn with_secondary_label(mut self, span: Span, message: impl Into<String>) -> Self {
        self.secondary_labels.push((span, message.into()));
        self
    }
}

/// Trait for sinks that receive and display diagnostics.
pub trait Reporter {
    /// Emit a single diagnostic against the provided source text.
    fn report(&mut self, diagnostic: &Diagnostic, source: &str, filename: &str);

    /// Returns `true` if at least one error has been reported.
    fn has_errors(&self) -> bool;
}

// ── Rendering helpers ─────────────────────────────────────────────────────────

fn render_to_writer(
    writer: &mut dyn WriteColor,
    diagnostic: &Diagnostic,
    source: &str,
    filename: &str,
) {
    use codespan_reporting::diagnostic::{Diagnostic as CsDiag, Label, Severity as CsSeverity};
    use codespan_reporting::files::SimpleFiles;
    use codespan_reporting::term;

    let mut files: SimpleFiles<&str, &str> = SimpleFiles::new();
    let file_id = files.add(filename, source);

    let cs_severity = match diagnostic.severity {
        Severity::Error => CsSeverity::Error,
        Severity::Warning => CsSeverity::Warning,
        Severity::Note => CsSeverity::Note,
    };

    // TODO(Phase 5 - Refactor UTF-8 Column Alignment):
    // Byte ranges below are fed directly to `codespan_reporting`. That library
    // derives visual columns internally from byte offsets and will misplace
    // carets for multi-byte UTF-8 sequences if `start`/`end` do not land on
    // valid char boundaries (or for wide glyphs). Safe today because all input
    // is ASCII. Fix by validating char boundaries here, or adopt
    // `codespan_reporting`'s own location API and drop `Span::col` (see the
    // full resolution plan above the `Span` declaration).
    let mut labels = vec![
        Label::primary(file_id, diagnostic.span.start..diagnostic.span.end)
            .with_message(&diagnostic.message),
    ];

    for (span, msg) in &diagnostic.secondary_labels {
        labels.push(Label::secondary(file_id, span.start..span.end).with_message(msg));
    }

    let mut cs_diag = CsDiag::new(cs_severity)
        .with_message(&diagnostic.message)
        .with_labels(labels);

    if let Some(code) = diagnostic.code {
        cs_diag = cs_diag.with_code(code);
    }

    if let Some(ref suggestion) = diagnostic.suggestion {
        cs_diag = cs_diag.with_notes(vec![format!("suggestion: {suggestion}")]);
    }

    let config = term::Config::default();
    if let Err(e) = term::emit(writer, &config, &files, &cs_diag) {
        eprintln!("valua: failed to render diagnostic: {e}");
    }
}

/// Render a diagnostic to a plain string with no ANSI color codes.
/// Intended for tests that verify visual layout of error output.
pub fn render_diagnostic_to_string(
    diagnostic: &Diagnostic,
    source: &str,
    filename: &str,
) -> String {
    use codespan_reporting::term::termcolor::Buffer;
    let mut buf = Buffer::no_color();
    render_to_writer(&mut buf, diagnostic, source, filename);
    String::from_utf8_lossy(buf.as_slice()).into_owned()
}

/// Writes diagnostics to stderr using `codespan-reporting`.
pub struct ConsoleReporter {
    error_count: usize,
    color: ColorChoice,
}

impl ConsoleReporter {
    /// Create a reporter with explicit color control.
    pub fn new(color: ColorChoice) -> Self {
        Self {
            error_count: 0,
            color,
        }
    }

    /// Create a reporter that auto-detects color support on stderr.
    pub fn stderr() -> Self {
        Self::new(ColorChoice::Auto)
    }
}

impl Reporter for ConsoleReporter {
    fn report(&mut self, diagnostic: &Diagnostic, source: &str, filename: &str) {
        if diagnostic.severity == Severity::Error {
            self.error_count += 1;
        }
        let writer = StandardStream::stderr(self.color);
        let mut lock = writer.lock();
        render_to_writer(&mut lock, diagnostic, source, filename);
    }

    fn has_errors(&self) -> bool {
        self.error_count > 0
    }
}

/// A simple in-memory reporter useful for testing.
#[derive(Debug, Default)]
pub struct CollectingReporter {
    pub diagnostics: Vec<Diagnostic>,
}

impl Reporter for CollectingReporter {
    fn report(&mut self, diagnostic: &Diagnostic, _source: &str, _filename: &str) {
        self.diagnostics.push(diagnostic.clone());
    }

    fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: usize, end: usize, line: u32, col: u32) -> Span {
        Span::new(start, end, line, col)
    }

    #[test]
    fn test_span_merge_covers_both_endpoints() {
        let a = span(0, 5, 1, 1);
        let b = span(3, 10, 2, 3);
        let m = a.merge(b);
        assert_eq!(m.start, 0);
        assert_eq!(m.end, 10);
    }

    #[test]
    fn test_span_merge_col_from_earlier_span() {
        // col must come from the span with smaller start, not always self
        let earlier = span(0, 5, 1, 7);
        let later = span(6, 10, 1, 15);
        // self is later, other is earlier — col must still be 7
        let m = later.merge(earlier);
        assert_eq!(m.col, 7, "col should be from whichever span starts first");
    }

    #[test]
    fn test_span_display() {
        let s = span(0, 5, 3, 7);
        assert_eq!(format!("{s}"), "3:7");
    }

    #[test]
    fn test_diagnostic_builder() {
        let s = span(0, 1, 1, 1);
        let d = Diagnostic::error("bad token", s)
            .with_code("E0001")
            .with_suggestion("remove it");
        assert_eq!(d.severity, Severity::Error);
        assert_eq!(d.code, Some("E0001"));
        assert!(d.suggestion.is_some());
    }

    #[test]
    fn test_diagnostic_secondary_label() {
        let s1 = span(0, 5, 1, 1);
        let s2 = span(10, 15, 3, 1);
        let d = Diagnostic::error("mutation", s2).with_secondary_label(s1, "declared here");
        assert_eq!(d.secondary_labels.len(), 1);
        assert_eq!(d.secondary_labels[0].1, "declared here");
    }

    #[test]
    fn test_collecting_reporter_tracks_errors() {
        let mut r = CollectingReporter::default();
        let s = span(0, 1, 1, 1);
        assert!(!r.has_errors());
        r.report(&Diagnostic::warning("w", s), "", "f");
        assert!(!r.has_errors());
        r.report(&Diagnostic::error("e", s), "", "f");
        assert!(r.has_errors());
        assert_eq!(r.diagnostics.len(), 2);
    }
}
