//! A checked target principal open composes the existing physical blowup map.
//! It never introduces another blowup or repeats an exceptional birth.
use super::*;
#[derive(Clone, Debug)]
pub(super) struct RestrictedGeometry {
    pub parent: Arc<RelativeBlowupGeometry>,
    pub open: Arc<LocalizedCoverHistory>,
    pub exceptional: Option<Poly>,
    pub jacobian: Poly,
    pub original_coordinate_jacobian: Poly,
}
impl RelativeBlowupGeometry {
    pub fn target_restriction(
        &self,
    ) -> Option<(&Arc<RelativeBlowupGeometry>, &Arc<LocalizedCoverHistory>)> {
        self.restriction.as_ref().map(|r| (&r.parent, &r.open))
    }
}
/// One local chart only. The caller must retain its entire issued cover.
pub fn restrict_relative_chart(
    original: Arc<RelativeRecursiveChart>,
    open: Arc<LocalizedCoverHistory>,
    b: &mut Budget,
) -> Result<Arc<RelativeRecursiveChart>> {
    if !Arc::ptr_eq(original.parent().geometry(), original.geometry())
        || !Arc::ptr_eq(original.geometry().ledger(), original.history().ledger())
        || !Arc::ptr_eq(open.open().source(), original.history())
        || !Arc::ptr_eq(
            open.open().source().ledger().frame(),
            original.geometry().frame(),
        )
        || !Arc::ptr_eq(
            open.open().cover().algebra(),
            original.geometry().frame().local(),
        )
        || !open.history().same_root(original.history())
        || open.history().stage() != original.history().stage()
        || open.history().births() != original.history().births()
        || open.history().old_snapshot() != original.history().old_snapshot()
    {
        return Err(Error::Invalid("post-blowup target open owner/history"));
    }
    let extension = open.open().extension();
    let local = open.history().ledger().frame().local();
    let exceptional = original
        .geometry()
        .exceptional()
        .map(|e| extension.pull(e, b))
        .transpose()?;
    let jacobian = extension.pull(original.geometry().jacobian(), b)?;
    let original_coordinate_jacobian =
        extension.pull(original.geometry().original_coordinate_jacobian(), b)?;
    b.reserve_slots(original.geometry().divisors().len())?;
    let mut divisors = Vec::new();
    for old in original.geometry().divisors() {
        let total = extension.pull(&old.total, b)?;
        let strict_equation = extension.pull(&old.strict_equation, b)?;
        let unit = extension.pull(&old.unit, b)?;
        // Principal localization keeps its pulled ledger, including factors
        // that become units. Absence means an omitted ledger entry, not merely
        // an empty divisor locus on this open.
        let absent = old.absent;
        if absent && !super::super::helpers::unit(local, &strict_equation, b)? {
            return Err(Error::Invalid("restricted absent divisor is not a unit"));
        }
        let factor = if old.exceptional_power == 0 {
            local.ring().one()
        } else {
            b.power(
                exceptional
                    .as_ref()
                    .ok_or(Error::Invalid("restricted strict divisor exceptional"))?,
                old.exceptional_power,
            )?
        };
        let strict_total = b.mul(&factor, &strict_equation)?;
        let recombined = b.mul(&strict_total, &unit)?;
        if !local.zero(&(&total - &recombined), b)? {
            return Err(Error::Invalid("restricted strict-divisor recombination"));
        }
        let active = open
            .history()
            .ledger()
            .divisors()
            .iter()
            .find(|d| d.id == old.id);
        if absent != active.is_none() || active.is_some_and(|d| d.equation != strict_equation) {
            return Err(Error::Invalid("restricted strict-divisor ledger"));
        }
        divisors.push(DivisorReceipt {
            id: old.id,
            total,
            exceptional_power: old.exceptional_power,
            strict_equation,
            unit,
            absent,
        });
    }
    let geometry = Arc::new(RelativeBlowupGeometry {
        center: original.geometry.center.clone(),
        open: original.geometry.open.clone(),
        geometry: original.geometry.geometry.clone(),
        restriction: Some(Arc::new(RestrictedGeometry {
            parent: original.geometry.clone(),
            open: open.clone(),
            exceptional,
            jacobian,
            original_coordinate_jacobian,
        })),
        ledger: open.history().ledger().clone(),
        divisors,
        born: original.geometry.born,
    });
    let source = original.parent.source.clone();
    let target = Arc::new(MarkedIdeal::new(
        extension.ideal(original.parent.target.ideal(), b)?,
        original.parent.target.mark(),
        b,
    )?);
    let factor = geometry
        .exceptional()
        .map(|e| b.power(e, source.mark()))
        .transpose()?
        .unwrap_or_else(|| local.ring().one());
    b.reserve_slots(original.parent.generators.len())?;
    let mut generators = Vec::new();
    for g in &original.parent.generators {
        let total = extension.pull(&g.total, b)?;
        let quotient = extension.pull(&g.quotient, b)?;
        let recombined = b.mul(&factor, &quotient)?;
        let direct = geometry.pull(&g.source, b)?;
        if !local.zero(&(&total - &recombined), b)? || !local.zero(&(&direct - &total), b)? {
            return Err(Error::Invalid("restricted controlled-generator identity"));
        }
        generators.push(ControlledGenerator {
            source: g.source.clone(),
            total,
            quotient,
        });
    }
    let parent = Arc::new(RelativeMarkedTransform {
        geometry: geometry.clone(),
        source,
        target,
        generators,
    });
    Ok(Arc::new(RelativeRecursiveChart {
        geometry,
        history: open.history().clone(),
        parent,
    }))
}
#[cfg(test)]
mod tests;
