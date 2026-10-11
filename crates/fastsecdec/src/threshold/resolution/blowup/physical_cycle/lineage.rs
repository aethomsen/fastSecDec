//! The actual original marked source through its accepted physical maps.
//! Current scope: original coefficient ascent followed by supported continuation.
use super::super::super::*;
use super::super::{general_transform::controlled_marked, helpers::Result};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct OriginalGeneratorPullback {
    source: Poly,
    total: Poly,
    exceptional_factor: Poly,
    current_quotient: Poly,
}
impl OriginalGeneratorPullback {
    pub(super) fn checked(
        source: Poly,
        total: Poly,
        exceptional_factor: Poly,
        current_quotient: Poly,
        local: &LocalizedAlgebra,
        b: &mut Budget,
    ) -> Result<Self> {
        let expected = b.mul(&exceptional_factor, &current_quotient)?;
        if !local.zero(&(&total - &expected), b)? {
            return Err(Error::Invalid("original generator physical recombination"));
        }
        Ok(Self {
            source,
            total,
            exceptional_factor,
            current_quotient,
        })
    }

    pub fn source(&self) -> &Poly {
        &self.source
    }
    pub fn total(&self) -> &Poly {
        &self.total
    }
    pub fn exceptional_factor(&self) -> &Poly {
        &self.exceptional_factor
    }
    pub fn current_quotient(&self) -> &Poly {
        &self.current_quotient
    }
}
#[derive(Clone, Debug)]
pub struct OriginalCompanionPullback {
    presentation: Arc<EmbeddedPresentation>,
    transform: Arc<RelativeMarkedTransform>,
    cosupport: Arc<MarkedCosupport>,
}
impl OriginalCompanionPullback {
    pub fn presentation(&self) -> &Arc<EmbeddedPresentation> {
        &self.presentation
    }
    pub fn transform(&self) -> &Arc<RelativeMarkedTransform> {
        &self.transform
    }
    pub fn cosupport(&self) -> &Arc<MarkedCosupport> {
        &self.cosupport
    }
}
#[derive(Clone, Debug)]
pub struct AmbientCycleCarry {
    exceptional: super::AccumulatedExceptional,
    center: Arc<ContinuedSupportedCenter>,
    original: Arc<EmbeddedChildCycle>,
    previous: Arc<RelativeRecursiveChart>,
    chart: Arc<RelativeRecursiveChart>,
    generators: Vec<OriginalGeneratorPullback>,
    companions: Vec<OriginalCompanionPullback>,
    original_coordinate_jacobian: Poly,
}
impl AmbientCycleCarry {
    pub fn prepare(
        center: Arc<ContinuedSupportedCenter>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !matches!(chart.geometry().center().origin(),RecursiveCenterOrigin::SupportedProblem(c)if Arc::ptr_eq(c,&center))
        {
            return Err(Error::Invalid("ambient cycle actual physical center"));
        }
        let previous = center.as_ref().drop().inventory().prior().chart().clone();
        if !Arc::ptr_eq(chart.parent().source(), previous.parent().target())
            || !Arc::ptr_eq(chart.geometry().center().history(), previous.history())
        {
            return Err(Error::Invalid("ambient cycle original source chain"));
        }
        let original = center
            .ancestors()
            .first()
            .ok_or(Error::Invalid("ambient cycle missing original ancestor"))?
            .original_cycle()
            .clone();
        if !center.ancestors().iter().all(|a| {
            Arc::ptr_eq(a.original_cycle(), &original) && Arc::ptr_eq(a.chart(), &previous)
        }) {
            return Err(Error::ResourceIncomplete(
                "ambient cycle needs common original parent hierarchy",
            ));
        }
        if !matches!(previous.geometry().center().origin(),RecursiveCenterOrigin::OriginalProblem(c)if Arc::ptr_eq(c.origin(),&original))
        {
            return Err(Error::ResourceIncomplete(
                "ambient cycle needs general original physical ancestry",
            ));
        }
        if !Arc::ptr_eq(
            original.center().parent_source(),
            previous.parent().source(),
        ) || previous.parent().source().mark() != chart.parent().target().mark()
        {
            return Err(Error::Invalid("ambient cycle original marked input"));
        }
        let exceptional = super::AccumulatedExceptional::prepare(&previous, &chart, b)?;
        let local = chart.geometry().frame().local();
        let factor = exceptional.product().clone();
        b.reserve_slots(previous.parent().generators().len())?;
        let mut generators = Vec::new();
        for g in previous.parent().generators() {
            let first_total = previous.geometry().pull(&g.source, b)?;
            if !previous
                .geometry()
                .frame()
                .local()
                .zero(&(&first_total - &g.total), b)?
            {
                return Err(Error::Invalid("ambient cycle original first pullback"));
            }
            let next = chart
                .parent()
                .generators()
                .iter()
                .find(|n| n.source == g.quotient)
                .ok_or(Error::Invalid(
                    "ambient cycle original generator association",
                ))?;
            let total = chart.geometry().pull(&first_total, b)?;
            generators.push(OriginalGeneratorPullback::checked(
                g.source.clone(),
                total,
                factor.clone(),
                next.quotient.clone(),
                local,
                b,
            )?);
        }
        if generators.len() != previous.parent().source().ideal().generators().len() {
            return Err(Error::Invalid("ambient cycle original generator inventory"));
        }
        let first_jacobian = chart
            .geometry()
            .pull(previous.geometry().original_coordinate_jacobian(), b)?;
        let original_coordinate_jacobian = b.mul(
            &first_jacobian,
            chart.geometry().original_coordinate_jacobian(),
        )?;
        b.reserve_slots(center.ancestors().len())?;
        let mut companions = Vec::new();
        for (i, presentation) in center.ancestors().iter().enumerate() {
            let original_g = original.center().coefficient().contact().source();
            if !Arc::ptr_eq(presentation.companion().geometry(), previous.geometry())
                || presentation.companion().source().mark() != original_g.algebraic_maximum_order()
                || presentation.companion().source().ideal() != original_g.source().as_ref()
            {
                return Err(Error::Invalid(
                    "ambient cycle original companion construction",
                ));
            }

            let transform = controlled_marked(
                chart.geometry().clone(),
                presentation.companion().target().clone(),
                &format!("{namespace}_G{i}"),
                b,
            )?;
            let cosupport = MarkedCosupport::prove(
                chart.geometry().frame().clone(),
                transform.target().clone(),
                b,
            )?;
            companions.push(OriginalCompanionPullback {
                presentation: presentation.clone(),
                transform,
                cosupport,
            });
        }
        Ok(Arc::new(Self {
            exceptional,
            center,
            original,
            previous,
            chart,
            generators,
            companions,
            original_coordinate_jacobian,
        }))
    }
    pub fn exceptional(&self) -> &super::AccumulatedExceptional {
        &self.exceptional
    }
    pub fn center(&self) -> &Arc<ContinuedSupportedCenter> {
        &self.center
    }
    pub fn original(&self) -> &Arc<EmbeddedChildCycle> {
        &self.original
    }
    pub fn previous(&self) -> &Arc<RelativeRecursiveChart> {
        &self.previous
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn original_source(&self) -> &Arc<MarkedIdeal> {
        self.previous.parent().source()
    }
    pub fn generators(&self) -> &[OriginalGeneratorPullback] {
        &self.generators
    }
    pub fn companions(&self) -> &[OriginalCompanionPullback] {
        &self.companions
    }
    pub fn original_coordinate_jacobian(&self) -> &Poly {
        &self.original_coordinate_jacobian
    }
}
