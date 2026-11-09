//! Mission manifests (`manifest.pb`) and scenario lock files (`scenario.lock.json`).
//!
//! * [`Manifest`] is the prost type `sortie.manifest.v1.Manifest`; [`load_manifest`] reads a
//!   `.pb` or `.json` file, [`decode_manifest`] / [`json::manifest_from_json`] decode bytes or
//!   text, [`json::manifest_to_json`] writes the human-readable `manifest.json` exactly like
//!   the Python generator, [`validate`] checks every rule (names, ids, geometry, templates).
//! * [`ScenarioLock`] is the typed form of `scenario.lock.json`; [`schema_sha256`] computes the
//!   canonical schema hash that Python (`sortie-gen`) must reproduce byte for byte;
//!   [`lock::compare`] tells how a lock and a manifest disagree.
//! * [`naming`] holds the runtime naming rules (`<template>#<session4>-<n>`) and the
//!   uniqueness check of explicit names.
//! * [`digest`] is the `.miz` bootstrap digest: sha256 of the exact `manifest.pb` bytes.

pub mod json;
pub mod lock;
pub mod naming;
pub mod validate;

use std::path::Path;

use prost::Message as _;
use sha2::{Digest as _, Sha256};

pub use json::{JsonError, manifest_from_json, manifest_to_json};
pub use lock::{
    GeometryLock, IdentSymbol, KindSymbol, LOCK_FORMAT, LockDiff, LockError, ScenarioLock,
    SymbolSet, ZoneGeometry, canonical_json, compare, schema_sha256,
};
pub use sortie_proto::manifest::v1 as pb;
pub use sortie_proto::manifest::v1::Manifest;
pub use validate::{InvalidManifest, Issue, ensure_valid, validate};

/// Errors reading or writing manifests and lock files.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("cannot read {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not a valid sortie.manifest.v1.Manifest: {source}")]
    Decode {
        path: String,
        #[source]
        source: prost::DecodeError,
    },
    #[error("{path}: {source}")]
    Json {
        path: String,
        #[source]
        source: JsonError,
    },
    #[error(transparent)]
    Lock(#[from] LockError),
}

/// Decodes `manifest.pb` bytes.
pub fn decode_manifest(bytes: &[u8]) -> Result<Manifest, prost::DecodeError> {
    Manifest::decode(bytes)
}

/// Reads a manifest file: `*.json` as proto3 JSON (`manifest.json`), anything else as
/// protobuf (`manifest.pb`). Remember that the digest is only defined over the `.pb` bytes.
pub fn load_manifest(path: impl AsRef<Path>) -> Result<Manifest, ManifestError> {
    let path = path.as_ref();
    let is_json = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("json"));
    let bytes = std::fs::read(path).map_err(|source| ManifestError::Io {
        path: path.display().to_string(),
        source,
    })?;
    if is_json {
        let text = String::from_utf8(bytes).map_err(|e| ManifestError::Json {
            path: path.display().to_string(),
            source: JsonError {
                path: "<root>".into(),
                message: format!("not UTF-8: {e}"),
            },
        })?;
        return manifest_from_json(&text).map_err(|source| ManifestError::Json {
            path: path.display().to_string(),
            source,
        });
    }
    decode_manifest(&bytes).map_err(|source| ManifestError::Decode {
        path: path.display().to_string(),
        source,
    })
}

/// Encodes a manifest to `manifest.pb` bytes (prost encoding; the generator's Python encoding
/// may differ in byte order of map entries, which is why the digest is taken over the bytes as
/// registered, never over a re-encoding).
#[must_use]
pub fn encode_manifest(m: &Manifest) -> Vec<u8> {
    m.encode_to_vec()
}

/// Lowercase hex sha256 of the exact `manifest.pb` bytes: the `digest` of
/// `SORTIE.attach{...}` in the `.miz` bootstrap and of `RegisterResponse.digest`.
#[must_use]
pub fn digest(manifest_pb: &[u8]) -> String {
    hex::encode(Sha256::digest(manifest_pb))
}

/// Looks up a group by symbol.
#[must_use]
pub fn group<'m>(m: &'m Manifest, ident: &str) -> Option<&'m pb::Group> {
    m.groups.iter().find(|g| g.ident == ident)
}

/// Looks up a zone by symbol.
#[must_use]
pub fn zone<'m>(m: &'m Manifest, ident: &str) -> Option<&'m pb::Zone> {
    m.zones.iter().find(|z| z.ident == ident)
}

/// Looks up a template by symbol.
#[must_use]
pub fn template<'m>(m: &'m Manifest, ident: &str) -> Option<&'m pb::Template> {
    m.templates.iter().find(|t| t.ident == ident)
}

/// Looks up a flag by symbol.
#[must_use]
pub fn flag<'m>(m: &'m Manifest, ident: &str) -> Option<&'m pb::Flag> {
    m.flags.iter().find(|f| f.ident == ident)
}

/// Looks up an airbase by symbol.
#[must_use]
pub fn airbase<'m>(m: &'m Manifest, ident: &str) -> Option<&'m pb::Airbase> {
    m.airbases.iter().find(|a| a.ident == ident)
}

/// Looks up a menu by symbol.
#[must_use]
pub fn menu<'m>(m: &'m Manifest, ident: &str) -> Option<&'m pb::Menu> {
    m.menus.iter().find(|x| x.ident == ident)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_lowercase_hex_sha256() {
        assert_eq!(
            digest(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
