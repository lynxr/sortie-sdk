# A ground template whose units face apart (0.4.0): the logic spawns it with
# `.at_formation(..)`, which keeps that spread. Built offline by
# python/sortie-gen/tests/test_sdk_snippets.py.

from dcs import vehicles
from dcs.terrain import caucasus
from sortie_gen import Scenario

sc = Scenario("battery_demo", schema=1, theatre=caucasus.Caucasus(), start="2026-06-01T08:00:00Z")

# snippet:start gen-ground-template
launcher = vehicles.AirDefence.S_300PS_5P85C_ln
# Две пусковые под опорным курсом 90°: одна смотрит на 75°, другая на 105°.
sc.ground_template("Battery", ident="BATTERY", country="Russia",
                   at=sc.latlon(42.27, 42.90), units=[launcher, launcher],
                   heading=90, unit_headings=[-15, 15], spacing=40)
# snippet:end gen-ground-template

if __name__ == "__main__":
    sc.cli()
