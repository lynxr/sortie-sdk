# examples/op_north/scenario.py
from dcs import vehicles, planes
from dcs.terrain import caucasus
from sortie_gen import Scenario, StartType

sc = Scenario("op_north", schema=3, theatre=caucasus.Caucasus(),
              start="2026-11-01T06:30:00Z", time_limit="3h")   # лимит закрывает ход
obj  = sc.zone("Objective", ident="OBJECTIVE", at=sc.latlon(44.62, 40.10), radius=500)
ring = sc.zone("SAM Ring", ident="SAM_RING", at=obj.center, radius=60_000)
sc.ground_group("SAM North", ident="SAM_NORTH", country="Russia", at=obj.center,
    units=[vehicles.AirDefence.S_300PS_54K6_cp, vehicles.AirDefence.S_300PS_40B6M_tr,
           vehicles.AirDefence.S_300PS_64H6E_sr, *[vehicles.AirDefence.S_300PS_5P85C_ln] * 4],
    roles={"sam", "north"})
sc.air_template("Red CAP", planes.Su_27, ident="RED_CAP", country="Russia", size=2)  # в manifest, не в .miz
sc.airbase("Maykop-Khanskaya", ident="MAYKOP_KHANSKAYA")
sc.player_flight("Blue 1", planes.F_16C_50, country="USA",
                 airbase=sc.theatre.airports["Senaki-Kolkhi"], start=StartType.Cold, size=2)
sc.flag("SAM_NORTH_DOWN")
sc.menu("SMOKE_OBJECTIVE", "Smoke the objective")

if __name__ == "__main__":
    sc.cli()   # lock | build | register | launch | watch | results
