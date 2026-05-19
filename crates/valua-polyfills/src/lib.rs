//! Embedded Lua polyfill strings injected during Lua 5.5 → 5.1 transpilation.

/// Bitwise operation emulation for pure Lua 5.1 (no native bit library).
///
/// Covers: `band`, `bor`, `bxor`, `bnot`, `lshift`, `rshift`.
/// Semantics match LuaJIT's `bit` library: 32-bit signed integer domain.
pub const BITWISE_FALLBACK: &str = r#"local _bU=0x100000000
local _bS=0x80000000
local function _bu(n) n=math.floor(n)%_bU if n<0 then n=n+_bU end return n end
local function _bs(n) n=_bu(n) if n>=_bS then n=n-_bU end return n end
local bit={}
function bit.band(a,b) a=_bu(a) b=_bu(b) local r=0 local m=1 while a>0 or b>0 do if a%2==1 and b%2==1 then r=r+m end a=math.floor(a/2) b=math.floor(b/2) m=m*2 end return _bs(r) end
function bit.bor(a,b) a=_bu(a) b=_bu(b) local r=0 local m=1 while a>0 or b>0 do if a%2==1 or b%2==1 then r=r+m end a=math.floor(a/2) b=math.floor(b/2) m=m*2 end return _bs(r) end
function bit.bxor(a,b) a=_bu(a) b=_bu(b) local r=0 local m=1 while a>0 or b>0 do if a%2~=b%2 then r=r+m end a=math.floor(a/2) b=math.floor(b/2) m=m*2 end return _bs(r) end
function bit.bnot(a) return _bs(0xFFFFFFFF-_bu(a)) end
function bit.lshift(a,n) n=n%32 a=_bu(a) for _=1,n do a=(a*2)%_bU end return _bs(a) end
function bit.rshift(a,n) n=n%32 a=_bu(a) for _=1,n do a=math.floor(a/2) end return _bs(a) end"#;

/// Runtime helper for `<close>` attribute semantics (deferred cleanup via pcall).
pub const CLOSE_RUNTIME: &str = "";

/// Extensions to the `string` library present in 5.4/5.5 but absent in 5.1.
pub const STRING_EXTENSIONS: &str = "";

/// Extensions to the `math` library present in 5.4/5.5 but absent in 5.1
/// (e.g., `math.tointeger`, `math.type`).
pub const MATH_EXTENSIONS: &str = "";

/// Flags indicating which Lua 5.5 features are used in a translation unit.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FeatureSet {
    /// Any of `&`, `|`, `~` (binary xor), `~` (unary not), `<<`, `>>`.
    pub bitwise_ops: bool,
    /// Any `local x <close>` declaration.
    pub close_attribute: bool,
    /// `//` integer division (Lua 5.3+, absent in Lua 5.1 / LuaJIT).
    pub integer_div: bool,
    pub string_extensions: bool,
    pub math_extensions: bool,
}

impl FeatureSet {
    /// Returns `true` if no polyfills are needed.
    pub fn is_empty(&self) -> bool {
        !self.bitwise_ops
            && !self.close_attribute
            && !self.integer_div
            && !self.string_extensions
            && !self.math_extensions
    }
}

/// Concatenates the polyfill strings required by `features` into a single Lua chunk.
///
/// Returns an empty string when no features are active.
pub fn polyfills_for(features: &FeatureSet) -> String {
    let mut parts: Vec<&'static str> = Vec::new();

    if features.bitwise_ops {
        parts.push(BITWISE_FALLBACK);
    }
    if features.close_attribute {
        parts.push(CLOSE_RUNTIME);
    }
    // integer_div is lowered by BitwiseOpTransform's sibling pass, not a preamble string.
    if features.string_extensions {
        parts.push(STRING_EXTENSIONS);
    }
    if features.math_extensions {
        parts.push(MATH_EXTENSIONS);
    }

    parts.retain(|s| !s.is_empty());
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_polyfills_when_empty() {
        let fs = FeatureSet::default();
        assert!(fs.is_empty());
        assert_eq!(polyfills_for(&fs), "");
    }

    #[test]
    fn test_bitwise_polyfill_included() {
        let fs = FeatureSet {
            bitwise_ops: true,
            ..Default::default()
        };
        assert!(!fs.is_empty());
        // BITWISE_FALLBACK is "" until Phase 3 fills it; routing must not panic.
        let _ = polyfills_for(&fs);
    }

    #[test]
    fn test_feature_set_is_empty_requires_all_false() {
        let mut fs = FeatureSet::default();
        assert!(fs.is_empty());

        fs.bitwise_ops = true;
        assert!(!fs.is_empty());
        fs.bitwise_ops = false;

        fs.close_attribute = true;
        assert!(!fs.is_empty());
        fs.close_attribute = false;

        fs.integer_div = true;
        assert!(!fs.is_empty());
        fs.integer_div = false;

        fs.string_extensions = true;
        assert!(!fs.is_empty());
        fs.string_extensions = false;

        fs.math_extensions = true;
        assert!(!fs.is_empty());
    }

    #[test]
    fn test_polyfills_for_bitwise_returns_nonempty() {
        // BITWISE_FALLBACK is now filled (Phase 3); bitwise_ops must produce output.
        let fs = FeatureSet {
            bitwise_ops: true,
            ..Default::default()
        };
        assert!(!polyfills_for(&fs).is_empty());
    }

    #[test]
    fn test_polyfills_for_close_only_still_empty() {
        // CLOSE_RUNTIME is still "" until Phase 4.
        let fs = FeatureSet {
            close_attribute: true,
            ..Default::default()
        };
        assert_eq!(polyfills_for(&fs), "");
    }

    #[test]
    fn test_integer_div_not_in_polyfill_output() {
        // integer_div is handled by transform pass, not a polyfill string.
        let fs = FeatureSet {
            integer_div: true,
            ..Default::default()
        };
        // Should not panic and should return empty (no string for integer_div).
        assert_eq!(polyfills_for(&fs), "");
    }
}
