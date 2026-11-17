"""``sc.airbase(..., munitions=...)``: an airbase with a limited, known stock of munitions.

pydcs leaves every airport's warehouse unlimited (aircraft, munitions, fuel). On an unlimited
stock DCS's ``Warehouse`` functions are reported to change nothing, so a mission that counts
items needs a base whose munitions are limited; the mission editor writes such a stock as a
list of ``{wsType = {a, b, c, d}, initialAmount = n}``.
"""

from __future__ import annotations

import pytest
from dcs.terrain import caucasus

from sortie_gen import Scenario
from sortie_gen.errors import ScenarioError

from test_player_airfields import build, miz_tables

AIM_9M = (4, 4, 7, 22)
AIM_120C = (4, 4, 7, 106)


def scenario() -> Scenario:
    return Scenario("test_sc", schema=1, theatre=caucasus.Caucasus(), start="2026-11-01T06:30:00Z", source=None)


def stock(warehouse) -> dict[tuple[int, ...], int]:
    """``{wsType: initialAmount}`` of a warehouse table of the ``.miz``."""
    return {
        tuple(int(w.wsType[i]) for i in range(1, 5)): int(w.initialAmount)
        for _, w in sorted(warehouse.weapons.items())
    }


def test_an_airbase_keeps_unlimited_munitions_by_default():
    sc = scenario()
    kutaisi = sc.airbase("Kutaisi", ident="KUTAISI", coalition="blue")
    _, warehouses = miz_tables(build(sc).miz)
    w = warehouses.airports[kutaisi.id]
    assert (w.unlimitedMunitions, w.unlimitedAircrafts, w.unlimitedFuel) == (True, True, True)
    assert len(list(w.weapons.items())) == 0


def test_munitions_limit_the_stock_to_what_is_listed():
    sc = scenario()
    senaki = sc.airbase("Senaki-Kolkhi", ident="SENAKI", coalition="red", munitions={AIM_9M: 20, AIM_120C: 12})
    other = sc.airbase("Kutaisi", ident="KUTAISI")
    _, warehouses = miz_tables(build(sc).miz)
    w = warehouses.airports[senaki.id]
    assert w.unlimitedMunitions is False
    # Only munitions: aircraft and fuel stay as pydcs leaves them.
    assert (w.unlimitedAircrafts, w.unlimitedFuel) == (True, True)
    assert stock(w) == {AIM_9M: 20, AIM_120C: 12}
    assert w.coalition == "RED"
    assert warehouses.airports[other.id].unlimitedMunitions is True


def test_an_empty_stock_is_refused():
    """A limited stock with no entry is not "no weapons": the game's mission editor reads an
    empty list as its default stock (100 of every weapon), and unlisted weapons get 0 only
    next to a listed one. So the way to an empty base is one weapon with amount 0."""
    sc = scenario()
    with pytest.raises(ScenarioError, match="at least one weapon"):
        sc.airbase("Senaki-Kolkhi", ident="SENAKI", munitions={})


def test_a_stock_of_zero_is_limited():
    sc = scenario()
    senaki = sc.airbase("Senaki-Kolkhi", ident="SENAKI", munitions={AIM_9M: 0})
    _, warehouses = miz_tables(build(sc).miz)
    w = warehouses.airports[senaki.id]
    assert w.unlimitedMunitions is False and stock(w) == {AIM_9M: 0}


def test_two_builds_write_the_same_stock():
    sc = scenario()
    sc.airbase("Senaki-Kolkhi", ident="SENAKI", munitions={AIM_120C: 12, AIM_9M: 20})
    assert build(sc).miz == build(sc).miz


@pytest.mark.parametrize(
    "munitions, what",
    [
        ({(4, 4, 7): 5}, "four numbers"),
        ({"weapons.missiles.AIM_120C": 5}, "four numbers"),
        ({(4, 4, 7, -1): 5}, "four numbers"),
        ({AIM_9M: -1}, "0..1000000"),
        ({AIM_9M: 1.5}, "0..1000000"),
        ({AIM_9M: 2_000_000}, "0..1000000"),
        ([AIM_9M], "a mapping"),
    ],
)
def test_a_bad_stock_is_refused_when_declared(munitions, what):
    sc = scenario()
    with pytest.raises(ScenarioError, match=what):
        sc.airbase("Senaki-Kolkhi", ident="SENAKI", munitions=munitions)
