use super::super::*;
use super::{
    general::*,
    helpers::*,
    standard::{StandardGeometry, standard_chart},
};
use std::sync::Arc;
mod restricted;
use restricted::RestrictedGeometry;
pub use restricted::restrict_relative_chart;

#[derive(Clone, Debug)]
pub struct RelativeBlowupGeometry {
    center: Arc<CheckedRecursiveCenter>,
    open: Arc<RecursiveAdaptedOpen>,
    geometry: Arc<StandardGeometry>,
    restriction: Option<Arc<RestrictedGeometry>>,
    ledger: Arc<VerifiedRelativeSnc>,
    divisors: Vec<DivisorReceipt>,
    born: Option<(BoundaryId, u64)>,
}
impl RelativeBlowupGeometry {
    pub fn center(&self) -> &Arc<CheckedRecursiveCenter> {
        &self.center
    }
    pub fn open(&self) -> &Arc<RecursiveAdaptedOpen> {
        &self.open
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.restriction
            .as_ref()
            .map_or(&self.geometry.frame, |r| r.open.history().ledger().frame())
    }
    pub fn ledger(&self) -> &Arc<VerifiedRelativeSnc> {
        &self.ledger
    }
    pub fn divisors(&self) -> &[DivisorReceipt] {
        &self.divisors
    }
    pub fn born(&self) -> Option<(BoundaryId, u64)> {
        self.born
    }
    pub fn exceptional(&self) -> Option<&Poly> {
        self.restriction.as_ref().map_or_else(
            || self.geometry.exceptional.as_ref(),
            |r| r.exceptional.as_ref(),
        )
    }
    pub fn pivot_normal(&self) -> Option<usize> {
        self.geometry
            .pivot
            .and_then(|p| self.geometry.center_axes.iter().position(|a| *a == p))
    }
    pub fn jacobian(&self) -> &Poly {
        self.restriction
            .as_ref()
            .map_or(&self.geometry.jacobian, |r| &r.jacobian)
    }
    pub fn original_coordinate_jacobian(&self) -> &Poly {
        self.restriction
            .as_ref()
            .map_or(&self.geometry.original_coordinate_jacobian, |r| {
                &r.original_coordinate_jacobian
            })
    }
    pub fn pull(&self, p: &Poly, b: &mut Budget) -> Result<Poly> {
        match &self.restriction {
            Some(r) => r.open.open().extension().pull(&r.parent.pull(p, b)?, b),
            None => self.geometry.pull_from_source(p, b),
        }
    }
}
#[derive(Clone, Debug)]
pub struct RelativeMarkedTransform {
    geometry: Arc<RelativeBlowupGeometry>,
    source: Arc<MarkedIdeal>,
    target: Arc<MarkedIdeal>,
    generators: Vec<ControlledGenerator>,
}
impl RelativeMarkedTransform {
    pub fn geometry(&self) -> &Arc<RelativeBlowupGeometry> {
        &self.geometry
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn target(&self) -> &Arc<MarkedIdeal> {
        &self.target
    }
    pub fn generators(&self) -> &[ControlledGenerator] {
        &self.generators
    }
}
#[derive(Clone, Debug)]
pub struct RelativeRecursiveChart {
    geometry: Arc<RelativeBlowupGeometry>,
    history: Arc<ResolutionHistory>,
    parent: Arc<RelativeMarkedTransform>,
}
impl RelativeRecursiveChart {
    pub fn geometry(&self) -> &Arc<RelativeBlowupGeometry> {
        &self.geometry
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn parent(&self) -> &Arc<RelativeMarkedTransform> {
        &self.parent
    }
}
#[derive(Clone, Debug)]
pub struct RelativeRecursiveBlowup {
    cover: Arc<RecursiveAdaptation>,
    charts: Vec<Arc<RelativeRecursiveChart>>,
}
impl RelativeRecursiveBlowup {
    pub fn cover(&self) -> &Arc<RecursiveAdaptation> {
        &self.cover
    }
    pub fn charts(&self) -> &[Arc<RelativeRecursiveChart>] {
        &self.charts
    }
}
pub fn blowup_recursive_center(
    cover: Arc<RecursiveAdaptation>,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<RelativeRecursiveBlowup>> {
    let mut charts = Vec::new();
    for (oi, open) in cover.opens().iter().enumerate() {
        let pivots = if open.identity_side().is_some() {
            vec![None]
        } else {
            open.center_axes().iter().map(|a| Some(*a)).collect()
        };
        b.reserve_slots(pivots.len())?;
        for pivot in pivots {
            let ns = format!(
                "{namespace}_open{oi}_pivot{}",
                pivot.map_or_else(|| "identity".into(), |p| p.to_string())
            );
            let geometry = one(cover.center().clone(), open.clone(), pivot, &ns, b)?;
            let parent =
                controlled_marked(geometry.clone(), cover.center().source().clone(), &ns, b)?;
            let history = cover.center().history().advanced_relative(&geometry, b)?;
            charts.push(Arc::new(RelativeRecursiveChart {
                geometry,
                history,
                parent,
            }));
        }
    }
    Ok(Arc::new(RelativeRecursiveBlowup { cover, charts }))
}
fn one(
    center: Arc<CheckedRecursiveCenter>,
    open: Arc<RecursiveAdaptedOpen>,
    pivot: Option<usize>,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<RelativeBlowupGeometry>> {
    if !Arc::ptr_eq(&open.geometry.source, center.history().ledger().frame())
        || (open.identity_side().is_some() != pivot.is_none())
    {
        return Err(Error::Invalid("relative blowup source/open owner"));
    }
    let geometry = standard_chart(
        open.geometry.clone(),
        open.center_axes(),
        pivot,
        namespace,
        b,
    )?;
    let frame = &geometry.frame;
    let local = frame.local();
    let ring = local.ring();
    let born = if pivot.is_some() {
        Some(center.history().next_transition()?)
    } else {
        None
    };
    // Independently establish exceptional Cartier regularity before quotient
    // discovery. Units are regular too and retain the empty-exceptional case.
    if let Some(e) = &geometry.exceptional {
        let id = born.ok_or(Error::Invalid("missing relative birth"))?.0;
        match verify_initial_relative_snc(
            frame.clone(),
            vec![InitialDivisor {
                id,
                equation: e.clone(),
            }],
            b,
        )? {
            SncProduction::Verified(_) => {}
            SncProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
            SncProduction::Unresolved { .. } => {
                return Err(Error::Invalid("standard exceptional not relative Cartier"));
            }
        }
    }
    let mut active = Vec::new();
    let mut divisors = Vec::new();
    b.reserve_slots(center.history().ledger().divisors().len())?;
    for (i, d) in center.history().ledger().divisors().iter().enumerate() {
        let total = geometry.pull_from_source(&d.equation, b)?;
        let power = usize::from(pivot.is_some() && center.contained_boundaries()[i]);
        let strict = if power == 1 {
            let e = geometry
                .exceptional
                .as_ref()
                .ok_or(Error::Invalid("missing relative exceptional"))?;
            super::super::companion::divide_regular_in_localization(
                local,
                e,
                &total,
                &format!("{namespace}_strict{i}"),
                b,
            )?
            .0
        } else {
            total.clone()
        };
        let factor = if power == 1 {
            geometry.exceptional.as_ref().unwrap().clone()
        } else {
            ring.one()
        };
        if !local.zero(&(&total - &b.mul(&factor, &strict)?), b)? {
            return Err(Error::Invalid("relative old divisor recombination"));
        }
        let absent = unit(local, &strict, b)?;
        if !absent {
            active.push(InitialDivisor {
                id: d.id,
                equation: strict.clone(),
            });
        }
        divisors.push(DivisorReceipt {
            id: d.id,
            total,
            exceptional_power: power,
            strict_equation: strict,
            unit: ring.one(),
            absent,
        });
    }
    if let (Some(e), Some((id, _))) = (&geometry.exceptional, born)
        && !unit(local, e, b)?
    {
        active.push(InitialDivisor {
            id,
            equation: e.clone(),
        });
    }
    let ledger = match verify_initial_relative_snc(frame.clone(), active, b)? {
        SncProduction::Verified(v) => v,
        SncProduction::Incomplete { reason, .. } => return Err(Error::ResourceIncomplete(reason)),
        SncProduction::Unresolved { .. } => {
            return Err(Error::Invalid("relative transformed boundary not SNC"));
        }
    };
    Ok(Arc::new(RelativeBlowupGeometry {
        center,
        open,
        geometry,
        restriction: None,
        ledger,
        divisors,
        born,
    }))
}
/// Exact controlled transform in the checked ambient chart. Embedded-support
/// marked ideals need the separate strict-support owner before using division.
pub(crate) fn controlled_marked(
    geometry: Arc<RelativeBlowupGeometry>,
    source: Arc<MarkedIdeal>,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<RelativeMarkedTransform>> {
    if source.ideal().ring() != geometry.center.history().ledger().frame().local().ring() {
        return Err(Error::Invalid("relative controlled source ring"));
    }
    let local = geometry.frame().local();
    let ring = local.ring();
    let mut generators = Vec::new();
    b.reserve_slots(source.ideal().generators().len())?;
    for (i, f) in source.ideal().generators().iter().enumerate() {
        let total = geometry.pull(f, b)?;
        let mut quotient = total.clone();
        if let Some(e) = geometry.exceptional() {
            for j in 0..source.mark() {
                quotient = super::super::companion::divide_regular_in_localization(
                    local,
                    e,
                    &quotient,
                    &format!("{namespace}_marked{i}_{j}"),
                    b,
                )?
                .0;
            }
            let factor = b.power(e, source.mark())?;
            if !local.zero(&(&total - &b.mul(&factor, &quotient)?), b)? {
                return Err(Error::Invalid("relative controlled full recombination"));
            }
        }
        generators.push(ControlledGenerator {
            source: f.clone(),
            total,
            quotient,
        });
    }
    let target = Arc::new(MarkedIdeal::new(
        Ideal::new(
            ring.clone(),
            generators.iter().map(|g| g.quotient.clone()).collect(),
            b,
        )?,
        source.mark(),
        b,
    )?);
    Ok(Arc::new(RelativeMarkedTransform {
        geometry,
        source,
        target,
        generators,
    }))
}
