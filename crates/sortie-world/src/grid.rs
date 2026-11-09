//! A uniform spatial grid on the map plane (runtime x/z = north/east) for zone queries.
//!
//! Every entity with a finite position sits in exactly one square cell of [`CELL_M`] metres.
//! A zone query visits only the cells that overlap the zone's bounding box; for cells that lie
//! completely inside the zone (exact for circles) the per-entity containment test is skipped.
//! When the bounding box covers more cells than are occupied, the occupied cells are visited
//! instead, so a huge zone over a sparse world costs at most one check per occupied cell.

use std::collections::HashMap;

use sortie_types::{Bounds, Eid, MapPoint, ZoneShape};

/// Cell edge in metres. Small enough that a 500 m objective zone touches a handful of cells,
/// large enough that a 60 km SAM ring over a few hundred units is cheap.
pub const CELL_M: f64 = 2000.0;

type Cell = (i64, i64);

#[derive(Clone, Debug, Default)]
pub(crate) struct Grid {
    cells: HashMap<Cell, Vec<Eid>>,
    at: HashMap<Eid, Cell>,
}

fn cell_of(p: MapPoint) -> Option<Cell> {
    // `as` saturates; positions are bounded by the theatre anyway.
    p.is_finite().then(|| {
        (
            (p.north / CELL_M).floor() as i64,
            (p.east / CELL_M).floor() as i64,
        )
    })
}

fn cell_bounds((n, e): Cell) -> Bounds {
    let min = MapPoint::new(n as f64 * CELL_M, e as f64 * CELL_M);
    Bounds {
        min,
        max: min.offset(CELL_M, CELL_M),
    }
}

impl Grid {
    /// Places `eid` at `p` (or removes it from the grid when `p` is `None` or not finite).
    pub(crate) fn set(&mut self, eid: Eid, p: Option<MapPoint>) {
        let new = p.and_then(cell_of);
        let old = self.at.get(&eid).copied();
        if old == new {
            return;
        }
        if let Some(c) = old {
            self.detach(eid, c);
        }
        match new {
            Some(c) => {
                self.cells.entry(c).or_default().push(eid);
                self.at.insert(eid, c);
            }
            None => {
                self.at.remove(&eid);
            }
        }
    }

    pub(crate) fn remove(&mut self, eid: Eid) {
        if let Some(c) = self.at.remove(&eid) {
            self.detach(eid, c);
        }
    }

    fn detach(&mut self, eid: Eid, c: Cell) {
        if let Some(list) = self.cells.get_mut(&c) {
            if let Some(i) = list.iter().position(|e| *e == eid) {
                list.swap_remove(i);
            }
            if list.is_empty() {
                self.cells.remove(&c);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.cells.clear();
        self.at.clear();
    }

    /// Calls `visit(eid, certainly_inside)` for every entity whose cell overlaps `shape`'s
    /// bounding box; `certainly_inside` is true when the whole cell lies inside the shape.
    /// Stops early when `visit` returns `false`. Visiting order is unspecified.
    pub(crate) fn visit(&self, shape: &ZoneShape<'_>, mut visit: impl FnMut(Eid, bool) -> bool) {
        let Some(b) = shape.bounds() else {
            return;
        };
        let (Some(lo), Some(hi)) = (cell_of(b.min), cell_of(b.max)) else {
            return;
        };
        let span = (hi.0 - lo.0 + 1).saturating_mul(hi.1 - lo.1 + 1);
        let mut on_cell = |c: Cell, list: &Vec<Eid>| {
            let full = shape.contains_bounds(&cell_bounds(c));
            list.iter().all(|e| visit(*e, full))
        };
        if span <= self.cells.len() as i64 {
            for n in lo.0..=hi.0 {
                for e in lo.1..=hi.1 {
                    if let Some(list) = self.cells.get(&(n, e))
                        && !on_cell((n, e), list)
                    {
                        return;
                    }
                }
            }
        } else {
            for (c, list) in &self.cells {
                if (lo.0..=hi.0).contains(&c.0)
                    && (lo.1..=hi.1).contains(&c.1)
                    && !on_cell(*c, list)
                {
                    return;
                }
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.at.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eid(i: u32) -> Eid {
        Eid::from_parts(i, 1).unwrap()
    }

    fn collect(g: &Grid, shape: ZoneShape<'_>) -> Vec<(Eid, bool)> {
        let mut out = Vec::new();
        g.visit(&shape, |e, full| {
            out.push((e, full));
            true
        });
        out.sort();
        out
    }

    #[test]
    fn moves_between_cells_and_queries_by_bounds() {
        let mut g = Grid::default();
        g.set(eid(1), Some(MapPoint::new(100.0, 100.0)));
        g.set(eid(2), Some(MapPoint::new(-100.0, -100.0)));
        g.set(eid(3), Some(MapPoint::new(50_000.0, 0.0)));
        g.set(eid(4), Some(MapPoint::new(f64::NAN, 0.0)));
        assert_eq!(g.len(), 3, "non-finite positions are not indexed");

        let small = ZoneShape::Circle {
            center: MapPoint::new(0.0, 0.0),
            radius_m: 500.0,
        };
        let hits: Vec<Eid> = collect(&g, small).into_iter().map(|(e, _)| e).collect();
        assert_eq!(hits, vec![eid(1), eid(2)]);

        g.set(eid(1), Some(MapPoint::new(50_100.0, 0.0)));
        let hits: Vec<Eid> = collect(&g, small).into_iter().map(|(e, _)| e).collect();
        assert_eq!(hits, vec![eid(2)]);

        // A huge zone over a sparse grid walks occupied cells; inner cells are "full".
        let huge = ZoneShape::Circle {
            center: MapPoint::new(0.0, 0.0),
            radius_m: 200_000.0,
        };
        let all = collect(&g, huge);
        assert_eq!(all.len(), 3);
        assert!(all.iter().all(|(_, full)| *full));

        g.remove(eid(2));
        g.set(eid(3), None);
        assert_eq!(g.len(), 1);
        assert!(collect(&g, small).is_empty());
    }

    #[test]
    fn early_exit() {
        let mut g = Grid::default();
        for i in 1..=10 {
            g.set(eid(i), Some(MapPoint::new(10.0 * f64::from(i), 0.0)));
        }
        let mut seen = 0;
        g.visit(
            &ZoneShape::Circle {
                center: MapPoint::default(),
                radius_m: 1000.0,
            },
            |_, _| {
                seen += 1;
                false
            },
        );
        assert_eq!(seen, 1);
    }
}
