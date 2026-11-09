# Установка рантайма

Рантайм — это движок `sortied`, утилита `sortie` и мост для DCS. Он ставится один раз на
машину с DCS. Логику и сценарий вы пишете отдельно, с этим SDK.

## Архив

Скачайте `sortie-runtime-0.1.0-win64.zip` со страницы выпуска
[v0.1.0](https://github.com/lynxr/sortie-sdk/releases/tag/v0.1.0). Всё содержимое
архива лежит в одной папке `sortie-runtime-0.1.0-win64`. Распакуйте архив и
перенесите эту папку в постоянное место, например в `C:\Sortie`:

```powershell
Expand-Archive .\sortie-runtime-0.1.0-win64.zip -DestinationPath .
Move-Item .\sortie-runtime-0.1.0-win64 C:\Sortie
```

Внутри папки:

| Файл | Что это |
| --- | --- |
| `bin\sortied.exe` | движок |
| `bin\sortie.exe` | установка в DCS, проверка установки, наблюдение за движком |
| `Scripts\` | мост: DLL `sortie_bridge` и Lua-слой, разложенные так же, как в Saved Games |
| `install\MissionScripting.snippet.lua` | строка, которую `install` добавляет в `MissionScripting.lua` |
| `README.md` | краткая инструкция |
| `LICENSE.txt`, `THIRD-PARTY-NOTICES.txt` | лицензия рантайма и сторонних компонентов |
| `SHA256SUMS` | контрольные суммы файлов внутри папки |

Сам архив сверяется с файлом `SHA256SUMS` на странице выпуска (контрольные суммы архива рантайма
и колеса генератора).

Windows помечает файлы из интернета. Чтобы SmartScreen не спрашивал при каждом запуске, снимите
пометку:

```powershell
Get-ChildItem C:\Sortie -Recurse | Unblock-File
```

Сверить файл с `SHA256SUMS` можно так:

```powershell
Get-FileHash C:\Sortie\bin\sortied.exe -Algorithm SHA256
```

## Установка в DCS

Откройте PowerShell от имени администратора и выполните:

```powershell
cd C:\Sortie
.\bin\sortie.exe install
.\bin\sortie.exe doctor
```

`install` находит DCS и папку Saved Games сам и:

* копирует Lua-слой в `Saved Games\DCS\Scripts\Sortie\` и хук в
  `Saved Games\DCS\Scripts\Hooks\sortie-hook.lua`;
* кладёт DLL моста в `Scripts\Sortie\bin\` с версией в имени. DLL начинает работать при
  следующем запуске DCS;
* создаёт секрет связи `Saved Games\DCS\Sortie\link.secret` и файл токенов
  `Saved Games\DCS\Sortie\tokens.toml` с токенами ролей `admin` и `logic`;
* добавляет одну строку в `<папка DCS>\Scripts\MissionScripting.lua`, перед санитайзером. Через
  неё среда миссии загружает мост. Это единственная правка в папке DCS, и ради неё нужны права
  администратора. Прежний файл сохраняется рядом.

Если запускать от имени администратора нельзя, поставьте всё остальное без правки, а строку
добавьте позже из консоли администратора:

```powershell
.\bin\sortie.exe install --no-patch
.\bin\sortie.exe doctor --fix
```

### Флаги

| Флаг | Что делает |
| --- | --- |
| `--instance <имя>` | экземпляр DCS: папка `Saved Games\DCS.<имя>` (например, `server` для dedicated server) |
| `--saved-games <папка>` | папка Saved Games экземпляра, если её не удаётся найти |
| `--dcs <папка>` | папка установки DCS, если их несколько или DCS не находится |
| `--dll <файл>` | поставить другую DLL моста |
| `--from <папка>` | взять файлы из другой распакованной копии рантайма |
| `--dry-run` | только показать, что изменится |
| `--no-patch` | не трогать `MissionScripting.lua` |

У флагов `--instance`, `--saved-games` и `--dcs` есть переменные окружения: `SORTIE_INSTANCE`,
`SORTIE_SAVED_GAMES`, `SORTIE_DCS`.

DCS ищется в реестре (ключи `Software\Eagle Dynamics\DCS World`, `DCS World OpenBeta`,
`DCS World Server`), затем в `Program Files` и в библиотеках Steam. Поэтому DCS, поставленная
на другой диск, находится без флагов. Если установок несколько, команда печатает, какую взяла;
другую выберите флагом `--dcs`.

## Перезапустите DCS

Если DCS была запущена во время установки, закройте её и запустите снова. Перезапуска миссии
мало: часть моста, которая работает в среде hooks, загружается только при старте самой DCS.
Новая DLL тоже начинает работать со следующего запуска DCS.

## Проверка: `sortie doctor`

`doctor` печатает таблицу проверок:

* строка в `MissionScripting.lua` на месте и стоит перед санитайзером;
* хук, Lua-слой и DLL моста установлены и совпадают с тем, что ставил `install`;
* DLL моста подходит к `lua.dll` установленной DCS;
* есть секрет связи и токены;
* у dedicated server непустой `missionList`.

Код выхода 1 значит, что что-то требует внимания; строка с проблемой подсказывает, что делать.
Обновление DCS перезаписывает `MissionScripting.lua` и стирает строку Sortie. После каждого
обновления DCS запускайте `doctor`, а если строка пропала — `doctor --fix` от имени
администратора.

## Запуск движка

Для работы на своей машине запускайте движок в режиме разработки:

```powershell
C:\Sortie\bin\sortied.exe --dev
```

* `sortied` слушает API на `127.0.0.1:50777`. Логика и генератор подключаются к этому адресу по
  умолчанию.
* Мост находит движок сам. DCS и `sortied` можно запускать в любом порядке.
* При старте `sortied` сам выполняет проверки `doctor` и пишет в лог, что требует внимания.
* `--dev` разрешает локальным программам работать без токена. Взамен он требует ключ `.key(..)`
  у каждого спавна и уничтожения группы в логике (см. [Логику на Rust](sdk.md)).

Без `--dev` каждая программа предъявляет токен из `Saved Games\DCS\Sortie\tokens.toml`:

* логика — токен роли `logic` в переменной окружения `SORTIE_TOKEN`;
* генератор (`validate`, `register`, `launch`, `restart`) — токен роли `admin` или
  `generator` во флаге `--token` или в `SORTIE_TOKEN`;
* `sortie status` и другие команды утилиты читают `tokens.toml` сами.

Роли и формат файла — в [Эксплуатации](operations.md).

Убедиться, что движок работает и видит мост:

```powershell
C:\Sortie\bin\sortie.exe status
```

## Удаление

Удалите из папки Saved Games экземпляра `Scripts\Sortie\`, `Scripts\Hooks\sortie-hook.lua` и
`Sortie\` (там же токены, хранилище движка и логи). В `<папка DCS>\Scripts\MissionScripting.lua`
удалите блок между строками `-- >>> sortie` и `-- <<< sortie` или верните файл через
восстановление DCS (repair). Зарегистрированные миссии лежат в `Missions\Sortie\`.
