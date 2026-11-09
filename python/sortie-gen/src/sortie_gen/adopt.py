"""``sortie-gen adopt my.miz``: inspect a mission made in the editor before building on it.

Lists coalitions, groups, units and trigger zones, flags what would break Sortie's rules
(duplicate or reserved names), and warns that pydcs may move aircraft parking when it saves.
"""

from __future__ import annotations

import re
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

from . import _pydcs  # noqa: F401  (configures pydcs)
from . import miz as mizmod

PARKING_WARNING = (
    "pydcs re-assigns parking slots when it loads and saves a mission: aircraft that start "
    "on the ground may end up on a different parking spot than in the editor. Check the "
    "player slots after building, or place them with Scenario.player_flight()."
)


@dataclass
class AdoptedGroup:
    coalition: str
    country: str
    category: str
    name: str
    group_id: int
    units: list[str]
    types: list[str]
    north: float
    east: float
    client: bool
    suggested_ident: str


@dataclass
class AdoptReport:
    path: str
    theatre: str
    start: str
    groups: list[AdoptedGroup] = field(default_factory=list)
    zones: list[dict[str, Any]] = field(default_factory=list)
    has_bootstrap: bool = False
    warnings: list[str] = field(default_factory=list)

    def to_json(self) -> dict[str, Any]:
        return asdict(self)


def suggest_ident(name: str) -> str:
    ident = re.sub(r"[^A-Z0-9]+", "_", name.upper()).strip("_")
    if not ident or not ident[0].isalpha():
        ident = "G_" + ident
    return ident


def adopt(path: str | Path) -> AdoptReport:
    m = mizmod.load(path)
    report = AdoptReport(path=str(path), theatre=str(m.terrain.name), start=m.start_time.isoformat())
    seen_groups: dict[str, int] = {}
    seen_units: dict[str, int] = {}
    any_parked_client = False
    for side, col in m.coalition.items():
        for country in col.countries.values():
            for category, groups in (
                ("ground", country.vehicle_group),
                ("ship", country.ship_group),
                ("air", country.plane_group),
                ("helo", country.helicopter_group),
                ("static", country.static_group),
            ):
                for g in groups:
                    client = category in ("air", "helo") and any(
                        _pydcs.skill_name(getattr(u, "skill", None)) in ("Client", "Player") for u in g.units
                    )
                    if client and g.points and "TakeOffParking" in str(getattr(g.points[0], "type", "")):
                        any_parked_client = True
                    pos = g.units[0].position if g.units else g.position
                    report.groups.append(
                        AdoptedGroup(
                            coalition=side,
                            country=country.name,
                            category=category,
                            name=g.name,
                            group_id=int(g.id),
                            units=[u.name for u in g.units],
                            types=[str(u.type) for u in g.units],
                            north=float(pos.x),
                            east=float(pos.y),
                            client=client,
                            suggested_ident=suggest_ident(g.name),
                        )
                    )
                    if g.name in seen_groups:
                        report.warnings.append(f"two groups are named {g.name!r}: DCS keeps only one of them")
                    seen_groups[g.name] = int(g.id)
                    if "#" in g.name:
                        report.warnings.append(
                            f"group {g.name!r} contains '#', which Sortie reserves for spawned groups: rename it"
                        )
                    for u in g.units:
                        if u.name in seen_units:
                            report.warnings.append(f"two units are named {u.name!r}")
                        seen_units[u.name] = int(u.id)
    for z in m.triggers.zones():
        report.zones.append(
            {
                "name": z.name,
                "north": float(z.position.x),
                "east": float(z.position.y),
                "radius": float(getattr(z, "radius", 0) or 0),
                "suggested_ident": suggest_ident(z.name),
            }
        )
    report.has_bootstrap = mizmod.find_bootstrap(m) is not None
    report.groups.sort(key=lambda g: g.group_id)
    if any_parked_client or any(g.category in ("air", "helo") for g in report.groups):
        report.warnings.append(PARKING_WARNING)
    return report


def render(report: AdoptReport) -> str:
    lines = [
        f"{report.path}: {report.theatre}, start {report.start}, "
        f"{len(report.groups)} groups, {len(report.zones)} trigger zones"
        + (", has a Sortie bootstrap" if report.has_bootstrap else ""),
        "",
        f"  {'id':>4}  {'coalition':<9} {'country':<12} {'kind':<6} {'group':<28} units  suggested ident",
    ]
    for g in report.groups:
        kind = g.category + ("*" if g.client else "")
        lines.append(
            f"  {g.group_id:>4}  {g.coalition:<9} {g.country:<12} {kind:<6} {g.name:<28} {len(g.units):>5}  {g.suggested_ident}"
        )
    if any(g.client for g in report.groups):
        lines.append("  (* = player slots)")
    if report.zones:
        lines.append("")
        lines.append("  trigger zones:")
        for z in report.zones:
            lines.append(
                f"    {z['name']:<28} north {z['north']:.1f} east {z['east']:.1f} r {z['radius']:.0f} m  -> {z['suggested_ident']}"
            )
    for w in report.warnings:
        lines.append("")
        lines.append(f"warning: {w}")
    return "\n".join(lines) + "\n"
