# Сценарий на Python

Статическая часть миссии описывается на Python пакетом `sortie-gen` поверх
[pydcs](https://github.com/pydcs/dcs) 1.0.0: карта, время старта, зоны, наземные группы, шаблоны
для спавна, аэродромы, слоты игроков, флаги и меню. Генератор собирает из сценария `.miz`,
manifest и `scenario.lock.json` и проверяет имена так же, как потом их проверят движок и логика.

## Сценарий целиком

Это сценарий, на котором построены примеры раздела [Логика на Rust](sdk.md):

```python
from dcs import planes, vehicles
from dcs.terrain import caucasus
from sortie_gen import Scenario, StartType

sc = Scenario("my_mission", schema=1, theatre=caucasus.Caucasus(),
              start="2026-06-01T08:00:00Z", time_limit="2h")

target = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)
patrol = sc.zone("Patrol", ident="PATROL", at=target.center, radius=40_000)
sc.zone("Front", ident="FRONT", points=[sc.latlon(42.35, 42.80), sc.latlon(42.35, 43.00),
                                        sc.latlon(42.20, 43.00), sc.latlon(42.20, 42.80)])

sc.ground_group("Armor", ident="ARMOR", country="Russia", at=target.center,
                units=[vehicles.Armor.T_72B] * 3 + [vehicles.Armor.BMP_2] * 2,
                heading=90, formation="line", spacing=30, roles={"armor", "target"})
sc.ground_group("SAM", ident="SAM", country="Russia", at=sc.latlon(42.285, 42.915),
                units=[vehicles.AirDefence.Osa_9A33_ln], roles={"sam"})
sc.ground_group("AAA", ident="AAA", country="Russia", at=sc.latlon(42.26, 42.885),
                units=[vehicles.AirDefence.ZSU_23_4_Shilka] * 2)

mig = planes.MiG_29S
sc.air_template("Red CAP", mig, ident="RED_CAP", country="Russia", size=2, airbase="Nalchik",
                payload=[mig.Pylon1.R_73__AA_11_Archer____Infra_Red,
                         mig.Pylon3.R_27R__AA_10_Alamo_A____Semi_Act_Rdr,
                         mig.Pylon5.R_27R__AA_10_Alamo_A____Semi_Act_Rdr,
                         mig.Pylon7.R_73__AA_11_Archer____Infra_Red])

sc.airbase("Nalchik", ident="NALCHIK", coalition="red")
sc.airbase("Kutaisi", ident="KUTAISI")   # владельца задаст полёт игроков ниже
sc.player_flight("Blue 1", planes.Su_25T, country="Georgia",
                 airbase="Kutaisi", start=StartType.Cold, size=2)

sc.flag("TARGET_DOWN")
sc.menu("SMOKE_TARGET", "Дым на цели", path=["Цель"])
sc.menu("TARGET_INFO", "Где цель?", path=["Цель"])

if __name__ == "__main__":
    sc.cli()
```

Сценарий — обычный скрипт. `python scenario.py <команда>` выполняет команду генератора (см.
[команды](#команды-сценария)); запускайте его через uv: `uv run python scenario.py build`.

## Заголовок

```python
from dcs import planes, vehicles
from dcs.terrain import caucasus
from sortie_gen import Scenario, StartType

sc = Scenario("my_mission", schema=1, theatre=caucasus.Caucasus(),
              start="2026-06-01T08:00:00Z", time_limit="2h")
```

* Имя сценария — `[a-z][a-z0-9_]*`. Из него получаются имя модуля в Rust (`my_mission`) и маркер
  (`MyMission`).
* `schema` — номер схемы сценария, целое не меньше 1. Его сверяют bootstrap миссии, движок и
  логика.
* `theatre` — карта pydcs: `caucasus.Caucasus()`, `syria.Syria()` и другие.
* `start` — время старта в RFC 3339. Без часового пояса это местное время карты, как в редакторе.
* `time_limit` — лимит сессии: `"3h"`, `"1h30m"`, `"90m"` или число секунд. Когда он истекает,
  движок заканчивает сессию с исходом `TimeLimit`. Без лимита сессию заканчивает только логика.

## Позиции

* `sc.latlon(широта, долгота)` — точка по широте и долготе.
* `sc.point(север, восток)` — точка в координатах редактора: `x` на север, `y` на восток, метры.
* `zone.center` — центр объявленной зоны.

Голый кортеж `(x, y)` генератор не принимает: по нему не понять, что это за координаты.

## Зоны

```python
target = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)
patrol = sc.zone("Patrol", ident="PATROL", at=target.center, radius=40_000)
sc.zone("Front", ident="FRONT", points=[sc.latlon(42.35, 42.80), sc.latlon(42.35, 43.00),
                                        sc.latlon(42.20, 43.00), sc.latlon(42.20, 42.80)])
```

Зона — круг (`at` и `radius` в метрах) или четырёхугольник из четырёх точек (`points`). С `ident`
зона становится символом `zones::TARGET` в Rust, с геометрией: `zones::TARGET.center()` и
`contains(..)` работают в логике без движка. Без `ident` зона попадает только в `.miz`.

## Наземные группы

```python
sc.ground_group("Armor", ident="ARMOR", country="Russia", at=target.center,
                units=[vehicles.Armor.T_72B] * 3 + [vehicles.Armor.BMP_2] * 2,
                heading=90, formation="line", spacing=30, roles={"armor", "target"})
sc.ground_group("SAM", ident="SAM", country="Russia", at=sc.latlon(42.285, 42.915),
                units=[vehicles.AirDefence.Osa_9A33_ln], roles={"sam"})
sc.ground_group("AAA", ident="AAA", country="Russia", at=sc.latlon(42.26, 42.885),
                units=[vehicles.AirDefence.ZSU_23_4_Shilka] * 2)
```

* Юниты называются `<имя группы>-<k>`: `Armor-1`, `Armor-2` и так далее.
* `formation` — `line`, `star`, `rectangle` или `vee`; `spacing` — расстояние между юнитами в
  метрах; первый юнит стоит в `at`.
* Ещё есть `heading` (градусы), `skill`, `late_activation`, `hidden`.
* `roles` — метки группы, `[a-z][a-z0-9_-]*`; они попадают в lock.
* С `ident` группа становится символом `groups::ARMOR` типа `GroupRef<MyMission, Ground>`.

## Шаблоны для спавна

```python
mig = planes.MiG_29S
sc.air_template("Red CAP", mig, ident="RED_CAP", country="Russia", size=2, airbase="Nalchik",
                payload=[mig.Pylon1.R_73__AA_11_Archer____Infra_Red,
                         mig.Pylon3.R_27R__AA_10_Alamo_A____Semi_Act_Rdr,
                         mig.Pylon5.R_27R__AA_10_Alamo_A____Semi_Act_Rdr,
                         mig.Pylon7.R_73__AA_11_Archer____Infra_Red])
```

Шаблон авиагруппы в `.miz` не попадает. Он хранится в manifest, и логика спавнит его
(`ctx.spawn_group(templates::RED_CAP)`). Параметры: тип самолёта или вертолёта, `ident`
(обязателен), страна, `size`, `task`, `skill`, место по умолчанию (`at` или `airbase`),
`altitude`, `speed`, `payload`.

Подвеска. По умолчанию генератор работает герметично: только с данными, встроенными в pydcs, без
установленной DCS. Поэтому один и тот же сценарий собирается одинаково на любой машине. Подвесок
по умолчанию в этом режиме нет, и шаблон без `payload` появится без оружия; генератор
предупредит. Перечислите пилоны, как в примере, или задайте переменную окружения
`SORTIE_GEN_DCS_DATA=installed`: тогда генератор возьмёт подвески установленной DCS, но сборка
станет зависеть от машины.

## Аэродромы и слоты игроков

```python
sc.airbase("Nalchik", ident="NALCHIK", coalition="red")
sc.airbase("Kutaisi", ident="KUTAISI")   # владельца задаст полёт игроков ниже
sc.player_flight("Blue 1", planes.Su_25T, country="Georgia",
                 airbase="Kutaisi", start=StartType.Cold, size=2)
```

* `sc.airbase(имя, ident=...)` делает аэродром символом `airbases::NALCHIK` для логики.
  `coalition` меняет владельца аэродрома в миссии; без него остаётся владелец по умолчанию.
* `sc.player_flight(..)` создаёт клиентские слоты. Старт: `StartType.Cold`, `Hot`, `Runway` или
  `Air` (тогда нужны `at` и `altitude`). Ещё есть `task`, `skill`, `payload`. Полёты игроков
  попадают в manifest, но не в lock: символов для них нет.

**Правило аэродрома.** В сетевой игре DCS даёт занять стоянку только той коалиции, которой
принадлежит аэродром. На нейтральном или чужом аэродроме слот не занять, и игрок остаётся
наблюдателем. Поэтому полёт игроков со стартом `Cold`, `Hot` или `Runway` отдаёт свой аэродром
коалиции полёта. Сборка падает с объяснением, если:

* сценарий объявил этот аэродром за другую коалицию (`sc.airbase(..., coalition=...)`);
* на одном аэродроме стоят игроки двух коалиций.

Если хук `@sc.pydcs` сам ставит клиентскую группу на чужой аэродром, генератор предупредит.

## Флаги и меню

```python
sc.flag("TARGET_DOWN")
sc.menu("SMOKE_TARGET", "Дым на цели", path=["Цель"])
sc.menu("TARGET_INFO", "Где цель?", path=["Цель"])
```

* `sc.flag(ident, name=None, initial=0)` — флаг миссии `flags::TARGET_DOWN`. Имя флага в DCS по
  умолчанию равно `ident`; начальное значение ставит триггер в `.miz`.
* `sc.menu(ident, надпись, path=[...])` — команда меню F10 `menus::SMOKE_TARGET`. `path` — путь
  подменю. В игре команда появляется, когда её добавит логика.

## Статика, погода и брифинг

Чего нет в API сценария, делается через pydcs напрямую. Функция с `@sc.pydcs` получает миссию
pydcs при каждой сборке, после объявлений Sortie:

```python
@sc.pydcs
def extras(m: dcs.Mission) -> None:
    # Статический объект: штаб рядом с целью (в .miz, без символа для Rust).
    hq = dcs.mapping.Point(target.center.x + 800, target.center.y, m.terrain)
    m.static_group(m.country("Russia"), "HQ", dcs.statics.Fortification._Command_Center, hq)
    # Погода и брифинг.
    m.weather.clouds_density = 4
    m.weather.clouds_base = 2_500
    m.set_description_bluetask_text("Уничтожить бронегруппу в зоне Target.")
```

* Группы и статика, добавленные хуком, попадают в manifest без `ident`. Символов для них нет;
  логика находит их по имени через мир и события.
* Генератор проверяет уникальность всех имён и после хуков.

Свой DO SCRIPT в хуке пишите через `sortie_gen.miz.InlineDoScript`, а не через `DoScript` из
pydcs:

```python
@sc.pydcs
def hello(m: dcs.Mission) -> None:
    trig = TriggerStart(comment="hello")
    trig.add_action(InlineDoScript('trigger.action.outText("Миссия загружена", 10)'))
    m.triggerrules.triggers.append(trig)
```

`DoScript` из pydcs хранит код в словаре миссии, а редактор миссий читает такое действие как
код. Миссия, пересохранённая в редакторе, выполнит имя ключа словаря как Lua и упадёт.
`InlineDoScript` пишет код прямо в действие, как это делает сам редактор. Bootstrap Sortie
устроен так же.

## Правила имён

Генератор отказывает сразу, с понятным сообщением:

* имена групп уникальны среди групп, шаблонов и слотов, не длиннее 128 байт, без пробелов по
  краям и управляющих символов;
* `#` в именах групп занят: так движок называет группы, созданные спавном;
* `ident` — `[A-Z][A-Z0-9_]*`, уникален среди символов своего вида;
* страна, аэродром, тип юнита и задача проверяются по данным pydcs, с подсказкой похожего имени.

## Команды сценария

```text
python scenario.py lock      [--out FILE] [--check]
python scenario.py build     [--out DIR] [--stem NAME] [--no-miz] [--build-id ULID] [--lock FILE] [--locked]
python scenario.py validate  [--engine ADDR] [--token T]
python scenario.py register  [--out DIR] [--lock FILE] [--locked] [--no-validate] [--engine ADDR] [--token T]
python scenario.py launch    [--build-id ID] [--out DIR] [--lock FILE] [--locked] [--no-validate] [--engine ADDR] [--token T]
python scenario.py restart   [--engine ADDR] [--token T]
python scenario.py watch     [--session S] [--kinds birth,dead,...] [--from-seq N] [--engine ADDR] [--token T]
python scenario.py results   [--session S] [--json] [--engine ADDR] [--token T]
```

| Команда | Что делает |
| --- | --- |
| `lock` | пишет `scenario.lock.json`, если изменился набор символов, и говорит, что изменилось; `--check` только проверяет (код 1, если lock устарел) |
| `build` | собирает `dist\<build_id>.miz`, `.manifest.pb` и `.manifest.json` без движка; обновляет lock |
| `validate` | спрашивает DCS через движок, знает ли она каждый тип юнита и статики сценария, и печатает таблицу |
| `register` | берёт у движка build id, собирает, проверяет типы и регистрирует сборку; движок кладёт `.miz` в `Saved Games\DCS\Missions\Sortie\` |
| `launch` | регистрирует свежую сборку и просит движок загрузить её в DCS; с `--build-id` загружает уже зарегистрированную |
| `restart` | перезагружает миссию, которую сейчас крутит DCS: сессия кончается с причиной `restart`, начинается новая |
| `watch` | печатает состояние движка и события сессии до Ctrl-C |
| `results` | итоги последней сессии (или `--session`); `--json` — полностью |

* `--locked` у `build`, `register` и `launch` не переписывает lock, а падает, если он устарел.
  Вместе с `lock --check` это удобно для CI.
* `launch` и `restart` загружают миссию серверной функцией DCS `net.load_mission`. Они для
  dedicated server и хоста сетевой игры, и в живой игре пока не проверялись. В одиночной игре
  миссию открывают и перезапускают из меню DCS; `restart` там движок отклоняет сразу.
* Адрес движка — `--engine`, `SORTIE_ENGINE` или `127.0.0.1:50777`. Токен роли `generator` или
  `admin` — `--token` или `SORTIE_TOKEN`; с `sortied --dev` он не нужен.
* Коды выхода: 0 — готово, 1 — ошибка, 2 — возможность появится в следующей версии.

Если движок не запущен, команда скажет об этом одной строкой:

```text
error: cannot reach the Sortie engine at 127.0.0.1:50777 (...). Is sortied running? Start it with `sortied --dev` for local work, or point --engine / SORTIE_ENGINE at it.
```

### Проверка типов

pydcs знает типы из своей базы, а DCS на сервере может их не знать: опечатка в типе из хука,
модуль или мод, которого на сервере нет. Такая миссия загрузится без части юнитов. Поэтому
`register` и `launch` перед регистрацией спрашивают движок о каждом типе сборки. Движок
спрашивает миссию, которая сейчас идёт в DCS (подойдёт любая миссия с мостом, например lobby).

* Неизвестный тип останавливает команду; в ошибке видно, где он используется. `--no-validate`
  пропускает проверку.
* Если проверить нельзя (в DCS не идёт миссия, DCS на паузе), команда предупредит и
  зарегистрирует сборку без проверки.

## Что получается на выходе

| Файл | Что это |
| --- | --- |
| `<build_id>.miz` | миссия pydcs и один триггер MISSION START с bootstrap Sortie |
| `<build_id>.manifest.pb` | manifest: группы, юниты, зоны, шаблоны, флаги, аэродромы, меню, слоты |
| `<build_id>.manifest.json` | то же для людей |
| `scenario.lock.json` | символы сценария и геометрия зон для логики |

Bootstrap при старте миссии вызывает мост и передаёт ему имя сценария, build id, digest
manifest и номер схемы. Движок сверяет их с зарегистрированной сборкой и начинает сессию.

`build_id` — ULID. Офлайн-сборка выводит его из содержимого, поэтому повторная сборка того же
сценария даёт побайтно те же файлы. `register` берёт свежий build id у движка. Файлы пишутся
атомарно.

## Редактор миссий

Собранную `.miz` можно открыть в редакторе миссий, пересохранить и запустить оттуда: bootstrap
останется рабочим. Но если в редакторе поменять группы, юниты или зоны, manifest перестанет им
соответствовать. Такие правки делайте в сценарии и пересобирайте. Не загружайте и не
пересохраняйте готовую `.miz` через pydcs: он вернёт DO SCRIPT в форму со словарём.

## Сборка из кода

Сценарий можно собрать без командной строки:

```python
def build_to(out_dir: Path) -> Path:
    b = sc.build()               # в памяти, без движка; build id выводится из содержимого
    for text in b.warnings:      # те же предупреждения, что печатает `build`
        print("warning:", text)
    print(b.build_id, b.digest)  # digest = sha256 manifest.pb, его сверяет bootstrap
    files = b.write(out_dir)     # <build_id>.miz, .manifest.pb, .manifest.json
    return files["miz"]
```

У результата `sc.build()` есть `build_id`, `digest`, `miz` (байты), `manifest_pb`,
`manifest_json`, `lock`, `warnings` и `type_uses` (типы юнитов и где они используются).

## Команда `sortie-gen`

```text
sortie-gen lobby  [--out missions/sortie-lobby.miz] [--theatre caucasus]
sortie-gen adopt  my.miz [--json]
sortie-gen lock|build|validate|register|launch|restart|watch|results <scenario.py> [аргументы]
```

* `lobby` — миссия-заглушка для списка миссий dedicated server: без юнитов, с одним сообщением о
  том, загружен ли мост.
* `adopt` — разбор миссии, сделанной в редакторе: коалиции, группы, юниты, зоны, предложенные
  `ident`, конфликтующие и занятые имена. Помогает перенести миссию в сценарий.
* Команды сценария можно запускать и так: `uv run sortie-gen build scenario.py`.

`sortie-gen worker` (цикл кампании) в этой версии не работает и выходит с кодом 2.
