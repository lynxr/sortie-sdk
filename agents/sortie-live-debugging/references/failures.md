# Failure signatures (for agents)

Symptom first. Check the links of the chain in this order: DCS screen and `dcs.log`, then
`sortie status`, then the `sortied` console, then the logic's stderr.

## The mission does not attach

| Symptom | Cause | Fix |
| --- | --- | --- |
| screen: `Sortie bridge is not installed: run sortie doctor` | the mission found no `SORTIE` global: the `MissionScripting.lua` line is missing (a DCS update rewrites the file) or the Lua boot failed | `sortie doctor`; `sortie doctor --fix` as administrator; then read the `Sortie:` error in `dcs.log` |
| `dcs.log`: ``Sortie: lfs/io/package are not available: the Sortie line must come before the sanitizer in MissionScripting.lua (run `sortie doctor --fix`)`` | the line sits after the sanitizer | `sortie doctor --fix` as administrator |
| `dcs.log`: ``Sortie: Scripts/Sortie/active.txt is missing: run `sortie install` `` | no bridge installed in this Saved Games folder (or another instance) | `sortie install` with the right `--instance`, restart DCS |
| screen: ``Sortie: build <id> (<scenario>) is not registered with this engine; register it with `sortie-gen register` ...`` | the `.miz` came from an offline `build`, or the engine's database is another one (`--db memory` forgets on restart) | `scenario.py register` and open the registered copy from `Missions\Sortie`, or start `sortied --manifest dist\<id>.manifest.pb` |
| screen: `Sortie: the mission's digest ... does not match build ...` | the `.miz` was changed after the build or belongs to another build | build and register again; edit `scenario.py`, not the `.miz` |
| screen: `Sortie: the mission says scenario ...` / `... schema ...` | the bootstrap and the registered build disagree | rebuild and register |
| nothing on screen; `sortie status`: `link mission: down`, note `no mission link: no Sortie mission is running` | `sortied` is not running, the mission has not started, or the link points elsewhere | start `sortied`; same `--instance` for `install`, `doctor` and `sortied`; `dcs.log`: `mission link down: <error>; retrying` names the reason |
| note `the DCS hooks link is not connected (is DCS running with Scripts/Hooks/sortie-hook.lua?)` | DCS was not restarted after the install, or the hook is missing | restart DCS; `sortie doctor` checks `hook` |
| note ``the Lua layer DCS loaded differs from what `sortie install` put there (...): run `sortie install` again and restart DCS itself`` | an update of the layer while DCS ran | as it says (for the mission part: restart the mission) |
| `dcs.log`: ``[string "DictKey_..."]:1: '=' expected near '<eof>'`` | a DO SCRIPT in pydcs's dictionary form | rebuild from `scenario.py`; hooks use `sortie_gen.miz.InlineDoScript`; never load/save a built `.miz` with pydcs |

## The logic does not run

| Symptom (logic stderr) | Cause | Fix |
| --- | --- | --- |
| ``cannot reach the engine at http://127.0.0.1:50777: ... (is `sortied` running?); retrying in N s`` | no engine at that address | start `sortied`; `--engine` / `SORTIE_ENGINE` |
| `attaching`, then nothing | no session of this scenario runs yet; the logic waits for one | start the mission; the scenario name of the logic and the mission must match |
| ``error[sortie::transport]: Logic.Attach at ... failed: ... (set SORTIE_TOKEN to a token with the `logic` role)``, exit code 2 | `sortied` without `--dev` needs a token | `SORTIE_TOKEN` from `Saved Games\DCS\Sortie\tokens.toml` (role `logic`), or `sortied --dev` on your own machine |
| ``error[sortie::rejected]: the engine rejected this logic: logic `x` was built against a different `s` schema: N symbol(s) missing from the running mission``, exit code 2 | the mission in DCS lacks symbols the logic was built with | build the logic and the mission from the same `scenario.lock.json`; `register` and load the new build |
| ``another PRIMARY logic is attached for `s` `` | a second copy of the logic | stop the old process |
| `the engine shut this logic down`, exit code 0 | the engine stopped, or the logic stopped reading (a blocked thread: 4096 unread messages) | restart it; remove the blocking code |
| `session started` with `kind=AfterCrash` after a restart of the logic | expected: same session, the mission function replays | nothing; keys, `once` and timers keep it from repeating |

## The mission runs, but wrong

| Symptom | Cause | Fix |
| --- | --- | --- |
| a player picks a slot and stays a spectator (multiplayer) | the airfield belongs to another coalition | `player_flight` sets the owner; for a client group from a hook, set the airfield's coalition in the hook; read the build warnings |
| `sortie status` shows the session, but `wait_until` on units never fires and `sortie events` stays silent | the world stays empty: an older runtime reads DCS 2.9.31's string unit ids as numbers | install the current runtime, restart DCS |
| `sortie::invalid`: spawn or destroy without a key | `sortied --dev` or the logic's `--dev` | `.key("step-name")` |
| a spawn twice after a logic restart | the spawn had no key, or its key changes between runs | a stable key per step |
| `sortie::timeout` on spawns, note `N command(s) waiting: spawn rate limit (2 groups / 24 units per second)` | too many spawns at once | spawn in a loop, cap the count, raise `.timeout(..)` |
| commands hang, `sortie status` says `PAUSED` | DCS is paused: mission commands wait | unpause, or `.on_pause(v1::PausePolicy::Fail)` |
| `ctx.group(groups::X)` → `sortie::not_in_world` | late activation or the group is dead | spawn it from a template instead; check `alive_count()` |
| `ctx.group(groups::X)` → `sortie::unknown_symbol` | the mission in DCS was built from another lock | rebuild both sides from one lock |
| `register` stops: `DCS on the server does not know N type(s) ...` | a misspelled type or a module or mod missing on the server | fix the type or install it there; `--no-validate` skips |
| `register` warns `unit and static types not checked: ...` | no mission ran in DCS, or it was paused | run the lobby (`sortie-gen lobby`) unpaused |
| `scenario.py restart` → `FAILED_PRECONDITION`: `Missions.Restart needs DCS running as a multiplayer server ...` | single player | restart from the DCS menu, or host in multiplayer |
| `restart` → `DCS did not say whether it runs multiplayer: the Sortie hooks layer DCS loaded predates ...` | DCS was not restarted after `sortie install` | restart DCS itself |
| `scenario.py launch` fails on a single-player DCS | `launch` loads missions through the server API | open the registered `.miz` yourself |

## `sortie doctor` checks

| Check | When it fails | Fix |
| --- | --- | --- |
| `dcs install`, `saved games`, `writedir` | wrong folder or instance | `--dcs <DCS World folder>`, `--instance <name>`, `--saved-games <folder>` |
| `MissionScripting`, `sanitizer` | the line is missing, misplaced, or the sanitizer was changed | `sortie doctor --fix` as administrator; restore the stock sanitizer |
| `hook`, `lua layer`, `bridge dll` | files missing or changed | `sortie install`, then restart DCS |
| `lua.dll` | this DCS build changed `lua.dll`: the bridge does not match it | a runtime release built for this DCS version is needed: report it |
| `link secret`, `api tokens` | not created yet | `sortie install` |
| `missionList` | an empty mission list on a dedicated server (the server does not start) | add a mission, e.g. a `sortie-gen lobby` mission |
| `unsafe api` (warning) | `autoexec.cfg` opens `net.allow_unsafe_api` to missions | Sortie does not need it |
