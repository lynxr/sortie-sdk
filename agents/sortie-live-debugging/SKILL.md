---
name: sortie-live-debugging
description: Sortie in a live DCS World: install and `sortie doctor`, `sortied --dev`, attaching the mission and the logic, reading dcs.log, the sortied log, sortie-bridge.log and the logic's stderr. Use when a Sortie mission does not attach or misbehaves in DCS, when the DCS screen shows a "Sortie ..." message, after installing or updating Sortie or DCS, when players cannot take multiplayer slots, after a Mission Editor round trip, or when an error mentions the bridge, the engine, a session or a token.
---

# Debugging Sortie in a live DCS

The chain, top to bottom; a problem is the first link that is down:

```text
DCS (mission and hooks environments)
  └─ bridge DLL + Lua layer    loaded by one line in <DCS>\Scripts\MissionScripting.lua
                               and by Saved Games\DCS\Scripts\Hooks\sortie-hook.lua
       └─ link (named pipe) ─► sortied (gRPC 127.0.0.1:50777)
                                 ◄─ your logic binary (Logic.Attach)
                                 ◄─ sortie-gen (register, launch, watch, results)
                                 ◄─ sortie CLI (status, events, top)
```

The Saved Games folder is `%USERPROFILE%\Saved Games\DCS` (a dedicated server started with
`-w DCS.server`: `DCS.server`, and every command gets `--instance server`).

## Setup on a machine with DCS

1. Unzip `sortie-runtime-0.1.0-win64.zip` from the GitHub release of
   `lynxr/sortie-sdk` and follow its README; check it against `SHA256SUMS`.
2. `sortie.exe install` from a console **run as administrator**: it copies the Lua layer, the
   hook and the bridge DLL into Saved Games, creates `Sortie\link.secret` and
   `Sortie\tokens.toml`, and adds one line to `<DCS>\Scripts\MissionScripting.lua` (the only
   change in the DCS folder). `--dry-run` shows the plan; `--no-patch` leaves
   `MissionScripting.lua` alone; `--dcs <folder>` picks the install.
3. **Restart DCS itself** (not only the mission). The hooks part of the layer and a new bridge
   DLL load only when DCS starts.
4. `sortie.exe doctor`: done when it exits 0. `doctor --fix` (administrator) restores the
   `MissionScripting.lua` line after a DCS update overwrote it.
5. `sortied.exe --dev`: local API calls need no token; spawns and destroys need `.key()`. Keep
   the console open: it logs every attach and refusal. DCS and `sortied` start in any order.
6. Register the mission (`uv run python scenario.py register`), start the logic
   (`cargo run --release -- --dev`), then fly: open
   `Saved Games\DCS\Missions\Sortie\<scenario>-<build_id>.miz` (single player) or
   `scenario.py launch` (dedicated server).
7. Done when `sortie status` shows `session <id> (<scenario>)` with `1 logic client(s)` and the
   logic's stderr says `session started`.

## Where to look

| Source | What it shows |
| --- | --- |
| the DCS screen | `Sortie bridge is not installed: run sortie doctor` (the mission found no bridge); `Sortie: <reason>` (the engine refused the mission) |
| `sortie status` | engine and bridge versions, DCS version, `link hooks/mission: up/down`, the session, model time, `PAUSED`, logic clients, `note` lines with the reason something is degraded |
| `sortie events --kind dead --limit 20`, `scenario.py watch` | the live event stream |
| `sortie top` | health, links, credits, queues, refreshed every second |
| `Saved Games\DCS\Logs\dcs.log` | Lua layer lines start with `Sortie:` (mission) or come from the `SORTIE` subsystem (hooks): link up/down, boot errors. `Select-String Sortie dcs.log` |
| `Saved Games\DCS\Logs\sortie-bridge.log` | the bridge DLL |
| `Saved Games\DCS\Sortie\Logs\sortied.log.*` | the engine (also on its console), including the `doctor` checks it runs at start |
| the logic's stderr | attach, `session started` / `session ended`, every recorded error with its Rust call site; `SORTIE_LOG=debug` for more |
| `scenario.py results` | the outcome, fates, kills, flags and report of the last session |

## The development loop

- **Logic change:** stop the logic (Ctrl-C), `cargo build --release`, start it again. The
  running session continues; the mission function starts with `Start::AfterCrash` and replays:
  keyed commands, `once` and named timers do not repeat. (`sortie dev` with hot reload is not
  available yet.)
- **Scenario change without new symbols** (positions, units, weather): `register` a new
  build and load it. A new build is a new turn: `Start::Fresh`.
- **New or renamed symbols:** `lock`, rebuild the logic, `register`, load the new build. An
  old logic is refused by a mission that lacks its symbols.
- **Same build again:** `scenario.py restart` (multiplayer host or server), or load the same
  `.miz` again in single player: `Start::Restart`, the world is back to the manifest.
- Only one logic per scenario can be attached: stop the old process before starting a new
  one.

## Facts that cause most surprises

- **Restart DCS after `sortie install`.** A new DLL is named in `Scripts\Sortie\pending.txt`
  and becomes active at the next DCS start; the hooks Lua loads only at DCS start. A stale
  layer shows as a `sortie status` note: "the Lua layer DCS loaded differs from what
  `sortie install` put there ...: run `sortie install` again and restart DCS itself".
- **Multiplayer slots and airfields.** DCS offers a parking slot only to the coalition that
  owns the airfield. A player who stays a spectator after picking a slot: the airfield belongs
  to someone else. `player_flight` handles it; a client group added in a `@sc.pydcs` hook needs
  its airfield owner set in the hook (the build warns).
- **Unit ids are strings in DCS 2.9.31.** `Unit:getID()` returns a string there. The Lua layer
  and the engine of this runtime accept both forms; in the SDK `Entity.unit_id` is a number.
  A world that stays empty (no units in `wait_until`, nothing in `sortie events`) with an older
  runtime: install this one and restart DCS.
- **Mission Editor round trips are safe** for a built `.miz` (open, re-save, fly from the
  editor): the bootstrap is an inline DO SCRIPT. The `dcs.log` error
  `[string "DictKey_..."]:1: '=' expected near '<eof>'` means a DO SCRIPT in the dictionary
  form: the `.miz` went through a pydcs load and save, or a hook used pydcs's `DoScript`
  instead of `sortie_gen.miz.InlineDoScript`. Groups, units and zones edited in the editor no
  longer match the manifest: edit `scenario.py`.
- **Pause stops model time.** Commands to the mission wait while DCS is paused (unless sent
  with `.on_pause(Fail)`), `ctx.sleep` and timers stop, and the type check of `register` cannot
  run.
- **Measurements of DCS itself** (event ids, timer cadence, sampling cost) are not part of SDK
  0.1.0: `sortie probe` prints "`sortie probe` lands in M0" and exits with code 2;
  `sortie dev`, `sortie trace` and `sortie record` are not built yet either (M4). Report what
  you measure with the DCS version, FPS and hardware.

Failure signatures and fixes, symptom first: [references/failures.md](references/failures.md).
Mission-side errors (`sortie::*` codes): the sortie-mission-logic skill,
`references/errors.md`. Docs (Russian): https://lynxr.github.io/sortie-sdk/.
