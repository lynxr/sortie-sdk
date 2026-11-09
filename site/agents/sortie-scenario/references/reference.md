# sortie-gen reference (for agents)

`from sortie_gen import Scenario, StartType, Coalition`. Unit and aircraft types are pydcs
classes (`dcs.vehicles.*`, `dcs.planes.*`, `dcs.helicopters.*`); countries are pydcs country
names (`"Russia"`, `"USA"`); airbases are names of the map's airports (`"Krymsk"`) or
`sc.theatre.airports[..]`. Unknown names fail with a "did you mean" hint.

## Declarations

| Declaration | Parameters (defaults) | Result |
| --- | --- | --- |
| `Scenario(name, *, schema, theatre, start, time_limit=None, description=None)` | `name`: `[a-z][a-z0-9_]*`; `schema`: int ≥ 1; `theatre`: a pydcs terrain (`caucasus.Caucasus()`); `start`: RFC 3339, without an offset the map's local time; `time_limit`: `"3h"`, `"1h30m"`, `"90m"`, seconds, `None` = no limit | the scenario |
| `sc.zone(name, *, ident=None, at=, radius=)` or `points=[p1, p2, p3, p4]`; `hidden=True`, `color=None` | a circle or a quad | `ZoneHandle` (`.center`) |
| `sc.ground_group(name, *, ident=None, country, at, units=[..], heading=0.0, formation="star", spacing=40.0, skill="Excellent", roles=None, late_activation=False, hidden=False)` | `formation`: `line`, `star`, `rectangle`, `vee`; the first unit stands at `at`; units named `<name>-<k>` | `GroupHandle` |
| `sc.air_template(name, unit_type, *, ident, country, size=1, task=None, at=None, airbase=None, altitude=None, speed=None, skill="High", roles=None, payload=None)` | kept out of the `.miz`; `task` defaults to the type's main task; placement when the logic gives none: `at`, else `airbase`, else the first declared airbase, else the coalition's bullseye; `altitude` 6000 m (2000 m for helicopters); `speed` in km/h; `payload`: a pydcs loadout name or a list of pylon tuples | `TemplateHandle` |
| `sc.airbase(name, *, ident, coalition=None)` | `coalition`: `Coalition.Red/Blue/Neutral` or `"red"`; without it the map's default owner stays (except player airfields) | `AirbaseHandle` (`.position`) |
| `sc.player_flight(name, unit_type, *, country, airbase, start=StartType.Cold, size=1, task=None, skill="Client", payload=None, at=None, altitude=3000.0)` | `start`: `StartType.Cold`, `Hot`, `Runway`, `Air` (`Air` needs `at=`); the type must be flyable; goes to `manifest.player_flights`, not to the lock | `FlightHandle` |
| `sc.flag(ident, *, name=None, initial=0)` | DCS flag name defaults to the ident; a non-zero `initial` is set by a MISSION START trigger | `FlagHandle` |
| `sc.menu(ident, label, *, path=())` | `path`: sub-menus, e.g. `["Support"]` | `MenuHandle` |
| `@sc.pydcs` on `def hook(mission)` | runs after the declarations on every build; names are checked for uniqueness afterwards | the function |
| `sc.latlon(lat, lon)`, `sc.point(north, east)` | positions; `latlon` is rounded to 1 mm | `dcs.mapping.Point` |
| `sc.time_limit = "2h"` | changes the limit after construction | |
| `sc.build(build_id=None)` | in-memory build: `.build_id`, `.manifest`, `.miz`, `.lock`, `.warnings`, `.write(out_dir)` | `Build` |
| `sc.cli()` | the command line below | exits |

Idents are optional for zones and ground groups (no ident: no Rust symbol) and required for
templates, airbases, flags and menus.

## `python scenario.py` flags

```text
lock      [--out FILE] [--check]
build     [--out DIR] [--lock FILE] [--locked] [--build-id ULID] [--stem NAME] [--no-miz]
validate  [--engine ADDR] [--token T] [--timeout S]
register  [--out DIR] [--lock FILE] [--locked] [--no-validate] [--engine ADDR] [--token T] [--timeout S]
launch    [--build-id ID] [--out DIR] [--lock FILE] [--locked] [--no-validate] [--engine ADDR] [--token T] [--timeout S]
restart   [--engine ADDR] [--token T] [--timeout S]
watch     [--session S] [--kinds birth,dead,...] [--from-seq N] [--engine ADDR] [--token T] [--timeout S]
results   [--session S] [--json] [--engine ADDR] [--token T] [--timeout S]
```

- `--timeout`: seconds per engine call (default 15).
- `watch --kinds`: `birth`, `dead`, `unit_lost`, `kill`, `hit`, `shot`, `takeoff`, `land`,
  `base_captured`, `mark`, `player_entered_unit`, `unit_gone`, `menu_pressed`,
  `player_connected`, `player_disconnected`, `player_changed_slot`, `chat_message`,
  `simulation_paused`, `simulation_resumed`, `gap`, `unknown`.
- `build --no-miz --stem NAME --out DIR` writes only the manifest files.
- `launch --build-id ID` builds and checks nothing: it loads a build registered earlier.

## `sortie-gen`

```text
sortie-gen lock|build|validate|register|launch <scenario.py> [flags]
sortie-gen restart|watch|results [flags]                 # no scenario file needed
sortie-gen lobby [--out missions/sortie-lobby.miz] [--theatre caucasus]
sortie-gen adopt my.miz [--json]
```

- `lobby` writes the idle mission for a dedicated server's `missionList` (no units, no
  bootstrap; it shows whether the bridge loaded). The engine uses the running lobby to check
  types.
- `adopt` reports an editor mission: groups, units, zones, proposed idents, names that break
  the rules. pydcs may move aircraft to other parking spots when it saves a mission: place
  player slots with `player_flight`.
- `worker` (the campaign loop) is not available yet: it exits with code 2.

## Errors and warnings

The CLI prints `error: ...` without a traceback. In code they are `sortie_gen` exceptions:
`InvalidName`, `NameConflict`, `ScenarioError`, `LockError`, `UnknownTypes`, `EngineError`,
`EngineUnavailable`, `EngineNotReady`, `NotYet` (all `GeneratorError`).

| Message (start) | Fix |
| --- | --- |
| `... ident 'x' must match [A-Z][A-Z0-9_]*` | idents are Rust constants: `SAM_NORTH` |
| `a group named 'X' already exists in this scenario` / `two groups are named 'X'` | DCS would destroy one of them: rename |
| `... contains '#', which is reserved for spawned groups` | drop the `#` |
| `pass a position made with sc.latlon(lat, lon), sc.point(north, east), a zone or an airbase` | a bare tuple is ambiguous |
| `player flights 'A' (blue) and 'B' (red) both start at X` | one airfield, one coalition: move a flight or use an air start |
| `player flight 'A' (blue) starts at X, which the scenario gives to red` | give the airbase to the flight's coalition or start elsewhere |
| warning `player group 'A' (blue) starts at X, which belongs to red` | a hook placed a client group: set the airfield owner in the hook |
| warning `template 'X' (Su-27) spawns without weapons` | `payload=[..]` pylon tuples, or `SORTIE_GEN_DCS_DATA=installed` |
| `pydcs knows no loadout 'X' for Y` | a loadout name needs `SORTIE_GEN_DCS_DATA=installed`; or list the pylons |
| `scenario.lock.json is stale (...); run python scenario.py lock` | after `--locked` / `--check`: run `lock`, rebuild the logic |
| warning `the symbol set changed: rebuild the mission logic (cargo build)` | rebuild the logic before running it against the new build |
| `DCS on the server does not know N type(s) this scenario uses:` | fix the type name, or install the module or mod on the server; `--no-validate` skips the check |
| warning `unit and static types not checked: ... (registering anyway)` | no mission ran in DCS, or it was paused; start the lobby to get the check |
| `cannot reach the Sortie engine at 127.0.0.1:50777 (...). Is sortied running?` | start `sortied --dev` locally, or point `--engine` / `SORTIE_ENGINE` at it |
| `... FAILED_PRECONDITION ...` on `restart` | single player, or the hooks part of Sortie predates the install: restart DCS itself |
