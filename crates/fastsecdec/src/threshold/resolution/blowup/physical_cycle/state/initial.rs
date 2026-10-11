//! Direct source-bound entrypoint, before its first actual physical pivot.
use super::*;
#[derive(Clone, Debug)]
pub struct OriginalPhysicalProblem {
    root: Arc<OriginalPhysicalRoot>,
}
impl OriginalPhysicalProblem {
    pub fn new(center: Arc<CompanionCenter>, b: &mut Budget) -> Result<Arc<Self>> {
        let owner = EmbeddedChildCycle::new(center.clone(), b)?;
        let history = center
            .coefficient()
            .source()
            .restriction()
            .history()
            .clone();
        if !Arc::ptr_eq(center.frame(), history.ledger().frame()) {
            return Err(Error::Invalid("initial physical frame/history"));
        }
        b.reserve_slots(center.parent_source().ideal().generators().len())?;
        Ok(Arc::new(Self {
            root: Arc::new(OriginalPhysicalRoot {
                owner,
                source: center.parent_source().clone(),
                history,
            }),
        }))
    }
    pub fn root(&self) -> &Arc<OriginalPhysicalRoot> {
        &self.root
    }
    pub fn center(&self) -> &Arc<CompanionCenter> {
        self.root.owner().center()
    }
    pub fn advance(
        self: &Arc<Self>,
        chart: Arc<RelativeRecursiveChart>,
        b: &mut Budget,
    ) -> Result<Arc<OriginalPhysicalState>> {
        if !matches!(chart.geometry().center().origin(),RecursiveCenterOrigin::Companion(c)if Arc::ptr_eq(c,self.center()))
            || !Arc::ptr_eq(chart.parent().source(), self.root.source())
            || !Arc::ptr_eq(chart.geometry().center().history(), self.root.history())
        {
            return Err(Error::Invalid("initial physical actual center/source"));
        }
        let exceptional = AccumulatedExceptional::first(&chart, b)?;
        b.reserve_slots(chart.parent().generators().len())?;
        let mut generators = Vec::new();
        for g in chart.parent().generators() {
            let total = chart.geometry().pull(&g.source, b)?;
            if !chart
                .geometry()
                .frame()
                .local()
                .zero(&(&total - &g.total), b)?
            {
                return Err(Error::Invalid("initial physical source pullback"));
            }
            generators.push(OriginalGeneratorPullback::checked(
                g.source.clone(),
                total,
                exceptional.product().clone(),
                g.quotient.clone(),
                chart.geometry().frame().local(),
                b,
            )?);
        }
        let jacobian = chart.geometry().original_coordinate_jacobian().clone();
        Ok(Arc::new(OriginalPhysicalState {
            root: self.root.clone(),
            edge: PhysicalLineageEdge::Initial {
                problem: self.clone(),
                chart: chart.clone(),
            },
            chart: chart.clone(),
            generators,
            exceptional,
            jacobian,
            active: ActivePhysicalCycle::First {
                origin: self.root.owner().clone(),
                chart,
            },
            retired: None,
            depth: 1,
        }))
    }
}
