"""``missions/sortie-lobby.miz``: the mission a dedicated server runs between Sortie missions.

It has no units and no Sortie bootstrap (nothing attaches to it); its only trigger tells
whoever looks at the screen whether the bridge loaded. The engine asks the running lobby
(any running mission would do) for ``Generator.ValidateTypes`` (``Unit.getDescByName`` /
``StaticObject.getDescByName``) and replaces it with ``net.load_mission`` when a build is
launched.
"""

from __future__ import annotations

from datetime import datetime
from typing import Any

from . import _pydcs  # noqa: F401  (configures pydcs)
from . import miz as mizmod

import dcs  # noqa: E402
from dcs import terrain as dcs_terrain  # noqa: E402

LOBBY_COMMENT = "sortie lobby"
LOBBY_SCRIPT = (
    'if SORTIE then trigger.action.outText("Sortie lobby: bridge " .. tostring(SORTIE.version()) '
    '.. " is ready, waiting for the engine to launch a mission", 15) '
    f'else trigger.action.outText("{mizmod.NOT_INSTALLED_TEXT}", 30) end'
)

THEATRES: dict[str, Any] = {
    "caucasus": dcs_terrain.Caucasus,
    "nevada": dcs_terrain.Nevada,
    "normandy": dcs_terrain.Normandy,
    "persiangulf": dcs_terrain.PersianGulf,
    "thechannel": dcs_terrain.TheChannel,
    "syria": dcs_terrain.Syria,
    "marianaislands": dcs_terrain.MarianaIslands,
    "sinai": dcs_terrain.Sinai,
    "kola": dcs_terrain.Kola,
    "afghanistan": dcs_terrain.Afghanistan,
    "falklands": dcs_terrain.Falklands,
    "germany": dcs_terrain.Germany,
}


def theatre(name: str) -> Any:
    key = name.replace("-", "").replace("_", "").replace(" ", "").lower()
    if key not in THEATRES:
        raise ValueError(f"unknown theatre {name!r} (one of {', '.join(sorted(THEATRES))})")
    return THEATRES[key]()


def build_lobby(theatre_name: str = "caucasus") -> bytes:
    """The lobby ``.miz`` bytes (deterministic)."""
    terrain = theatre(theatre_name)
    m = dcs.Mission(terrain)
    m.start_time = datetime(2026, 1, 1, 12, 0, 0, tzinfo=terrain.utc_offset)
    m.set_sortie_text("Sortie lobby")
    m.set_description_text(
        "Sortie lobby: the server idles here until the Sortie engine launches a generated mission."
    )
    mizmod.add_start_script(m, LOBBY_SCRIPT, comment=LOBBY_COMMENT)
    return mizmod.mission_bytes(m)
