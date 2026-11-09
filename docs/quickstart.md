# Быстрый старт

От двух архивов до первой миссии в игре. Понадобятся DCS, Rust, uv и Git (см.
[требования](index.md#требования)).

## 1. Рантайм

Скачайте и установите рантайм по [инструкции](install-runtime.md): `sortie install`,
`sortie doctor`, перезапуск DCS. Дальше считается, что он распакован в `C:\Sortie`.

## 2. Шаблон миссии

На странице выпуска [v0.1.0](https://github.com/lynxr/sortie-sdk/releases/tag/v0.1.0)
скачайте исходники SDK («Source code (zip)») и распакуйте. Скопируйте из них папку `template`
туда, где будет жить ваша миссия, например в `C:\Missions\my_mission`. Или возьмите шаблон через
Git:

```powershell
git clone --branch v0.1.0 https://github.com/lynxr/sortie-sdk sortie-sdk
Copy-Item -Recurse sortie-sdk\template C:\Missions\my_mission
```

В шаблоне:

| Файл | Что это |
| --- | --- |
| `scenario.py` | сценарий: карта, зона цели, три группы противника, слоты игроков |
| `scenario.lock.json` | символы сценария для логики; его пишет генератор |
| `pyproject.toml` | Python-проект с зависимостью `sortie-gen` |
| `Cargo.toml` | крейт логики с зависимостями `sortie` и `sortie-build` |
| `build.rs` | превращает `scenario.lock.json` в типизированные константы |
| `src/lib.rs` | логика миссии |
| `src/main.rs` | `main` программы логики |
| `AGENTS.md` | инструкции для ИИ-агентов (Codex, Cursor, Claude Code); навыки — в папке `agents/` SDK |

Зависимости в `Cargo.toml` и `pyproject.toml` уже указывают на этот выпуск SDK. Если репозиторий
SDK закрытый, сначала настройте [доступ к нему](dependency.md#закрытый-репозиторий).

## 3. Сценарий

<!-- snippet: template-scenario -->
```python
from dcs import planes, vehicles
from dcs.terrain import caucasus
from sortie_gen import Scenario, StartType

sc = Scenario("my_mission", schema=1, theatre=caucasus.Caucasus(),
              start="2026-06-01T08:00:00Z", time_limit="2h")

# Зона цели: круг радиусом 3 км. ident делает её символом zones::TARGET в Rust.
target = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)

# Противник в зоне цели: три наземные группы.
sc.ground_group("Armor", ident="ARMOR", country="Russia", at=target.center,
                units=[vehicles.Armor.T_72B] * 3 + [vehicles.Armor.BMP_2] * 2)
sc.ground_group("SAM", ident="SAM", country="Russia", at=sc.latlon(42.285, 42.915),
                units=[vehicles.AirDefence.Osa_9A33_ln])
sc.ground_group("AAA", ident="AAA", country="Russia", at=sc.latlon(42.26, 42.885),
                units=[vehicles.AirDefence.ZSU_23_4_Shilka] * 2)

# Слоты игроков: два Су-25Т на стоянке Кутаиси. Холодный старт отдаёт аэродром
# коалиции полёта (синим), иначе в сетевой игре слот не занять.
sc.player_flight("Blue 1", planes.Su_25T, country="Georgia",
                 airbase="Kutaisi", start=StartType.Cold, size=2)

if __name__ == "__main__":
    sc.cli()   # lock | build | validate | register | launch | restart | watch | results
```

Су-25Т бесплатный, он есть в любой DCS. Проверьте, что сценарий собирается:

```powershell
cd C:\Missions\my_mission
uv run python scenario.py lock
uv run python scenario.py build
```

Первый запуск `uv` создаёт `.venv` и ставит `sortie-gen` с pydcs. `lock` переписывает
`scenario.lock.json`, только если изменился набор символов, и говорит, что изменилось. `build`
собирает `dist\<build_id>.miz` и manifest без движка: так видно, что сценарий корректен.

## 4. Логика

<!-- snippet: template-mission -->
```rust
#[sortie::mission]
pub async fn run(ctx: Ctx<MyMission>, start: Start) -> sortie::Result<()> {
    let armor = ctx.group(groups::ARMOR)?; // Group<Ground>: группа есть в мире
    let sam = ctx.group(groups::SAM)?;

    if start.is_fresh() {
        ctx.message(Coalition::Blue)
            .text("Цель: бронегруппа в зоне Target. Её прикрывают «Оса» и «Шилки».")
            .secs(20)
            .key("intro")
            .await?;
    }

    // ЗРК молчит, пока синие самолёты не подойдут к цели ближе 15 км.
    sam.set_option(Alarm::Green).key("sam-green").await?;
    ctx.wait_until(|w| {
        w.units()
            .coalition(Coalition::Blue)
            .air()
            .within(zones::TARGET.center(), km(15))
            .any()
    })
    .await;
    if sam.is_alive() {
        sam.set_option(Alarm::Red).key("sam-red").await?;
        ctx.message(Coalition::Blue)
            .text("«Оса» включила радар!")
            .key("sam-alert")
            .await?;
    }

    // Победа, когда от бронегруппы ничего не осталось.
    ctx.wait_until(|w| w.group(&armor).alive_count() == 0).await;
    ctx.message(Coalition::Blue)
        .text("Цель уничтожена. Миссия выполнена!")
        .secs(30)
        .key("win")
        .await?;
    ctx.end_session(Outcome::BlueWin)
        .text("бронегруппа уничтожена")
        .await
}
```

`groups::ARMOR`, `groups::SAM` и `zones::TARGET` сгенерированы из `scenario.lock.json`. Их
подключают `build.rs` и одна строка в начале `src/lib.rs`:

<!-- snippet: template-build-rs -->
```rust
fn main() {
    sortie_build::scenario("my_mission", "scenario.lock.json").emit();
}
```

`src/main.rs` — одна строка:

<!-- snippet: template-main-rs -->
```rust
sortie::main!(my_mission_logic::run);
```

Соберите логику:

```powershell
cargo build --release
```

Первая сборка скачивает зависимости и занимает несколько минут.

## 5. Движок и логика

В отдельном окне запустите движок:

```powershell
C:\Sortie\bin\sortied.exe --dev
```

В другом окне — логику:

```powershell
cd C:\Missions\my_mission
cargo run --release
```

Логика подключается к `sortied` и ждёт сессию `my_mission`. Если движок ещё не запущен, она
повторяет попытки сама.

## 6. Регистрация миссии

```powershell
uv run python scenario.py register
```

`register` берёт у движка новый build id, собирает миссию, проверяет типы юнитов и отдаёт сборку
движку. Движок кладёт её в `Saved Games\DCS\Missions\Sortie\my_mission-<build_id>.miz`. Если в
DCS сейчас не идёт ни одна миссия, проверить типы нельзя: команда предупредит и зарегистрирует
сборку без проверки.

## 7. Запуск в DCS

* **Одиночная игра.** В DCS откройте «Миссия» и выберите файл из
  `Saved Games\DCS\Missions\Sortie\`. Займите слот Blue 1.
* **Сетевая игра, хост — ваш клиент DCS.** «Мультиплеер» → «Новый сервер», добавьте ту же миссию
  в список и запустите. Игроки занимают слоты Blue 1.
* **Dedicated server.** Нужна установка с `--instance` и миссия-заглушка в списке миссий сервера
  (см. [Эксплуатацию](operations.md#dedicated-server)). Сборку загружает сам движок:

  ```powershell
  uv run python scenario.py launch
  ```

При старте миссии мост сообщает движку build id сборки. Движок сверяет его с зарегистрированной
сборкой и начинает сессию, а логика получает вызов `run`. Если мост не установлен, на экране
появится «Sortie bridge is not installed: run sortie doctor».

## 8. Что вы увидите

* Сразу после старта миссии синим придёт сообщение о цели.
* Когда синий самолёт подойдёт к цели ближе 15 км, «Оса» включит радар и придёт сообщение.
* Когда бронегруппа будет уничтожена, придёт «Цель уничтожена», и сессия закончится с победой
  синих. Миссия в DCS при этом продолжается.

Наблюдать за сессией:

```powershell
uv run python scenario.py watch     # события сессии до Ctrl-C
uv run python scenario.py results   # итоги последней сессии
C:\Sortie\bin\sortie.exe status     # движок, мост, сессия
```

## 9. Дальше

* Изменили логику — пересоберите и перезапустите её, DCS перезапускать не нужно. Новый процесс
  подключится к той же сессии и получит `Start::AfterCrash`. Команды с тем же `.key(..)` второй
  раз в DCS не уйдут.
* Изменили сценарий — выполните `lock`. Если изменился набор символов, пересоберите логику и
  зарегистрируйте новую сборку. Сдвиг зоны или группы логику пересобирать не заставляет.
* Переименовать шаблон: имя сценария стоит в `scenario.py` (`Scenario("my_mission", ...)`), в
  `build.rs`, в `src/lib.rs` (`include_scenario!(my_mission)` и `use my_mission::...`); имя
  крейта — в `Cargo.toml` и в `src/main.rs`. После переименования выполните `lock`.

Подробно об API — в разделах [Логика на Rust](sdk.md) и [Сценарий на Python](generator.md).
