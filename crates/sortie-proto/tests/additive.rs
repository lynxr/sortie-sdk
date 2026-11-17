//! Additivity of the public protocol: every message field, enum value and rpc of the v0.1.0
//! release keeps its full name, number, type, label and oneof (spec section 8.4).
//!
//! `tests/baseline/v0_1_0.txt` lists them, one line each, as written by [`describe`] from the
//! v0.1.0 descriptor set. Files are not part of a line: a message may move to another file of
//! the same package (GroupSel and UnitSel moved from commands.proto to common.proto in 0.2.0),
//! which keeps the wire format and the full name. The link package is private and not listed.
//!
//! `SORTIE_WRITE_PROTO_BASELINE=1 cargo test -p sortie-proto --test additive` rewrites the
//! file from the current tree; that is only right on the release that defines the baseline.

use std::collections::BTreeSet;
use std::path::PathBuf;

use prost::Message as _;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, EnumDescriptorProto, FileDescriptorSet};

fn baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/baseline/v0_1_0.txt")
}

fn type_name(t: Type) -> &'static str {
    match t {
        Type::Double => "double",
        Type::Float => "float",
        Type::Int64 => "int64",
        Type::Uint64 => "uint64",
        Type::Int32 => "int32",
        Type::Fixed64 => "fixed64",
        Type::Fixed32 => "fixed32",
        Type::Bool => "bool",
        Type::String => "string",
        Type::Group => "group",
        Type::Message => "message",
        Type::Bytes => "bytes",
        Type::Uint32 => "uint32",
        Type::Enum => "enum",
        Type::Sfixed32 => "sfixed32",
        Type::Sfixed64 => "sfixed64",
        Type::Sint32 => "sint32",
        Type::Sint64 => "sint64",
    }
}

fn enum_lines(prefix: &str, e: &EnumDescriptorProto, out: &mut BTreeSet<String>) {
    let full = format!("{prefix}.{}", e.name());
    for v in &e.value {
        out.insert(format!("enum {full}.{} = {}", v.name(), v.number()));
    }
}

fn message_lines(prefix: &str, m: &DescriptorProto, out: &mut BTreeSet<String>) {
    let full = format!("{prefix}.{}", m.name());
    for f in &m.field {
        let label = match f.label() {
            Label::Optional if f.proto3_optional() => "proto3_optional",
            Label::Optional => "singular",
            Label::Repeated => "repeated",
            Label::Required => "required",
        };
        // A proto3 `optional` lives in a synthetic oneof; only real oneofs are part of a line.
        let oneof = match f.oneof_index {
            Some(i) if !f.proto3_optional() => format!(
                " oneof {}",
                m.oneof_decl
                    .get(usize::try_from(i).unwrap_or(usize::MAX))
                    .map_or("?", |o| o.name())
            ),
            _ => String::new(),
        };
        let referenced = if f.type_name().is_empty() {
            String::new()
        } else {
            format!(" {}", f.type_name())
        };
        out.insert(format!(
            "field {full}.{} = {} {label} {}{referenced}{oneof}",
            f.name(),
            f.number(),
            type_name(f.r#type()),
        ));
    }
    // Map entry types are nested messages too (`<Field>Entry`, key = 1, value = 2).
    for nested in &m.nested_type {
        message_lines(&full, nested, out);
    }
    for e in &m.enum_type {
        enum_lines(&full, e, out);
    }
}

/// One line per field, enum value and rpc of the public packages.
fn describe(set: &FileDescriptorSet) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for file in &set.file {
        let package = file.package();
        if package.starts_with("sortie.link") || !package.starts_with("sortie.") {
            continue;
        }
        let prefix = format!(".{package}");
        for m in &file.message_type {
            message_lines(&prefix, m, &mut out);
        }
        for e in &file.enum_type {
            enum_lines(&prefix, e, &mut out);
        }
        for s in &file.service {
            for rpc in &s.method {
                out.insert(format!(
                    "rpc {prefix}.{}.{}({}{}) -> {}{}",
                    s.name(),
                    rpc.name(),
                    if rpc.client_streaming() {
                        "stream "
                    } else {
                        ""
                    },
                    rpc.input_type(),
                    if rpc.server_streaming() {
                        "stream "
                    } else {
                        ""
                    },
                    rpc.output_type(),
                ));
            }
        }
    }
    out
}

fn current() -> BTreeSet<String> {
    let set = FileDescriptorSet::decode(sortie_proto::FILE_DESCRIPTOR_SET).expect("descriptor set");
    describe(&set)
}

#[test]
fn every_v0_1_0_field_enum_value_and_rpc_is_unchanged() {
    let now = current();
    let path = baseline_path();
    if std::env::var_os("SORTIE_WRITE_PROTO_BASELINE").is_some() {
        let mut text = String::from(
            "# sortie.v1 and sortie.manifest.v1 as released in v0.1.0 (see tests/additive.rs).\n",
        );
        for line in &now {
            text.push_str(line);
            text.push('\n');
        }
        std::fs::write(&path, text).expect("write baseline");
        return;
    }
    let text = std::fs::read_to_string(&path).expect("read the baseline");
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    // v0.1.0 has 693 lines; a truncated file must not pass as "nothing changed".
    assert!(lines.len() >= 690, "the baseline is truncated");
    let missing: Vec<&str> = lines.into_iter().filter(|l| !now.contains(*l)).collect();
    assert!(
        missing.is_empty(),
        "the protocol is additive only; these v0.1.0 lines changed or vanished:\n{}",
        missing.join("\n")
    );
}

#[test]
fn moved_selectors_keep_their_names_and_wire_format() {
    let set = FileDescriptorSet::decode(sortie_proto::FILE_DESCRIPTOR_SET).expect("descriptor set");
    let common = set
        .file
        .iter()
        .find(|f| f.name() == "sortie/v1/common.proto")
        .expect("common.proto");
    for name in ["GroupSel", "UnitSel"] {
        assert!(
            common.message_type.iter().any(|m| m.name() == name),
            "{name} lives in common.proto since 0.2.0"
        );
    }
    // The bytes of a selector written by a v0.1.0 peer decode to the same value.
    let sel = sortie_proto::v1::GroupSel {
        sel: Some(sortie_proto::v1::group_sel::Sel::Name("Red CAP 1".into())),
    };
    let bytes = sel.encode_to_vec();
    assert_eq!(
        bytes[0],
        (2 << 3) | 2,
        "GroupSel.name is field 2, length-delimited"
    );
    assert_eq!(
        sortie_proto::v1::GroupSel::decode(bytes.as_slice()).ok(),
        Some(sel)
    );
    let unit = sortie_proto::v1::UnitSel {
        sel: Some(sortie_proto::v1::unit_sel::Sel::Eid(
            sortie_proto::eid::pack(3, 1),
        )),
    };
    let bytes = unit.encode_to_vec();
    assert_eq!(bytes[0], 1 << 3, "UnitSel.eid is field 1, varint");
    assert_eq!(
        sortie_proto::v1::UnitSel::decode(bytes.as_slice()).ok(),
        Some(unit)
    );
}
