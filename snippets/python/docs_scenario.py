# The scenario of the docs: the symbols every Rust example of sdk/docs uses (crate
# sortie-snippets). `cargo xtask fixtures` writes its lock to sdk/snippets/scenario/.

# snippet:start gen-scenario
# snippet:start gen-header
from dcs import planes, vehicles
from dcs.terrain import caucasus
from sortie_gen import Scenario, StartType

sc = Scenario("my_mission", schema=1, theatre=caucasus.Caucasus(),
              start="2026-06-01T08:00:00Z", time_limit="2h")
# snippet:end gen-header

# snippet:start gen-zones
target = sc.zone("Target", ident="TARGET", at=sc.latlon(42.27, 42.90), radius=3_000)
patrol = sc.zone("Patrol", ident="PATROL", at=target.center, radius=40_000)
sc.zone("Front", ident="FRONT", points=[sc.latlon(42.35, 42.80), sc.latlon(42.35, 43.00),
                                        sc.latlon(42.20, 43.00), sc.latlon(42.20, 42.80)])
# snippet:end gen-zones

# snippet:start gen-ground
sc.ground_group("Armor", ident="ARMOR", country="Russia", at=target.center,
                units=[vehicles.Armor.T_72B] * 3 + [vehicles.Armor.BMP_2] * 2,
                heading=90, formation="line", spacing=30, roles={"armor", "target"})
sc.ground_group("SAM", ident="SAM", country="Russia", at=sc.latlon(42.285, 42.915),
                units=[vehicles.AirDefence.Osa_9A33_ln], roles={"sam"})
sc.ground_group("AAA", ident="AAA", country="Russia", at=sc.latlon(42.26, 42.885),
                units=[vehicles.AirDefence.ZSU_23_4_Shilka] * 2)
# snippet:end gen-ground

# snippet:start gen-template
mig = planes.MiG_29S
sc.air_template("Red CAP", mig, ident="RED_CAP", country="Russia", size=2, airbase="Nalchik",
                payload=[mig.Pylon1.R_73__AA_11_Archer____Infra_Red,
                         mig.Pylon3.R_27R__AA_10_Alamo_A____Semi_Act_Rdr,
                         mig.Pylon5.R_27R__AA_10_Alamo_A____Semi_Act_Rdr,
                         mig.Pylon7.R_73__AA_11_Archer____Infra_Red])
# snippet:end gen-template

# snippet:start gen-airbases
sc.airbase("Nalchik", ident="NALCHIK", coalition="red")
sc.airbase("Kutaisi", ident="KUTAISI")   # владельца задаст полёт игроков ниже
sc.player_flight("Blue 1", planes.Su_25T, country="Georgia",
                 airbase="Kutaisi", start=StartType.Cold, size=2)
# snippet:end gen-airbases

# snippet:start gen-flags-menus
sc.flag("TARGET_DOWN")
sc.menu("SMOKE_TARGET", "Дым на цели", path=["Цель"])
sc.menu("TARGET_INFO", "Где цель?", path=["Цель"])
# snippet:end gen-flags-menus

if __name__ == "__main__":
    sc.cli()
# snippet:end gen-scenario
