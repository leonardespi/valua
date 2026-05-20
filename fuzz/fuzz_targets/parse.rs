#![no_main]
use libfuzzer_sys::fuzz_target;

// Invariant: parse() must never panic on any byte sequence.
// Returns Err(ParseError) on invalid input; Err is acceptable. Panic is not.
fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = valua_parser::parse(s);
    }
});
