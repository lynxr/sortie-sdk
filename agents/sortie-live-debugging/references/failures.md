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
| `sortied`: `an event's time is far ahead of model time: it moves no clock`; `sortie status`: `N event times far ahead of model time ignored` | an event came with another clock than model time: an F10 mark's time of day from a Lua layer older than 0.4.0, or time acceleration in single player. The engine's clock is not affected (before 0.4.0 such a mark froze ticks and timers) | `sortie install` and restart the mission; nothing under time acceleration |
| `ctx.destroy` of a static, or `spawn_static` with `Replace`, → `sortie::lua_error`: ``DCS did not remove the static `X`: it still exists 10 pumps after destroy()`` | DCS kept the object (reported for FARPs; not measured); the world keeps the static and its airbase alive | do not plan on removing a FARP; give the new static another name, or leave the old one |
| `sortie::timeout` on spawns, note `N command(s) waiting: spawn rate limit (2 groups / 24 units per second)` | too many spawns at once | spawn in a loop, cap the count, raise `.timeout(..)` |
| `sortie::timeout`: "the deadline passed before the command could start (tasking delay after spawn (2 frames and 0.5 s))", and the mission function stopped there | a `set_task` / `set_option` right after a spawn waited for the delay until its deadline and never reached DCS (`e.limit()`: `TaskingDelay`); with 0.3.0 an F10 mark froze the clock the delay counts on | engine 0.4.0 (the clock and the hold are fixed); key the command, retry it once on `Timeout` and log the rest instead of returning it with `?` |
| `sortie::timeout`: "`op` was sent to DCS but no result arrived before the deadline" | DCS did not answer in time; the command may still run (`e.limit()` is `None`) | retry only with the same `.key(..)` (it joins the late result); check `sortie top` for a busy pump |
| `sortie::invalid`: "key `k` was already used for a different command in session ..." | the command under that key changed (a new heading, target or option), also after a logic restart | a new key for the changed command (`k-v2`) |
| `sortied` does not start: ``engine.toml [calls]: `Controller.setOption`: `enum:..`: an enum table is 2 to 6 Lua names from one of the enum roots (AI, Airbase, ...)`` | an owner entry names an enum table of 7 names, from a data root (`env`, `net`, `DCS`, `timer`, `Warehouse`) or, in a hooks entry, a DCS table; 0.3.0 refused even 5 names | name the published table's whole path from an enum root (`enum:AI.Option.Ground.val.ALARM_STATE`), or declare the argument `number` |
| every unit of a spawned ground group faces the same heading (Tacview yaw), the template's offsets between them lost | `.at(point, heading)` gives every unit that one heading and does not turn the formation (0.1.0 to 0.4.0) | `.at_formation(point, heading)` (API revision 3) under a new key; the template's reference heading is `Template.heading_rad`, else its first unit's |
| `sortie::rejected`: "rate limit: the heavy calls of (writer) cost more than 20 ms of DCS time per second (dcs_time.heavy ...); the next one may go in N s" | a heavy call (a long road path) took more DCS time than the writer's heavy budget holds (200 ms, refilled 20 ms a second); reads are not held | wait the N s before the next heavy call; ask a road path once per route; ends further apart cost more (at most 300 km) |
| `sortie::rejected`: "rate limit: the reads of (writer) cost more than 20 ms of DCS time per second" | a read or a batch (any batch, a mutate in it too) cost more DCS time than the read budget; since 0.4.0 a single read waits at most 1 s, a batch until the whole debt is paid back | retry after `e.retry_after()`; split big batches and spread them; a costly read belongs to class heavy |
| `sortie::rejected`: "rate limit: the previous heavy call of (writer) has not come back yet ..." (`DcsTime`, `dcs_time.heavy`, no `retry_after()`) | a heavy call sent while the logic's previous heavy call is still queued, running or awaited after a `Timeout` (0.4.0: one at a time per writer) | send the next heavy call after the previous one answered, not two at once |
| a command `.after(a)` fails at once with `a`'s code (`Rejected`, `Invalid` ...) and `e.limit()` `After` | `a` failed; the command never ran, and sent again it fails the same way | send `a` again, then this command `.after(..)` the new one |
| `sortie::rejected`: "rate limit: (writer) may make 200 read calls per second" | the read count: a batch is charged every call, so a 256-read batch leaves the next one 1.28 s to wait | `e.limit()` (`CallRate`, `call_rate.read`) and `e.retry_after()` name it: wait that long, or send smaller batches spread over time |
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
