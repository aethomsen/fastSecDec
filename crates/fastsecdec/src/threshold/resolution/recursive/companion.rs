use super::*;

/// A construction/admissibility edge, not an equivalence between the original
/// parent and the old-incidence coefficient sum. No transform/reset authority.
#[derive(Clone, Debug)]
pub struct CompanionCenter {
    coefficient: Arc<CompanionCoefficientChart>,
    child: Arc<RecursiveCenter>,
    parent_source: Arc<MarkedIdeal>,
    ideal: Ideal,
    normals: Vec<Poly>,
    clearings: Vec<UnitClearing>,
}
impl CompanionCenter {
    pub fn coefficient(&self) -> &Arc<CompanionCoefficientChart> {
        &self.coefficient
    }
    pub fn child(&self) -> &Arc<RecursiveCenter> {
        &self.child
    }
    pub fn parent_source(&self) -> &Arc<MarkedIdeal> {
        &self.parent_source
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.coefficient.contact().source().frame()
    }
    pub fn ideal(&self) -> &Ideal {
        &self.ideal
    }
    pub fn normals(&self) -> &[Poly] {
        &self.normals
    }
    pub fn clearings(&self) -> &[UnitClearing] {
        &self.clearings
    }
    pub fn old_incidence(&self) -> usize {
        self.coefficient.coefficient().algebraic_maximum_old_count()
    }
}
#[derive(Clone, Debug)]
pub enum CompanionCenterOutcome {
    Pending,
    ChildComponentOpensRequired(Arc<RecursiveComponentCover>),
    Center(Arc<CompanionCenter>),
    ChildNeedsMoreOpens(RecursiveOutcome),
    EmptyChild(RecursiveOutcome),
}
#[derive(Clone, Debug)]
pub struct CompanionFirstCenter {
    coefficient: Arc<CompanionCoefficientChart>,
    child_source: Arc<MarkedIdeal>,
    child: BoundaryFreeFirstCenter,
    outcome: CompanionCenterOutcome,
}
impl CompanionFirstCenter {
    /// The lower presentation starts with empty exceptional boundary by BM
    /// Section 5 I.B; its ambient embedding and parent's history stay retained.
    pub fn new(
        coefficient: Arc<CompanionCoefficientChart>,
        namespace: &str,
        reverse: bool,
    ) -> Result<Self> {
        let child_source = Arc::new(coefficient.coefficient().coefficient().clone());
        let child = BoundaryFreeFirstCenter::new(
            coefficient.contact().contact().clone(),
            child_source.clone(),
            namespace,
            reverse,
        )?;
        Ok(Self {
            coefficient,
            child_source,
            child,
            outcome: CompanionCenterOutcome::Pending,
        })
    }
    /// Preserve the original coefficient source while using a checked initial
    /// principal-power auxiliary presentation inside its native child driver.
    pub fn new_with_principal_powers(
        coefficient: Arc<CompanionCoefficientChart>,
        namespace: &str,
        reverse: bool,
    ) -> Result<Self> {
        let child_source = Arc::new(coefficient.coefficient().coefficient().clone());
        let child = BoundaryFreeFirstCenter::new_with_principal_powers(
            coefficient.contact().contact().clone(),
            child_source.clone(),
            namespace,
            reverse,
        )?;
        Ok(Self {
            coefficient,
            child_source,
            child,
            outcome: CompanionCenterOutcome::Pending,
        })
    }
    /// Forward the actual interrupted lower recursion with its original outer
    /// companion owner. This does not initialize another auxiliary history.
    pub fn finite_contact_dispatcher(
        &self,
        namespace: String,
        b: &mut Budget,
    ) -> Result<Option<CompanionContactDispatcher>> {
        let Some(child) = self
            .child
            .finite_contact_dispatcher(format!("{namespace}_child"), b)?
        else {
            return Ok(None);
        };
        Ok(Some(CompanionContactDispatcher::prepare(
            self.coefficient.clone(),
            &self.child_source,
            child,
            namespace,
            b,
        )?))
    }
    pub fn outcome(&self) -> &CompanionCenterOutcome {
        &self.outcome
    }
    pub fn accepted_child_steps(&self) -> usize {
        self.child.accepted_steps()
    }
    pub fn advance(&mut self, b: &mut Budget) -> Result<RecursiveAdvance> {
        if matches!(
            self.outcome,
            CompanionCenterOutcome::ChildComponentOpensRequired(_)
        ) {
            return Ok(RecursiveAdvance::Incomplete {
                reason: super::components::COMPONENT_REASON,
            });
        }
        if !matches!(self.outcome, CompanionCenterOutcome::Pending) {
            return Ok(RecursiveAdvance::Complete);
        }
        let mut proposed = self.clone();
        match proposed.step(b) {
            Ok(status) => {
                *self = proposed;
                Ok(status)
            }
            Err(Error::ResourceIncomplete(reason)) => Ok(RecursiveAdvance::Incomplete { reason }),
            Err(e) => Err(e),
        }
    }
    fn step(&mut self, b: &mut Budget) -> Result<RecursiveAdvance> {
        match self.child.advance(b)? {
            RecursiveAdvance::Progress => return Ok(RecursiveAdvance::Progress),
            incomplete @ RecursiveAdvance::Incomplete { .. } => {
                if let RecursiveOutcome::ComponentOpensRequired(cover) = self.child.outcome() {
                    self.outcome =
                        CompanionCenterOutcome::ChildComponentOpensRequired(cover.clone());
                }
                return Ok(incomplete);
            }
            RecursiveAdvance::Complete => {}
        }
        let center = match self.child.outcome() {
            RecursiveOutcome::Center(c) => c.clone(),
            RecursiveOutcome::TopLevelZeroIdeal { frame, source } => {
                if !Arc::ptr_eq(frame, self.coefficient.contact().contact())
                    || !Arc::ptr_eq(source, &self.child_source)
                {
                    return Err(Error::Invalid("companion zero child owner"));
                }
                for f in source.ideal().generators() {
                    if !frame.local().zero(f, b)? {
                        return Err(Error::Invalid("companion zero child identity"));
                    }
                }
                Arc::new(RecursiveCenter {
                    frame: frame.clone(),
                    source: source.clone(),
                    ideal: Ideal::new(frame.local().ring().clone(), vec![], b)?,
                    normals: vec![],
                    invariant: vec![],
                    lift: None,
                })
            }
            out @ RecursiveOutcome::MultipleContactOpensRequired { .. } => {
                self.outcome = CompanionCenterOutcome::ChildNeedsMoreOpens(out.clone());
                return Ok(RecursiveAdvance::Complete);
            }
            out @ RecursiveOutcome::EmptyMarkedCosupport { .. } => {
                self.outcome = CompanionCenterOutcome::EmptyChild(out.clone());
                return Ok(RecursiveAdvance::Complete);
            }
            RecursiveOutcome::ComponentOpensRequired(_) => {
                return Err(Error::Invalid(
                    "completed child has unresolved component cover",
                ));
            }
            RecursiveOutcome::Pending => {
                return Err(Error::Invalid("completed companion child pending"));
            }
        };
        self.outcome =
            CompanionCenterOutcome::Center(Arc::new(lift(self.coefficient.clone(), center, b)?));
        Ok(RecursiveAdvance::Complete)
    }
}
fn lift(
    coefficient: Arc<CompanionCoefficientChart>,
    child: Arc<RecursiveCenter>,
    b: &mut Budget,
) -> Result<CompanionCenter> {
    let open = coefficient.source();
    let q = coefficient.contact();
    let old = coefficient.coefficient();
    if !Arc::ptr_eq(old.contact(), q)
        || !Arc::ptr_eq(old.history(), open.restriction().history())
        || !Arc::ptr_eq(q.source(), open.contact())
        || !Arc::ptr_eq(child.frame(), q.contact())
        || child.source().ideal() != old.coefficient().ideal()
        || child.source().mark() != old.coefficient().mark()
    {
        return Err(Error::Invalid("companion recursion construction owners"));
    }
    let parent_source = Arc::new(MarkedIdeal::new(
        open.restriction()
            .open()
            .extension()
            .ideal(open.order().current_source().ideal(), b)?,
        open.order().current_source().mark(),
        b,
    )?);
    let (ideal, normals, clearings) =
        super::lift::lift_geometry(q, q.source().frame(), &parent_source, &child, b)?;
    // Keep the incidence sum as its actual lower-dimensional owner. Verify its
    // full marked order along the chosen child; no sampled incidence test.
    if let Some(boundary) = old.boundary_ideal() {
        let local = q.contact().local();
        let center = local.ideal().sum(child.ideal(), b)?;
        let mut layer = boundary.ideal().clone();
        for j in 0..boundary.mark() {
            for f in layer.generators() {
                if !center.contains(f, local.unit_relations(), b)? {
                    return Err(Error::Invalid(
                        "companion center outside old incidence maximum",
                    ));
                }
            }
            if j + 1 < boundary.mark() {
                layer = super::super::differential::relative_differential_ideal(
                    q.contact(),
                    &layer,
                    b,
                )?;
            }
        }
    }
    Ok(CompanionCenter {
        coefficient,
        child,
        parent_source,
        ideal,
        normals,
        clearings,
    })
}
