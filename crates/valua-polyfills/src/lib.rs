//! Embedded Lua polyfill strings injected during Lua 5.4 → 5.1 transpilation.

/// Bitwise operation emulation via LuaJIT `bit` / `bit32` fallback table.
///
/// Covers: `band`, `bor`, `bxor`, `bnot`, `lshift`, `rshift`.
pub const BITWISE_FALLBACK: &str = "";

/// Runtime helper for `<close>` attribute semantics (deferred cleanup via pcall).
pub const CLOSE_RUNTIME: &str = "";

/// Extensions to the `string` library present in 5.4 but absent in 5.1.
pub const STRING_EXTENSIONS: &str = "";

/// Extensions to the `math` library present in 5.4 but absent in 5.1
/// (e.g., `math.tointeger`, `math.type`).
pub const MATH_EXTENSIONS: &str = "";

/// Flags indicating which Lua 5.4 features are used in a translation unit.
#[derive(Debug, Clone, Default)]
pub struct FeatureSet {
    pub bitwise_ops: bool,
    pub close_attribute: bool,
    pub string_extensions: bool,
    pub math_extensions: bool,
}

impl FeatureSet {
    /// Returns `true` if no polyfills are needed.
    pub fn is_empty(&self) -> bool {
        !self.bitwise_ops
            && !self.close_attribute
            && !self.string_extensions
            && !self.math_extensions
    }
}

/// Concatenates the polyfill strings required by `features` into a single Lua chunk.
///
/// Returns an empty string when no features are active.
pub fn polyfills_for(features: &FeatureSet) -> String {
    // TODO: collect required polyfill strings and join with newlines
    let mut parts: Vec<&'static str> = Vec::new();

    if features.bitwise_ops {
        parts.push(BITWISE_FALLBACK);
    }
    if features.close_attribute {
        parts.push(CLOSE_RUNTIME);
    }
    if features.string_extensions {
        parts.push(STRING_EXTENSIONS);
    }
    if features.math_extensions {
        parts.push(MATH_EXTENSIONS);
    }

    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "TODO: polyfills_for returns empty string when FeatureSet is default"]
    fn test_no_polyfills_when_empty() {
        todo!()
    }

    #[test]
    #[ignore = "TODO: polyfills_for includes BITWISE_FALLBACK when bitwise_ops is set"]
    fn test_bitwise_polyfill_included() {
        todo!()
    }
}
