# Подключение SDK

SDK не публикуется на crates.io и PyPI. Cargo и uv берут его прямо из Git-репозитория
[lynxr/sortie-sdk](https://github.com/lynxr/sortie-sdk) по тегу выпуска `v0.1.0`.
В шаблоне миссии это уже настроено; ниже — как подключить SDK к своему проекту.

## Логика на Rust

```toml
[dependencies]
sortie = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.1.0" }
# Только если логика хранит свои структуры через ctx.state (derive Serialize/Deserialize):
serde = { version = "1", features = ["derive"] }

[build-dependencies]
sortie-build = { git = "https://github.com/lynxr/sortie-sdk", tag = "v0.1.0" }
```

* `sortie` — сам SDK: контекст миссии, команды, мир, события, макросы.
* `sortie-build` — помощник для `build.rs`: делает из `scenario.lock.json` модуль с
  типизированными символами сценария.
* Остальные крейты SDK (`sortie-macros`, `sortie-types`, `sortie-world`, `sortie-manifest`,
  `sortie-proto`) cargo подтянет сам; отдельно их подключать не нужно.

Тег закреплён: `cargo update` не переводит проект на новую версию SDK. Чтобы обновиться,
поменяйте тег в обеих строках.

## Генератор на Python

Добавьте `sortie-gen` в свой uv-проект из того же репозитория:

```powershell
uv add "sortie-gen @ git+https://github.com/lynxr/sortie-sdk@v0.1.0#subdirectory=python/sortie-gen"
```

Или из wheel со страницы выпуска [v0.1.0](https://github.com/lynxr/sortie-sdk/releases/tag/v0.1.0),
если Git недоступен:

```powershell
uv add .\sortie_gen-0.1.0-py3-none-any.whl
```

`sortie-gen` требует Python 3.11 или новее и ставит pydcs 1.0.0, grpcio и protobuf. Сценарий
запускается через uv: `uv run python scenario.py build`.

## Закрытый репозиторий

Если репозиторий SDK закрытый, cargo и uv должны уметь в него войти. Проще всего отдать это
Git: в Git для Windows уже есть Git Credential Manager.

1. Проверьте, что Git сам открывает репозиторий:

   ```powershell
   git ls-remote https://github.com/lynxr/sortie-sdk
   ```

   В первый раз откроется окно входа GitHub. После входа Git запомнит учётные данные.

2. Скажите cargo ходить в Git через командную строку Git. Добавьте в `.cargo\config.toml`
   проекта или в `%USERPROFILE%\.cargo\config.toml`:

   ```toml
   [net]
   git-fetch-with-cli = true
   ```

   То же можно задать переменной окружения `CARGO_NET_GIT_FETCH_WITH_CLI=true`.

3. uv и так вызывает Git, поэтому ему хватает шага 1.

Без окна входа (CI, сервер) используйте personal access token GitHub с доступом только на
чтение содержимого репозитория. Git Credential Manager примет его вместо пароля. Не вписывайте
токен в `Cargo.toml`, `pyproject.toml` или URL: эти файлы попадают в Git.

## Версии

Версия SDK, генератора и рантайма одна: 0.1.0. Обновляйте их вместе: новый тег в
`Cargo.toml` и `pyproject.toml` и новый архив рантайма (см.
[Эксплуатацию](operations.md#обновление)).
