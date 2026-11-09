# Эксплуатация

## Где что лежит

Всё, что ставит и пишет Sortie, лежит в папке Saved Games экземпляра DCS:
`%USERPROFILE%\Saved Games\DCS`, а для `--instance server` — `Saved Games\DCS.server`.

| Путь | Что |
| --- | --- |
| `Scripts\Sortie\sortie-mission.lua`, `Scripts\Sortie\layer\` | Lua-слой моста |
| `Scripts\Sortie\bin\sortie_bridge-<версия>.dll` | DLL моста; версия в имени, потому что загруженную DLL Windows перезаписать не даст |
| `Scripts\Sortie\pending.txt`, `active.txt` | какую DLL загрузить; при старте DCS `pending` становится `active` |
| `Scripts\Hooks\sortie-hook.lua` | хук: загружает мост в среду hooks DCS |
| `Sortie\link.secret` | общий секрет моста и движка |
| `Sortie\link.addr` | адрес, на котором слушает `sortied`; мост читает его перед каждым подключением |
| `Sortie\tokens.toml` | токены API |
| `Sortie\sortie.db` | хранилище движка: сборки, состояние, итоги |
| `Sortie\Logs\` | логи `sortied` |
| `Logs\sortie-bridge.log` | лог моста |
| `Missions\Sortie\` | зарегистрированные сборки: `<сценарий>-<build_id>.miz` |

В папке DCS Sortie меняет одну строку в `Scripts\MissionScripting.lua`. Больше ничего в
установке DCS не трогается.

## `sortied`

```text
sortied [--instance <имя>] [--saved-games <папка>] [--dcs <папка DCS>]
        [--api 127.0.0.1:50777] [--link pipe:<имя> | tcp:<host:port>]
        [--db <файл> | memory] [--tokens <файл>] [--manifest <файл.manifest.pb>]
        [--dev] [--dev-eval] [--log-dir <папка> | none]
```

| Флаг | Переменная | Что делает |
| --- | --- | --- |
| `--instance` | `SORTIE_INSTANCE` | экземпляр DCS; то же имя, что у `sortie install` |
| `--saved-games` | `SORTIE_SAVED_GAMES` | папка Saved Games экземпляра |
| `--dcs` | `SORTIE_DCS` | папка DCS для проверок `doctor` при старте |
| `--api` | `SORTIE_API` | адрес API; держите его на `127.0.0.1`: API управляет сервером |
| `--link` | `SORTIE_LINK` | где ждать мост; по умолчанию именованный канал экземпляра |
| `--db` | `SORTIE_DB` | файл хранилища; `memory` — без файла, для экспериментов |
| `--tokens` | `SORTIE_TOKENS` | файл токенов вместо `Sortie\tokens.toml` |
| `--manifest` | `SORTIE_MANIFEST` | manifest для миссии, которую не регистрировали (собрана `build` и открыта вручную) |
| `--dev` |  | локальные программы работают без токена; спавн и уничтожение в логике — только с `.key(..)` |
| `--dev-eval` |  | разрешить отладочные команды `DevEval` (только роль `admin`) |
| `--log-dir` | `SORTIE_LOG_DIR` | папка логов; `none` — только консоль |

Подробность логов задаёт `SORTIE_LOG` (по умолчанию `info`), например `SORTIE_LOG=debug`.

## Токены

```toml
# Saved Games\DCS\Sortie\tokens.toml
[[token]]
name = "my_mission logic"
role = "logic"          # admin | logic | generator | observer
token = "6f1c..."       # клиент шлёт заголовок `authorization: Bearer <token>`
```

| Роль | Что можно |
| --- | --- |
| `observer` | читать: версии, здоровье, мир, события, состояние, итоги |
| `logic` | плюс подключать логику и отдавать команды, писать состояние |
| `generator` | регистрировать и запускать сборки, перезапускать миссию, писать состояние |
| `admin` | всё |

`sortie install` создаёт токены ролей `admin` и `logic`. Токен `generator` добавьте сами, если
не хотите отдавать генератору `admin`; токен — любая длинная случайная строка. `sortied` читает
файл при старте.

* Логика берёт токен из `SORTIE_TOKEN`.
* Генератор — из `--token` или `SORTIE_TOKEN`.
* `sortie status`, `events`, `top` — из `--token`, `SORTIE_TOKEN` или сами из `tokens.toml`.

С `sortied --dev` программы на той же машине работают без токена.

## Dedicated server

Dedicated server с Sortie ещё не проверялся в живой игре. Ниже — как он должен работать.

* Сервер запускается с `-w DCS.<имя>` и пишет в `Saved Games\DCS.<имя>`. Передайте то же имя
  всем командам: `sortie install --instance <имя>`, `sortie doctor --instance <имя>`,
  `sortied --instance <имя>`. Мост берёт имя экземпляра из своей папки сам.
* Если DCS World Server стоит рядом с игрой, укажите его папку явно:

  ```powershell
  .\bin\sortie.exe install --instance server --dcs "C:\Program Files\Eagle Dynamics\DCS World Server"
  .\bin\sortie.exe doctor --instance server --dcs "C:\Program Files\Eagle Dynamics\DCS World Server"
  .\bin\sortied.exe --instance server
  ```

* С пустым списком миссий сервер не стартует. Положите в список миссию-заглушку: в ней нет
  юнитов, только сообщение о том, загружен ли мост. `doctor` проверяет, что список не пустой.

  ```powershell
  uv run sortie-gen lobby --out sortie-lobby.miz
  ```

* `python scenario.py launch` регистрирует сборку и просит движок загрузить её в DCS.
  `python scenario.py restart` перезагружает текущую миссию: сессия кончается с причиной
  `restart`, начинается новая, логика получает `Start::Restart`.
* На паузе скрипты миссии не выполняются. Команды логики ждут конца паузы, если не задано
  `.on_pause(Fail)`. Проверка типов на сервере, который стоит на паузе без игроков, не пройдёт:
  генератор предупредит и зарегистрирует сборку без неё.
* Перезапуска DCS после краша или зависания Sortie пока не делает. Если на машине есть
  DCSServerBot, эту работу делает он.

## Логи

| Что | Где |
| --- | --- |
| движок | консоль `sortied` и `Saved Games\DCS\Sortie\Logs\sortied.log.<дата>` |
| мост | `Saved Games\DCS\Logs\sortie-bridge.log` |
| Lua-слой | `Saved Games\DCS\Logs\dcs.log`: состояние связи и ошибки |
| логика | консоль программы логики; подробность — `SORTIE_LOG` |

## Наблюдение

```text
sortie status                                   # движок, мост, сессия
sortie events [--kind dead] [--from-seq N] [--limit N]
sortie top [--interval-ms 1000]                 # здоровье, связи, очереди
```

Общие флаги утилиты: `--instance`, `--saved-games`, `--dcs`, `--api`, `--token`.

## Обновление

**Рантайм.** Распакуйте новый архив рядом со старым и выполните из новой папки
`.\bin\sortie.exe install`. Новая DLL ляжет рядом со старой, а запущенная DCS продолжит работать
со старой. Перезапустите DCS: при старте она возьмёт новую DLL и новую часть Lua-слоя для hooks.
`sortie doctor` подтвердит версию. Перезапустите и `sortied`.

**SDK.** Поменяйте тег в `Cargo.toml` и `pyproject.toml` (см. [Подключение SDK](dependency.md)),
пересоберите логику и зарегистрируйте новую сборку. Версии SDK и рантайма должны совпадать.

**DCS.** Обновление DCS перезаписывает `MissionScripting.lua` и стирает строку Sortie. Тогда
мост не загрузится в миссию, и на экране будет «Sortie bridge is not installed: run sortie
doctor». После каждого обновления DCS запускайте `sortie doctor`; если строка пропала —
`sortie doctor --fix` от имени администратора.
