use valua_diagnostics::{render_diagnostic_to_string, Diagnostic, Span};

// "local x = 42": '4' is at byte 10, col 11 (1-based)
const SOURCE_SINGLE: &str = "local x = 42";
const COL_42: u32 = 11;

fn span(start: usize, end: usize, line: u32, col: u32) -> Span {
    Span::new(start, end, line, col)
}

#[test]
fn test_console_reporter_visual_alignment() {
    let diagnostic = Diagnostic::error("integer overflow", span(10, 12, 1, COL_42))
        .with_code("E0101")
        .with_suggestion("use a float literal instead");

    let output = render_diagnostic_to_string(&diagnostic, SOURCE_SINGLE, "test.lua");

    // Structural checks
    assert!(
        output.contains("error[E0101]:"),
        "missing error code header\n{output}"
    );
    assert!(
        output.contains("test.lua:1:11"),
        "missing file:line:col\n{output}"
    );
    assert!(
        output.contains(SOURCE_SINGLE),
        "missing source line\n{output}"
    );
    assert!(
        output.contains("use a float literal instead"),
        "missing suggestion\n{output}"
    );

    // Caret column alignment.
    // codespan-reporting uses '│' (U+2502) as gutter separator, not ASCII '|'.
    // Caret line format: "  │<spaces>^^ message"
    // After '│', codespan writes (col-1) spaces before the first '^', giving total = col-1.
    let caret_line = output
        .lines()
        .find(|l| l.contains('^'))
        .unwrap_or_else(|| panic!("no caret line in output:\n{output}"));

    const GUTTER_SEP: char = '│';
    let pipe_byte = caret_line
        .find(GUTTER_SEP)
        .expect("caret line must contain '│' gutter separator");
    let after_pipe = &caret_line[pipe_byte + GUTTER_SEP.len_utf8()..];
    let spaces = after_pipe.chars().take_while(|&c| c == ' ').count();
    // codespan: after '│', places 1 gutter space + (col-1) source-content spaces = col spaces
    // before the first '^', so the caret visually aligns under the error token.
    assert_eq!(
        spaces, COL_42 as usize,
        "caret misaligned: got {spaces} spaces after '│', expected {COL_42}\n{caret_line}"
    );
}

#[test]
fn test_console_reporter_secondary_label_alignment() {
    // "local <const> x = 1\nx = 2"
    // 'x' declaration: byte 14, col 15 (line 1)
    // 'x' assignment:  byte 20, col 1  (line 2)
    let source = "local <const> x = 1\nx = 2";
    let decl_span = span(14, 15, 1, 15);
    let mutate_span = span(20, 21, 2, 1);

    let diagnostic = Diagnostic::error("cannot assign to const variable", mutate_span)
        .with_code("E0301")
        .with_secondary_label(decl_span, "declared as const here")
        .with_suggestion("remove the <const> attribute");

    let output = render_diagnostic_to_string(&diagnostic, source, "test.lua");

    assert!(
        output.contains("error[E0301]:"),
        "missing error code\n{output}"
    );
    assert!(
        output.contains("test.lua:2:1"),
        "missing mutation site location\n{output}"
    );
    assert!(
        output.contains("declared as const here"),
        "missing secondary label\n{output}"
    );
    assert!(
        output.contains("remove the <const> attribute"),
        "missing suggestion\n{output}"
    );
}
