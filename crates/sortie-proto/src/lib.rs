//! Generated protobuf code for Sortie, plus hand-written helpers.
//!
//! Module layout mirrors the protobuf packages:
//!
//! * [`v1`] — `sortie.v1`, the public API (messages + tonic clients/servers with feature `grpc`);
//! * [`manifest::v1`] — `sortie.manifest.v1`, the mission manifest (messages only);
//! * `link::v1` — `sortie.link.v1`, the private bridge <-> engine link (messages only), with
//!   the feature `link` (the engine side; not part of the SDK).
//!
//! Hand-written helpers live in [`lua`] (LuaValue constructors, accessors, `From` impls and a
//! serde bridge), [`eid`] (packing of entity ids) and [`FILE_DESCRIPTOR_SET`] (reflection).
//!
//! This is the only crate that contains generated code, so a future move from tonic to the
//! `grpc` crate touches only this crate.

pub mod eid;
pub mod lua;

/// `sortie.v1`: the public API.
#[allow(
    clippy::all,
    clippy::pedantic,
    missing_docs,
    rustdoc::invalid_html_tags,
    rustdoc::bare_urls
)]
pub mod v1 {
    include!(concat!(env!("OUT_DIR"), "/sortie.v1.rs"));
}

/// `sortie.link.*`: the private bridge <-> engine link (feature `link`).
#[cfg(feature = "link")]
pub mod link {
    /// `sortie.link.v1`.
    #[allow(
        clippy::all,
        clippy::pedantic,
        missing_docs,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/sortie.link.v1.rs"));
    }
}

/// `sortie.manifest.*`: the mission manifest.
pub mod manifest {
    /// `sortie.manifest.v1`.
    #[allow(
        clippy::all,
        clippy::pedantic,
        missing_docs,
        rustdoc::invalid_html_tags,
        rustdoc::bare_urls
    )]
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/sortie.manifest.v1.rs"));
    }
}

/// Largest gRPC message of the `Logic` service, both directions: the engine's server and the
/// SDK's client set it as their decoding and encoding limit. `Start` and `LogicDown.resync`
/// carry the whole world (thousands of units), which outgrows tonic's default decoding limit of
/// 4 MiB.
pub const LOGIC_MAX_MESSAGE: usize = 64 * 1024 * 1024;

/// Encoded `google.protobuf.FileDescriptorSet` of every compiled Sortie proto file (for
/// `tonic-reflection` and tools).
pub const FILE_DESCRIPTOR_SET: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/sortie_descriptor.bin"));

pub use lua::{LuaSerdeError, from_lua, to_lua};
pub use v1::{LuaNil, LuaPair, LuaTable, LuaValue};
