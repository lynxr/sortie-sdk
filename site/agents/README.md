# Sortie for AI coding agents

This folder helps coding agents work on a Sortie mission (Sortie SDK 0.1.0). It is
written in English; the SDK docs are in Russian.

| File | For | What it is |
| --- | --- | --- |
| [sortie-mission-logic/](sortie-mission-logic/SKILL.md) | Claude Code | skill: the mission logic in Rust (`Ctx`, keys, waits, events, menus, state, long multiplayer missions, errors) |
| [sortie-scenario/](sortie-scenario/SKILL.md) | Claude Code | skill: `scenario.py` with sortie-gen and its commands |
| [sortie-live-debugging/](sortie-live-debugging/SKILL.md) | Claude Code | skill: install, logs and failures in a live DCS |
| [AGENTS.md](AGENTS.md) | Codex, Cursor, others | instructions for the root of a mission repository |
| [llms.txt](../llms.txt) | any agent that reads the web | an index of the docs and skills ([llmstxt.org](https://llmstxt.org) format) |

The site build packs this whole folder into a zip:
[sortie-agent-skills-0.1.0.zip](sortie-agent-skills-0.1.0.zip).
Its root holds the three skill folders, `AGENTS.md`, `llms.txt` and this file.

## Claude Code

A skill is a folder with a `SKILL.md`. Claude Code loads skills from `.claude/skills/` of the
project and from `~/.claude/skills/` of the user, and picks a skill by its description when the
task fits (writing logic, editing `scenario.py`, a failure in DCS).

Below, `<skills>` is the unzipped zip or the `agents` folder of the SDK repository.

Into one mission repository (commit them with it, so everyone working on it gets them):

```powershell
New-Item -ItemType Directory -Force .claude\skills | Out-Null
Copy-Item -Recurse -Force <skills>\sortie-* .claude\skills\
```

For every project of this user:

```powershell
New-Item -ItemType Directory -Force "$HOME\.claude\skills" | Out-Null
Copy-Item -Recurse -Force <skills>\sortie-* "$HOME\.claude\skills\"
```

```bash
mkdir -p .claude/skills && cp -r <skills>/sortie-* .claude/skills/
```

Copy `AGENTS.md` to the repository root too, and add a `CLAUDE.md` with the line
`@AGENTS.md` so Claude Code reads it. After an SDK update, replace the folders with the new
version's.

## Codex, Cursor and other agents

- Put `AGENTS.md` at the root of the mission repository. Codex and Cursor read it there; for
  other tools, point their rules file at it.
- Give an agent that reads the web the index [https://lynxr.github.io/sortie-sdk/llms.txt](../llms.txt),
  or the whole text in one file:
  [https://lynxr.github.io/sortie-sdk/llms-full.txt](../llms-full.txt).
- The skills are plain Markdown: an agent without skill support can read
  `agents/<skill>/SKILL.md` and its `references/` when the task matches the skill's
  `description`.

## How the examples are checked

Every Rust and Python example in these files is compiled: they come from the `frontline`
example scenario (`snippets/python/skills_scenario.py`) and the code in `snippets/src/skills/`
of the SDK repository.
