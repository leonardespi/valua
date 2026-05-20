#![no_main]
use libfuzzer_sys::fuzz_target;

// Invariant: Lexer::tokenize() must never panic on any byte sequence.
// Returns Err(LexError) on invalid input; Err is acceptable. Panic is not.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = valua_lexer::Lexer::new(s).tokenize();
    }
});
