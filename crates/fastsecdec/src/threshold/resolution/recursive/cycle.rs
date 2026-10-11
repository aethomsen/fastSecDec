use super::*;

/// A cycle's fixed old exceptional set, separate from geometric birth history.
/// A genuine construction event may start its first cycle; a subsequent cycle
/// on the same marked problem requires an actual proved residual-order drop.
#[derive(Clone, Debug)]
pub struct CycleSnapshot {
    origin: CycleOrigin,
    history: Arc<ResolutionHistory>,
    old: Vec<BoundaryId>,
}
#[derive(Clone, Debug)]
pub enum CycleOrigin {
    AfterResidualDrop(Arc<ResidualDrop>),
    AfterSupportedDrop(Arc<SupportedResidualDrop>),
    AfterAmbientDrop(Arc<AmbientResidualDrop>),
    InitialProblem {
        problem: Arc<InitialProblem>,
        order: Arc<ComponentResidualOrder>,
    },
}
impl CycleSnapshot {
    pub(crate) fn initial(
        problem: Arc<InitialProblem>,
        order: Arc<ComponentResidualOrder>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        problem.check_order(&order)?;
        if order.algebraic_maximum_on_cosupport() == 0 {
            return Err(Error::Invalid(
                "initial monomial problem needs no positive companion",
            ));
        }
        let history = problem.history().clone();
        b.reserve_slots(history.ledger().divisors().len())?;
        let old = history.ledger().divisors().iter().map(|d| d.id).collect();
        Ok(Arc::new(Self {
            origin: CycleOrigin::InitialProblem { problem, order },
            history,
            old,
        }))
    }
    pub fn after_first_drop(origin: Arc<ResidualDrop>, b: &mut Budget) -> Result<Arc<Self>> {
        if origin.current().algebraic_maximum_on_cosupport() == 0 {
            return Err(Error::Invalid(
                "zero residual maximum uses monomial terminal",
            ));
        }
        let history = origin.current().factor().data().history().clone();
        b.reserve_slots(history.ledger().divisors().len())?;
        let old = history.ledger().divisors().iter().map(|d| d.id).collect();
        Ok(Arc::new(Self {
            origin: CycleOrigin::AfterResidualDrop(origin),
            history,
            old,
        }))
    }
    pub fn after_supported_drop(
        origin: Arc<SupportedResidualDrop>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if origin.inventory().proved_maximum().is_none()
            || origin.current().algebraic_maximum_on_cosupport() == 0
        {
            return Err(Error::Invalid(
                "supported cycle missing complete positive drop",
            ));
        }
        let history = origin.current().factor().data().history().clone();
        b.reserve_slots(history.ledger().divisors().len())?;
        let old = history.ledger().divisors().iter().map(|d| d.id).collect();
        Ok(Arc::new(Self {
            origin: CycleOrigin::AfterSupportedDrop(origin),
            history,
            old,
        }))
    }
    pub fn after_ambient_drop(
        origin: Arc<AmbientResidualDrop>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if origin.inventory().proved_maximum().is_none()
            || origin.current().algebraic_maximum_on_cosupport() == 0
        {
            return Err(Error::Invalid("ambient cycle needs complete positive drop"));
        }
        let history = origin.current().factor().data().history().clone();
        b.reserve_slots(history.ledger().divisors().len())?;
        let old = history.ledger().divisors().iter().map(|d| d.id).collect();
        Ok(Arc::new(Self {
            origin: CycleOrigin::AfterAmbientDrop(origin),
            history,
            old,
        }))
    }
    pub fn origin(&self) -> &CycleOrigin {
        &self.origin
    }
    pub fn birth_history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn old_ids(&self) -> &[BoundaryId] {
        &self.old
    }
    pub(crate) fn check_history(&self, history: &ResolutionHistory) -> Result<()> {
        if !history.same_root(&self.history)
            || !history.chart_path().starts_with(self.history.chart_path())
            || self
                .old
                .iter()
                .any(|id| history.births().get(id) != self.history.births().get(id))
        {
            return Err(Error::Invalid("cycle snapshot geometric ancestry"));
        }
        Ok(())
    }
    pub(crate) fn check_open(&self, open: &CompanionContactOpen) -> Result<()> {
        let order = match &self.origin {
            CycleOrigin::AfterResidualDrop(d) => d.current(),
            CycleOrigin::AfterSupportedDrop(d) => d.current(),
            CycleOrigin::AfterAmbientDrop(d) => d.current(),
            CycleOrigin::InitialProblem { order, .. } => order,
        };
        if !Arc::ptr_eq(open.order(), order) {
            return Err(Error::Invalid("cycle snapshot companion source owner"));
        }
        self.check_history(open.restriction().history())
    }
}
