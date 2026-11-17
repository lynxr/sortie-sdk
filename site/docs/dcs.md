# Вызовы DCS

`ctx.dcs()` вызывает функции скриптового API DCS, у которых в SDK ещё нет своей типизированной
операции: высота и тип поверхности, ветер, владелец аэродрома, склад, лазерные пятна, звуки и
многое другое. Каждая функция описана в каталоге DCS API — файле `catalog/dcs_api.toml` в
репозитории SDK. Для каждой записи каталога генерируется обычный метод SDK с типами аргументов и
результата; полный список — в [справочнике каталога](dcs-catalog.md).

Движок проверяет каждый вызов по каталогу до того, как что-то попадёт в DCS: путь функции,
окружение, класс объекта, типы и диапазоны аргументов. Ошибка в аргументе — `Error::Invalid` с
сигнатурой записи в сообщении, а не ошибка Lua в игре.

## Ревизия API

Вызовы DCS — API ревизии 1. Логика объявляет её в атрибуте миссии; без него вызов вернёт
`Error::Revision`, и в движок ничего не уйдёт:

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
enable = ["Airbase.setCoalition", "Airbase.autoCapture", "Warehouse.addItem"]
```

Пока функция выключена, вызов возвращает `Error::Rejected` с именем capability
`call:mission:<путь>`. Изменяющий вызов повторяет своё действие при каждом выполнении, поэтому
ставьте ему `.key(..)`: повтор с тем же ключом вернёт сохранённый результат и не дойдёт до DCS.

```rust
let dcs = ctx.dcs();
let kutaisi = dcs.airbase(airbases::KUTAISI);
kutaisi.auto_capture(false).key("kutaisi-manual").await?;
kutaisi
    .set_coalition(Coalition::Blue)
    .key("kutaisi-blue")
    .await?;
// Склад аэродрома: по селектору аэродрома.
let stock = dcs.warehouse(airbases::KUTAISI);
stock
    .add_item("weapons.missiles.AIM_120C", 8.0)
    .key("resupply-1")
    .await?;
let left: f64 = stock.item_count("weapons.missiles.AIM_120C").await?;
let inventory: Value = stock.inventory(None).await?; // большая таблица: класс heavy
```

## Эффекты и звуки

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
| `read` | QUERY | включено (кроме функций «проба») | нельзя: `Error::Invalid` | 200 вызовов и 20 мс времени DCS в секунду |
| `heavy` | BULK, по одному | включено (запись владельца не с get/is/has — выключена, как `mutate`) | нельзя | 2 в секунду |
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
    .airbase(airbases::NALCHIK)
    .set_coalition(Coalition::Blue)
    .key("nalchik-blue")
    .await
{
    Ok(()) => {}
    // Владелец движка не включил функцию: capability call:mission:Airbase.setCoalition.
    Err(Error::Rejected(_)) => ctx.report().set("nalchik", "manual capture is off"),
    Err(e) => return Err(e),
}
```

| Ошибка | Когда |
| --- | --- |
| `Error::Rejected` | функции нет в каталоге, она выключена, её нельзя вызывать никогда (Lua-код, файлы, обратные вызовы), превышен лимит |
| `Error::Invalid` | аргументы не подходят к записи каталога; ключ на чтении; объект не того класса |
| `Error::Gone` | объекта уже нет в DCS (или Eid устарел) |
| `Error::Unavailable` | функции нет в этой сборке DCS |
| `Error::ResultNotKept` | повтор вызова с ключом, результат которого был больше 64 КиБ: вызов не выполнялся снова, а его значения движок не хранит |
| `Error::Revision` | логика не объявила `api_rev = 1` |

## Чего зеркало не узнаёт

Изменение, сделанное вызовом, зеркало мира видит только при следующем проходе сэмплера:
владельца аэродрома после `set_coalition` — на следующем обходе аэродромов, флаг — при
следующем чтении флагов. Где у SDK есть типизированная операция, обёртка помечена
`#[deprecated]` и называет её: например, дым — это `ctx.smoke(..)`.

## Вызов по имени

Функцию каталога без обёртки (например, добавленную владельцем движка в `engine.toml`) можно
вызвать по строке пути. Это функция крейта за cargo-фичей `dcs-raw`:
`sortie = { ..., features = ["dcs-raw"] }`. Движок проверяет такой вызов так же, как вызов
обёртки.

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
чтений из полной корзины (200 в секунду) проходят, но следующий пакет придётся подождать.

Сколько стоила каждая команда, а не только пакет, говорит `ctx.take_command_timings()`: по
одной записи `CommandTiming` на каждую команду с ответом (вызов DCS, пакет, типизированная
операция; ошибки тоже) — операция, цель (путь Lua у вызова), `cost_us` (время внутри DCS),
`round_trip` (от отправки до ответа: очередь и лимиты движка, ожидание ближайшего pump миссии
раз в 0,02 с, связь в обе стороны) и модельное время ответа. Записи забираются один раз; SDK
держит не больше 4096, лишние старые отбрасывает и считает в `dropped`.

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
            dcs.warehouse("Kutaisi").add_item("weapons.missiles.AIM_120C", 8, key="resupply-1")
        except CommandFailed as e:
            print(e.code, e.capability)   # REJECTED call:mission:Warehouse.addItem
```
