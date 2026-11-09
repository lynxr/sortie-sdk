---
name: sortie-scenario
description: Sortie scenarios in Python (`scenario.py` with sortie-gen over pydcs, DCS World). Use when writing or changing a scenario (zones, ground groups, air templates, statics or other pydcs content, airbases, player flights and their airfield coalition, flags, F10 menus), when running `lock`, `build`, `validate`, `register`, `launch`, `restart`, `watch` or `results`, when a sortie-gen error or warning appears, or when the Rust logic needs a new symbol.
---

# Sortie scenarios

`scenario.py` declares the static part of a mission. sortie-gen (a wrapper over pydcs 1.0.0)
builds three things from it:

- the `.miz`, with one MISSION START trigger whose DO SCRIPT calls `SORTIE.attach{..}`;
- the manifest (`<build_id>.manifest.pb` and `.json`): groups, units, zones, templates, flags,
  airbases, menus, player slots, as the engine and the logic see them;
- `scenario.lock.json`: the symbols (idents) the Rust logic compiles against. It is the
  contract between the two sides: commit it, never edit it.

An `ident=` makes a symbol: `groups::SAM_NORTH`, `zones::FRONT`, `templates::RED_CAP`,
`airbases::MAYKOP`, `flags::SAM_DOWN`, `menus::SITREP` in Rust. Without an ident a zone is
only in the `.miz`, and a group is in the `.miz` and the manifest but has no Rust symbol.

## Setup

```powershell
uv add "sortie-gen @ git+https://github.com/lynxr/sortie-sdk@v0.1.0#subdirectory=python/sortie-gen"
```

Or install the `sortie_gen` wheel from the GitHub release. Python 3.11+.

## A scenario, step by step

The `frontline` scenario that the mission-logic skill uses. Header: name, schema, map, start,
time limit.

```python
from dcs import planes, statics, vehicles
from dcs.terrain import caucasus

from sortie_gen import Coalition, Scenario, StartType

sc = Scenario(
    "frontline",
    schema=1,
    theatre=caucasus.Caucasus(),
    start="2026-11-01T07:00:00Z",  # RFC 3339; without an offset: the map's local time
    time_limit="4h",  # the engine ends the session (turn) after 4 h of model time
)
```

Zones: a circle (`at` + `radius`, metres) or a quad (`points=[4 points]`).

```python
obj = sc.zone("Objective", ident="OBJECTIVE", at=sc.latlon(44.62, 40.10), radius=500)
ring = sc.zone("SAM Ring", ident="SAM_RING", at=obj.center, radius=60_000)
front = sc.zone("Front", ident="FRONT", at=sc.latlon(44.95, 38.60), radius=20_000)
sc.zone("Editor note", at=sc.latlon(44.70, 40.00), radius=1_000)  # no ident: .miz only
```

Ground groups are placed in the `.miz`; unit `k` is named `<group>-<k>`.

```python
sc.ground_group(
    "SAM North",
    ident="SAM_NORTH",
    country="Russia",
    at=obj.center,
    units=[
        vehicles.AirDefence.S_300PS_54K6_cp,
        vehicles.AirDefence.S_300PS_40B6M_tr,
        vehicles.AirDefence.S_300PS_64H6E_sr,
        *[vehicles.AirDefence.S_300PS_5P85C_ln] * 4,
    ],
    roles={"sam", "north"},
)
sc.ground_group(
    "Red Armor",
    ident="RED_ARMOR",
    country="Russia",
    at=front.center,
    units=[vehicles.Armor.T_72B] * 4,
    formation="line",
    heading=270,
    roles={"armor"},
)
```

Air templates stay out of the `.miz`: the logic spawns them (`ctx.spawn_group`).

```python
sc.air_template(
    "Red CAP",
    planes.Su_27,
    ident="RED_CAP",
    country="Russia",
    size=2,
    airbase="Maykop-Khanskaya",  # default placement when the logic gives none
    payload=[  # pylon tuples: the hermetic build knows no default loadout for this task
        planes.Su_27.Pylon1.R_73__AA_11_Archer____Infra_Red,
        planes.Su_27.Pylon10.R_73__AA_11_Archer____Infra_Red,
    ],
)
```

Airbases (symbols for the logic) and player slots:

```python
sc.airbase("Maykop-Khanskaya", ident="MAYKOP", coalition=Coalition.Red)
sc.airbase("Krymsk", ident="KRYMSK")  # the player flights below give it to blue
sc.player_flight(
    "Viper 1", planes.F_16C_50, country="USA", airbase="Krymsk", start=StartType.Cold, size=2
)
sc.player_flight(
    "Hog 1", planes.A_10C_2, country="USA", airbase="Krymsk", start=StartType.Hot, size=2
)
```

Flags and F10 menu commands (the logic installs the menus):

```python
sc.flag("SAM_DOWN")
sc.menu("SMOKE_OBJECTIVE", "Smoke the objective", path=["Support"])
sc.menu("SITREP", "Situation report")
```

Everything Sortie does not wrap (statics, air groups placed in the `.miz`, weather, briefing,
your own triggers) goes in a `@sc.pydcs` hook. It runs on every build after the declarations.
Groups it adds are in the manifest without an ident; reach them by DCS name.

```python
from dcs.triggers import TriggerStart

from sortie_gen.miz import InlineDoScript


@sc.pydcs
def extras(m):
    # What Sortie does not wrap: statics, weather, briefing, your own triggers.
    hq = sc.latlon(44.61, 40.08)
    m.static_group(m.country("Russia"), "Red HQ", statics.Fortification._Command_Center, hq)
    t = TriggerStart(comment="welcome")
    t.add_action(InlineDoScript('trigger.action.outText("Frontline is live", 15)'))
    m.triggerrules.triggers.append(t)
```

```python
if __name__ == "__main__":
    sc.cli()  # lock | build | validate | register | launch | restart | watch | results
```

All signatures and defaults: [references/reference.md](references/reference.md).

## Rules

1. **Positions** are `sc.latlon(lat, lon)`, `sc.point(north, east)` (editor axes, metres), a
   zone (`zone.center` or the handle) or an airbase handle. A bare tuple is refused.
2. **Names.** DCS group names are unique across groups, templates and player flights, at most
   128 bytes, without `#` (reserved for spawned groups: `<template>#<session4>-<n>`). Idents
   match `[A-Z][A-Z0-9_]*` and are unique per kind; roles `[a-z][a-z0-9_-]*`; the scenario name
   `[a-z][a-z0-9_]*` and not a Rust keyword. sortie-gen refuses a bad name at the declaration.
3. **The airfield coalition rule (multiplayer).** DCS offers a parking slot only to the
   coalition that owns the airfield; at a neutral or enemy airfield the player stays a
   spectator. So `player_flight` with a `Cold`, `Hot` or `Runway` start gives its airfield to
   the flight's coalition. The build fails when the scenario gives that airbase to another
   coalition (`airbase(.., coalition=..)`) or starts flights of two coalitions there. A client
   group added by a `@sc.pydcs` hook gets only a warning: fix its airfield owner in the hook.
4. **Late activation** (`late_activation=True`) has no activate command in the SDK: a group
   that appears later is an `air_template` the logic spawns.
5. **A symbol change needs both sides.** Add or rename an ident → `lock` → `cargo build` of
   the logic → `register` a new build. Moving a zone or a unit changes no symbol: the logic
   needs no change (the running manifest's geometry wins).

## Commands

Run from the mission repository: `uv run python scenario.py <command>`.

| Command | Does | Needs |
| --- | --- | --- |
| `lock [--check]` | writes `scenario.lock.json`; `--check` only fails if it is stale (CI) | nothing |
| `build [--out DIR] [--no-miz] [--stem NAME] [--locked]` | offline build into `dist/` (or `--out`); updates the lock unless `--locked` | nothing |
| `validate` | asks DCS whether it knows every unit and static type | a running mission with Sortie (the lobby is enough) |
| `register [--out DIR] [--no-validate]` | engine build id, build, type check, `Missions.Register`; the engine stores the `.miz` in `Saved Games\DCS\Missions\Sortie\` | `sortied` |
| `launch [--build-id ID] [--no-validate]` | registers a fresh build (without `--build-id`) and loads it in DCS | a dedicated server with Sortie |
| `restart` | reloads the running mission: its session ends (reason `restart`), a new one starts with `Start::Restart` | a multiplayer server or host; single player refuses at once |
| `watch [--kinds dead,birth] [--from-seq N] [--session S]` | engine health and the live event stream | `sortied` |
| `results [--session S] [--json]` | results of the last (or a given) session: outcome, fates, kills, flags, report | `sortied` |

The engine address is `--engine`, `SORTIE_ENGINE` or `127.0.0.1:50777`; the token is
`--token` or `SORTIE_TOKEN` (role `generator`; not needed with `sortied --dev`). Exit codes: 0
done, 1 error, 2 a feature that is not built yet.

Single player: `launch` needs a server, so `register`, then open
`Saved Games\DCS\Missions\Sortie\<scenario>-<build_id>.miz` in DCS yourself.

## Builds and turns

- **Determinism.** An offline `build` derives the build id from the content (start time,
  manifest, mission): the same scenario gives byte-identical files on one machine; the manifest
  and the lock are meant to match across machines too. By default the build uses only the data
  inside pydcs (hermetic): a template without a default loadout warns "spawns without weapons";
  pass `payload=` (pylon tuples, as above). `SORTIE_GEN_DCS_DATA=installed` uses the installed
  DCS loadouts and makes the build machine-dependent.
- **A build is a turn.** `register` takes a fresh build id from the engine every time, so a new
  registration starts a new turn: `Start::Fresh`, fresh keys, `once` and `once_per_turn`.
  `restart` keeps the build (same turn).
- **Type check.** `register` and `launch` stop when DCS on the server does not know a type
  (a typo, a missing module or mod) and print where it is used. When the check cannot run (no
  mission, the server is paused, an older engine) they warn and register anyway.
  `--no-validate` skips it.

## The Mission Editor and DO SCRIPT

- A built `.miz` can be opened, re-saved and flown from the Mission Editor: the bootstrap is
  stored the way the editor writes DO SCRIPT (code inline in the action), so the mission still
  attaches.
- Change groups, units and zones in `scenario.py`, not in the editor: the manifest describes
  the build, and an edited `.miz` no longer matches it.
- A DO SCRIPT added in a `@sc.pydcs` hook uses `sortie_gen.miz.InlineDoScript`, not pydcs's
  `DoScript` (the editor would run its dictionary key as Lua).
- Do not load and save a built `.miz` with pydcs: it rewrites every DO SCRIPT into the
  dictionary form.
- `uv run sortie-gen adopt my.miz [--json]` lists the coalitions, groups, units and zones of an
  editor mission with proposed idents: the start for porting it to `scenario.py`.

## More

- [references/reference.md](references/reference.md): every declaration with its parameters,
  the CLI flags, the `sortie-gen` command, and the generator's errors.
- The Rust side: the sortie-mission-logic skill. Running in DCS: the sortie-live-debugging
  skill. Docs (Russian): https://lynxr.github.io/sortie-sdk/.
