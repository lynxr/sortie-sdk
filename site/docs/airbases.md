# Аэродромы: владелец, автозахват, радио, стоянки и полосы

`ctx.airbase_control(..)` меняет у списка аэродромов (до 32) владельца, автозахват DCS и
радиомолчание диспетчеров одной командой: DCS применяет всё за один кадр и сразу читает каждую
базу обратно. Это типизированная операция API ревизии 3, а не [вызов DCS](dcs.md): движок
проверяет весь список до отправки, мир узнаёт нового владельца сразу, а у каждой базы свой
результат. О каждой смене владельца, которую DCS сообщает событием или которую движок сам
делает или читает, логика узнаёт одним событием `AirbaseOwnerChanged`
([ниже](#смена-владельца-событие)). Владельца, которого другой скрипт сменил без события, мир
узнаёт только при чтении: логика, которой это важно, опрашивает `ctx.airbases_state(..)`.
Стоянки, полосы и текущее состояние баз читают типизированные запросы
([ниже](#стоянки-полосы-и-состояние-запросы)), запас складов аэродромов — запрос склада, а
меняет его и переводит между складами команда `ctx.warehouse_op()` ([Склады](warehouses.md)), а
как DCS называет предметы склада, говорит `ctx.item_names()`
([Имена предметов](warehouses.md#имена-предметов)).

```rust
#[sortie::mission(api_rev = 3)] // ctx.airbase_control — API ревизии 3
pub async fn front_moved(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Оба аэродрома — красным, автозахват DCS выключен, диспетчеры молчат. Всё — за один кадр.
    let out = ctx
        .airbase_control([airbases::NALCHIK, airbases::KUTAISI])
        .owner(Coalition::Red)
        .auto_capture(false) // применяется раньше владельца: DCS не вернёт базу сама
        .radio_silent(true)
        .key("front-moved-1")
        .await?;
    // Мир знает нового владельца сразу, как команда вернулась.
    let red = ctx
        .world()
        .airbase_by_name("Kutaisi")
        .is_some_and(|a| a.coalition == Some(Coalition::Red));
    // У каждой базы свой результат: что DCS прочитала до и после, или её ошибка.
    for base in &out.bases {
        match base.error() {
            None => {
                let after = base.after.and_then(|s| s.owner);
                ctx.report().set(&base.name, format!("{after:?}"));
            }
            // DCS не нашла базу (Error::Gone) или сеттер упал (Error::Lua) — только у неё.
            Some(e) => ctx.report().set(&base.name, e.code()),
        }
    }
    ctx.report()
        .set("all_ok", out.ok_count() == out.bases.len() && red);
    ctx.end_session(Outcome::Draw).await
}
```

* Аэродром выбирают как везде в SDK (`sortie::sel::AirbaseSel`): символ сценария
  (`airbases::KUTAISI`), имя DCS (`AirbaseSel::name("Kutaisi")`), Eid аэродрома или Eid
  статика FARP или корабля, который его несёт. В одном списке каждая база — не больше одного
  раза, в том числе под разными селекторами.
* Задайте хотя бы одно из трёх: `.owner(Coalition::Red / Blue / Neutral)`,
  `.auto_capture(bool)`, `.radio_silent(bool)`. Незаданное не меняется. Порядок в DCS у
  каждой базы: прочитать состояние, автозахват, владелец, радио, прочитать снова. Автозахват
  идёт первым, чтобы DCS не отдала базу обратно по наземным войскам рядом.
* Результат — `AirbasesControlled { bases }`, по одной `AirbaseControlled { eid, name, before,
  after }` на базу в порядке списка; `before` и `after` — `AirbaseState { owner,
  auto_capture, radio_silent }`, как их прочитала DCS (`None`, где DCS не ответила).
  `base.is_ok()` и `base.error()` — ошибка этой базы: `Error::Gone` (такой базы в DCS уже нет)
  или `Error::Lua` (сеттер DCS упал; `after` говорит, что DCS читает теперь). Остальные базы
  при этом выполняются, а сама команда успешна. `out.ok_count()` — сколько баз приняли всё.
* Новый владелец, которого DCS прочитала после изменения, сразу в `ctx.world()`: ждать
  ничего не нужно. В ответе команды и в мире — снимки этих баз.
* Захват, который DCS подняла до команды, может прийти в движок уже после её ответа (ответы
  мост отправляет сразу, а события ждут своей очереди). Владельца он не откатывает: движок
  сравнивает их место в порядке событий DCS, а не порядок прихода. Логика получит его
  `BaseCaptured`, но не `AirbaseOwnerChanged`; `ctx.world()` на время этого события берёт
  владельца из него (так делает любой SDK), а следующее обновление мира возвращает
  владельца движка.
* Вся команда — один кадр DCS: около 11 вызовов DCS на базу (найти базу, прочитать, до трёх
  сеттеров, прочитать снова), поэтому в одной команде не больше 32 баз (`cmd::MAX_AIRBASES`).
  Больше баз — несколько команд, у каждой свой кадр. Сколько база стоит в кадре, и
  `setCoalition` особенно, ещё не измерено (`AWC-T20`).
* Ключ (`.key(..)`) как у любой меняющей команды: повтор с тем же ключом вернёт сохранённый
  ответ и до DCS не дойдёт. В `sortie dev` ключ не обязателен, но после горячей перезагрузки
  логика выполнит команду снова — с ключом она не изменит базы второй раз.
* Полоса REALTIME, по умолчанию 10 с на ответ.

## FARP и отказы

```rust
// FARP — тоже аэродром: по имени, по Eid аэродрома или по Eid его статика.
let farp = ctx
    .spawn_static("FARP", "USA")
    .named("FARP North")
    .at(p, 0.0)
    .heliport(Heliport::new())
    .key("farp-north")
    .await?;
ctx.airbase_control([AirbaseSel::eid(farp.eid), AirbaseSel::name("Nalchik")])
    .radio_silent(false) // только радио; владелец и автозахват не меняются
    .key("farp-north-radio")
    .await?;
// Базы, которой нет в мире, команда не отправляет вовсе: Error::Invalid с её именем.
match ctx
    .airbase_control([AirbaseSel::name("Batumi")])
    .owner(Coalition::Blue)
    .await
{
    Err(Error::Invalid(f)) => ctx.report().set("refused", f.message.clone()),
    other => {
        other?;
    }
}
```

До DCS не доходит ничего, если команда неверна целиком, — `Error::Invalid` с тем, что не так:

* базы нет в мире: имя, которого мир не знает, символ сценария без такой базы в миссии;
* база в списке дважды;
* список пуст или длиннее 32;
* не задано ни одной настройки.

База, чей носитель уничтожен (снятый FARP, потопленный корабль), — `Error::Gone`; устаревший
Eid — тоже `Error::Gone`.

## Смена владельца: событие

`AirbaseOwnerChanged` — одно событие на каждую смену владельца аэродрома в мире движка, что бы
о ней ни сообщило: захват самой DCS (`S_EVENT_BASE_CAPTURED`), ваш или чужой
`ctx.airbase_control(..)`, чтение DCS, которое нашло другого владельца. Это событие ревизии 2:
логика объявляет `#[sortie::mission(api_rev = 3, event_rev = 2)]`, иначе не получает его
вовсе (событие `BaseCaptured` при этом приходит как раньше, при любой ревизии).

```rust
#[sortie::mission(api_rev = 3, event_rev = 2)] // AirbaseOwnerChanged — событие ревизии 2
pub async fn owner_events(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Одно событие на каждую смену владельца — что бы о ней ни сообщило движку.
    let mut owners = ctx.events().of::<v1::AirbaseOwnerChanged>();
    while let Some(e) = owners.next().await {
        let name = e
            .airbase
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_default();
        // Мир уже знает нового владельца, когда событие пришло.
        let in_world = ctx.world().airbase_by_name(&name).and_then(|a| a.coalition);
        let why = match e.cause() {
            v1::OwnerChangeCause::Captured => "захват DCS", // S_EVENT_BASE_CAPTURED
            v1::OwnerChangeCause::Control => "airbase_control",
            v1::OwnerChangeCause::Discovered => "найдено при чтении DCS",
            v1::OwnerChangeCause::Unspecified => "?",
        };
        // previous() — Unspecified, если прежнего владельца движок не знал.
        ctx.report().set(
            &name,
            format!(
                "{:?} -> {:?} ({why}), мир: {in_world:?}",
                e.previous(),
                e.owner()
            ),
        );
    }
    ctx.end_session(Outcome::Draw).await
}
```

* Поля: `airbase` — снимок базы после смены (`AirbaseSnap`, его `coalition` — новый
  владелец), `previous` — прежний владелец (`Unspecified`, если движок его не знал: база,
  которую мир впервые увидел в этом захвате), `owner` — новый, `cause` — кто сообщил:
  `Captured` (DCS подняла `S_EVENT_BASE_CAPTURED`; сначала приходит её `BaseCaptured`, сразу
  за ним это событие), `Control` (команда `airbase_control`; событие приходит раньше ответа
  команды), `Discovered` (чтение DCS нашло другого владельца без события: снимок мира после
  разрыва связи или потери событий, FARP, который `spawn_static` оставил с прежним именем).
* Ровно одно событие на смену. Если DCS сообщает о том, что мир уже знает (захват базы,
  которую только что передал `airbase_control`, повтор того же захвата, снимок, подтвердивший
  событие), второго события нет. Туда и обратно — две смены, два события. Захват, который DCS
  подняла раньше, чем `airbase_control` или запрос состояния прочитали базу, но который пришёл
  в движок после их ответа, ничего не меняет (только его `BaseCaptured`, см. выше).
* Первый снимок мира в сессии ничего не публикует: он лишь учит мир владельцам, которых DCS
  держит на старте (в манифесте владельцы из редактора).
* Чего движок не знает. Владельца, которого DCS сменила без события (другой скрипт вызвал
  `Airbase.setCoalition`, сырой вызов `ctx.dcs().airbase(..).set_coalition(..)`), мир узнаёт только со
  следующим снимком мира — после разрыва связи или потери событий — или при запросе состояния
  этой базы (`ctx.airbase(..).state()`, [ниже](#стоянки-полосы-и-состояние-запросы)), а не сразу; регулярного
  обхода аэродромов нет, и логика, которой нужны такие смены, сама опрашивает
  `ctx.airbases_state(..)`. Смену туда и обратно между двумя чтениями без события не видно
  вовсе.
  Поднимает ли DCS `S_EVENT_BASE_CAPTURED` на `setCoalition`, ещё не измерено (`AWC-T21`).

Фильтры потока: `.airbase(..)` — база (символ сценария, имя DCS, Eid базы или её FARP или
корабля), `.cause(..)` — одна причина, `.coalition(..)` — новый владелец.

```rust
// Только захваты Кутаиси самой DCS (наземными войсками).
let mut kutaisi = ctx
    .events()
    .of::<v1::AirbaseOwnerChanged>()
    .airbase(airbases::KUTAISI)
    .cause(v1::OwnerChangeCause::Captured);
// Всё, что перешло к красным: coalition() — новый владелец.
let mut to_red = ctx
    .events()
    .of::<v1::AirbaseOwnerChanged>()
    .coalition(Coalition::Red);
if let Some(e) = kutaisi.next().await {
    ctx.message(Coalition::Blue)
        .text(format!("Кутаиси захвачен: {:?}", e.owner()))
        .await?;
}
let _lost = to_red.next().await;
```

Логика, объявившая `event_rev` ниже 2, или движок, который ревизию 2 не знает (до 0.4.0), — и
`of::<v1::AirbaseOwnerChanged>()` пишет в журнал предупреждение с причиной: событий не будет.
Ревизия событий 2 — часть API ревизии 3: `#[sortie::mission(api_rev = 2, event_rev = 2)]` не
компилируется, а логике старого SDK с таким объявлением движок отдаёт ревизию событий 1, как
отдавал движок 0.3.0.
То же событие идёт в HTTP-поток `GET /api/v1/events` как `airbase_owner_changed`
([Web API](web-api.md)).

## Стоянки, полосы и состояние: запросы

Три чтения аэродромов, тоже API ревизии 3: стоянки (`Airbase.getParking`), полосы
(`Airbase.getRunways`) и состояние — владелец, автозахват, радиомолчание. Это запросы: в DCS
ничего не меняется, ключ не нужен. Одна база — `ctx.airbase(sel)` и `.parking()`,
`.runways()`, `.state()`; много баз одним запросом — `ctx.airbases_parking(..)`,
`ctx.airbases_runways(..)`, `ctx.airbases_state(..)`.

```rust
#[sortie::mission(api_rev = 3)] // запросы аэродромов — API ревизии 3
pub async fn place_aircraft(ctx: Ctx<MyMission>, _start: Start) -> sortie::Result<()> {
    // Свободные стоянки Кутаиси: только те, что DCS отдаёт как свободные.
    let free = ctx
        .airbase(airbases::KUTAISI)
        .parking()
        .available_only()
        .await?;
    // Тип стоянки — по имени; число, которого SDK не знает, — TerminalType::Unknown.
    let open = free
        .iter()
        .filter(|s| {
            matches!(
                s.terminal,
                Some(TerminalType::OpenAir | TerminalType::OpenOrShelter)
            )
        })
        .count();
    // Все стоянки: какие заняты и кем (Eid мира, если DCS назвала объект и мир его держит).
    let all = ctx.airbase(airbases::KUTAISI).parking().await?;
    let taken = all.iter().filter(|s| s.taken == Some(true)).count();
    // Полосы: название, курс (радианы, как отдаёт DCS), длина и ширина в метрах, центр.
    for rw in ctx.airbase(airbases::KUTAISI).runways().await? {
        ctx.report().set(
            &format!("runway {}", rw.name),
            format!(
                "{:?} deg, {:?} m",
                rw.heading_deg().map(f64::round),
                rw.length
            ),
        );
    }
    ctx.report().set("open", open);
    ctx.report().set("taken", taken);
    ctx.end_session(Outcome::Draw).await
}
```

* `.parking()` — `Vec<ParkingSpot>` в порядке DCS: `term_index` (номер стоянки),
  `terminal` — тип по имени (`TerminalType::Runway` 16, `HelicopterOnly` 40, `Shelter` 68,
  `OpenOrShelter` 72, `SmallShelter` 100, `OpenAir` 104 — значения из документации DCS;
  число, которого SDK не знает, приходит как `TerminalType::Unknown("unknown(<n>)")`), `point`,
  `taken` (стоянки нет среди свободных, которые отдаёт `getParking(true)`; `None`, если DCS
  этого не ответила), `dist_to_runway` (`fDistToRW`, метры). `.available_only()` — только
  свободные (одно чтение DCS вместо двух), у каждой `taken == Some(false)`, `s.is_free()`.
* Кем занята стоянка: документация DCS описывает `TO_AC` как булево значение, оно приходит в
  `to_ac`. Если DCS всё же назовёт там объект, он будет в `taken_by` (Eid мира, если мир его
  держит) и `taken_by_name`. `term_index_0` — `Term_Index_0` как есть (не документировано).
* `.runways()` — `Vec<Runway>`: `name` (DCS отдаёт число, «может не совпадать» с табличкой),
  `course` — **в радианах, как отдаёт DCS** (по документации это курс со знаком минус),
  `length` и `width` в метрах, `center`. `rw.heading_deg()` — истинный курс в градусах 0..360
  по этому прочтению документации (не измерено). У FARP и корабля полос нет.
* `.state()` — `AirbaseState { owner, auto_capture, radio_silent }`, как читает её
  `airbase_control`. Если DCS прочитала другого владельца, чем знает мир, мир получает его сразу,
  а логика — одно `AirbaseOwnerChanged` с причиной `Discovered` (выше): так опрос владельцев
  находит смену, которую другой скрипт сделал без события. Если же DCS сообщила о захвате уже
  после чтения и движок получил его первым, мир оставляет этого, более нового владельца.
* Ошибка базы у запроса одной базы — ошибка запроса: базы нет в DCS или мир держит её мёртвой
  (снятый FARP, потопленный корабль), устаревший Eid — `Error::Gone`; чтение DCS упало —
  `Error::Lua`.

```rust
// Владелец, автозахват и радио многих баз — одним запросом (DCS читает 32 базы за кадр).
let read = ctx
    .airbases_state([airbases::KUTAISI, airbases::NALCHIK])
    .await?;
for base in &read.bases {
    match (base.value, base.error()) {
        (Some(s), _) => ctx.report().set(&base.name, format!("{:?}", s.owner)),
        // У каждой базы своя ошибка: Error::Gone — базы нет в DCS или в мире она мертва.
        (None, Some(e)) => ctx.report().set(&base.name, e.code()),
        (None, None) => {}
    }
}
// Стоянки до 16 баз одним запросом; база, не поместившаяся в ответ, — Error::Rejected.
let parking = ctx
    .airbases_parking([AirbaseSel::name("Kutaisi"), AirbaseSel::name("FARP North")])
    .available_only()
    .await?;
ctx.report().set("bases_ok", parking.ok_count());
```

* Много баз: до 16 для стоянок (на большом аэродроме их сотни), до 256 для полос и
  состояния. Каждая база — не больше одного раза. Результат — `AirbasesRead { bases }`, по
  одной `AirbaseRead { eid, name, value }` на базу в порядке списка: `value` — то, что ответила
  DCS, `None` при ошибке этой базы; `base.error()` — её ошибка (`Error::Gone`, `Error::Lua`,
  `Error::Rejected` — стоянки этой базы не поместились в ответ, спросите её отдельно).
  `read.ok_count()` — сколько баз ответили. Сам запрос при этом успешен.
* Имя, которого мир не знает, запрос всё равно отправляет (DCS ответит или скажет
  `Error::Gone`); мёртвую базу и устаревший Eid движок отвечает сам, до DCS они не доходят.
  Целиком запрос отвергается (`Error::Invalid`) только когда список пуст или длиннее предела,
  база в нём дважды, символа нет в сценарии или Eid — не аэродрома.
* Нагрузка на DCS: слой читает по частям, чтобы большой список не останавливал игру, —
  состояние и полосы по 32 базы за кадр (около 6 вызовов DCS на базу для состояния), стоянки
  около 300 за кадр; в ответе не больше 8000 стоянок. Ответ приходит из кадра, в котором
  прочитана последняя база. Полоса QUERY, по умолчанию 10 с на ответ.

## Ревизия и возможности

* Логика объявляет `#[sortie::mission(api_rev = 3)]`; с меньшей ревизией команда —
  `Error::Revision`, против движка с ревизией API ниже 3 — `Error::EngineTooOld`, и ничего не
  отправляется.
* Возможность `caps::AIRBASE_CONTROL` (`"airbase_control"`): включена по умолчанию, как
  `spawn_static`; владелец движка выключает её в `engine.toml`
  (`[operations] disable = ["airbase_control"]`), тогда команда — `Error::Rejected`. Если в
  сборке DCS нет одной из её функций (`Airbase.setCoalition`, `Airbase.autoCapture`,
  `Airbase.setRadioSilentMode` и чтений к ним), её состояние — `MISSING_IN_DCS`. Требовать её
  заранее: `#[sortie::mission(api_rev = 3, requires = [caps::AIRBASE_CONTROL])]` —
  [возможности движка](capabilities.md).
* У запросов свои возможности, тоже включённые по умолчанию: `caps::AIRBASE_PARKING`
  (`"airbase_parking"`, нужна `Airbase.getParking`), `caps::AIRBASE_RUNWAYS`
  (`"airbase_runways"`, `Airbase.getRunways`), `caps::AIRBASE_STATE` (`"airbase_state"`,
  `Airbase.getCoalition`; автозахвата и радио, которых нет в сборке DCS, в ответе просто нет).
  Каждую можно выключить отдельно, и без одной функции DCS остальные работают. Сырые чтения
  каталога `ctx.dcs().airbase(..).parking(..)` и `.runways()` остаются и не устарели: они
  отдают таблицу DCS целиком.
* Сырые обёртки каталога `ctx.dcs().airbase(..).set_coalition(..)`, `.auto_capture(..)` и
  `.set_radio_silent_mode(..)` остаются, но помечены `#[deprecated]`: после них мир узнаёт
  владельца только со следующим снимком мира (после разрыва связи или потери событий) или при
  запросе состояния этой базы — регулярного обхода аэродромов нет, — и каждая обёртка —
  отдельная команда на одну базу.
  Чтения (`coalition()`, `auto_capture_is_on()`, `radio_silent_mode()`) не устарели.

## Что не измерено

Что `Airbase.setCoalition` сразу меняет владельца, что после `autoCapture(false)` DCS не
отдаёт базу сама и что радиомолчание слышно в игре, на реальной DCS ещё не измерено (живые
проверки `AWC-T1`, `AWC-T20`, `AWC-T9`; проба `wide_probe`, шаг Foothold). Поднимает ли DCS
`S_EVENT_BASE_CAPTURED` на `setCoalition` и что `airbase_control` даёт ровно одно
`AirbaseOwnerChanged` на базу — тоже (`AWC-T21`, тот же шаг пробы). Что `getParking(true)`
пропускает именно занятые стоянки, что значат `TO_AC` и `Term_Index_0`, сколько стоит
`getParking` большого аэродрома, что `course` полосы — курс со знаком минус в радианах и что
состояние многих баз читается так же, как сырыми вызовами, — тоже не измерено (`AWC-T22`,
`AWC-T23`, `AWC-T24`; тот же шаг пробы на Кутаиси и FARP пробы). Сколько стоит в кадре одна
база у `airbase_control` (около 11 вызовов DCS) и у запроса состояния (около 6), не измерено:
отсюда пределы 32 базы на команду и 32 базы за кадр у запросов (`AWC-T20`, `AWC-T24`).

## Тестирование

FakeDcs хранит у каждого аэродрома владельца, автозахват (включён в новой миссии) и
радиомолчание (выключено): `dcs.airbase_coalition(имя)`, `dcs.airbase_auto_capture(имя)`,
`dcs.airbase_radio_silent(имя)`. Сырые чтения и сеттеры каталога работают с тем же
состоянием.

Смена владельца в тестах: `dcs.capture_airbase(имя, Coalition::Red)` — захват, как его
сообщает DCS (`S_EVENT_BASE_CAPTURED` со следующим кадром; логика получит `BaseCaptured` и одно
`AirbaseOwnerChanged` с причиной `Captured`); `dcs.set_airbase_coalition_silently(имя, ..)` —
владелец меняется без события, а `dcs.lose_events(1)` заставляет движок снять мир заново, и
смена приходит с причиной `Discovered` (или её находит `ctx.airbase(..).state()`). Ни сырой
`setCoalition`, ни `airbase_control` в FakeDcs `S_EVENT_BASE_CAPTURED` не поднимают.
`dcs.hold_events()` держит события DCS в очереди моста, как когда у движка нет кредитов, а
ответы команд уходят сразу; `dcs.release_events()` отправляет их. Так тест видит захват,
который DCS подняла до `airbase_control`, а движок получил после её ответа.

Стоянки и полосы в FakeDcs: у каждого аэродрома манифеста четыре стоянки (1 `OpenAir`,
2 `OpenOrShelter`, 3 `Shelter`, 4 `HelicopterOnly`) и полоса «9» (курс −π/2, то есть 90°,
2500 × 60 м, в точке аэродрома); у FARP четыре вертолётные стоянки, у одиночной площадки одна,
полос нет. `dcs.take_parking_spot(имя, номер, Some("Юнит"))` занимает стоянку (запрос покажет
`taken` и Eid юнита), `dcs.free_parking_spot(имя, номер)` освобождает. Сырые
`Airbase.getParking` и `getRunways` отдают ту же раскладку в форме DCS.
