# What a scenario adds through pydcs directly (`@sc.pydcs`): statics, weather, briefing and an
# own DO SCRIPT trigger. Built offline by python/sortie-gen/tests/test_sdk_snippets.py.

import dcs
from dcs import vehicles
from dcs.terrain import caucasus
from dcs.triggers import TriggerStart
from sortie_gen import Scenario
from sortie_gen.miz import InlineDoScript

sc = Scenario("hooks_demo", schema=1, theatre=caucasus.Caucasus(), start="2026-06-01T08:00:00Z")
target = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)
sc.ground_group("Armor", ident="ARMOR", country="Russia", at=target.center,
                units=[vehicles.Armor.T_72B] * 3)


# snippet:start gen-pydcs-hook
@sc.pydcs
def extras(m: dcs.Mission) -> None:
    # Статический объект: штаб рядом с целью (в .miz, без символа для Rust).
    hq = dcs.mapping.Point(target.center.x + 800, target.center.y, m.terrain)
    m.static_group(m.country("Russia"), "HQ", dcs.statics.Fortification._Command_Center, hq)
    # Погода и брифинг.
    m.weather.clouds_density = 4
    m.weather.clouds_base = 2_500
    m.set_description_bluetask_text("Уничтожить бронегруппу в зоне Target.")
# snippet:end gen-pydcs-hook


# snippet:start gen-do-script
@sc.pydcs
def hello(m: dcs.Mission) -> None:
    trig = TriggerStart(comment="hello")
    trig.add_action(InlineDoScript('trigger.action.outText("Миссия загружена", 10)'))
    m.triggerrules.triggers.append(trig)
# snippet:end gen-do-script


if __name__ == "__main__":
    sc.cli()
