# Логика на Rust

Логика миссии — обычная программа на Rust с крейтом `sortie`. Она подключается к движку
`sortied`, получает сессию, когда в DCS стартует миссия, и выполняет вашу функцию миссии. Мир
логика читает синхронно, из локальной копии. Команды в DCS — это `async`-вызовы, которые
возвращают результат из игры.

Готовый проект — шаблон из [быстрого старта](quickstart.md). Большой пример —
[examples/op_north](https://github.com/lynxr/sortie-sdk/tree/v0.1.0/examples/op_north)
в репозитории SDK.

## Крейт логики

Зависимости — `sortie` и, для `build.rs`, `sortie-build` (см. [Подключение SDK](dependency.md)).
`build.rs` делает из `scenario.lock.json` модуль с символами сценария. Путь к lock-файлу —
относительно `Cargo.toml`:

<!-- snippet: template-build-rs -->
```rust
fn main() {
    sortie_build::scenario("my_mission", "scenario.lock.json").emit();
}
```

`src/main.rs` — одна строка. Макрос разбирает аргументы командной строки, подключается к движку,
обслуживает сессии и переподключается при обрыве связи:

<!-- snippet: template-main-rs -->
```rust
sortie::main!(my_mission_logic::run);
```

Вся логика живёт в `src/lib.rs`:

<!-- snippet: sdk-lib -->
```rust
use sortie::prelude::*;

sortie::include_scenario!(my_mission); // pub mod my_mission из scenario.lock.json
use my_mission::{MyMission, groups};

#[sortie::mission]
pub async fn run(ctx: Ctx<MyMission>, start: Start) -> sortie::Result<()> {
    if start == Start::Restart {
        ctx.message(Coalition::Blue)
            .text("Миссия перезапущена")
            .key("restarted")
            .await?;
    }
    let sam = ctx.group(groups::SAM)?; // Group<Ground>
    sam.set_option(Alarm::Red).key("sam-red").await?;
    ctx.until_session_end().await; // держать сессию до конца
    Ok(())
}
```

* `sortie::prelude::*` даёт всё, что обычно нужно: `Ctx`, `Start`, `Coalition`, `Alarm`, `Roe`,
  `Outcome`, единицы `secs`, `mins`, `km`, `ft`, `kts` и другие.
* `sortie::include_scenario!(my_mission)` подключает модуль, который сгенерировал `build.rs`.
  Имя должно совпадать с первым аргументом `sortie_build::scenario(..)`.
* `#[sortie::mission]` проверяет сигнатуру функции миссии. Ошибка в ней видна при компиляции, в
  месте определения функции.

## Символы сценария

Модуль сценария содержит маркер сценария (`MyMission` для `my_mission`) и модули `groups`,
`zones`, `templates`, `flags`, `airbases`, `menus`. Каждый символ — константа своего типа:

<!-- snippet: sdk-symbols -->
```rust
use my_mission::{airbases, flags, menus, templates, zones};
use sortie::types::{AirbaseRef, FlagRef, GroupRef, MenuRef, TemplateRef, ZoneRef};

pub const SAM: GroupRef<MyMission, Ground> = groups::SAM;
pub const RED_CAP: TemplateRef<MyMission, Air> = templates::RED_CAP;
pub const TARGET: ZoneRef<MyMission> = zones::TARGET;
pub const NALCHIK: AirbaseRef<MyMission> = airbases::NALCHIK;
pub const TARGET_DOWN: FlagRef<MyMission> = flags::TARGET_DOWN;
pub const SMOKE_TARGET: MenuRef<MyMission> = menus::SMOKE_TARGET;
```

* Вид группы входит в тип: `Group<Ground>` нельзя отправить в воздушный патруль, такой код не
  скомпилируется. Символ другого сценария тоже не подойдёт.
* Символ появляется, только если в сценарии у объекта есть `ident`. Группы без `ident` доступны
  по имени через мир и события.
* У зон в константе есть геометрия: `zones::TARGET.center()` и `zones::TARGET.contains(point)`
  работают без движка.
* `scenario.lock.json` меняется, только когда меняется набор символов. Сдвиг зоны или группы
  в сценарии пересборки логики не требует.

## Функция миссии и сессии

Сессия — один запуск миссии в DCS. Функция миссии вызывается на каждую сессию, и аргумент
`Start` говорит почему:

| `Start` | Когда |
| --- | --- |
| `Fresh` | миссию загрузили в DCS: новая сессия |
| `Restart` | миссию перезапустили (`scenario.py restart`): новая сессия того же хода |
| `AfterCrash` | процесс логики перезапустился, а сессия та же |
| `Resumed` | движок восстановил сессию после своего перезапуска (появится в следующих версиях) |

Ход — это одна сборка миссии (build id). Рестарт той же сборки — новая сессия того же хода;
новая сборка (`register`) — новый ход.

Функция может закончиться раньше сессии: фоновые задачи и меню продолжат работать. Когда сессия
кончается, все задачи сессии останавливаются, а незавершённые команды получают
`Error::SessionEnded`. Это не считается ошибкой логики.

Факты о сессии: `ctx.session_id()`, `ctx.scenario()`, `ctx.turn()` (build id хода),
`ctx.is_session_ended()` и часы:

<!-- snippet: sdk-clock -->
```rust
let now = ctx.model_time(); // секунды модельного времени
let paused = ctx.paused(); // DCS на паузе
```

## Команды

Каждая команда — построитель. Ничего не происходит, пока команду не дождались через `.await`;
`.await` возвращает результат из DCS.

<!-- snippet: sdk-message -->
```rust
ctx.message(Coalition::Blue)
    .text("Цель в 35 км к востоку от Кутаиси")
    .secs(20) // сколько секунд показывать (по умолчанию 10)
    .key("brief")
    .await?;
ctx.smoke(zones::TARGET.center(), SmokeColor::Red).await?;
```

Общие настройки любой команды:

| Метод | Что делает |
| --- | --- |
| `.key("имя")` | ключ идемпотентности: команда с ключом, который уже выполнился в этой сессии, сразу возвращает сохранённый результат и до DCS не доходит |
| `.timeout(secs(5))` | `Error::Timeout`, если DCS не ответила вовремя |
| `.after(&other)` | выполнить только после команды `other`; если она упала, упадёт и эта |
| `.on_pause(PausePolicy::Fail)` | на паузе сразу `Error::Paused`; по умолчанию команда ждёт конца паузы |

`PausePolicy` лежит в `sortie::proto::v1`.

### Ключи и идемпотентность

Функция миссии выполняется заново после перезапуска процесса логики (`Start::AfterCrash`). Ключ
не даёт повторить то, что уже сделано: второй спавн с тем же ключом вернёт ту же группу, а не
создаст новую.

* Ключ действует в пределах сессии. После рестарта миссии ключи снова свободны.
* Если команда с ключом получила `Timeout`, уже уйдя в DCS, повтор с тем же ключом не отправит её
  второй раз, а дождётся её позднего результата.
* Если связь с миссией оборвалась, пока команда была в пути, повтор с тем же ключом получит
  `Error::Unavailable`: неизвестно, выполнилась ли она. Для спавна движок проверяет это по
  снимку мира после переподключения: если группы нет, ключ снова свободен.
* С `sortied --dev` спавн и уничтожение без ключа отклоняются (`Error::Invalid`). Давайте ключи
  всем командам, которые нельзя повторять.

### Сообщения, дым, флаги

* `ctx.message(кому)`: кому — `Coalition::Blue`, группа (`&group` или нажавшая меню группа) или
  `sortie::MessageTarget::All`. Дальше `.text(..)`, `.secs(..)` (по умолчанию 10 с),
  `.clear_view()`.
* `ctx.smoke(точка, цвет)`: точка на земле (`MapPoint`, например `zones::TARGET.center()`) или в
  воздухе (`Vec3`).
* Флаги миссии объявляются в сценарии (`sc.flag(..)`):

<!-- snippet: sdk-flags -->
```rust
ctx.flag(flags::TARGET_DOWN).set(true).await?; // bool или i32
let value: i32 = ctx.flag(flags::TARGET_DOWN).get().await?;
```

### Группы и опции ИИ

`ctx.group(groups::X)` возвращает живую группу, сверенную с миссией. Ошибки:
`Error::UnknownSymbol`, если в запущенной миссии такого символа нет (логика собрана с другим
lock), и `Error::NotInWorld`, если символ есть, а живых юнитов сейчас нет.

<!-- snippet: sdk-options -->
```rust
let sam = ctx.group(groups::SAM)?;
sam.set_option(Alarm::Red).key("sam-red").await?;
sam.set_option(Roe::WeaponFree)
    .key("sam-free")
    .timeout(secs(5)) // Error::Timeout, если DCS не ответила за 5 с
    .await?;
```

* `Alarm`: `Auto`, `Green`, `Red`. `Roe`: `WeaponFree`, `OpenFireWeaponFree`, `OpenFire`,
  `ReturnFire`, `WeaponHold`.
* У группы есть `name()`, `alive_count()`, `is_alive()`. Группа привязана к одной «жизни»:
  группа, заново созданная под тем же именем, — это другой объект.

Уничтожить группу:

<!-- snippet: sdk-destroy -->
```rust
let aaa = ctx.group(groups::AAA)?;
aaa.destroy().key("aaa-removed").await?;
```

### Спавн и задачи авиации

Воздушные группы для спавна объявляются в сценарии шаблонами (`sc.air_template(..)`). Спавн
возвращает `Group<Air>` (или `Group<Helo>`), когда группа уже появилась в DCS:

<!-- snippet: sdk-spawn -->
```rust
let cap = ctx
    .spawn_group(templates::RED_CAP.named("Red CAP 1"))
    .from_airbase(airbases::NALCHIK, StartType::Runway)
    .key("red-cap-1")
    .await?; // Group<Air>: группа уже в DCS
let patrol = Task::cap(zones::PATROL)
    .altitude(ft(25_000))
    .speed(kts(420));
cap.push_task(patrol).timeout(secs(5)).await?;
```

* Без `.named(..)` группа получает имя `<имя шаблона>#<4 знака сессии>-<номер>`. Занятое имя —
  `Error::NameConflict`.
* Без `.from_airbase(..)` и `.at(..)` группа появляется там, где указал сценарий.
* Движок спавнит не больше 2 групп и 24 юнитов в секунду. Задачу только что созданной группе он
  отдаёт не раньше чем через 2 кадра и 0,5 с модельного времени.

Спавн в воздухе и другие задачи:

<!-- snippet: sdk-spawn-at -->
```rust
let point = zones::PATROL.center().alt(m(6_000)); // Vec3 над центром зоны
let cap = ctx
    .spawn_group(templates::RED_CAP)
    .at(point, 0.0) // курс в радианах
    .key("red-cap-air")
    .await?;
let orbit = Task::orbit(zones::PATROL.center())
    .race_track(zones::TARGET.center())
    .altitude(ft(20_000));
cap.set_task(orbit).key("red-cap-orbit").await?;
// ...
cap.push_task(Task::rtb(airbases::NALCHIK)).await?;
```

| Задача | Что делает |
| --- | --- |
| `Task::cap(зона)` | патруль над зоной; по умолчанию 6000 м и 420 узлов, меняется `.altitude(..)` и `.speed(..)` |
| `Task::orbit(точка)` | круг над точкой; `.race_track(вторая)` — «ипподром»; по умолчанию 6000 м и 350 узлов |
| `Task::rtb(аэродром)` | возврат на аэродром |
| `Task::raw(таблица)` | сырая таблица задачи DCS, без проверок |

`push_task` кладёт задачу поверх текущих (`Controller.pushTask`), `set_task` заменяет их
(`Controller.setTask`). Обе есть только у воздушных групп.

## Ожидание

<!-- snippet: sdk-sleep-timer -->
```rust
ctx.sleep(secs(30)).await; // 30 с модельного времени, пауза DCS его останавливает
ctx.timer("red-cap-delay", mins(15)).await; // дедлайн хранится в движке
```

* `ctx.sleep(..)` считает модельное время DCS: пауза его останавливает, ускорение времени
  ускоряет.
* `ctx.timer("имя", ..)` при первом вызове сохраняет дедлайн в движке. Повторный вызов после
  перезапуска логики ждёт тот же дедлайн, а не начинает отсчёт заново. Таймеры действуют в
  пределах сессии.

`ctx.wait_until(..)` ждёт условия на мире. Условие проверяется сразу, после каждого тика и после
каждого события, без запросов к движку:

<!-- snippet: sdk-wait-until -->
```rust
// Синий самолёт в зоне Target.
ctx.wait_until(|w| {
    w.units()
        .coalition(Coalition::Blue)
        .air()
        .any_inside(zones::TARGET)
})
.await;
// От бронегруппы осталось не больше двух машин.
ctx.wait_until(|w| w.group(&groups::ARMOR).alive_count() <= 2)
    .await;
```

Ждите состояния, а не события: «живых не осталось» верно и после перезапуска логики, а событие
гибели к тому времени уже прошло.

`ctx.until_session_end()` ждёт конца сессии по любой причине и возвращает исход, текст и
причину (`EndReason`: `Logic`, `TimeLimit`, `Restart`, `MissionUnloaded` и другие):

<!-- snippet: sdk-session-end -->
```rust
let end = ctx.until_session_end().await;
eprintln!("сессия закончилась: {:?}, {:?}", end.outcome, end.reason);
```

## Чтение мира

`ctx.world()` — локальная копия мира. Её обновляет движок на каждом тике; запросов к DCS чтение
не делает.

<!-- snippet: sdk-world -->
```rust
let world = ctx.world(); // не держите его через .await
let blue_players = world.players().coalition(Coalition::Blue).count();
let nearest_red = world
    .units()
    .coalition(Coalition::Red)
    .ground()
    .nearest_to(zones::TARGET.center())
    .map(|unit| unit.name.clone());
let blue_in_front = world
    .units()
    .coalition(Coalition::Blue)
    .count_inside(zones::FRONT);
let armor = world.group(&groups::ARMOR);
let armor_center = armor.center(); // Option<MapPoint>: середина живых машин
```

| Запрос | Что выбирает |
| --- | --- |
| `w.units()` | живые юниты; дальше фильтры |
| `.coalition(..)`, `.air()`, `.planes()`, `.helos()`, `.ground()`, `.ships()` | коалиция и категория |
| `.players()`, `.in_group(..)`, `.of_type("T-72B")` | игроки, группа, тип |
| `.in_zone(..)`, `.within(точка, km(10))` | зона или круг |
| `.including_dead()`, `.dead()` | вместе с погибшими или только погибшие |
| `.count()`, `.any()`, `.first()`, `.iter()`, `.nearest_to(точка)` | результат |
| `.any_inside(зона)`, `.count_inside(зона)`, `.inside(&зона)` | сразу с зоной |
| `w.players()`, `w.statics()` | юниты игроков, статические объекты |
| `w.group(&g)` | группа по символу, по `Group` или по имени: `alive_count()`, `size()`, `is_dead()`, `center()`, `any_inside(..)`, `all_inside(..)` |

У юнита есть имя, тип, коалиция, категория, группа, позиция, курс, скорость, игрок и `dead`.

Не держите `ctx.world()` через `.await`: пока ссылка жива, движок не может обновить мир. Clippy
ловит это правилом `await_holding_refcell_ref`.

## События

`ctx.events()` — события сессии с момента подписки. Когда вы видите событие, мир уже обновлён:
в обработчике гибели погибший юнит уже не считается живым.

<!-- snippet: sdk-events -->
```rust
let armor = ctx.group(groups::ARMOR)?;
let mut deaths = ctx.events().of::<v1::Dead>().in_group(&armor);
while let Some(dead) = deaths.next().await {
    let name = dead.initiator.map(|unit| unit.name).unwrap_or_default();
    ctx.message(Coalition::Blue)
        .text(format!("Уничтожен {name}"))
        .await?;
}
```

`.of::<T>()` выбирает один тип события; фильтры: `.in_group(..)`, `.coalition(..)`,
`.filter(..)`. Типы событий лежат в `sortie::proto::v1`: `Birth`, `Dead`, `UnitLost`, `Kill`,
`Hit`, `Shot`, `Takeoff`, `Land`, `BaseCaptured`, `Mark`, `PlayerEnteredUnit`, `UnitGone`,
`PlayerConnected`, `PlayerDisconnected`, `PlayerChangedSlot`, `ChatMessage`,
`SimulationPaused`, `SimulationResumed`, `Gap`.

<!-- snippet: sdk-events-players -->
```rust
let mut entered = ctx
    .events()
    .of::<v1::PlayerEnteredUnit>()
    .coalition(Coalition::Blue);
while let Some(e) = entered.next().await {
    ctx.message(Coalition::Blue)
        .text(format!("{} занял слот", e.player))
        .await?;
}
```

<!-- snippet: sdk-events-filter -->
```rust
let mut hits = ctx.events().of::<v1::Hit>().filter(|hit| {
    hit.target
        .as_ref()
        .is_some_and(|target| target.type_name == "T-72B")
});
while let Some(hit) = hits.next().await {
    let weapon = hit.weapon.map(|w| w.type_name).unwrap_or_default();
    eprintln!("попадание в танк: {weapon}");
}
```

Все события подряд:

<!-- snippet: sdk-events-all -->
```rust
let mut events = ctx.events();
while let Some(event) = events.next().await {
    match event.kind {
        Some(v1::event::Kind::Takeoff(t)) => {
            eprintln!("взлёт: {:?}", t.initiator.map(|u| u.name));
        }
        Some(v1::event::Kind::Gap(_)) => eprintln!("часть событий потеряна"),
        _ => {}
    }
}
```

`Gap` значит, что часть событий потеряна; мир при этом остаётся точным. Подписчик, который
отстал больше чем на 1024 события, пропускает самые старые и пишет предупреждение в лог.

## Меню F10

Команды меню объявляются в сценарии: `sc.menu(ident, надпись, path=[...])`. Логика добавляет их
в игру и читает нажатия. Меню для всей коалиции не знает, кто нажал:

<!-- snippet: sdk-menu -->
```rust
let info = ctx
    .menu(Coalition::Blue)
    .command(menus::TARGET_INFO)
    .key("menu-info")
    .await?;
let mut presses = info.presses();
while presses.next().await.is_some() {
    ctx.message(Coalition::Blue)
        .text("Цель: зона Target, 35 км восточнее Кутаиси")
        .await?;
}
```

`.per_group()` ставит команду каждой группе коалиции отдельно, и нажатие несёт группу. Ответ
можно отправить только ей:

<!-- snippet: sdk-menu-per-group -->
```rust
let smoke = ctx
    .menu(Coalition::Blue)
    .per_group()
    .command(menus::SMOKE_TARGET)
    .key("menu-smoke")
    .await?;
ctx.spawn(async move |ctx| {
    let mut presses = smoke.presses();
    while let Some(press) = presses.next().await {
        ctx.smoke(zones::TARGET.center(), SmokeColor::Red).await?;
        ctx.message(press.group) // ответ только той группе, что нажала
            .text("Красный дым на цели")
            .await?;
    }
    Ok(())
});
```

Нажатия копятся с момента, когда движок подтвердил команду меню, поэтому ранние нажатия не
теряются. Убрать команду:

<!-- snippet: sdk-menu-remove -->
```rust
info.remove().await?;
```

## Фоновые задачи

`ctx.spawn(..)` запускает задачу, которая живёт до конца сессии:

<!-- snippet: sdk-spawn-task -->
```rust
let watcher = ctx.spawn(async |ctx| {
    loop {
        ctx.sleep(mins(5)).await;
        let left = ctx.world().group(&groups::ARMOR).alive_count();
        ctx.message(Coalition::Blue)
            .text(format!("Осталось машин: {left}"))
            .await?;
    }
});
// ...
watcher.abort(); // необязательно: задачи и так заканчиваются вместе с сессией
```

Ошибка (кроме `SessionEnded`) или паника в задаче записывается в лог логики с местом в коде.
Остальные задачи продолжают работать.

## Состояние

Движок хранит JSON-документы. Они переживают перезапуск логики, а документы кампании — и новые
сборки миссии:

<!-- snippet: sdk-state -->
```rust
#[derive(Default, Serialize, Deserialize)]
pub struct Campaign {
    pub targets_destroyed: u32,
}

pub async fn count_kill(ctx: &Ctx<MyMission>) -> sortie::Result<()> {
    let campaign = ctx.state::<Campaign>("campaign").await?; // область: кампания
    campaign.update(|c| c.targets_destroyed += 1).await?; // запись со сверкой ревизии
    let total = campaign.get().targets_destroyed;
    ctx.report().set("targets_destroyed", total);
    Ok(())
}
```

Тип документа должен реализовать `Serialize`, `Deserialize` и `Default`; для derive добавьте
`serde` в зависимости. `update` пишет со сверкой ревизии: если документ успел поменять кто-то
другой, `update` перечитает его и применит изменение снова. Ещё есть `get()`, `set(..)`,
`reload()`, `revision()`.

| Область | Сколько живёт | Как открыть |
| --- | --- | --- |
| сессия | одну сессию; рестарт миссии начинает с нуля | `ctx.state_in(StateScope::Session, ..)` |
| ход | одну сборку миссии; переживает её рестарты | `ctx.state_in(StateScope::Turn, ..)` |
| кампания | все ходы сценария | `ctx.state(..)` |

<!-- snippet: sdk-state-scopes -->
```rust
let seen = ctx
    .state_in::<Vec<String>>(StateScope::Session, "seen")
    .await?;
seen.update(|names| names.push("Blue 1".into())).await?;
let attempts = ctx.state_in::<u32>(StateScope::Turn, "attempts").await?;
attempts.update(|n| *n += 1).await?;
```

Выполнить блок один раз:

<!-- snippet: sdk-once -->
```rust
// Один раз за сессию: перезапуск процесса логики не повторит.
ctx.once("intro", async |ctx| {
    ctx.message(Coalition::Blue).text("Удачной охоты!").await
})
.await?;
// Один раз за ход: не повторит и рестарт миссии.
ctx.once_per_turn("target-down", async |ctx| {
    ctx.flag(flags::TARGET_DOWN).set(true).await
})
.await?;
```

`once` возвращает `Ok(None)`, если блок уже выполнялся.

## Конец сессии и итоги

<!-- snippet: sdk-end -->
```rust
ctx.report().set("armor_left", 0);
ctx.end_session(Outcome::BlueWin)
    .text("Бронегруппа уничтожена")
    .await
```

* `ctx.report().set(ключ, значение)` пишет значение в итоги сессии. Запись не ждёт ответа;
  ошибка попадёт в лог логики.
* `ctx.end_session(..)` заканчивает сессию с исходом `BlueWin`, `RedWin`, `Draw` или `Aborted`.
  Текст сохраняется в итогах, в DCS он не показывается. Сама миссия в DCS продолжается.
* Если логика не закончила сессию, её закроет лимит времени из сценария (`time_limit`) с
  исходом `TimeLimit`.
* Итоги смотрит `python scenario.py results`.

## Ошибки

Всё, что может пойти не так, — `sortie::Error`. Ошибка команды несёт место в вашем коде, где
команду создали, и, если она пришла из Lua, — Lua traceback.

<!-- snippet: sdk-errors -->
```rust
match cap.push_task(Task::rtb(airbases::NALCHIK)).await {
    Ok(()) => {}
    Err(sortie::Error::Gone(_)) => {} // группы уже нет: её сбили
    Err(e) => {
        eprintln!("{}", e.report()); // место в коде, Lua traceback, подсказка
        return Err(e);
    }
}
```

| Ошибка | Что значит |
| --- | --- |
| `Gone` | объекта уже нет в DCS: погиб или исчез |
| `Timeout` | DCS не ответила вовремя |
| `Lua` | ошибка в Lua-слое; смотрите traceback |
| `Rejected` | движок отклонил команду; причина в сообщении |
| `SessionEnded` | сессия кончилась раньше команды; нормальный исход для работы в конце сессии |
| `Invalid` | команда не подходит цели, не хватает аргумента, или в dev-режиме нет `.key(..)` |
| `Unavailable` | движок не может выполнить команду сейчас |
| `NameConflict` | живая группа с таким именем уже есть |
| `Paused` | DCS на паузе, а у команды `.on_pause(Fail)` |
| `Conflict` | документ состояния поменял кто-то другой |
| `UnknownSymbol` | в запущенной миссии нет символа: логика собрана с другим lock |
| `NotInWorld` | символ есть, а живого объекта нет |
| `State` | хранилище движка отказало |
| `Transport` | нет связи с движком |
| `NotYet` | возможность появится в следующей версии |

`e.code()` даёт стабильный код вида `sortie::gone`, `e.help()` — подсказку, `e.report()` —
многострочный отчёт: место в коде, Lua traceback, id сессии и команды, подсказка.

## Запуск программы логики

```text
my_mission_logic [--engine <url>] [--build-id <id>] [--logic <name>] [--dev]

  --engine <url>    адрес движка (SORTIE_ENGINE, по умолчанию http://127.0.0.1:50777)
  --build-id <id>   build id программы, который она сообщает движку (SORTIE_BUILD_ID)
  --logic <name>    имя логики (по умолчанию имя программы)
  --dev             dev-режим: спавн и уничтожение только с .key() (SORTIE_DEV=1)
```

* Токен роли `logic` берётся из `SORTIE_TOKEN`; с `sortied --dev` он не нужен.
* Фильтр логов — `SORTIE_LOG` (по умолчанию `info`), например `SORTIE_LOG=debug`.
* При обрыве связи логика переподключается с паузой от 1 до 30 с.
* Код выхода 0 — движок остановил логику; 2 — неверные аргументы, неверный токен или движок
  отклонил логику (её надо пересобрать с lock запущенной миссии).
* Рантайм однопоточный: `Ctx` дёшево клонируется, но не `Send`.

Флаг `--standby` (горячая замена логики) зарезервирован: движок этой версии такую логику
отклоняет.

## Карта API

| Нужно | Где |
| --- | --- |
| функция миссии | `#[sortie::mission] async fn run(ctx: Ctx<M>, start: Start) -> sortie::Result<()>` |
| программа | `sortie::main!(crate_name::run)` |
| символы | `sortie::include_scenario!(name)`, `build.rs` с `sortie_build::scenario(..)` |
| группы | `ctx.group(..)`, `group.set_option(..)`, `push_task`, `set_task`, `destroy` |
| спавн | `ctx.spawn_group(templates::X)`, `.named`, `.from_airbase`, `.at` |
| сообщения, дым, флаги | `ctx.message(..)`, `ctx.smoke(..)`, `ctx.flag(..).set(..)` / `.get()` |
| меню | `ctx.menu(coalition)`, `.per_group()`, `.command(menus::X)`, `presses()`, `remove()` |
| время | `ctx.sleep`, `ctx.timer`, `ctx.model_time`, `ctx.paused` |
| мир | `ctx.world()`, `ctx.wait_until(..)` |
| события | `ctx.events()`, `.of::<v1::Dead>()`, `.in_group`, `.coalition`, `.filter` |
| задачи | `ctx.spawn(..)` |
| состояние | `ctx.state`, `ctx.state_in`, `ctx.once`, `ctx.once_per_turn` |
| итоги | `ctx.report().set(..)`, `ctx.end_session(..)`, `ctx.until_session_end()` |
| ошибки | `sortie::Error`, `code()`, `help()`, `report()` |
