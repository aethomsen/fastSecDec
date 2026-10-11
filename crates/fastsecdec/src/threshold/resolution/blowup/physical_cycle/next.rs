//! A new ambient companion cycle through an actual source-open and physical map.
//! These local receipts do not glue centers or cover the real density domain.
use super::super::super::*;
use super::super::{helpers::Result, *};
use super::{AmbientResidualDrop, OriginalGeneratorPullback};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AmbientContinuationCenter {
    drop: Arc<AmbientResidualDrop>,
    center: Arc<CompanionCenter>,
    source_open: Arc<LocalizedCoverHistory>,
}
impl AmbientContinuationCenter {
    pub fn prepare(
        drop: Arc<AmbientResidualDrop>,
        center: Arc<CompanionCenter>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let cycle = center
            .coefficient()
            .coefficient()
            .cycle()
            .ok_or(Error::Invalid("ambient continuation missing new cycle"))?;
        if !matches!(cycle.origin(),CycleOrigin::AfterAmbientDrop(d)if Arc::ptr_eq(d,&drop))
            || !Arc::ptr_eq(center.coefficient().source().order(), drop.current())
        {
            return Err(Error::Invalid("ambient continuation actual drop/companion"));
        }
        let source_open = center.coefficient().source().restriction().clone();
        let prior = drop.inventory().carry().chart();
        if !Arc::ptr_eq(source_open.open().source(), prior.history())
            || !Arc::ptr_eq(center.frame(), source_open.history().ledger().frame())
            || source_open.history().births() != prior.history().births()
            || !source_open.history().same_root(prior.history())
        {
            return Err(Error::Invalid(
                "ambient continuation actual physical localization",
            ));
        }
        let expected = source_open
            .open()
            .extension()
            .ideal(prior.parent().target().ideal(), b)?;
        if center.parent_source().mark() != prior.parent().target().mark()
            || center.parent_source().ideal() != &expected
        {
            return Err(Error::Invalid(
                "ambient continuation original source restriction",
            ));
        }
        Ok(Arc::new(Self {
            drop,
            center,
            source_open,
        }))
    }
    pub fn residual_drop(&self) -> &Arc<AmbientResidualDrop> {
        &self.drop
    }
    pub fn center(&self) -> &Arc<CompanionCenter> {
        &self.center
    }
    /// This open is from an actual marked-cosupport cover, not a full density
    /// cover. The complete upper cover remains in the retained residual order.
    pub fn source_open(&self) -> &Arc<LocalizedCoverHistory> {
        &self.source_open
    }
}

#[derive(Clone, Debug)]
pub struct AmbientNextChart {
    owner: Arc<AmbientContinuationCenter>,
    chart: Arc<RelativeRecursiveChart>,
    generators: Vec<OriginalGeneratorPullback>,
    original_coordinate_jacobian: Poly,
}
impl AmbientNextChart {
    pub fn prepare(
        owner: Arc<AmbientContinuationCenter>,
        chart: Arc<RelativeRecursiveChart>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !matches!(chart.geometry().center().origin(),RecursiveCenterOrigin::Companion(c)if Arc::ptr_eq(c,owner.center()))
            || !Arc::ptr_eq(chart.parent().source(), owner.center().parent_source())
            || !Arc::ptr_eq(
                chart.geometry().center().history(),
                owner.source_open().history(),
            )
        {
            return Err(Error::Invalid("ambient next chart actual checked center"));
        }
        let carry = owner.drop.inventory().carry();
        let extension = owner.source_open().open().extension();
        let pull = |p: &Poly, b: &mut Budget| chart.geometry().pull(&extension.pull(p, b)?, b);
        let local = chart.geometry().frame().local();
        let exceptional = match chart.geometry().exceptional() {
            Some(e) => b.power(e, chart.parent().source().mark())?,
            None => local.ring().one(),
        };
        b.reserve_slots(carry.generators().len())?;
        let mut generators = Vec::new();
        for original in carry.generators() {
            let restricted = extension.pull(original.current_quotient(), b)?;
            let current = chart
                .parent()
                .generators()
                .iter()
                .find(|g| g.source == restricted)
                .ok_or(Error::Invalid(
                    "ambient next original generator restriction",
                ))?;
            let total = pull(original.total(), b)?;
            let old_factor = pull(original.exceptional_factor(), b)?;
            let factor = b.mul(&old_factor, &exceptional)?;
            generators.push(OriginalGeneratorPullback::checked(
                original.source().clone(),
                total,
                factor,
                current.quotient.clone(),
                local,
                b,
            )?);
        }
        let previous_jacobian = pull(carry.original_coordinate_jacobian(), b)?;
        let original_coordinate_jacobian = b.mul(
            &previous_jacobian,
            chart.geometry().original_coordinate_jacobian(),
        )?;
        Ok(Arc::new(Self {
            owner,
            chart,
            generators,
            original_coordinate_jacobian,
        }))
    }
    pub fn owner(&self) -> &Arc<AmbientContinuationCenter> {
        &self.owner
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn generators(&self) -> &[OriginalGeneratorPullback] {
        &self.generators
    }
    pub fn original_coordinate_jacobian(&self) -> &Poly {
        &self.original_coordinate_jacobian
    }
}
