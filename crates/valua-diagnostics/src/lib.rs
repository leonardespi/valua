//! Diagnostic types for reporting errors, warnings, and notes with source spans.

use codespan_reporting::term::termcolor::ColorChoice;

#[cfg(feature = "serde")]
use serde::Serialize;

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
    /// 1-based column number.
    pub col: u32,
}

impl Span {
    /// Create a new span from raw fields.
    pub fn new(start: usize, end: usize, line: u32, col: u32) -> Self {
        Self { start, end, line, col }
    }

    /// Placeholder span for generated nodes that have no source location.
    pub fn dummy() -> Self {
        Self { start: 0, end: 0, line: 0, col: 0 }
    }

    /// Merge two spans into one that covers both.
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            line: self.line.min(other.line),
            col: self.col,
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

/// A structured diagnostic message with optional error code and fix suggestion.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
    /// Short error code, e.g. `"E0001"`.
    pub code: Option<&'static str>,
    /// Human-readable fix hint shown below the message.
    pub suggestion: Option<String>,
}

impl Diagnostic {
    /// Build an error-level diagnostic.
    pub fn error(message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Error, message: message.into(), span, code: None, suggestion: None }
    }

    /// Build a warning-level diagnostic.
    pub fn warning(message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Warning, message: message.into(), span, code: None, suggestion: None }
    }

    /// Build a note-level diagnostic.
    pub fn note(message: impl Into<String>, span: Span) -> Self {
        Self { severity: Severity::Note, message: message.into(), span, code: None, suggestion: None }
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
}

/// Trait for sinks that receive and display diagnostics.
pub trait Reporter {
    /// Emit a single diagnostic against the provided source text.
    fn report(&mut self, diagnostic: &Diagnostic, source: &str, filename: &str);

    /// Returns `true` if at least one error has been reported.
    fn has_errors(&self) -> bool;
}

/// Writes diagnostics to stderr using `codespan-reporting`.
pub struct ConsoleReporter {
    error_count: usize,
    #[allow(dead_code)]
    color: ColorChoice,
}

impl ConsoleReporter {
    /// Create a reporter with explicit color control.
    pub fn new(color: ColorChoice) -> Self {
        Self { error_count: 0, color }
    }

    /// Create a reporter that auto-detects color support on stderr.
    pub fn stderr() -> Self {
        Self::new(ColorChoice::Auto)
    }
}

impl Reporter for ConsoleReporter {
    fn report(&mut self, diagnostic: &Diagnostic, _source: &str, _filename: &str) {
        if diagnostic.severity == Severity::Error {
            self.error_count += 1;
        }
        // TODO: render via codespan_reporting::term::emit
        todo!("wire up codespan-reporting term::emit for pretty terminal output")
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
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: verify Span::merge covers both endpoints"]
    fn test_span_merge() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: verify Diagnostic builder chains code and suggestion"]
    fn test_diagnostic_builder() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: CollectingReporter::has_errors is true after an error"]
    fn test_collecting_reporter_tracks_errors() {
        todo!()
    }
}
