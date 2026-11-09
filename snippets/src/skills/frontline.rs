//! What `sortie::include_scenario!(frontline)` includes for the scenario of
//! `sdk/snippets/python/skills_scenario.py`: the output of `sortie-build` for its lock (schema 1,
//! hash 28aa6fe0…), kept here so the skills' examples compile without a build script. If the
//! scenario's symbols change, regenerate the lock and update this file to match.

/// Scenario marker for `frontline` (schema 1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Frontline;

impl ::sortie::types::Scenario for Frontline {
    const NAME: &'static str = "frontline";
    const SCHEMA: u32 = 1;
    const SCHEMA_SHA256: &'static str =
        "28aa6fe0b6e78e7517798c4a7110d2b0cca110855dba5277a8542b51255ab2c7";
    const SYMBOLS: &'static [::sortie::types::Symbol] = &[
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Group, "RED_ARMOR"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Group, "SAM_NORTH"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Zone, "FRONT"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Zone, "OBJECTIVE"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Zone, "SAM_RING"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Template, "RED_CAP"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Flag, "SAM_DOWN"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Airbase, "KRYMSK"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Airbase, "MAYKOP"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Menu, "SITREP"),
        ::sortie::types::Symbol::new(::sortie::types::SymbolKind::Menu, "SMOKE_OBJECTIVE"),
    ];
}

/// Groups placed in the mission.
pub mod groups {
    /// Group `RED_ARMOR` (ground; roles: armor).
    pub const RED_ARMOR: ::sortie::types::GroupRef<super::Frontline, ::sortie::types::Ground> =
        ::sortie::types::GroupRef::new("RED_ARMOR");
    /// Group `SAM_NORTH` (ground; roles: north, sam).
    pub const SAM_NORTH: ::sortie::types::GroupRef<super::Frontline, ::sortie::types::Ground> =
        ::sortie::types::GroupRef::new("SAM_NORTH");
}

/// Trigger zones (geometry from scenario.lock.json; the running manifest wins at run time).
pub mod zones {
    /// Zone `FRONT`: circle, r = 20000 m.
    pub const FRONT: ::sortie::types::ZoneRef<super::Frontline> = ::sortie::types::ZoneRef::circle(
        "FRONT",
        ::sortie::types::MapPoint::new(-5442.75, 342233.756),
        20000.0,
    );
    /// Zone `OBJECTIVE`: circle, r = 500 m.
    pub const OBJECTIVE: ::sortie::types::ZoneRef<super::Frontline> =
        ::sortie::types::ZoneRef::circle(
            "OBJECTIVE",
            ::sortie::types::MapPoint::new(-32798.016, 463776.327),
            500.0,
        );
    /// Zone `SAM_RING`: circle, r = 60000 m.
    pub const SAM_RING: ::sortie::types::ZoneRef<super::Frontline> =
        ::sortie::types::ZoneRef::circle(
            "SAM_RING",
            ::sortie::types::MapPoint::new(-32798.016, 463776.327),
            60000.0,
        );
}

/// Spawnable templates (kept out of the .miz).
pub mod templates {
    /// Template `RED_CAP` (air).
    pub const RED_CAP: ::sortie::types::TemplateRef<super::Frontline, ::sortie::types::Air> =
        ::sortie::types::TemplateRef::new("RED_CAP");
}

/// User flags.
pub mod flags {
    /// Flag `SAM_DOWN`.
    pub const SAM_DOWN: ::sortie::types::FlagRef<super::Frontline> =
        ::sortie::types::FlagRef::new("SAM_DOWN");
}

/// Airbases.
pub mod airbases {
    /// Airbase `KRYMSK`.
    pub const KRYMSK: ::sortie::types::AirbaseRef<super::Frontline> =
        ::sortie::types::AirbaseRef::new("KRYMSK");
    /// Airbase `MAYKOP`.
    pub const MAYKOP: ::sortie::types::AirbaseRef<super::Frontline> =
        ::sortie::types::AirbaseRef::new("MAYKOP");
}

/// F10 radio menu commands.
pub mod menus {
    /// Menu `SITREP`.
    pub const SITREP: ::sortie::types::MenuRef<super::Frontline> =
        ::sortie::types::MenuRef::new("SITREP");
    /// Menu `SMOKE_OBJECTIVE`.
    pub const SMOKE_OBJECTIVE: ::sortie::types::MenuRef<super::Frontline> =
        ::sortie::types::MenuRef::new("SMOKE_OBJECTIVE");
}
