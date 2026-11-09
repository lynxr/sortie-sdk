"""The `frontline` scenario of the agent skills (sdk/agents/sortie-scenario).

The regions between the snippet markers are the code blocks of the skill, in file order; the
Rust examples of sortie-mission-logic use the symbols this scenario declares
(sdk/snippets/src/skills/frontline.rs is the module `include_scenario!(frontline)` makes of its
lock). Run it like any scenario: `python skills_scenario.py lock --out <file>`.
"""

# snippet:start skills-scenario-header
from dcs import planes, statics, vehicles
from dcs.terrain import caucasus

from sortie_gen import Coalition, Scenario, StartType

sc = Scenario(
    "frontline",
    schema=1,
    theatre=caucasus.Caucasus(),
    start="2026-11-01T07:00:00Z",  # RFC 3339; without an offset: the map's local time
    time_limit="4h",  # the engine ends the session (turn) after 4 h of model time
)
# snippet:end skills-scenario-header

# snippet:start skills-scenario-zones
obj = sc.zone("Objective", ident="OBJECTIVE", at=sc.latlon(44.62, 40.10), radius=500)
ring = sc.zone("SAM Ring", ident="SAM_RING", at=obj.center, radius=60_000)
front = sc.zone("Front", ident="FRONT", at=sc.latlon(44.95, 38.60), radius=20_000)
sc.zone("Editor note", at=sc.latlon(44.70, 40.00), radius=1_000)  # no ident: .miz only
# snippet:end skills-scenario-zones

# snippet:start skills-scenario-groups
sc.ground_group(
    "SAM North",
    ident="SAM_NORTH",
    country="Russia",
    at=obj.center,
    units=[
        vehicles.AirDefence.S_300PS_54K6_cp,
        vehicles.AirDefence.S_300PS_40B6M_tr,
        vehicles.AirDefence.S_300PS_64H6E_sr,
        *[vehicles.AirDefence.S_300PS_5P85C_ln] * 4,
    ],
    roles={"sam", "north"},
)
sc.ground_group(
    "Red Armor",
    ident="RED_ARMOR",
    country="Russia",
    at=front.center,
    units=[vehicles.Armor.T_72B] * 4,
    formation="line",
    heading=270,
    roles={"armor"},
)
# snippet:end skills-scenario-groups

# snippet:start skills-scenario-templates
sc.air_template(
    "Red CAP",
    planes.Su_27,
    ident="RED_CAP",
    country="Russia",
    size=2,
    airbase="Maykop-Khanskaya",  # default placement when the logic gives none
    payload=[  # pylon tuples: the hermetic build knows no default loadout for this task
        planes.Su_27.Pylon1.R_73__AA_11_Archer____Infra_Red,
        planes.Su_27.Pylon10.R_73__AA_11_Archer____Infra_Red,
    ],
)
# snippet:end skills-scenario-templates

# snippet:start skills-scenario-airbases
sc.airbase("Maykop-Khanskaya", ident="MAYKOP", coalition=Coalition.Red)
sc.airbase("Krymsk", ident="KRYMSK")  # the player flights below give it to blue
sc.player_flight(
    "Viper 1", planes.F_16C_50, country="USA", airbase="Krymsk", start=StartType.Cold, size=2
)
sc.player_flight(
    "Hog 1", planes.A_10C_2, country="USA", airbase="Krymsk", start=StartType.Hot, size=2
)
# snippet:end skills-scenario-airbases

# snippet:start skills-scenario-flags-menus
sc.flag("SAM_DOWN")
sc.menu("SMOKE_OBJECTIVE", "Smoke the objective", path=["Support"])
sc.menu("SITREP", "Situation report")
# snippet:end skills-scenario-flags-menus

# snippet:start skills-scenario-pydcs
from dcs.triggers import TriggerStart

from sortie_gen.miz import InlineDoScript


@sc.pydcs
def extras(m):
    # What Sortie does not wrap: statics, weather, briefing, your own triggers.
    hq = sc.latlon(44.61, 40.08)
    m.static_group(m.country("Russia"), "Red HQ", statics.Fortification._Command_Center, hq)
    t = TriggerStart(comment="welcome")
    t.add_action(InlineDoScript('trigger.action.outText("Frontline is live", 15)'))
    m.triggerrules.triggers.append(t)
# snippet:end skills-scenario-pydcs

# snippet:start skills-scenario-main
if __name__ == "__main__":
    sc.cli()  # lock | build | validate | register | launch | restart | watch | results
# snippet:end skills-scenario-main
