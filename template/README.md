# my_mission — шаблон миссии Sortie

Минимальный, но рабочий проект миссии для DCS World на Sortie SDK 0.1.0. Скопируйте
эту папку, переименуйте и пишите свою миссию.

* `scenario.py` — сценарий на Python: Кавказ, зона цели, три группы противника, два слота игроков
  на Су-25Т в Кутаиси.
* `src/lib.rs` — логика на Rust: ЗРК молчит, пока синие не подойдут к цели ближе 15 км; когда
  бронегруппа уничтожена, синим приходит сообщение и сессия кончается победой синих.

## Что нужно

* установленный рантайм Sortie: `sortie install`, `sortie doctor`, перезапуск DCS;
* Rust stable не ниже 1.89, на Windows — Visual Studio Build Tools с C++;
* [uv](https://docs.astral.sh/uv/) и Git.

Если репозиторий SDK закрытый, настройте доступ к нему (раздел «Подключение SDK» документации).

## Запуск

```powershell
uv run python scenario.py lock          # scenario.lock.json: символы для логики
cargo build --release                   # логика

C:\Sortie\bin\sortied.exe --dev         # движок, в отдельном окне
cargo run --release                     # логика, в отдельном окне

uv run python scenario.py register      # сборка миссии; .miz появится в Saved Games\DCS\Missions\Sortie
```

Откройте миссию в DCS (одиночная игра или свой сервер) и займите слот Blue 1. Итоги сессии —
`uv run python scenario.py results`, события — `uv run python scenario.py watch`.

## Файлы

| Файл | Что это |
| --- | --- |
| `scenario.py` | сценарий; `uv run python scenario.py --help` покажет команды |
| `scenario.lock.json` | символы сценария; его пишет `scenario.py lock`, руками не правится |
| `pyproject.toml` | Python-проект с зависимостью `sortie-gen` |
| `Cargo.toml` | крейт логики с зависимостями `sortie` и `sortie-build` |
| `build.rs` | делает из `scenario.lock.json` модуль `my_mission` с символами |
| `src/lib.rs` | функция миссии `run` |
| `src/main.rs` | `main` программы логики |
| `AGENTS.md` | инструкции для ИИ-агентов (Codex, Cursor, Claude Code); навыки — в папке `agents/` SDK |

## Как переименовать

1. Имя сценария (`[a-z][a-z0-9_]*`): в `scenario.py` (`Scenario("my_mission", ...)`), в `build.rs`
   (первый аргумент `sortie_build::scenario`) и в `src/lib.rs` (`include_scenario!(my_mission)` и
   `use my_mission::...`). Маркер сценария в Rust — то же имя в CamelCase (`MyMission`).
2. Имя крейта: `name` в `Cargo.toml` и путь в `src/main.rs` (`my_mission_logic::run`).
3. Выполните `uv run python scenario.py lock` и `cargo build`.

## Документация

* сайт: https://lynxr.github.io/sortie-sdk/
* репозиторий SDK: https://github.com/lynxr/sortie-sdk
