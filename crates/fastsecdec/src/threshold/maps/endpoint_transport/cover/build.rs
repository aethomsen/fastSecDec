use super::{CoverLeaf, EndpointChartCover, HalfCube, SplitInterval};
use crate::{
    generation::coordinates_from_parts,
    threshold::maps::endpoint_transport::{
        EndpointTransport, Error, FaceSide, Limits, Result, Work,
        chart::{self, ChartImages, ValuationCheck},
        clearing::{Ring, pull},
        native, sector_error,
    },
};
use fastsecdec_sectors::{Decomposition, ParametricDomain, PolynomialSupport};
use std::{collections::BTreeSet, ops::ControlFlow, sync::Arc};
use symbolica::atom::{Atom, AtomCore, Symbol};

fn overflow() -> Error {
    Error::ResourceIncomplete("child cover inventory allowance".into())
}
fn retained(parent: &EndpointTransport, leaves: usize, halves: usize, n: usize) -> Result<usize> {
    let terms = parent.links.len().checked_mul(n).ok_or_else(overflow)?;
    let factors = parent
        .identities
        .len()
        .checked_mul(n.checked_add(2).ok_or_else(overflow)?)
        .ok_or_else(overflow)?;
    let per_leaf = terms
        .checked_add(factors)
        .and_then(|v| v.checked_add(n.checked_mul(4)?))
        .ok_or_else(overflow)?;
    leaves
        .checked_mul(per_leaf)
        .and_then(|v| v.checked_add(halves.checked_mul(n.checked_mul(3)?)?))
        .ok_or_else(overflow)
}
fn fresh(parent: &EndpointTransport, n: usize, work: &mut Work<'_>) -> Result<Vec<Symbol>> {
    let mut occupied = parent
        .parent
        .source()
        .decomposition()
        .occupied_symbols()
        .clone();
    occupied.extend(parent.target_units.iter().copied());
    occupied.extend(parent.parent.source().unit_coordinates().iter().copied());
    for root in &parent.roots {
        occupied.insert(root.section.root_symbol());
        occupied.extend(root.section.chart().parameters.iter().copied());
    }
    for namespace in 0usize.. {
        work.check()?;
        let symbols = (0..n)
            .map(|i| symbolica::symbol!(format!("fastsecdec::endpoint_cover_{namespace}::u{i}")))
            .collect::<Vec<_>>();
        if symbols.iter().all(|s| !occupied.contains(s)) {
            return Ok(symbols);
        }
    }
    unreachable!("finite occupied inventory")
}
fn geometry(
    parent: &EndpointTransport,
    units: &[Symbol],
    split: &[usize],
    ancestor_images: &[Atom],
    work: &mut Work<'_>,
) -> Result<(Vec<PolynomialSupport>, Vec<usize>, Decomposition)> {
    let ring = Ring::new(
        units
            .iter()
            .copied()
            .chain(parent.roots.iter().map(|r| r.section.root_symbol()))
            .collect(),
    )?;
    let selected = split.iter().map(|&i| units[i]).collect::<Vec<_>>();
    let ancestor = parent.parent.source().unit_coordinates();
    let mut supports = Vec::new();
    let mut indices = Vec::with_capacity(parent.identities.len());
    for identity in &parent.identities {
        work.check()?;
        // Include ALL old numerator monomial content. This exact same
        // transformed numerator supplies the later native-valuation comparison.
        let numerator = pull(identity.numerator(), ancestor, ancestor_images);
        let p = ring.polynomial(&numerator, work)?;
        let support =
            crate::parametric::polynomial_support(&p.to_expression(), &selected).map_err(native)?;
        let i = if let Some(i) = supports.iter().position(|s| s == &support) {
            i
        } else {
            supports.push(support);
            supports.len() - 1
        };
        indices.push(i);
    }
    let mut failure = None;
    let options = work.limits.sectors.clone();
    let result =
        fastsecdec_sectors::decompose(ParametricDomain::UnitCube, &supports, &options, |_| {
            match work.check() {
                Ok(()) => ControlFlow::Continue(()),
                Err(e) => {
                    failure = Some(e);
                    ControlFlow::Break(())
                }
            }
        });
    if let Some(e) = failure {
        return Err(e);
    }
    Ok((supports, indices, result.map_err(sector_error)?))
}
pub(super) fn prepare(
    parent: Arc<EndpointTransport>,
    parent_chart: usize,
    axes: &[Symbol],
    masks: &[usize],
    limits: &Limits,
    progress: &mut dyn FnMut() -> ControlFlow<()>,
) -> Result<Arc<EndpointChartCover>> {
    let mut work = Work {
        limits,
        budget: super::super::Budget::new(limits.algebra.clone()),
        callback: progress,
    };
    work.check()?;
    let original = parent
        .charts
        .get(parent_chart)
        .ok_or_else(|| Error::Invalid("parent chart ordinal out of range".into()))?;
    if axes.is_empty() {
        return Err(Error::Unsupported(
            "child cover requires nonempty split axes".into(),
        ));
    }
    if axes.iter().collect::<BTreeSet<_>>().len() != axes.len() {
        return Err(Error::Invalid("duplicate child split axis".into()));
    }
    let split = axes
        .iter()
        .map(|s| {
            parent
                .target_units
                .iter()
                .position(|u| u == s)
                .ok_or_else(|| {
                    Error::Invalid("child split axis is not a parent target role".into())
                })
        })
        .collect::<Result<Vec<_>>>()?;
    let count = 1usize
        .checked_shl(u32::try_from(axes.len()).map_err(|_| overflow())?)
        .ok_or_else(overflow)?;
    let refined = masks.iter().copied().collect::<BTreeSet<_>>();
    if refined.len() != masks.len() || masks.iter().any(|&m| m >= count) {
        return Err(Error::Invalid(
            "duplicate or out-of-range refined half mask".into(),
        ));
    }
    let n = parent.target_units.len();
    work.entries(retained(&parent, count, count, n)?)?;
    let targets = fresh(&parent, n, &mut work)?;
    let half_jacobian = Atom::num(2).pow(-(axes.len() as i64));
    let mut halves = Vec::with_capacity(count);
    let mut leaves = Vec::new();
    for mask in 0..count {
        work.check()?;
        let intervals = split
            .iter()
            .enumerate()
            .map(|(j, &axis)| SplitInterval {
                unit: parent.target_units[axis],
                axis,
                half: if mask & (1 << j) == 0 {
                    FaceSide::Lower
                } else {
                    FaceSide::Upper
                },
            })
            .collect::<Vec<_>>();
        let mut images = targets.iter().copied().map(Atom::var).collect::<Vec<_>>();
        for interval in &intervals {
            let half = Atom::var(targets[interval.axis]) / Atom::num(2);
            images[interval.axis] = if interval.half == FaceSide::Lower {
                half
            } else {
                Atom::one() - half
            };
        }
        let ancestor_images = original
            .unit_images()
            .iter()
            .map(|p| pull(p, &parent.target_units, &images))
            .collect::<Vec<_>>();
        let (supports, support_indices, fan) = if refined.contains(&mask) {
            let (s, i, g) = geometry(&parent, &targets, &split, &ancestor_images, &mut work)?;
            (s, i, Some(g))
        } else {
            (Vec::new(), Vec::new(), None)
        };
        let leaf_count = fan.as_ref().map_or(1, |g| g.sectors.len());
        let total = leaves.len().checked_add(leaf_count).ok_or_else(overflow)?;
        work.entries(retained(&parent, total, count, n)?)?;
        let first_leaf = leaves.len();
        for native_index in 0..leaf_count {
            work.check()?;
            let mut by = targets.iter().copied().map(Atom::var).collect::<Vec<_>>();
            let mut added = half_jacobian.clone();
            let valuation = if let Some(g) = &fan {
                let map = &g.sectors[native_index];
                map.validate(ParametricDomain::UnitCube).map_err(native)?;
                if map.source_dimension() != split.len()
                    || map.dimension() != split.len()
                    || map.factor_valuations.len() != supports.len()
                    || map.fixed_parameter.is_some()
                {
                    return Err(Error::Invalid("child native fan inventory differs".into()));
                }
                let selected = split.iter().map(|&i| targets[i]).collect::<Vec<_>>();
                let coordinates =
                    coordinates_from_parts(&selected, ParametricDomain::UnitCube, map, &selected);
                for (&i, p) in split.iter().zip(coordinates.images()) {
                    by[i] = p.clone();
                }
                added *= coordinates.measure_jacobian();
                Some(ValuationCheck {
                    selected: &split,
                    support_indices: &support_indices,
                    map,
                })
            } else {
                None
            };
            let child_images = images
                .iter()
                .map(|p| pull(p, &targets, &by))
                .collect::<Vec<_>>();
            let unit_images = original
                .unit_images()
                .iter()
                .map(|p| pull(p, &parent.target_units, &child_images))
                .collect::<Vec<_>>();
            let extra = pull(
                original.extra_jacobian(),
                &parent.target_units,
                &child_images,
            ) * &added;
            let chart = chart::prepare_images(
                &parent,
                ChartImages {
                    ordinal: leaves.len(),
                    target_units: &targets,
                    unit_images,
                    extra_jacobian: extra,
                    valuation,
                },
                &mut work,
            )?;
            leaves.push(CoverLeaf {
                half: mask,
                native_sector: fan.as_ref().map(|_| native_index),
                parent_chart_images: child_images,
                added_jacobian: added,
                chart,
            });
        }
        halves.push(HalfCube {
            mask,
            orientation: if mask.count_ones() % 2 == 0 { 1 } else { -1 },
            intervals,
            images,
            supports,
            support_indices,
            geometry: fan,
            first_leaf,
            leaf_count,
        });
    }
    work.check()?;
    Ok(Arc::new(EndpointChartCover {
        parent,
        parent_chart,
        split_axes: split,
        target_units: targets,
        halves,
        leaves,
    }))
}
