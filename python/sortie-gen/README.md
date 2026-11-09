# sortie-gen

Генератор миссий Sortie: обёртка над pydcs 1.0.0. Он собирает `.miz`, manifest
(`sortie.manifest.v1`) и `scenario.lock.json` и говорит с движком `sortied` по gRPC.

Сценарий — обычный Python-файл: объект `Scenario` и в конце `sc.cli()`. Команды сценария:

```sh
uv run python scenario.py lock       # scenario.lock.json: символы для Rust-логики
uv run python scenario.py build      # офлайн: dist/<build_id>.miz + manifest
uv run python scenario.py register   # с движком: build id от движка и регистрация сборки
uv run python scenario.py launch     # запустить зарегистрированную сборку на сервере
uv run python scenario.py results    # итоги сессии
```

Команды пакета:

```sh
uv run sortie-gen lobby --out sortie-lobby.miz   # лобби-миссия, в которой ждёт сервер
uv run sortie-gen adopt my.miz                   # разобрать миссию из редактора
```

Пример сценария — `examples/op_north/scenario.py`. Полное руководство по генератору — в
документации Sortie.

Переменные окружения: `SORTIE_ENGINE` (адрес движка, по умолчанию `127.0.0.1:50777`),
`SORTIE_TOKEN` (токен роли `generator`), `SORTIE_GEN_DCS_DATA=installed` (брать подвески и
раскраски из установленной DCS вместо встроенных в pydcs; сборка перестаёт быть одинаковой на
разных машинах).

## Разработка

```sh
uv sync
uv run pytest
```

Стабы gRPC в `src/sortie_gen/_proto` генерируются `uv run python scripts/gen_proto.py` из
`../../proto` и лежат в git; `pytest` проверяет, что они не устарели.
