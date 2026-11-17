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
| `Sortie\engine.toml` | настройки владельца движка (необязательный): скорость спавна, какие [вызовы DCS](dcs.md) включены |
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
        [--db <файл> | memory] [--tokens <файл>] [--engine-config <файл>]
        [--manifest <файл.manifest.pb>]
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
| `--engine-config` | `SORTIE_ENGINE_CONFIG` | файл настроек вместо `Sortie\engine.toml`; ошибка в нём не даёт движку стартовать |
| `--manifest` | `SORTIE_MANIFEST` | manifest для миссии, которую не регистрировали (собрана `build` и открыта вручную) |
| `--dev` |  | локальные программы работают без токена; спавн и уничтожение в логике — только с `.key(..)`; все изменяющие вызовы DCS включены |
| `--dev-eval` |  | разрешить отладочные команды `DevEval` (только роль `admin`) |
| `--log-dir` | `SORTIE_LOG_DIR` | папка логов; `none` — только консоль |

Подробность логов задаёт `SORTIE_LOG` (по умолчанию `info`), например `SORTIE_LOG=debug`.

## `engine.toml`

Необязательный файл настроек владельца движка:

```toml
# Saved Games\DCS\Sortie\engine.toml
spawn_gate = { groups_per_s = 2, units_per_s = 24 }   # спавн в секунду модельного времени

[calls]                     # каталог вызовов DCS
enable = ["Airbase.setCoalition", "Warehouse.addItem"]
disable = ["land.getIP"]

[[calls.call]]              # поменять границы аргумента
path = "trigger.action.explosion"
clamp = { power = [0, 2000] }

[operations]                # выключить типизированные операции
disable = ["remove_junk"]

[quarantine]                # карантин функций каталога и новая сборка DCS
clear_calls_on_new_build = false   # по умолчанию false
```

Изменяющие вызовы DCS выключены, пока их не перечислят в `enable` (или не запустят
`sortied --dev`). `sortied --dev` включает и изменяющие записи, которые владелец добавил сам в
`[[calls.call]]`; то, что владелец выключил (`enabled = false` у записи, список `disable`),
остаётся выключенным и с `--dev`. Ошибка в файле останавливает `sortied` при старте с сообщением
`engine.toml [calls]: ...`, в котором назван путь записи. Имена в `[operations]` — имена
[возможностей](capabilities.md) (`remove_junk`, `draw`, `dcs_call`, ...): такая операция
`DISABLED`, её команда — `Error::Rejected`.

`[quarantine] clear_calls_on_new_build` решает, снимает ли новая сборка DCS карантин функции
каталога (`call:mission:land.getHeight` после падения DCS внутри этого вызова, см. «Наблюдение»
ниже). По умолчанию `false`: не снимает, карантин остаётся до `sortie caps --release` или до
пройденного испытания. Сервер, который работает без человека, не должен второй раз падать на
том же вызове, пока человек не решил; вызов на карантине вместо этого получает понятную ошибку
`Error::Quarantined`. `true` — как в спецификации: мост новой сборки DCS снимает такие
карантины сам, как снимает карантин типизированной операции. Значение — только `true` или
`false`, другие ключи в `[quarantine]` — ошибка при старте `sortied`.

Правила слияния `[calls]` со встроенным каталогом:

* запись `[[calls.call]]` для встроенного пути меняет только `enabled`, `note`, добавляет
  `guards` (защиты только добавляются, убрать встроенную нельзя), сужает `clamp` (расширить
  встроенные границы нельзя) и повышает `class` (read < mutate < heavy < admin; понизить нельзя);
* защита `span_300km` (две точки не дальше 300 км друг от друга) допустима только у записи
  ровно с двумя обязательными аргументами `vec2` или `vec3`: это и есть точки, которые она
  измеряет. У любой другой записи, встроенной или вашей, она — ошибка при старте;
* подпись (`receiver`, `args`, `returns`, `kind`, `creates`, `destroys`, `documented`) и то, из
  чего генерируются обёртки SDK (`family`, `wrapper`, `since_api_rev`, `superseded_by`,
  `live_check`, `doc`), не меняются;
* новый путь — полная запись; его класс не ниже `mutate`, если имя не начинается с get, is или
  has; он начинается с одной из таблиц DCS, с которых слой Lua вызывает функции (`land`,
  `trigger`, `world`, `Unit`, `net`, `DCS` и т. д.); новая запись не может закрыть
  унаследованный встроенный метод (`Unit.destroy` вместо `Object.destroy`) — меняйте запись
  того класса, который метод определяет;
* в `enable` и `disable` метод пишется под классом, который его определяет, то есть путём
  самой записи каталога: `Object.destroy`, а не `Unit.destroy` или `Weapon.destroy`. Запись
  одна на все классы-наследники, поэтому её включение или выключение действует на юниты,
  статики, аэродромы и оружие сразу; имя через класс-наследник — ошибка при старте, в
  сообщении названа запись (`is the entry `Object.destroy``);
* пути из списка NEVER (`net.dostring_in`, `loadstring`, `os`, `io`, `_G`, обратные вызовы, пути
  к файлам) нельзя включить ни записью, ни через `enable`, ни с `--dev`; то же для путей с
  неописанной подписью и для `*`.

## Проверка сырых таблиц

Всё, что логика передаёт в DCS сырой таблицей, — `Task::raw(..)` (`SetTask.raw`), `.raw(..)`
статика (`SpawnStatic.raw`), аргументы `ctx.dcs()` и пакетов вызовов, — движок проверяет до
постановки команды в очередь, для любого клиента и любой ревизии API, включая логики 0.1.0:

* код Lua и имена файлов — отказ `sortie::rejected`, `capability = "dev_eval"`, в сообщении путь
  внутри таблицы (`SetTask.raw.params.tasks[2].params.action.id`). Отклоняются на любой глубине:
  `id = "Script"` и `id = "ScriptFile"` (`WrappedAction`, команды), строка под ключом `condition`
  (условие `ControlledTask` в `condition` и `stopCondition`), строка `command` внутри `params`,
  строка под ключом `file`. Регистр букв и пробелы вокруг значения не важны;
* NaN и бесконечность в ключах и значениях — `sortie::invalid` с путём; то же для чисел
  типизированных команд (точка спавна, орбита, дым, метка);
* вложенность до 40 таблиц и до 100 000 значений на команду.

С `sortied --dev-eval` команда токена `admin` с кодом проходит, а в журнал пишется предупреждение;
токен `logic` получает отказ и с этим флагом. Для отладки Lua на сервере разработки есть `DevEval`.

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
sortie caps [--prefix call:mission:] [--all]    # что не работает и почему
sortie caps --release call:mission:land.getHeight   # снять карантин (токен admin)
sortie caps --release-carried-over              # снять все карантины прежних сборок DCS
```

Если DCS упал во время команды, вид этой команды уходит на карантин (`QUARANTINED` в
`sortie caps`, заметка в `sortie status`) и остаётся там после перезапуска DCS и движка, пока
владелец не снимет его `sortie caps --release <имя>`. У универсального вызова на карантин
уходит одна функция каталога (её называет журнал сбоев моста,
`<Saved Games>\Logs\sortie-bridge.journal`); документированную функцию чтения движок один раз
испытывает при следующем старте миссии (не испытывает, пока универсальный вызов выключен в
`[operations]`). Функция, которая прошла испытание и снова дала сбой, остаётся на карантине
без второго испытания, пока владелец не снимет его. Функция, в которой мост отказал только
из-за медленных вызовов (три вызова дольше секунды за 10 минут; заметка `Slow, not a crash`),
в хранилище не попадает: отказ действует до перезапуска DCS и снимается сам. Обновление DCS такой карантин не снимает
(если не включён `[quarantine] clear_calls_on_new_build`), поэтому карантин, записанный на
прежней сборке,
движок называет перенесённым: заметка функции в `sortie caps` говорит «recorded on DCS
<прежняя>, carried over to the running build <текущая>», `sortie status` перечисляет все такие
функции (заметка `carried over to the running DCS build ...`, поле
`HealthResponse.carried_over_quarantines`). Снять их все разом — `sortie caps
--release-carried-over` (`Engine.ReleaseQuarantine` с `carried_over = true`); пока с запуска
движка не подключился ни один мост, текущая сборка неизвестна и команда отказывает.
Подробности — [Возможности движка](capabilities.md).

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
