# Логика на Rust

Логика миссии — обычная программа на Rust с крейтом `sortie`. Она подключается к движку
`sortied`, получает сессию, когда в DCS стартует миссия, и выполняет вашу функцию миссии. Мир
логика читает синхронно, из локальной копии. Команды в DCS — это `async`-вызовы, которые
возвращают результат из игры.

Готовый проект — шаблон из [быстрого старта](quickstart.md). Большой пример —
[examples/op_north](https://github.com/lynxr/sortie-sdk/tree/v0.4.0/examples/op_north)
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

Модельное время — секунды от начала миссии (`timer.getTime` в DCS); на паузе оно стоит. Это не
время суток: в миссии, которая начинается в 09:00, через минуту `ctx.model_time()` около 60, а
не 32 460. Этими часами идут тики, `ctx.sleep`, `ctx.timer`, `model_time` результатов команд и
событий. Время суток — `ctx.dcs().timer().abs_time()` (секунды от полуночи первого дня миссии).
Движок двигает свои часы только по модельному времени DCS; событие, время которого далеко
впереди (больше 5 с), часы не двигает (до 0.4.0 метка F10 останавливала их на времени суток,
и таймеры не срабатывали).

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
| `.timeout(secs(5))` | срок команды (по умолчанию 10 с, у спавна и уничтожения 30 с): не дошла до DCS или не получила ответа за срок — `Error::Timeout` ([какой из двух](#timeout-не-дошла-или-не-ответила)) |
| `.after(&other)` | выполнить только после команды `other`; если она упала, упадёт и эта |
| `.on_pause(PausePolicy::Fail)` | на паузе сразу `Error::Paused`; по умолчанию команда ждёт конца паузы |

`PausePolicy` лежит в `sortie::proto::v1`.

### Ключи и идемпотентность

Функция миссии выполняется заново после перезапуска процесса логики (`Start::AfterCrash`). Ключ
не даёт повторить то, что уже сделано: второй спавн с тем же ключом вернёт ту же группу, а не
создаст новую.

* Ключ действует в пределах сессии. После рестарта миссии ключи снова свободны.
* Ключ защищает одну команду. Другая команда (другие цель, параметры или вид) под ключом,
  который в этой сессии уже выполнился или ещё выполняется, — `Error::Invalid` («key `k` was
  already used for a different command in session ...»), и до DCS она не доходит. Это так и
  после перезапуска логики: изменили команду — дайте ей новый ключ (например, `sam-red-v2`).
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

Воздушные группы для спавна объявляются в сценарии шаблонами (`sc.air_template(..)`; наземные —
`sc.ground_template(..)`, [генератор](generator.md)). Спавн возвращает `Group<Air>` (или
`Group<Helo>`), когда группа уже появилась в DCS:

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
* `.at(точка, курс)` ставит первую машину шаблона в точку, остальные переносит со смещениями из
  шаблона, не поворачивая строй, и даёт **всем машинам один курс**; маршрут шаблона заменяет одна
  точка маршрута. `.at_formation(точка, курс)` (ревизия API 3) поворачивает строй целиком и
  сохраняет разницу курсов машин —
  [Объекты: группа из шаблона в точке](objects.md#группа-из-шаблона-в-точке).
* Движок спавнит не больше 2 групп и 24 юнитов в секунду, пока в сессии есть игрок (пока никого
  нет, например сразу после загрузки миссии на сервере, — сколько успевает мост, 20–50 в
  секунду). Очередь спавна не съедает срок команды: 100 спавнов, отправленных разом, выполнятся
  все.
* Задачу (`set_task`, `push_task`) и опцию ИИ (`set_option`) группе, созданной в этой сессии,
  движок держит у себя, пока после спавна не пройдут 2 кадра и 0,5 с модельного времени; это
  касается групп любого вида. Ждать в логике (`sleep`) не нужно. Пока спавн группы ещё в очереди
  или в DCS, это ожидание срок команды не съедает (с 0.4.0); сама задержка после ответа спавна
  (обычно до 1 с модельного времени) идёт в срок команды (по умолчанию 10 с). Вызовы
  `ctx.dcs()` (`Controller.setOption` и другие записи каталога) движок не задерживает.

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
| `Task::ground_route([точки])` | маршрут наземной группы (ревизия API 3): точки `RoutePoint::on_road(..)`, `off_road(..)`, `on_railroads(..)`, `RoutePoint::new(точка, GroundFormation::..)`, у точки и у маршрута `.speed(..)`; по умолчанию 20 км/ч ([ниже](#задачи-наземных-и-морских-групп)) |
| `Task::raw(таблица)` | сырая таблица задачи DCS, для любой группы; движок отклоняет в ней код Lua, имена файлов и NaN ([проверка сырых таблиц](operations.md#проверка-сырых-таблиц)) |

`push_task` кладёт задачу поверх текущих (`Controller.pushTask`), `set_task` заменяет их
(`Controller.setTask`). Обе есть у групп любого вида. `Task::cap`, `Task::orbit` и
`Task::rtb` — задачи самолётов и вертолётов, `Task::ground_route` — наземных групп: задача
не того вида группы не компилируется (CAP, orbit и маршрут), а RTB для наземной или морской
группы движок отклоняет (`Error::Invalid`, «only a ground route or a raw task applies to it»,
для морской — «only raw tasks apply to it»).

### Задачи наземных и морских групп

Наземную группу ведёт типизированный маршрут `Task::ground_route` (ревизия API 3:
`#[sortie::mission(api_rev = 3)]`). Пример: бронегруппа едет по дорогам на 8 км севернее цели,
а оттуда — к цели клином:

<!-- snippet: sdk-ground-route -->
```rust
let armor = ctx.group(groups::ARMOR)?; // Group<Ground>
let target = zones::TARGET.center();
let route = Task::ground_route([
    // по дорогам в точку в 8 км севернее цели
    RoutePoint::on_road(target.offset(8_000.0, 0.0)),
    // последний отрезок — напрямик, клином, 15 км/ч
    RoutePoint::new(target, GroundFormation::Vee).speed(kmh(15)),
])
.speed(kmh(40)); // для точек без своей скорости (без этого — 20 км/ч)
armor.set_task(route).key("armor-north").await?;
```

* Движок строит из маршрута задачу DCS `Mission` в том виде, в каком редактор миссий пишет
  маршрут техники. Первую точку он ставит там, где стоит первый живой юнит группы в тот кадр,
  когда DCS принимает задачу, с построением и скоростью первой точки маршрута. Свою точку
  старта давать не нужно.
* Построение и скорость точки — это построение и скорость отрезка, который в ней кончается.
  Так считает редактор миссий: отрезок идёт по дорогам, когда оба его конца — «по дорогам»,
  а время пути до точки — по её скорости.
* Построения: `on_road` (по дорогам), `off_road` (напрямик колонной), `on_railroads` (по
  железной дороге, для поездов), `GroundFormation::{Rank, Cone, Vee, Diamond, EchelonLeft,
  EchelonRight}` (напрямик шеренгой, конусом, клином, ромбом, уступом влево и вправо).
* Скорость — `Speed` (`kmh(..)`, `mps(..)`): у точки, иначе у маршрута, иначе 20 км/ч.
* Точки — это `MapPoint`: центр зоны, позиция юнита (`Vec3::map_point()`), точка пути по
  дорогам из `ctx.dcs().land().path_on_roads(..)` ([путь по дорогам](dcs.md#путь-по-дорогам)).
  В маршруте от 1 до 256 точек, длиннее движок отклоняет (`Error::Invalid`). Между двумя точками
  «по дорогам» редактор миссий сам прокладывает путь по дорогам (что DCS ведёт колонну так же,
  проверяет `AI-L4`), поэтому в маршрут идут только места, где колонна поворачивает или встаёт:
  из пути по дорогам — его конец (`path.last()`) или несколько точек поворота. Весь путь — чтобы
  измерить дорогу, а не для маршрута: в длинном пути точек может быть больше предела, и каждая
  стала бы в задаче DCS отдельным коротким отрезком. Точки поворота и длину без всего пути даёт
  [сводка пути](dcs.md#сводка-пути-по-дорогам), `ctx.dcs().land().road_path(..)`.
* Маршрут — только для `Group<Ground>`: для самолёта, вертолёта и корабля он не компилируется.
  Если та же задача придёт через `Task::from(..)`, движок её отклонит (`Error::Invalid`).
  Кораблю нужна сырая таблица (ниже).
* Логика ревизии ниже 3 получает `Error::Revision`, движок старше — `Error::EngineTooOld`, и в
  обоих случаях команда не уходит.
* Как DCS ведёт колонну (выбор дороги, выезд на ближайшую дорогу, точка в воде или за
  краем карты), вживую ещё не замерено: это проверка `AI-L4` живой пробы.

Чего в типизированном маршруте нет (остановка, задачи в точках маршрута, своё построение
`Custom`, маршрут корабля), задаёт сырая таблица задачи DCS, `Task::raw(..)`. Её принимает
любая группа. Пример: колонна встаёт.

<!-- snippet: sdk-ground-raw -->
```rust
use sortie::proto::LuaTable;
use sortie::proto::LuaValue;

// Задача DCS «Hold»: колонна встаёт, где стоит. Типизированной остановки в SDK нет.
let hold = LuaTable::new()
    .with("id", "Hold")
    .with("params", LuaTable::new());
let armor = ctx.group(groups::ARMOR)?;
armor.push_task(Task::raw(LuaValue::table(hold))).await?;
```

* Точка маршрута в сырой задаче `Mission` — таблица DCS: `type = "Turning Point"`, `action =
  "On Road"` (по дорогам) или `"Off Road"` (напрямик), `x` — север и `y` — восток в метрах
  карты (как у `MapPoint`; у `Vec3` это `x` и `z`), `speed` — в м/с. Первую точку ставьте там,
  где группа стоит сейчас: так начинает маршрут редактор миссий. Как DCS ведёт такую сырую
  задачу, проверяет `AI-L3` живой пробы.
* Задачу получает вся группа (контроллер группы DCS), а не отдельный юнит.
* Движок проверяет таблицу только на безопасность ([проверка сырых
  таблиц](operations.md#проверка-сырых-таблиц)). Ошибку в её форме может вернуть DCS
  (`sortie::lua_error`), а может и молча её проигнорировать.
* В SDK 0.3.0 и раньше `push_task` и `set_task` были только у воздушных групп, а
  типизированного маршрута не было.

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
`SimulationPaused`, `SimulationResumed`, `Gap`, `Unknown` (событие DCS без своего типа, см.
ниже); событий ревизии 1 — `ResourceEnded`, `CapabilityChanged`; ревизии 2 —
`AirbaseOwnerChanged` (смена владельца аэродрома, [Аэродромы](airbases.md#смена-владельца-событие)).
Событие ревизии выше объявленной в `#[sortie::mission(event_rev = N)]` не приходит, а
`.of::<T>()` пишет об этом предупреждение. Ревизия событий 2 — часть API ревизии 3: её
объявляют вместе, `#[sortie::mission(api_rev = 3, event_rev = 2)]`.

`model_time` любого события — модельное время, как у `ctx.model_time()`. У события метки F10
(`Mark`) так с 0.4.0: DCS кладёт в него время суток (по вики Hoggit; живая проверка `EVT-L18`
ещё не прошла), и слой Lua переводит его в модельное. Значение DCS лежит в `Mark.dcs_time`
(ревизия API 3; логика младшей ревизии этого поля не получает). Со слоем Lua старше 0.4.0 (не
сделан `sortie install`) `model_time` метки по-прежнему время суток, а `dcs_time` пуст; часы
движка от этого не страдают.

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
Пропуски в `event.seq` не значат потерю: `seq` — номер в журнале сессии, а подписчик с фильтром
или с меньшей ревизией событий видит только часть журнала. О потере говорит только `Gap`.

### Какое событие DCS это было

Движок ревизии API 1 называет у каждого события из DCS его запись `world.event`:
`event.dcs_event()` (`v1::DcsEvent`). Так логика отличает `S_EVENT_DEAD`, `S_EVENT_CRASH` и
`S_EVENT_PILOT_DEAD` (все три — `Dead`) и `S_EVENT_TAKEOFF` от `S_EVENT_RUNWAY_TAKEOFF` (оба —
`Takeoff`) без новой ревизии событий. Запись, которой нет в перечислении (новая сборка DCS), —
`DcsEvent::Unspecified` и её имя в `event.dcs_name`. События движка и хуков — `Unspecified`.

<!-- snippet: sdk-events-dcs -->
```rust
// Только обломки: гибель пилота DCS тоже присылает как Dead, а самолёт ещё летит.
let mut wrecks = ctx
    .events()
    .of::<v1::Dead>()
    .from_dcs(v1::DcsEvent::Dead)
    .from_dcs(v1::DcsEvent::Crash);
while let Some((dead, entry)) = wrecks.next_with_dcs_event().await {
    let name = dead.initiator.map(|u| u.name).unwrap_or_default();
    eprintln!("{name}: {entry:?}");
}
```

`.from_dcs(..)` можно звать несколько раз (подходит любая из записей); против движка 0.1.0
он не пропустит ничего (там записей нет) и пишет предупреждение.

С API ревизии 1 (`#[sortie::mission(api_rev = 1)]`) гибель пилота больше не делает юнит мёртвым
в мире логики: DCS присылает её, пока самолёт ещё летит. На ревизии 0 (по умолчанию у макроса)
`Dead` гибели пилота приходит с `dead = true`, как в 0.1.0, и мир логики считает юнит мёртвым
навсегда (в мире движка он жив). Выход игрока из юнита (`S_EVENT_PLAYER_LEAVE_UNIT`) убирает
игрока из `EntitySnap.player` на любой ревизии.

### Остальные события DCS

Событие DCS без своего типа приходит как `v1::Unknown` с полем `ctx` (`v1::EventContext`):
участники как снимки с Eid (`initiator`, `target`, `weapon`, `place`), `weapon_type`, `text`
(или комментарий), `pos`, `coalition`, `group_eid`, `idx`, все остальные ключи DCS в `extra`
(данные, два уровня) и объекты под другими ключами в `extra_objects`. Фильтры `.in_group` и
`.coalition` смотрят на `ctx.initiator`. `place` — аэродром (`airbase`) или объект (`entity`);
палуба корабля приходит как аэродром с именем корабля. `foreign = true` — событие поднял
другой скрипт миссии (например, MOOSE) через `world.onEvent`: в нём только данные, участники
найдены по их `id_` в мире. `Unknown.fields` — сырая таблица в виде 0.1.0, только прежние ключи
(новые — в `ctx`).

<!-- snippet: sdk-events-generic -->
```rust
let mut starts = ctx
    .events()
    .of::<v1::Unknown>()
    .from_dcs(v1::DcsEvent::EngineStartup)
    .coalition(Coalition::Blue);
while let Some(start) = starts.next().await {
    let ctx_ = start.ctx.unwrap_or_default();
    let unit = ctx_.initiator.map(|u| u.name).unwrap_or_default();
    // Ключи DCS без своего поля — в `extra`, как DCS их прислал.
    let sub = ctx_
        .extra
        .as_ref()
        .and_then(|x| x.get("subPlace"))
        .and_then(|v| v.as_f64());
    eprintln!("{unit} запустил двигатели (subPlace {sub:?})");
}
```

В FakeDcs такие события поднимает `dcs.raise_event(FakeEvent::dcs("S_EVENT_ENGINE_STARTUP")
.initiator("Viper-1").key("comment", ".."))`, событие другого скрипта —
`FakeEvent::foreign(id)`.

### Оружие и катапультировавшиеся пилоты

Оружие в полёте и катапультировавшиеся пилоты — не сущности мира: их нет в `ctx.world()`, но
у каждого есть Eid. Ракета, бомба или снаряд получает Eid в первом событии, где DCS её назвала
(`Shot`), и тот же Eid приходит в `Hit` и `Kill` этого оружия (`weapon.eid`; 0 — в событии нет
объекта оружия, только `weapon_type`, или в слоте оружия юнит при столкновении). Пилот и
кресло в `Unknown.ctx` событий катапультирования (`S_EVENT_EJECTION`,
`S_EVENT_LANDING_AFTER_EJECTION`) — снимки с Eid; кресло и пилот — разные объекты DCS с разными
Eid. Пока DCS ещё знает объект, `ctx.destroy(..)` убирает его по Eid (`Obj::weapon(&w)`,
`Obj::snap(&s)`; возможность `caps::DESTROY_WEAPON`); потом — `Error::Gone`. Eid оружия
действует до 30 с после первого попадания (и до 30 мин без событий), Eid пилота — пока его не
убрали.

<!-- snippet: sdk-events-weapons -->
```rust
let mut shots = ctx.events().of::<v1::Shot>();
let mut hits = ctx.events().of::<v1::Hit>();
let Some(shot) = shots.next().await else {
    return Ok(());
};
let missile = shot.weapon.unwrap_or_default();
// Тот же Eid в Hit и Kill этой ракеты.
while let Some(hit) = hits.next().await {
    if hit.weapon.as_ref().is_some_and(|w| w.eid == missile.eid) {
        eprintln!("{} попала", missile.type_name);
        break;
    }
}
// Следующую ракету — убрать в полёте (учебная тревога).
if let Some(next) = shots.next().await
    && let Some(obj) = next.weapon.as_ref().and_then(Obj::weapon)
{
    match ctx.destroy(obj).key("drill-missile").await {
        Ok(()) | Err(Error::Gone(_)) => {} // Gone: уже взорвалась
        Err(e) => return Err(e),
    }
}
```

Это видно и логике 0.1.0: `WeaponSnap.eid` больше не 0. В FakeDcs — `dcs.shoot(юнит, тип)`
(возвращает `id_` оружия), `dcs.weapon_hit(id_, цель)`, `dcs.eject(юнит)`,
`dcs.crew_landed(id_)`, `dcs.weapons()`, `dcs.crew()`.

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
теряются. Меню из нескольких пунктов и подменю, в том числе для одной группы, строит
`ctx.menus()` (ревизия API 1, страница [Деревья меню F10](menus.md)). Убрать команду:

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

Для сайта сервера логика публикует свои JSON-документы, а движок отдаёт их по HTTP:
`ctx.publish("front", &front).await?` (ревизия API 3). Документ принадлежит логике, а не сессии,
и тоже переживает перезапуски логики и движка; подробности — на странице
[Web API](web-api.md#документы-логики).

Сайт может и попросить логику что-то сделать: логика объявляет действие
(`ctx.api().action("send_cap")?`, ревизия API 3), сайт вызывает его запросом
`POST /api/v1/actions/<логика>/send_cap`, а ответ логики (`req.reply(&значение)` или
`req.fail(422, "почему")`) становится ответом на запрос; см.
[Web API](web-api.md#действия-логики).

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
| `Timeout` | прошёл срок команды: она не дошла до DCS (`limit()` называет, чего ждала) или ушла в DCS, а ответа не было ([ниже](#timeout-не-дошла-или-не-ответила)) |
| `Lua` | ошибка в Lua-слое; смотрите traceback |
| `Rejected` | движок отклонил команду: лимит (`limit()`, подождать `retry_after()`) или запрет (`is_forbidden()`, повтор не поможет); подробности в сообщении |
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

Отказ по лимиту (движок 0.4.0, ревизия API 3) разбирается без текста: `e.limit()` —
`Option<&Limit>` (вид `LimitKind`, имя бюджета, его числа), `e.retry_after()` — когда можно
отправить ту же команду снова, `e.is_limit()`, `e.is_forbidden()` (запрет: `Rejected` с
`capability()` и без лимита). Код ошибки и `capability()` остались как в 0.3.0; объявлять
ревизию в логике не нужно: движок заполняет поле для любой логики. Виды лимитов,
их бюджеты и короткий цикл повтора — в [вызовах DCS](dcs.md#отказ-по-лимиту). SDK сам команды
не повторяет. В `Failure` появилось поле `limit`: код, который собирает `Failure` литералом
без `..Default::default()`, его надо добавить.

### Timeout: не дошла или не ответила

`Error::Timeout` бывает двух видов, и с 0.4.0 их различает `e.limit()`:

* **Не дошла до DCS.** Срок прошёл, пока команда ждала в движке или в мосте; сообщение
  «the deadline passed before the command could start (...)» называет в скобках, чего она
  ждала, а `e.limit()` — то же данными: `TaskingDelay` (задержка после спавна), `HeavySlot`
  (идёт другой тяжёлый вызов), `LinkCredits`, `LinkDown`, `After` (команда из `.after(..)` не
  кончилась), `BridgeDeadline` (мост не успел её начать), `Queue`. Команда не выполнялась:
  повтор безопасен.
* **Команда из `.after(..)` упала.** Тогда эта команда тоже не выполнялась, но кончается не
  обязательно `Error::Timeout`: у неё код и `capability()` упавшей (у отклонённой — `Rejected`,
  у недопустимой — `Invalid`), `e.limit()` — `After` (`wait.after`), `retry_after()` — `None`,
  `is_forbidden()` — `false`. Та же команда, отправленная снова, сразу упадёт так же: движок
  помнит, что `.after(..)`-команда упала. Отправьте заново ту, после которой она идёт, и эту — с
  `.after(..)` новой.
* **Ушла в DCS, ответа нет.** Сообщение «`op` was sent to DCS but no result arrived before the
  deadline», `e.limit()` — `None`. Команда может ещё выполниться: повторяйте её только с тем
  же `.key(..)`, тогда повтор дождётся её позднего ответа, а не выполнит её второй раз.

Ошибка, которую функция миссии вернула через `?`, завершает функцию миссии (ошибка фоновой
задачи — эту задачу): рантайм записывает её, а снова функцию запустит только следующая сессия
или перезапуск логики. Фоновые задачи и меню продолжают работать, но всё, что функция делала бы
после этой команды, не случится. Поэтому одна необязательная команда — опция ИИ, сообщение,
дым — не должна завершать функцию миссии: дайте ей ключ, на `Timeout` повторите один раз,
остальное запишите и идите дальше:

<!-- snippet: sdk-errors-timeout -->
```rust
let sam = ctx.group(groups::SAM)?;
let go_red = || sam.set_option(Alarm::Red).key("sam-alarm-red");
let mut outcome = go_red().await;
if matches!(outcome, Err(sortie::Error::Timeout(_))) {
    // С тем же ключом: команда, которая всё же дошла до DCS, второй раз не выполнится.
    outcome = go_red().await;
}
if let Err(e) = outcome {
    // Необязательная команда: записать и продолжать, не завершая функцию миссии.
    eprintln!("{}", e.report());
}
```

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
| маршрут наземной группы (ревизия 3) | `Task::ground_route([..])`, `RoutePoint::on_road(..)`, `.off_road(..)`, `.on_railroads(..)`, `RoutePoint::new(.., GroundFormation::..)`, `.speed(..)` ([Задачи наземных групп](#задачи-наземных-и-морских-групп)); остальное наземной и морской группе — `Task::raw` |
| спавн | `ctx.spawn_group(templates::X)`, `.named`, `.from_airbase`, `.at` (один курс всем машинам), `.at_formation` (строй поворачивается целиком, ревизия 3) |
| статики, уничтожение, мусор (ревизия 1) | `ctx.spawn_static(тип, страна)`, `ctx.destroy(obj)`, `ctx.remove_junk(объёмы)` ([Объекты](objects.md)) |
| поиск объектов в объёме (ревизия 2) | `ctx.search_objects(объём)`, `.categories(..)`, `.limit(n)` ([Объекты](objects.md#поиск-объектов)) |
| пакет спавнов (ревизия 1) | `ctx.spawn_batch()`, `.group(..)`, `.static_object(..)`, `.add_group(..)`, `.add_static(..)`, `.stop_on_error()` ([Объекты](objects.md#пакет-спавнов)) |
| владелец, автозахват и радио аэродромов (ревизия 3) | `ctx.airbase_control(аэродромы)`, `.owner(..)`, `.auto_capture(..)`, `.radio_silent(..)` ([Аэродромы](airbases.md)) |
| смена владельца аэродрома (событие ревизии 2) | `ctx.events().of::<v1::AirbaseOwnerChanged>()`, `.airbase(..)`, `.cause(..)`, `.coalition(..)` ([Аэродромы](airbases.md#смена-владельца-событие)) |
| стоянки, полосы и состояние аэродромов (ревизия 3) | `ctx.airbase(аэродром).parking()` (`.available_only()`), `.runways()`, `.state()`; много баз — `ctx.airbases_parking(..)`, `ctx.airbases_runways(..)`, `ctx.airbases_state(..)` ([Аэродромы](airbases.md#стоянки-полосы-и-состояние-запросы)) |
| запас складов (ревизия 3) | `ctx.warehouse(склад).stock()` (`.items([..])`, `.liquids()`); много складов — `ctx.warehouses_stock(..)` ([Склады](warehouses.md)) |
| изменение запаса складов (ревизия 3) | `ctx.warehouse_op()`, `.set(..)`, `.add(..)`, `.remove(..)`, `.transfer(..)`, `.key(..)`: до 16 замен в одном кадре, всё или ничего; в dev ключ нужен каждой команде, и `set` тоже ([Склады](warehouses.md#изменить-запас)) |
| имена предметов и их wsType (ревизия 3) | `ctx.item_names()` (`.items([..])`) → `ItemNames`: `.ws_type_of(имя)`, `.names_of(ws)`, `.knows(..)`; карта DCS читается раз за процесс DCS; ключа не принимает ([Склады](warehouses.md#имена-предметов)) |
| сообщения, дым, флаги | `ctx.message(..)`, `ctx.smoke(..)`, `ctx.flag(..).set(..)` / `.get()` |
| меню | `ctx.menu(coalition)`, `.per_group()`, `.command(menus::X)`, `presses()`, `remove()` |
| время | `ctx.sleep`, `ctx.timer`, `ctx.model_time`, `ctx.paused` |
| мир | `ctx.world()`, `ctx.wait_until(..)` |
| события | `ctx.events()`, `.of::<v1::Dead>()`, `.in_group`, `.coalition`, `.filter` |
| задачи | `ctx.spawn(..)` |
| состояние | `ctx.state`, `ctx.state_in`, `ctx.once`, `ctx.once_per_turn` |
| документы для сайта (ревизия 3) | `ctx.publish(имя, &значение)`, `ctx.unpublish(имя)` ([Web API](web-api.md#документы-логики)) |
| действия для сайта (ревизия 3) | `ctx.api().action(имя)`, `next()`, `req.json()`, `req.reply(..)`, `req.fail(..)` ([Web API](web-api.md#действия-логики)) |
| итоги | `ctx.report().set(..)`, `ctx.end_session(..)`, `ctx.until_session_end()` |
| ошибки | `sortie::Error`, `code()`, `help()`, `report()`, `capability()` |
| возможности (ревизия 1) | `ctx.capabilities()`, `#[sortie::mission(requires = [caps::X])]`, `sortie::caps` ([Возможности движка](capabilities.md)) |
