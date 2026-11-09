//! The [`Scenario`] trait implemented by generated scenario markers (e.g. `OpNorth`).

use std::fmt::{self, Debug};

/// Implemented by the marker type `sortie-build` generates for each scenario.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a scenario marker",
    label = "not a scenario marker",
    note = "scenario markers are generated: `sortie_build::scenario(\"op_north\", ..)` in \
            build.rs and `sortie::include_scenario!(op_north)` define `op_north::OpNorth`, used \
            as `Ctx<op_north::OpNorth>`"
)]
pub trait Scenario: Copy + Default + Debug + Send + Sync + 'static {
    /// Scenario name, e.g. `"op_north"`.
    const NAME: &'static str;
    /// Schema number from scenario.lock.json.
    const SCHEMA: u32;
    /// Lowercase hex sha256 of the canonical symbol set (scenario.lock.json `schema_sha256`).
    const SCHEMA_SHA256: &'static str;
    /// Every symbol of the lock file, sorted by kind then ident.
    const SYMBOLS: &'static [Symbol];
}

/// What a symbol names.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SymbolKind {
    Group,
    Zone,
    Template,
    Flag,
    Airbase,
    Menu,
}

impl SymbolKind {
    /// Spelling used in `symbols_used` keys ("group", "zone", ...).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            SymbolKind::Group => "group",
            SymbolKind::Zone => "zone",
            SymbolKind::Template => "template",
            SymbolKind::Flag => "flag",
            SymbolKind::Airbase => "airbase",
            SymbolKind::Menu => "menu",
        }
    }
}

/// One symbol of a scenario.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Symbol {
    pub kind: SymbolKind,
    pub ident: &'static str,
}

impl Symbol {
    #[must_use]
    pub const fn new(kind: SymbolKind, ident: &'static str) -> Self {
        Self { kind, ident }
    }

    /// `"<kind>:<IDENT>"`, the form used in `sortie.v1.Attach.symbols_used`.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind.as_str(), self.ident)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind.as_str(), self.ident)
    }
}

/// A malformed `SCHEMA_SHA256`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("schema_sha256 must be 64 lowercase hex characters, got {0:?}")]
pub struct SchemaHashError(pub String);

/// The raw 32 bytes of `M::SCHEMA_SHA256`, as sent in `sortie.v1.Attach.scenario_schema_sha256`.
pub fn schema_sha256_bytes<M: Scenario>() -> Result<[u8; 32], SchemaHashError> {
    decode_hex32(M::SCHEMA_SHA256)
}

pub(crate) fn decode_hex32(s: &str) -> Result<[u8; 32], SchemaHashError> {
    let err = || SchemaHashError(s.to_owned());
    let bytes = s.as_bytes();
    if bytes.len() != 64 {
        return Err(err());
    }
    let nibble = |c: u8| match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        _ => None,
    };
    let mut out = [0u8; 32];
    for (i, &[hi, lo]) in bytes.as_chunks::<2>().0.iter().enumerate() {
        out[i] = (nibble(hi).ok_or_else(err)? << 4) | nibble(lo).ok_or_else(err)?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Default)]
    struct S;
    impl Scenario for S {
        const NAME: &'static str = "s";
        const SCHEMA: u32 = 1;
        const SCHEMA_SHA256: &'static str =
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
        const SYMBOLS: &'static [Symbol] = &[Symbol::new(SymbolKind::Group, "SAM_NORTH")];
    }

    #[test]
    fn hash_bytes_and_keys() {
        let b = schema_sha256_bytes::<S>().unwrap();
        assert_eq!(b[0], 0);
        assert_eq!(b[31], 0x1f);
        assert_eq!(S::SYMBOLS[0].key(), "group:SAM_NORTH");
        assert!(decode_hex32("ABCD").is_err());
        assert!(decode_hex32(&"G".repeat(64)).is_err());
    }
}
