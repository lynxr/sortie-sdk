# The scenario of the mission template (sdk/template/scenario.py), as the quickstart shows it.
# `cargo xtask docs-check` fails when the template drifts from this region.

# snippet:start template-scenario
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
# snippet:end template-scenario
