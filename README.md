# Sortie SDK

Sortie — внешний движок миссий для DCS World. Логика миссии — обычная асинхронная программа на
Rust: она читает мир из локальной реплики и отдаёт команды, а движок `sortied` выполняет их в
DCS. Генератор `sortie-gen` на Python собирает из сценария `.miz`, manifest и
`scenario.lock.json`, по которому логика получает типизированные символы сценария.

Это SDK версии 0.1.0.

* Документация: https://lynxr.github.io/sortie-sdk/ (исходники — в [`docs/`](docs/)).
* Навыки для ИИ-агентов, которые пишут миссии на SDK: [`agents/`](agents/).
* Релиз v0.1.0: https://github.com/lynxr/sortie-sdk/releases/tag/v0.1.0.
  В нём архив движка `sortie-runtime-0.1.0-win64.zip` (`sortied`, `sortie` и мост
  для DCS) и колесо генератора `sortie_gen-0.1.0-py3-none-any.whl`.

| Папка | Что в ней |
|---|---|
| `crates/` | Rust-крейты SDK: `sortie`, `sortie-build` и их зависимости |
| `python/sortie-gen/` | генератор миссий (Python, pydcs) |
| `proto/` | схемы protobuf API движка (`sortie.v1`) и manifest |
| `template/` | шаблон проекта миссии: начните с его копии |
| `examples/op_north/` | пример миссии |
| `docs/`, `site/` | документация и собранный из неё сайт |
| `agents/` | навыки для ИИ-агентов |

## Подключение

`Cargo.toml` логики миссии:

```toml
[dependencies]
sortie = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.1.0" }

[build-dependencies]
sortie-build = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.1.0" }
```

Генератор в проекте uv:

```powershell
uv add "sortie-gen @ git+https://github.com/lynxr/sortie-sdk@v0.1.0#subdirectory=python/sortie-gen"
```

или из колеса релиза: `uv add .\sortie_gen-0.1.0-py3-none-any.whl`.

Если репозиторий закрытый, cargo и uv нужен доступ к нему через git; как его настроить, описано в
документации.

## Чего пока нет

Тестов логики без DCS: `#[sortie::test]` и FakeDcs появятся в SDK 0.2. Пока логику проверяют на
живой DCS с `sortied --dev`. Подробности — в [CHANGELOG.md](CHANGELOG.md).

## Лицензия

SDK распространяется по лицензии MIT или Apache-2.0 на ваш выбор: [LICENSE-MIT](LICENSE-MIT),
[LICENSE-APACHE](LICENSE-APACHE). Движок из архива runtime поставляется по отдельной лицензии
(`LICENSE.txt` в архиве).
