//! Runtime naming rules for spawned groups and units.
//!
//! * Group: `<template name>#<session4>-<n>`, where `session4` is the last 4 characters of the
//!   session ULID and `n` counts spawns of that template in the session, starting at 1.
//!   Example: template "Red CAP", session `01JC6Z...7XQ2`, first spawn -> `Red CAP#7XQ2-1`.
//! * `.named("Red CAP 1")` replaces the generated group name. Explicit names follow
//!   [`check_group_name`] (no `#`, which is reserved for generated names) and must be unique:
//!   the engine rejects a name that a live group already has with `ERROR_CODE_NAME_CONFLICT`
//!   (DCS's `coalition.addGroup` would silently destroy the other group).
//! * Units of a spawned group: `<group name>-<k>`, `k` from 1 in template order.
//!   Example: `Red CAP 1-1`, `Red CAP 1-2`.
//! * Runtime spawns never set groupId/unitId; DCS assigns them (and the generator never
//!   renumbers the ids it wrote into the .miz).

use std::collections::HashMap;
use std::fmt;

/// Longest DCS name we accept, in bytes.
pub const MAX_NAME_BYTES: usize = 128;

/// Separator between a template name and the session part of a generated group name.
pub const RUNTIME_SEPARATOR: char = '#';

/// Last 4 characters of a session ULID (the whole string if shorter).
#[must_use]
pub fn session4(session: &str) -> &str {
    let start = session.char_indices().rev().nth(3).map_or(0, |(i, _)| i);
    &session[start..]
}

/// `<template>#<session4>-<n>`.
#[must_use]
pub fn runtime_group_name(template_name: &str, session: &str, n: u32) -> String {
    format!(
        "{template_name}{RUNTIME_SEPARATOR}{}-{n}",
        session4(session)
    )
}

/// `<group>-<k>` (k starts at 1).
#[must_use]
pub fn unit_name(group_name: &str, k: u32) -> String {
    format!("{group_name}-{k}")
}

/// Unit names `<group>-1 ..= <group>-<count>`.
#[must_use]
pub fn unit_names(group_name: &str, count: usize) -> Vec<String> {
    (1..=count)
        .map(|k| unit_name(group_name, u32::try_from(k).unwrap_or(u32::MAX)))
        .collect()
}

/// The parts of a generated group name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeName<'a> {
    pub template: &'a str,
    pub session4: &'a str,
    pub n: u32,
}

/// Splits `Red CAP#7XQ2-1` into its parts; `None` for names that are not generated.
#[must_use]
pub fn parse_runtime_group_name(name: &str) -> Option<RuntimeName<'_>> {
    let (template, rest) = name.rsplit_once(RUNTIME_SEPARATOR)?;
    let (s4, n) = rest.split_once('-')?;
    let n: u32 = n.parse().ok().filter(|n| *n >= 1)?;
    let ok = !template.is_empty()
        && !s4.is_empty()
        && s4.chars().count() <= 4
        && s4.chars().all(|c| c.is_ascii_alphanumeric());
    ok.then_some(RuntimeName {
        template,
        session4: s4,
        n,
    })
}

/// Why a name is not acceptable.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NameError {
    #[error("a group name must not be empty")]
    Empty,
    #[error("group name {0:?} is longer than {MAX_NAME_BYTES} bytes")]
    TooLong(String),
    #[error("group name {0:?} starts or ends with whitespace")]
    Whitespace(String),
    #[error("group name {0:?} contains a control character")]
    Control(String),
    #[error(
        "group name {0:?} contains '#', which is reserved for generated names (<template>#<session4>-<n>)"
    )]
    Reserved(String),
    #[error("a group named {0:?} already exists (DCS would destroy it); choose another name")]
    Conflict(String),
}

/// Rules for an explicit `.named("...")` group name.
pub fn check_group_name(name: &str) -> Result<(), NameError> {
    check_dcs_name(name)?;
    if name.contains(RUNTIME_SEPARATOR) {
        return Err(NameError::Reserved(name.to_owned()));
    }
    Ok(())
}

/// Rules for any DCS group/unit name (generated names included).
pub fn check_dcs_name(name: &str) -> Result<(), NameError> {
    if name.is_empty() {
        return Err(NameError::Empty);
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(NameError::TooLong(name.to_owned()));
    }
    if name.trim() != name {
        return Err(NameError::Whitespace(name.to_owned()));
    }
    if name.chars().any(char::is_control) {
        return Err(NameError::Control(name.to_owned()));
    }
    Ok(())
}

/// Checks an explicit name and that `is_taken` does not know it.
pub fn check_unique_group_name(
    name: &str,
    is_taken: impl Fn(&str) -> bool,
) -> Result<(), NameError> {
    check_group_name(name)?;
    if is_taken(name) {
        return Err(NameError::Conflict(name.to_owned()));
    }
    Ok(())
}

/// Hands out generated group names for one session: `<template>#<session4>-<n>` with `n`
/// counting per template from 1. A name that is somehow taken already (e.g. a group restored
/// after a restart) is skipped.
#[derive(Clone, Debug)]
pub struct RuntimeNamer {
    session4: String,
    next: HashMap<String, u32>,
}

impl RuntimeNamer {
    #[must_use]
    pub fn new(session: &str) -> Self {
        Self {
            session4: session4(session).to_owned(),
            next: HashMap::new(),
        }
    }

    /// The next free name for `template_name`.
    pub fn next_group_name(
        &mut self,
        template_name: &str,
        is_taken: impl Fn(&str) -> bool,
    ) -> String {
        let n = self.next.entry(template_name.to_owned()).or_insert(1);
        loop {
            let name = format!("{template_name}{RUNTIME_SEPARATOR}{}-{}", self.session4, *n);
            *n = n.saturating_add(1);
            if !is_taken(&name) {
                return name;
            }
        }
    }
}

impl fmt::Display for RuntimeName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{RUNTIME_SEPARATOR}{}-{}",
            self.template, self.session4, self.n
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(session4("01JC6Z00000000000000007XQ2"), "7XQ2");
        assert_eq!(session4("AB"), "AB");
        assert_eq!(
            runtime_group_name("Red CAP", "01JC6Z00000000000000007XQ2", 1),
            "Red CAP#7XQ2-1"
        );
        assert_eq!(unit_name("Red CAP 1", 2), "Red CAP 1-2");
        assert_eq!(unit_names("G", 2), vec!["G-1", "G-2"]);
    }

    #[test]
    fn parse_round_trips() {
        let p = parse_runtime_group_name("Red CAP#7XQ2-12").unwrap();
        assert_eq!(
            p,
            RuntimeName {
                template: "Red CAP",
                session4: "7XQ2",
                n: 12
            }
        );
        assert_eq!(p.to_string(), "Red CAP#7XQ2-12");
        assert_eq!(
            parse_runtime_group_name("A#B#0001-3").map(|p| p.template),
            Some("A#B")
        );
        for bad in [
            "Red CAP 1",
            "Red CAP#7XQ2",
            "#7XQ2-1",
            "Red#TOOLONG-1",
            "Red#7XQ2-0",
            "R#7XQ2-x",
        ] {
            assert_eq!(parse_runtime_group_name(bad), None, "{bad}");
        }
    }

    #[test]
    fn explicit_names() {
        assert!(check_group_name("Red CAP 1").is_ok());
        assert_eq!(check_group_name(""), Err(NameError::Empty));
        assert!(matches!(
            check_group_name(" x"),
            Err(NameError::Whitespace(_))
        ));
        assert!(matches!(
            check_group_name("a\tb"),
            Err(NameError::Control(_))
        ));
        assert!(matches!(
            check_group_name("Red CAP#1"),
            Err(NameError::Reserved(_))
        ));
        assert!(matches!(
            check_group_name(&"x".repeat(200)),
            Err(NameError::TooLong(_))
        ));
        assert!(check_dcs_name("Red CAP#7XQ2-1").is_ok());
        let taken = |n: &str| n == "Red CAP 1";
        assert!(matches!(
            check_unique_group_name("Red CAP 1", taken),
            Err(NameError::Conflict(_))
        ));
        assert!(check_unique_group_name("Red CAP 2", taken).is_ok());
        let msg = check_unique_group_name("Red CAP 1", taken)
            .unwrap_err()
            .to_string();
        assert!(msg.contains("already exists"), "{msg}");
    }

    #[test]
    fn namer_counts_per_template_and_skips_taken_names() {
        let mut n = RuntimeNamer::new("01JC6Z00000000000000007XQ2");
        let none = |_: &str| false;
        assert_eq!(n.next_group_name("Red CAP", none), "Red CAP#7XQ2-1");
        assert_eq!(n.next_group_name("Red CAP", none), "Red CAP#7XQ2-2");
        assert_eq!(n.next_group_name("Convoy", none), "Convoy#7XQ2-1");
        let taken = |s: &str| s == "Red CAP#7XQ2-3";
        assert_eq!(n.next_group_name("Red CAP", taken), "Red CAP#7XQ2-4");
    }
}
