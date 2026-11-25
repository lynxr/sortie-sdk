# Вызовы DCS

`ctx.dcs()` вызывает функции скриптового API DCS, у которых в SDK ещё нет своей типизированной
операции: высота и тип поверхности, ветер, груз в кабине, лазерные пятна, звуки и многое
другое (владелец аэродрома, автозахват и радиомолчание — типизированная операция
[`ctx.airbase_control`](airbases.md), чтение запаса складов — запрос
[`ctx.warehouse(..).stock()`](warehouses.md), изменение запаса — команда
[`ctx.warehouse_op()`](warehouses.md#изменить-запас)). Каждая функция описана в каталоге DCS API — файле `catalog/dcs_api.toml` в
репозитории SDK. Для каждой записи каталога генерируется обычный метод SDK с типами аргументов и
результата; полный список — в [справочнике каталога](dcs-catalog.md).

Движок проверяет каждый вызов по каталогу до того, как что-то попадёт в DCS: путь функции,
окружение, класс объекта, типы и диапазоны аргументов. Ошибка в аргументе — `Error::Invalid` с
сигнатурой записи в сообщении, а не ошибка Lua в игре.

## Ревизия API

Вызовы DCS — API ревизии 1. Логика объявляет её в атрибуте миссии; без него вызов вернёт
`Error::Revision`, и в движок ничего не уйдёт (путь по дорогам, `land().path_on_roads(..)`, его
сводка `land().road_path(..)` и чтения экрана слотов в `server()` — ревизии 3,
[ниже](#путь-по-дорогам)):

```rust
#[sortie::mission(api_rev = 1)] // вызовы DCS — API ревизии 1
pub async fn with_calls(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let height = ctx.dcs().land().height(zones::TARGET.center()).await?;
    ctx.report().set("target_height_m", height);
    ctx.end_session(Outcome::Draw).await
}
```

## Чтение

Функции собраны в семейства: `land()`, `atmosphere()`, `timer()`, `coord()`, `coalition()`,
`world()`, `env()`, `net()` и `server()` (окружение хуков) — функции без объекта; `unit(..)`,
`group(..)`, `static_object(..)`, `airbase(..)`, `warehouse(..)`, `controller(..)`, `spot(..)`,
`object(..)` — методы объекта DCS. Объект задаётся тем, что у логики уже есть: символом
сценария, Eid, `Group`, сущностью из мира или селектором из `sortie::sel`.

Вызовы окружения хуков (`net()`, `server()`, пути `net.*` и `DCS.*`) в этом выпуске (волна 1)
тоже требуют идущей сессии миссии: ключи и дескрипторы принадлежат сессии. Без неё (DCS в
меню, между миссиями) вызов отвечает `sortie::unavailable` («no mission session is running»).
Вызовы сервера без миссии появятся вместе с логикой уровня сервера (волна 6).

```rust
let dcs = ctx.dcs();
let p = zones::TARGET.center(); // MapPoint
let surface: String = dcs.land().surface_type(p).await?; // "LAND", "ROAD", "WATER", ...
let height: f64 = dcs.land().height(p).await?;
let (kelvin, pascal) = dcs
    .atmosphere()
    .temperature_and_pressure(p.alt(m(3_000)))
    .await?;
let owner: Coalition = dcs.airbase(airbases::KUTAISI).coalition().await?;
```

Числа перечислений DCS приходят строкой-ключом (`"ROAD"`, а не 4), коалиция — значением
`Coalition`, точки — `Vec3` или `MapPoint`. Точка на карте (`vec2` в каталоге) — это `MapPoint`
с осями севера и востока; `vec3` — `Vec3` в осях DCS (`x` — север, `y` — высота, `z` — восток).
Какой из двух видов берёт функция, сказано в её записи каталога: `land().height(p)` —
`MapPoint`, а `land().profile(a, b)` — два `Vec3`, у которых DCS читает `x` и `z`.

Число, которое DCS вернул строкой, приходит числом там, где каталог объявляет `number`: DCS
2.9.31 отдаёт `Unit:getID()` строкой `"1"`, поэтому `static_object(..).id()`,
`airbase(..).id()` и `group(..).id()` возвращают `f64` при любом ответе DCS.

### Путь по дорогам

`land().path_on_roads(..)` спрашивает у DCS путь по дорогам между двумя точками карты
(`land.findPathOnRoads`). Это API ревизии 3: логика объявляет
`#[sortie::mission(api_rev = 3)]` (иначе `Error::Revision`), и нужен движок этой ревизии
(иначе `Error::EngineTooOld`).

```rust
#[sortie::mission(api_rev = 3)] // land().path_on_roads — API ревизии 3
pub async fn road_path(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::ARMOR)?;
    let start = ctx.world().group(&armor).center();
    let goal = zones::TARGET.center();
    if let Some(a) = start {
        // Тип дорог и четыре числа: x — север, y — восток (оси MapPoint).
        let road: Option<Vec<MapPoint>> = ctx
            .dcs()
            .land()
            .path_on_roads("roads", a.north, a.east, goal.north, goal.east)
            .await?;
        match road {
            // Точки от начала к концу: длина пути по дорогам.
            Some(points) if points.len() > 1 => {
                let metres: f64 = points.windows(2).map(|w| w[0].distance(w[1])).sum();
                ctx.report().set("armor_road_km", (metres / 1000.0).round());
            }
            _ => ctx.report().set("armor_road_km", "нет пути"),
        }
    }
    ctx.end_session(Outcome::Draw).await
}
```

* Аргументы — как у функции DCS: тип дорог и четыре числа, `x` — север и `y` — восток начала
  и конца в метрах карты (оси `MapPoint`).
* Тип дорог — `"roads"`. Для железных дорог вики Hoggit документирует `"railroads"` и пишет,
  что DCS на деле читает `"rails"`; какое слово работает, ещё не проверено вживую.
* Ответ — `Option<Vec<MapPoint>>`: точки пути от начала к концу. `None` — DCS ответил nil;
  редактор миссий DCS понимает nil как «путь не найден». Что DCS вернёт для точки, куда дороги
  нет (nil, пустой список или путь до ближайшей дороги), как часто идут точки и сколько
  считается длинный путь, вживую ещё не замерено: проверка `ENV-LC7` живой пробы.
* Класс `heavy`: DCS ищет путь внутри кадра, и на большой карте длинный путь может остановить
  сервер на несколько кадров. Такие вызовы идут по одному в полосе BULK, не чаще 2 в секунду на
  писателя, не входят в пакет и ключа не берут; их время в DCS оплачивает свой бюджет писателя
  `dcs_time.heavy` ([лимиты](#время-dcs)): после долгого пути ждёт только следующий тяжёлый
  вызов, чтения идут сразу.
* Концы пути — не дальше 300 км друг от друга по карте: дальше — `Error::Invalid`, и в DCS
  ничего не уходит (проверка `span_300km`; у сводки — та же).
* Ехать колонну заставляет задача DCS `Mission` ([задачи наземных и морских
  групп](sdk.md#задачи-наземных-и-морских-групп)): по точкам с `action = "On Road"` DCS сам
  ведёт её по дорогам. Путь нужен, когда логике важны сами точки: длина пути, остановка в
  пути, есть ли дорога вообще.
* Длинный путь — это тысячи точек в ответе. Если нужны длина и концы пути, а не все точки,
  возьмите [сводку](#сводка-пути-по-дорогам).

### Сводка пути по дорогам

`land().road_path(тип, начало, конец, max_points, keep)` спрашивает у DCS тот же путь
(`land.findPathOnRoads`), но в логику он целиком не идёт: Lua-слой Sortie считает сводку внутри
DCS в том же кадре и отдаёт `Option<RoadPath>` (`sortie::dcs::RoadPath`). Это функция самого
слоя, запись каталога `sortie.findPathOnRoads`: API ревизии 3, класс `heavy`, как у пути целиком.

```rust
#[sortie::mission(api_rev = 3)] // land().road_path — API ревизии 3
pub async fn road_summary(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::ARMOR)?; // Group<Ground>
    let goal = zones::TARGET.center();
    let Some(from) = ctx.world().group(&armor).center() else {
        return ctx.end_session(Outcome::Draw).await;
    };
    // Сводку считает слой Sortie внутри DCS: длина и концы пути приходят без всех его точек.
    // Some(8.0) — вернуть не больше 8 точек: оба конца и места самых крутых поворотов.
    let summary = ctx
        .dcs()
        .land()
        .road_path("roads", from, goal, Some(8.0), None)
        .await?;
    match summary {
        // Дорога подходит к цели ближе 300 м: колонна едет по дорогам через повороты.
        Some(p) if p.points.len() > 1 && p.end_gap_m.is_some_and(|gap| gap < 300.0) => {
            ctx.report()
                .set("armor_road_km", (p.length_m / 1000.0).round());
            let via_turns = p.points[1..].iter().map(|&q| RoutePoint::on_road(q));
            armor.set_task(Task::ground_route(via_turns)).await?;
        }
        _ => ctx.report().set("armor_road_km", "нет пути"),
    }
    ctx.end_session(Outcome::Draw).await
}
```

* `count` — сколько точек в пути DCS, `length_m` — его длина по точкам в метрах, `first` и
  `last` — первая и последняя точки, `start_gap_m` и `end_gap_m` — сколько метров от первой
  точки до заданного начала и от последней до заданного конца (дорога может не доходить до
  точки). Всё это — о пути целиком.
* `points` — не больше `max_points` точек пути (от 2 до 10 000; меньше 2 или больше 10 000 —
  `Error::Invalid`, в DCS ничего не уйдёт), оба конца всегда на месте, порядок — от начала к
  концу. `keep`: `None` или `Some("turns")` — точки, где путь поворачивает сильнее всего (если
  остальные точки лежат на прямых участках, вернётся меньше); `Some("even")` — точки через
  равные расстояния вдоль пути. Другое слово — `Error::Invalid`. Без `max_points` (`None`)
  точек нет: только сводка.
* `thinned` — в `points` меньше точек, чем в пути (без `max_points` — всегда так, кроме пустого
  пути: при `count = 0` `thinned` ложно).
* `None` — DCS не нашёл пути (ответил nil). Пустой список DCS — `count = 0`, без концов.
* Аргументы — тип дорог и две точки карты `MapPoint` (у пути целиком — четыре числа).
* Маршрут колонны из сводки: точки `points` после первой как `RoutePoint::on_road(..)` —
  повороты пути, а дорогу между ними DCS найдёт сам; не больше 256 точек на маршрут.
* Нужен Lua-слой 0.4.0 (`sortie install`): старый слой не знает функций `sortie.*` и отвечает
  `MISSING` (`Error::Unavailable`). Совпадает ли сводка с путём целиком на живом DCS, проверяет
  шаг `ENV-LC7` живой пробы.

### Слоты FARP глазами сервера

Экран выбора слота в сетевой игре DCS строит список динамических слотов FARP и кораблей из трёх
функций окружения хуков; они есть в каталоге (ревизия API 3, семейство `server()`):
`farps_and_carriers()` (`DCS.getFarpsAndCarriersMissionData`) — какие FARP и корабли знает
экран, по id юнита; `dynamic_spawn_settings(id, is_airdrome)` (`DCS.getDynamicSpawnSettings`) —
есть ли у базы динамические слоты (`dynamicSpawnAvailable`) и горячий старт (`allowHotSpawn`);
`aircraft_in_warehouse(тип, id, is_farp)` (`DCS.getAircraftAmountInAirportWarehouse`) — сколько
машин типа даёт склад базы.

```rust
#[sortie::mission(api_rev = 3)] // чтения экрана слотов — API ревизии 3
pub async fn farp_slots(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    let server = ctx.dcs().server();
    // FARP и корабли, которые знает экран слотов на сервере: id юнита -> {name, coalition, ..}.
    let known: Value = server.farps_and_carriers().await?;
    let farps = known
        .get("farps")
        .and_then(Value::as_table)
        .unwrap_or_default();
    for (key, entry) in farps {
        let (Some(unit_id), Some(name)) = (key.as_f64(), entry.get("name").and_then(Value::as_str))
        else {
            continue;
        };
        // false: это не аэродром, а FARP или корабль, и id — его юнита.
        let settings: Option<Value> = server.dynamic_spawn_settings(unit_id, false).await?;
        let dynamic = settings
            .as_ref()
            .and_then(|t| t.get("dynamicSpawnAvailable"))
            .and_then(Value::as_bool);
        let hueys: f64 = server.aircraft_in_warehouse("UH-1H", unit_id, true).await?;
        ctx.report().set(
            name,
            format!("динамические слоты: {dynamic:?}, UH-1H на складе: {hueys}"),
        );
    }
    ctx.end_session(Outcome::Draw).await
}
```

* В документации ED этих функций нет; их аргументы и поля ответа взяты из скриптов экрана
  слотов DCS 2.9.31. Что они отвечают вживую, в том числе для FARP, созданного во время миссии,
  ещё не замерено (живая проверка `AWC-T30`).
* Это взгляд DCS **сервера**. Что видит подключённый игрок (особенно на выделенном сервере),
  эти чтения не доказывают.
* `farps_and_carriers()` — класс `heavy` (таблица всех FARP и кораблей миссии): по одному, не
  чаще 2 в секунду, не в пакете. Две другие — обычные чтения.
* Включить динамические слоты или горячий старт у FARP, созданного логикой, типизированной
  настройкой пока нельзя: DCS не документирует для этого ни функции, ни ключа. Что она делает с
  ключами `dynamicSpawn` и `allowHotStart` в таблице статика FARP (`SpawnStatic::raw`), меряет
  проба; до замера это не обещание. Запас такого FARP ставит
  [`ctx.warehouse_op()`](warehouses.md#изменить-запас).

## Объекты

Объекты DCS в результате — значения `DcsObject`: класс, Eid зеркала, имя DCS, тип. Объект можно
передать в следующий вызов. Методы базовых классов DCS (`Object`, `CoalitionObject`) есть в
семействе каждого класса-наследника. Метод выполняется через таблицу класса самого объекта, то
есть так же, как `unit:getCoalition()` в скрипте миссии. Путь записи каталога
(`CoalitionObject.getCoalition`) — её имя: `CoalitionObject` есть только в документации DCS,
таблицы с таким именем в игре нет.

```rust
let dcs = ctx.dcs();
// Объекты возвращаются как DcsObject: Eid зеркала, имя, тип.
let units: Vec<DcsObject> = dcs.group(groups::SAM).units().await?;
let mut damaged = Vec::new();
for u in &units {
    let life: f64 = dcs.unit(u).life().await?;
    let life0: f64 = dcs.unit(u).life0().await?;
    if life < life0 {
        damaged.push(u.name.clone());
    }
}
// Унаследованные методы есть у каждого класса: Object.getPoint у юнита.
let point: Vec3 = dcs.unit(&units[0]).point().await?;
// Группа по имени DCS, если у неё нет символа сценария.
let size: f64 = dcs.group(GroupSel::name("Armor")).size().await?;
```

Если зеркало не знало объект, движок заводит для него Eid по обычным правилам наблюдения.
Длинные списки (`coalition().groups(..)`, `world().airbases()`) возвращают лёгкие ссылки: у
объектов сверх первых 64 есть имя, но может не быть Eid — следующий проход сэмплера их увидит.
Контроллер и склад, полученные от объекта (`airbase(..).warehouse()`), возвращаются селектором,
по которому их можно вызвать снова.

## Изменения мира

Функции класса `mutate` меняют мир и по умолчанию выключены. Владелец движка включает нужные в
`<Saved Games>/Sortie/engine.toml`; `sortied --dev` включает все:

```toml
[calls]
enable = ["trigger.action.setUnitInternalCargo", "trigger.action.outSound"]
```

Пока функция выключена, вызов возвращает `Error::Rejected` с именем capability
`call:mission:<путь>`. Изменяющий вызов повторяет своё действие при каждом выполнении, поэтому
ставьте ему `.key(..)`: повтор с тем же ключом вернёт сохранённый результат и не дойдёт до DCS.

```rust
let dcs = ctx.dcs();
// Груз в кабине вертолёта: изменение, поэтому с ключом.
dcs.effects()
    .set_unit_internal_cargo(UnitSel::name("Rescue 1-1"), 800.0)
    .key("rescue-cargo-1")
    .await?;
// Склад аэродрома: по селектору аэродрома. Чтения ключа не берут.
let stock = dcs.warehouse(airbases::KUTAISI);
let left: f64 = stock.item_count("weapons.missiles.AIM_120C").await?;
// Или по wsType, четырём числам из Scripts/Database/wsTypes.lua игры.
let aim9: f64 = stock.item_count([4, 4, 7, 22]).await?;
let inventory: Value = stock.inventory(None).await?; // большая таблица: класс heavy
```

Прочитать склад целиком (безлимитный класс — безлимитным, а не нулём) или нужные предметы
многих складов одним запросом удобнее типизированным запросом `ctx.warehouse(..).stock()` и
`ctx.warehouses_stock(..)`, а менять запас — командой `ctx.warehouse_op()`: до 16 замен и
переводов между складами в одном кадре, всё или ничего, безлимитный склад не «меняется»
молча — страница [Склады](warehouses.md). Сырые `set_item`, `add_item`, `remove_item` и
жидкостные устарели (`#[deprecated]`, в каталоге `superseded_by = "warehouse_op"`).

## Эффекты и звуки

Предмет склада называется так, как его принимает DCS: именем (`"weapons.missiles.AIM_120C"`,
тип самолёта) или wsType — четырьмя целыми числами из `Scripts/Database/wsTypes.lua` игры
(`[4, 4, 7, 22]` или `(4, 4, 7, 22)`, тип `sortie::dcs::WarehouseItem`, он же
`sortie::cmd::WarehouseItem` у [запроса склада](warehouses.md); в Python — кортеж). Так
принимают `item_count` и устаревшие `add_item`, `remove_item` и `set_item`; фильтр
`inventory(..)` — только имя. Другая таблица — `Error::Invalid`, в DCS вызов не уходит. Последнее число wsType, по
сообщениям, у некоторых предметов меняется между сборками DCS: какой wsType у имени в вашей DCS,
говорит запрос [`ctx.item_names()`](warehouses.md#имена-предметов) — DCS отвечает сама, во время
игры. FakeDcs хранит запас по wsType отдельно от запаса по имени, пока тест не дал ему карту
имён (`dcs.set_item_names(..)`).

`Warehouse.getResourceMap` в каталоге нет, и вызывать его не нужно: карту имён и wsType читает
типизированный запрос `ctx.item_names()` (ревизия API 3), один раз за процесс DCS. Чтобы
проверить предметы кампании до наполнения склада, спросите `.items([..])` или `.knows(имя)` и
ставьте на склад только то, что DCS знает: опечатка в имени окажется в `unknown`, а не в
`ctx.warehouse_op()`.

Взрывы, ракеты, звуки и радиопередачи — в семействах `effects()` и `sounds()`. Числовые
аргументы с ценой или радиусом ограничены каталогом (мощность взрыва — не больше 5000);
значение вне диапазона — `Error::Invalid`. Звуковой файл должен лежать внутри `.miz`: путь
относительный, без `..`, буквы диска и ведущего `/`.

```rust
let dcs = ctx.dcs();
let target = zones::TARGET.center().alt(m(0));
dcs.effects()
    .explosion(target, 500.0) // мощность не больше 5000
    .key("depot-blast")
    .await?;
dcs.effects()
    .signal_flare(target, "Red", 0.0) // ключ таблицы trigger.flareColor
    .key("flare-1")
    .await?;
// Звук — файл внутри .miz: относительный путь, без "..", диска и ведущего "/".
dcs.sounds()
    .out_sound_for_coalition(Coalition::Blue, "l10n/DEFAULT/alarm.ogg")
    .key("alarm-1")
    .await?;
```

## Лазерные пятна

`spots().create_laser(..)` и `create_infra_red(..)` возвращают `Handle`: мост держит пятно, пока
его не уберёт `spot(h).destroy()` или пока не закончится сессия (тогда мост убирает его сам). В
сессии может быть не больше 256 таких объектов.

```rust
let dcs = ctx.dcs();
let target = zones::TARGET.center().alt(m(0));
// Лазерное пятно живёт под дескриптором, пока его не уберут или не кончится сессия.
let spot: Handle = dcs
    .spots()
    .create_laser(&jtac, None, target, Some(1688.0))
    .key("lase-1")
    .await?;
dcs.spot(spot).set_point(target).key("lase-1-move").await?;
dcs.spot(spot).destroy().key("lase-1-off").await?;
```

## Ключи, полосы, лимиты

| Класс | Полоса | По умолчанию | Ключ | Лимит на писателя |
| --- | --- | --- | --- | --- |
| `read` | QUERY | включено (кроме функций «проба») | нельзя: `Error::Invalid` | 200 вызовов в секунду и бюджет `dcs_time.read`: 20 мс времени DCS в секунду |
| `heavy` | BULK, по одному | включено (запись владельца не с get/is/has/find — выключена, как `mutate`) | нельзя | 2 в секунду и свой бюджет `dcs_time.heavy`: 20 мс времени DCS в секунду, не больше 200 мс сразу |
| `mutate` | REALTIME (хуки — CONTROL) | выключено | повтор с ключом не выполняется снова | 50 в секунду |
| `admin` | CONTROL | выключено | как `mutate` | 1 в секунду |

Повтор с тем же ключом возвращает сохранённый результат целиком, вместе с объектами и
дескрипторами: `create_laser(..).key("lase-1")` после перезапуска логики в той же сессии вернёт
тот же `Handle`, и пятно можно сдвинуть или убрать. Состояние объектов в ответе на повтор
читается из мира заново. Дескриптор в сохранённом результате не продлевает жизнь объекта: если
пятно уже убрано, вызов по нему ответит `Error::Gone`. Под ключом не хранятся только значения
результата больше 64 КиБ: повтор такого вызова возвращает `Error::ResultNotKept` — вызов уже
выполнен под этим ключом и снова не выполнялся; нужное прочитайте отдельным вызовом. Функции
«проба» (недокументированные чтения) выключены, пока их проверка не прошла на запущенной сборке
DCS.

### Время DCS

Вызов выполняется в кадре DCS, и `cost_us` ответа — сколько времени он занял там (мост
измеряет его вместе с упаковкой ответа). Это время оплачивает бюджет писателя (имя токена
логики, без токена — имя логики), когда приходит ответ; пока бюджет исчерпан, движок отвечает
на новую команду `Error::Rejected` с текстом `rate limit: ...` и в DCS ничего не отправляет;
с 0.4.0 `e.limit()` называет бюджет, а `e.retry_after()` — когда он снова пропустит команду
([отказ по лимиту](#отказ-по-лимиту)). Бюджетов два:

* `dcs_time.read` — чтения и **любой пакет**, какие бы вызовы в нём ни были (пакет с вызовом
  `mutate` тоже: его вызовы `mutate` при этом считаются и в лимите 50 в секунду): 20 мс в
  секунду, не больше 20 мс сразу. **Одиночное чтение** видит долг не глубже 20 мс: чтение или
  пакет, которые заняли больше, закрывают одиночные чтения писателя (`timer.getTime`,
  `Controller.hasTask`) не больше чем на 1 с. **Пакет** видит долг целиком: следующий пакет ждёт,
  пока долг не выплачен, так что пакетами бюджет не обойти. Пакет из 256 чтений, занявший 770 мс,
  оставляет 750 мс долга: одиночное чтение пройдёт через секунду, следующий пакет — через
  37,5 с. Одиночное чтение, которое само занимает больше 40 мс, может идти примерно раз в
  секунду; дорогой вызов — дело класса `heavy`. Отдельный вызов `mutate` или `admin` время DCS
  не оплачивает: его лимит — число вызовов.
* `dcs_time.heavy` — тяжёлые вызовы (`heavy`): 20 мс в секунду, не больше 200 мс сразу, долг
  хранится целиком. После тяжёлого вызова длиной `C` мс следующий тяжёлый вызов того же писателя
  ждёт около `(C − 200) / 20` с: после пути по дорогам за 2241 мс — 102 с; текст отказа называет
  бюджет и сколько осталось ждать. Чтения при этом не ждут: `timer.getTime` и
  `Controller.hasTask` проходят сразу после такого вызова. В SDK 0.3.0 тяжёлый вызов оплачивал
  бюджет чтений без нижней границы, и после 2241 мс чтения писателя отклонялись около 111 с.
* **Тяжёлые вызовы писателя идут по одному.** Пока предыдущий тяжёлый вызов не вернулся (ждёт
  тяжёлого слота, выполняется или ждёт позднего ответа после `Error::Timeout`), следующий
  отклоняется сразу: `Error::Rejected` с текстом `rate limit: the previous heavy call of ... has
  not come back yet`, `limit()` — `DcsTime` `dcs_time.heavy`, `retry_after()` — `None`: время
  предыдущего ещё не списано, и сколько ждать, неизвестно. В DCS он не уходит. Отправляйте
  следующий тяжёлый вызов после ответа предыдущего (а не два сразу в `join!`); тогда отказ по
  бюджету, если он будет, назовёт ожидание.

Счётчики вызовов (200, 50, 2, 1 в секунду) долг тоже помнят, но он невелик: пакет из 256
чтений из полной корзины на 200 оставляет 56 вызовов долга, и одиночное чтение ждёт меньше
0,3 с. Этот долг держит пакеты в том же темпе, что и одиночные вызовы.

* Вызов, на который движок уже ответил `Error::Timeout`, но который всё же выполнился в DCS,
  оплачивает своё время, когда приходит его поздний ответ.
* Долг принадлежит писателю, а не соединению: перезапуск логики под тем же токеном или именем
  его не обнуляет. Бюджет писателя, который 10 минут ничего не вызывал, движок забывает только
  полным, так что долг этим не прощается; перезапуск движка начинает всех с полных бюджетов.
* Бюджеты — на писателя: у другой логики (другого токена) свои.
* Тяжёлый вызов останавливает сервер DCS на всё время своей работы: слой Lua выполняет его в
  кадре, и асинхронным его сделать нельзя. Если вызов одной функции трижды за 10 минут
  длится дольше секунды, мост отказывает в ней до перезапуска DCS (`Error::Quarantined`,
  заметка `Slow, not a crash`; [возможности](capabilities.md)). Спрашивайте путь один раз на
  маршрут, а не на каждом тике.
* Проверка `span_300km`: у `land().profile(..)`, `land().path_on_roads(..)` и
  `land().road_path(..)` концы не дальше 300 км друг от друга по карте, иначе
  `Error::Invalid`, и в DCS ничего не уходит. Предел 300 км уточнит живая проверка `ENV-LC7`.

### Отказ по лимиту

С движка 0.4.0 (ревизия API 3) отказ по лимиту разбирается без текста: `e.limit()` говорит,
какой лимит отказал, `e.retry_after()` — когда можно отправить ту же команду снова. Код ошибки,
текст и `capability()` остались как в 0.3.0 (кроме текста отброшенного дескриптора чтения:
`handles.read` ниже): поле дополнительное, движок заполняет его для любой логики, а против
движка 0.3.0 оба метода возвращают `None`. SDK сам команды не повторяет; короткий цикл
повтора:

```rust
let land = ctx.dcs().land();
let spot = zones::TARGET.center();
let mut waits = 0;
let surface = loop {
    match land.surface_type(spot).await {
        Ok(kind) => break kind,
        // Лимит: подождать, сколько назвал движок, и отправить то же чтение снова.
        Err(e) if waits < 3 && e.retry_after().is_some() => {
            waits += 1;
            if let Some(l) = e.limit() {
                ctx.report().set("last_limit", l.budget.as_str());
            }
            ctx.sleep(e.retry_after().unwrap_or_default()).await;
        }
        // Запрет (функция выключена или её нет в каталоге): повтор не поможет.
        Err(e) if e.is_forbidden() => return Ok(()),
        Err(e) => return Err(e),
    }
};
```

| `limit().kind` | `limit().budget` | Ошибка | Что помогает |
| --- | --- | --- | --- |
| `CallRate` | `call_rate.read`, `call_rate.mutate`, `call_rate.heavy`, `call_rate.admin` | `Rejected` | подождать `retry_after()` |
| `DcsTime` | `dcs_time.read`, `dcs_time.heavy` ([время DCS](#время-dcs)) | `Rejected` | подождать `retry_after()`; без него (тяжёлый вызов, пока предыдущий не вернулся) — дождаться ответа предыдущего |
| `BatchSize` | `batch.calls`: 64 вызова, из одних чтений 256 | `Invalid` | разделить пакет |
| `RequestSize` | `request.bytes`: 64 КиБ на вызов или пакет | `Invalid` | разделить запрос |
| `CreatedHandles` | `handles.created`: 256 созданных вызовами объектов в сессии | `Rejected` | убрать ненужный (`destroy`) |
| `ReadHandles` | `handles.read`: 1024 дескриптора прочитанных объектов, лишний старый отброшен | `Gone` | прочитать объект снова |
| `SlowCalls` | `bridge.slow_calls`: три вызова функции дольше 1 с за 10 минут | `Quarantined` | ничего до перезапуска DCS |
| ожидания: `TaskingDelay`, `HeavySlot`, `LinkCredits`, `LinkDown`, `After`, `BridgeDeadline`, ... | `delay.tasking`, `slot.heavy`, `link.credits`, ... | `Timeout` | команда не выполнялась, повтор безопасен ([Timeout](sdk.md#timeout-не-дошла-или-не-ответила)) |
| `After`, когда команда из `.after(..)` упала | `wait.after` | код той команды (`Rejected`, `Invalid`, `Gone`, `Timeout` …) и её `capability()` | команда не выполнялась; та же команда снова упадёт так же сразу — отправьте заново ту, после которой она идёт, и эту с `.after(..)` новой |

* `retry_after()` — время от отказа до момента, когда **та же команда**, отправленная снова без
  изменений, пройдёт все свои бюджеты, если логика за это время не отправит ничего другого и не
  придёт ответ на её более раннюю команду (время DCS списывается, когда приходит ответ). Для
  счётчиков оно считается по пополнению корзины: пакет из 256 чтений из полной корзины на 200
  оставляет −56, и следующий такой же пакет ждёт 1,28 с (ему снова нужны все 200), а одиночное
  чтение — 0,285 с. Для времени DCS — пока бюджет снова не станет больше нуля (бюджет чтений
  для одиночного чтения — не дольше 1 с, для пакета — пока долг не выплачен). Где пополнения нет
  (размеры, предел числа, ожидание, тяжёлый вызов, пока предыдущий не вернулся),
  `retry_after()` — `None`.
* Если отказали сразу несколько бюджетов, `limit()` называет тот, которого ждать дольше, а
  `retry_after()` покрывает их все.
* Числа лимита — целые, в его единицах: вызовы, микросекунды времени DCS, байты, дескрипторы.
  `capacity` — сколько бюджет держит, `per_second` — сколько пополняется за секунду, `needed` —
  сколько нужно команде, `available` — сколько было при отказе (ниже нуля — долг). `writer` —
  чей бюджет: имя токена логики, без токена — имя логики.
* Пакет, отклонённый при проверке лимита, не выполнил ни одного вызова: движок проверяет пакет
  целиком до отправки в DCS.
* Запрет — отдельная причина: функции нет в каталоге, она выключена по умолчанию или владельцем
  движка, операция выключена в `engine.toml`. Это `Error::Rejected` с `capability()` и без
  `limit()`, `e.is_forbidden()` — `true`; повтор не поможет. Отказ слоя Lua по пределу
  созданных объектов тоже несёт `capability()` (как в 0.3.0), но у него есть `limit()`.
* Часы логики (`ctx.sleep`) идут по тикам, поэтому ожидание может кончиться чуть раньше срока:
  тогда движок откажет снова и назовёт остаток. Цикл выше это переживает.

Большая таблица приходит целиком или не приходит вовсе: вместо неё стоит `Value::Cut(n)` с
числом её ключей, а у результата — `truncated`. Так бывает, когда таблица глубже `max_depth`
(по умолчанию 6, не больше 12), когда вызов уже скопировал 50 000 записей таблиц и когда ответ
всей команды — все вызовы пакета вместе — был бы больше 150 000 значений (объект считается
вместе со своим описанием). Обёртка, которая ждёт список, на обрезанной таблице возвращает
`Error::Invalid`. Большой запрос делите на несколько команд: у каждой свой запас.

## Ошибки

```rust
match ctx
    .dcs()
    .sounds()
    .out_sound("l10n/DEFAULT/alarm.ogg")
    .key("alarm-all")
    .await
{
    Ok(()) => {}
    // Владелец движка не включил функцию: capability call:mission:trigger.action.outSound.
    Err(Error::Rejected(_)) => ctx.report().set("alarm", "sounds are off"),
    Err(e) => return Err(e),
}
```

| Ошибка | Когда |
| --- | --- |
| `Error::Rejected` | запрет: функции нет в каталоге, она выключена, её нельзя вызывать никогда (Lua-код, файлы, обратные вызовы) — `is_forbidden()`; или лимит: число вызовов, [время DCS](#время-dcs), созданные объекты — `limit()` и `retry_after()` ([отказ по лимиту](#отказ-по-лимиту)) |
| `Error::Invalid` | аргументы не подходят к записи каталога; ключ на чтении; объект не того класса; концы дальше 300 км (`span_300km`); пакет или запрос больше предела (`limit()`: `BatchSize`, `RequestSize`) |
| `Error::Gone` | объекта уже нет в DCS (или Eid устарел); дескриптор, отброшенный как самый старый из 1024 (`limit()`: `ReadHandles`) |
| `Error::Quarantined` | функция на карантине после падения DCS или сбоев моста; после трёх вызовов дольше 1 с — до перезапуска DCS (`limit()`: `SlowCalls`) |
| `Error::Timeout` | срок прошёл: с `limit()` вызов не выполнялся (ждал тяжёлый слот, кредиты связи, мост), без него ушёл в DCS и ответа не было |
| `Error::Unavailable` | функции нет в этой сборке DCS; для `sortie.*` — Lua-слой старше 0.4.0 |
| `Error::ResultNotKept` | повтор вызова с ключом, результат которого был больше 64 КиБ: вызов не выполнялся снова, а его значения движок не хранит |
| `Error::Revision` | логика не объявила `api_rev = 1` (для `path_on_roads` и `road_path` — `api_rev = 3`) |
| `Error::EngineTooOld` | движок старше ревизии вызова: `path_on_roads` и `road_path` требуют движка ревизии 3 |

## Чего зеркало не узнаёт

Изменение, сделанное вызовом, зеркало мира видит не сразу, а когда само снова прочитает DCS:
владельца аэродрома после `set_coalition` — только со следующим снимком мира или при запросе
состояния базы (`ctx.airbase(..).state()`; регулярного обхода аэродромов нет), флаг — при
следующем чтении флагов. Где у SDK есть типизированная операция, обёртка помечена
`#[deprecated]` и называет её: например, дым — это `ctx.smoke(..)`, владелец аэродрома,
автозахват и радиомолчание (`set_coalition`, `auto_capture`, `set_radio_silent_mode`) —
[`ctx.airbase_control(..)`](airbases.md), после которой мир знает нового владельца сразу.

## Вызов по имени

Функцию каталога без обёртки (например, добавленную владельцем движка в `engine.toml`) можно
вызвать по строке пути. Это функция крейта за cargo-фичей `dcs-raw`:
`sortie = { ..., features = ["dcs-raw"] }`. Движок проверяет такой вызов так же, как вызов
обёртки. Ревизию записи проверяет только обёртка: вызов по имени идёт с ревизией 1, и запись
ревизии 3 (`land.findPathOnRoads`) движок старше ревизии 3 отклонит с `Error::Rejected`, а не
`Error::EngineTooOld`.

```rust
use sortie::dcs::{CallResult, arg};
let dcs = ctx.dcs();
let r = dcs
    .call("land.getHeight")
    .arg(arg::vec2(zones::TARGET.center()))
    .await?;
let height = r.first().as_f64();
// Пакет: вызовы выполняются в одном кадре; result_of берёт объект предыдущего вызова.
let out = dcs
    .batch()
    .push(
        dcs.call("Airbase.getWarehouse")
            .on(arg::airbase(airbases::KUTAISI)),
    )
    .push(
        dcs.call("Warehouse.getItemCount")
            .on_result_of(0)
            .arg(arg::string("weapons.missiles.AIM_120C")),
    )
    .await?;
if let Some(CallResult::Ok(count)) = out.results.get(1) {
    let _ = count.first().as_f64();
}
```

`out.cost_us` — сколько времени DCS занял весь пакет в слое Lua, в микросекундах: пакет идёт в
одном кадре, это его доля в кадре. Пакет оплачивает в лимите писателя каждый свой вызов: 256
чтений из полной корзины (200 в секунду) проходят, но следующий пакет придётся подождать. Его
время в DCS оплачивает бюджет чтений `dcs_time.read`, даже если в пакете есть вызов `mutate`
([время DCS](#время-dcs)).

Сколько стоила каждая команда, а не только пакет, говорит `ctx.take_command_timings()`: по
одной записи `CommandTiming` на каждую команду с ответом (вызов DCS, пакет, типизированная
операция; ошибки тоже) — операция, цель (путь Lua у вызова), `cost_us` (время внутри DCS),
`round_trip` (от отправки до ответа: очередь и лимиты движка, ожидание ближайшего pump миссии
раз в 0,02 с, связь в обе стороны) и модельное время ответа. Записи забираются один раз; SDK
держит не больше 4096, лишние старые отбрасывает и считает в `dropped`.

### Запись владельца с перечислением DCS

Владелец движка добавляет функцию, которой нет в каталоге, записью `[[calls.call]]` в
`engine.toml` ([правила слияния](operations.md#enginetoml)). Аргумент-перечисление
пишется `enum:<таблица>`, где таблица — полный путь опубликованной таблицы DCS: от 2 до 6 имён
Lua (до 96 байт) от одного из корней перечислений `AI`, `Airbase`, `Controller`, `Group`,
`Object`, `Spot`, `StaticObject`, `Unit`, `Weapon`, `coalition`, `country`, `land`, `radio`,
`trigger`, `world`. Таблицы `env`, `net`, `DCS`, `timer` и другие таблицы данных
перечислениями не бывают: с такой записью движок не запускается и называет правило (так же
таблица из 7 имён). Пример — опция ИИ, которой нет у типизированного `set_option`:

```toml
[[calls.call]]
path = "Controller.setOption"
env = "mission"
receiver = "Controller"
class = "mutate"
args = [
    "enum:AI.Option.Ground.id option",
    "enum:AI.Option.Ground.val.ALARM_STATE value",
]
live_check = "PM-1"
enabled = true
```

```rust
use sortie::dcs::{arg, recv};
use sortie::sel::ControllerSel;
// Запись владельца `Controller.setOption` из engine.toml выше; вызов — по имени.
let sam = || recv::controller(ControllerSel::group(groups::SAM));
let alarm = arg::enum_key("AI.Option.Ground.id", "ALARM_STATE");
// Ключ: число слой Sortie берёт из таблицы запущенной DCS, когда вызов выполняется.
let red = arg::enum_key("AI.Option.Ground.val.ALARM_STATE", "RED");
let dcs = ctx.dcs();
dcs.call("Controller.setOption")
    .on(sam())
    .arg(alarm)
    .arg(red)
    .key("sam-alarm-red")
    .await?;
// Или число: слой проверит, что оно есть в таблице (иначе Error::Invalid, в DCS ничего).
dcs.call("Controller.setOption")
    .on(sam())
    .arg(arg::number(9.0)) // AI.Option.Ground.id.ALARM_STATE
    .arg(arg::number(1.0)) // GREEN
    .key("sam-alarm-green")
    .await?;
```

* Значение такого аргумента — ключ таблицы (`arg::enum_key(таблица, "RED")`, на проводе
  `AI.Option.Ground.val.ALARM_STATE.RED`): слой Sortie находит число в таблице запущенной DCS,
  когда вызов выполняется. Ключ другой таблицы — `Error::Invalid` до DCS.
* Или (с 0.4.0) само число таблицы (`arg::number(2.0)`): движок проверяет, что это конечное
  число, слой — что такое значение в таблице есть; иначе `Error::Invalid`, и функция DCS не
  вызывается. Слой Lua до 0.4.0 число в `enum:` не принимает (`Error::Invalid`): нужен
  `sortie install`.
* Один путь — одна запись: у `Controller.setOption` выше второй аргумент берёт только значения
  `ALARM_STATE`. Для других опций (ROE и прочих) объявите аргумент как `number` — тогда движок
  числа не проверяет. Тревога и ROE групп есть и в типизированном `set_option(Alarm::Red)`
  ([опции ИИ](sdk.md#группы-и-опции-ии)): он берёт числа из таблиц запущенной DCS сам.
* Мутирующую запись `sortied --dev` включает сам, без `--dev` её включает `enabled = true` или
  `enable = [..]`; изменяющий вызов — с `.key(..)`.

## Из Python

Тот же каталог доступен генератору: модуль `sortie_gen.dcs_api` строится из него, вызовы идут
через `EngineClient.dcs_call` с токеном `logic` или `admin`.

```python
def resupply(engine: str, token: str) -> None:
    with EngineClient(engine, token=token) as client:   # токен logic или admin
        dcs = Dcs(client)
        print(dcs.land().surface_type((-281_000.0, 647_000.0)))   # (север, восток) -> "LAND"
        units = dcs.group("SAM").units()                           # [DcsObject, ...]
        print([(u.name, u.eid) for u in units])
        try:
            dcs.sounds().out_sound("l10n/DEFAULT/alarm.ogg", key="alarm-1")
        except CommandFailed as e:
            print(e.code, e.capability)   # REJECTED call:mission:trigger.action.outSound
```

У `CommandFailed` с движком 0.4.0 есть `limit` (`sortie_gen.errors.Limit`: `kind` вроде
`"CALL_RATE"`, `budget`, `writer`, числа, `retry_after` в секундах) и `retry_after` — те же,
что `e.limit()` и `e.retry_after()` в Rust; `forbidden` — запрет без лимита. Движок 0.3.0 их
не присылает: `limit` — `None`.
