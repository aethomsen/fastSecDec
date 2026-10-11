use super::*;
use symbolica::symbol;
#[derive(Clone, Debug)]
pub enum RecursiveOutcome {
    Pending,
    ComponentOpensRequired(Arc<RecursiveComponentCover>),
    Center(Arc<RecursiveCenter>),
    EmptyMarkedCosupport {
        frame: Arc<EtaleFrame>,
        source: Arc<MarkedIdeal>,
        presentation: Arc<RecursivePresentation>,
    },
    TopLevelZeroIdeal {
        frame: Arc<EtaleFrame>,
        source: Arc<MarkedIdeal>,
    },
    MultipleContactOpensRequired {
        source: Arc<ProducedContactCover>,
        presentation: Arc<RecursivePresentation>,
    },
}
#[cfg(test)]
mod power_tests {
    use super::*;
    #[test]
    fn initial_power_recursion_budget_is_atomic() {
        let mut b = Budget::new(Limits::default());
        let center = super::super::power_tests::cusp_center(&mut b);
        let source = center.child().source().clone();
        let frame = center.child().frame().clone();
        let mut driver = BoundaryFreeFirstCenter::new_with_principal_powers(
            frame,
            source,
            "power_atomic",
            false,
        )
        .unwrap();
        let mut no_work = Budget::new(Limits {
            max_operations: 0,
            ..Limits::default()
        });
        assert!(matches!(
            driver.advance(&mut no_work).unwrap(),
            RecursiveAdvance::Incomplete { .. }
        ));
        assert_eq!(driver.accepted_steps(), 0);
        assert_eq!(driver.depth(), 0);
        assert!(driver.auxiliary_origin.is_none());
        assert!(matches!(driver.outcome(), RecursiveOutcome::Pending));
        assert!(matches!(
            driver.advance(&mut b).unwrap(),
            RecursiveAdvance::Progress
        ));
        assert_eq!(driver.accepted_steps(), 1);
        assert!(driver.auxiliary_origin.is_some());
        assert!(driver.stack[0].presentation().power().is_some());
    }
}
#[derive(Clone, Debug)]
enum Pending {
    Descend {
        frame: Arc<EtaleFrame>,
        source: Arc<MarkedIdeal>,
    },
    Ascend(Arc<RecursiveCenter>),
}
#[derive(Clone, Debug)]
pub struct BoundaryFreeFirstCenter {
    stack: Vec<Arc<RecursiveLevel>>,
    pending: Option<Pending>,
    outcome: RecursiveOutcome,
    namespace: String,
    reverse_contacts: bool,
    accepted_steps: usize,
    original_frame: Arc<EtaleFrame>,
    original_source: Arc<MarkedIdeal>,
    auxiliary_origin: Option<Arc<AuxiliaryRecursionOrigin>>,
    principal_powers: bool,
}
#[derive(Clone, Debug)]
pub enum RecursiveAdvance {
    Progress,
    Complete,
    Incomplete { reason: &'static str },
}
impl BoundaryFreeFirstCenter {
    pub fn new(
        frame: Arc<EtaleFrame>,
        source: Arc<MarkedIdeal>,
        namespace: &str,
        reverse_contacts: bool,
    ) -> Result<Self> {
        if source.ideal().ring() != frame.local().ring()
            || (0..frame.local().ring().len()).any(|i| frame.local().ring().is_parameter(i))
        {
            return Err(Error::Invalid("first-center fixed-parameter source/frame"));
        }
        Ok(Self {
            original_frame: frame.clone(),
            original_source: source.clone(),
            auxiliary_origin: None,
            principal_powers: false,
            stack: vec![],
            pending: Some(Pending::Descend { frame, source }),
            outcome: RecursiveOutcome::Pending,
            namespace: namespace.into(),
            reverse_contacts,
            accepted_steps: 0,
        })
    }
    /// Opt in only for this genuinely initial boundary-free auxiliary source.
    pub fn new_with_principal_powers(
        frame: Arc<EtaleFrame>,
        source: Arc<MarkedIdeal>,
        namespace: &str,
        reverse_contacts: bool,
    ) -> Result<Self> {
        let mut result = Self::new(frame, source, namespace, reverse_contacts)?;
        result.principal_powers = true;
        Ok(result)
    }
    pub fn outcome(&self) -> &RecursiveOutcome {
        &self.outcome
    }
    pub fn accepted_steps(&self) -> usize {
        self.accepted_steps
    }
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
    /// Caller-stepped, atomic accepted state. An incomplete native operation is
    /// retried from this state; it is not a durable or inner-work checkpoint.
    pub fn advance(&mut self, b: &mut Budget) -> Result<RecursiveAdvance> {
        if matches!(self.outcome, RecursiveOutcome::ComponentOpensRequired(_)) {
            return Ok(RecursiveAdvance::Incomplete {
                reason: super::components::COMPONENT_REASON,
            });
        }
        if self.pending.is_none() {
            return Ok(RecursiveAdvance::Complete);
        }
        let mut proposed = self.clone();
        match proposed.step(b) {
            Ok(()) => {
                proposed.accepted_steps = proposed
                    .accepted_steps
                    .checked_add(1)
                    .ok_or(Error::ResourceIncomplete("recursive step counter"))?;
                *self = proposed;
                Ok(
                    if matches!(self.outcome, RecursiveOutcome::ComponentOpensRequired(_)) {
                        RecursiveAdvance::Incomplete {
                            reason: super::components::COMPONENT_REASON,
                        }
                    } else if self.pending.is_none() {
                        RecursiveAdvance::Complete
                    } else {
                        RecursiveAdvance::Progress
                    },
                )
            }
            Err(Error::ResourceIncomplete(reason)) => Ok(RecursiveAdvance::Incomplete { reason }),
            Err(e) => Err(e),
        }
    }
    fn step(&mut self, b: &mut Budget) -> Result<()> {
        match self
            .pending
            .clone()
            .ok_or(Error::Invalid("recursive pending owner"))?
        {
            Pending::Ascend(child) => {
                if let Some(level) = self.stack.last().cloned() {
                    let center = Arc::new(super::lift::ascend(level, child, b)?);
                    self.stack.pop();
                    self.pending = Some(Pending::Ascend(center));
                } else {
                    self.pending = None;
                    self.outcome = RecursiveOutcome::Center(child);
                }
            }
            Pending::Descend { frame, source } => {
                if self.auxiliary_origin.is_none() {
                    if !self.stack.is_empty()
                        || !Arc::ptr_eq(&frame, &self.original_frame)
                        || !Arc::ptr_eq(&source, &self.original_source)
                    {
                        return Err(Error::Invalid("late auxiliary recursion origin"));
                    }
                    self.auxiliary_origin = Some(AuxiliaryRecursionOrigin::prepare(
                        frame.clone(),
                        source.clone(),
                        b,
                    )?);
                }
                let normalizer = QuotientNormalizer::prepare(frame.local().clone(), b)?;
                let normalization = match normalizer.normalize(source.clone(), b)? {
                    NormalizationOutcome::Complete(n) => Arc::new(*n),
                    NormalizationOutcome::Incomplete { reason, .. } => {
                        return Err(Error::ResourceIncomplete(reason));
                    }
                };
                let presentation = RecursivePresentation::prepare(
                    normalization.clone(),
                    if self.principal_powers
                        && self.stack.is_empty()
                        && Arc::ptr_eq(&source, &self.original_source)
                    {
                        self.auxiliary_origin.clone()
                    } else {
                        None
                    },
                    b,
                )?;
                presentation.verify(&frame, &source)?;
                match produce_ordinary_contact_cover(
                    frame.clone(),
                    Arc::new(presentation.effective_source().ideal().clone()),
                    b,
                )? {
                    OrderProduction::ZeroIdeal { .. } => {
                        if self.stack.is_empty() {
                            self.pending = None;
                            self.outcome = RecursiveOutcome::TopLevelZeroIdeal { frame, source };
                            return Ok(());
                        }
                        let center = RecursiveCenter {
                            ideal: Ideal::new(frame.local().ring().clone(), vec![], b)?,
                            frame,
                            source,
                            normals: vec![],
                            invariant: vec![],
                            lift: None,
                        };
                        self.pending = Some(Pending::Ascend(Arc::new(center)));
                    }
                    OrderProduction::UnitIdeal { .. } => self.empty(frame, source, presentation)?,
                    OrderProduction::TerminalParameterLocus { .. } => {
                        let cover = RecursiveComponentCover::prepare(
                            self.auxiliary_origin
                                .clone()
                                .ok_or(Error::Invalid("missing auxiliary origin"))?,
                            frame,
                            source,
                            normalization,
                            &self.stack,
                            &format!("{}_terminal_components", self.namespace),
                            b,
                        )?;
                        self.pending = None;
                        self.outcome = RecursiveOutcome::ComponentOpensRequired(cover);
                    }
                    OrderProduction::Incomplete { reason, .. } => {
                        return Err(Error::ResourceIncomplete(reason));
                    }
                    OrderProduction::ContactCover(cover) => {
                        if cover.algebraic_maximum_order() < presentation.effective_source().mark()
                        {
                            self.empty(frame, source, presentation)?;
                            return Ok(());
                        }
                        let cover = Arc::new(cover);
                        b.reserve_slots(cover.candidates().len())?;
                        let mut candidates = (0..cover.candidates().len()).collect::<Vec<_>>();
                        if self.reverse_contacts {
                            candidates.reverse();
                        }
                        let mut chosen = None;
                        for i in candidates {
                            let unit = frame.local().ideal().sum(
                                &Ideal::new(
                                    frame.local().ring().clone(),
                                    vec![cover.candidates()[i].differential.clone()],
                                    b,
                                )?,
                                b,
                            )?;
                            if unit.contains(
                                &frame.local().ring().one(),
                                frame.local().unit_relations(),
                                b,
                            )? {
                                chosen = Some(i);
                                break;
                            }
                        }
                        let Some(index) = chosen else {
                            self.pending = None;
                            self.outcome = RecursiveOutcome::MultipleContactOpensRequired {
                                source: cover,
                                presentation,
                            };
                            return Ok(());
                        };
                        let depth = self.stack.len();
                        let fresh = [
                            symbol!(format!("{}::d{depth}_z", self.namespace)),
                            symbol!(format!("{}::d{depth}_inv", self.namespace)),
                        ];
                        let q = match construct_contact_quotient(cover.clone(), index, fresh, b)? {
                            ContactProduction::Constructed(q) => Arc::new(*q),
                            ContactProduction::Incomplete { reason, .. } => {
                                return Err(Error::ResourceIncomplete(reason));
                            }
                            ContactProduction::EmptyContactOpen { .. } => {
                                return Err(Error::Invalid(
                                    "global unit contact unexpectedly empty",
                                ));
                            }
                        };
                        if q.contact().free_axes().len().checked_add(1)
                            != Some(frame.free_axes().len())
                        {
                            return Err(Error::Invalid("recursive contact dimension did not drop"));
                        }
                        let m = i64::try_from(cover.algebraic_maximum_order())
                            .map_err(|_| Error::ResourceIncomplete("recursive order conversion"))?;
                        let d = i64::try_from(presentation.effective_source().mark())
                            .map_err(|_| Error::ResourceIncomplete("recursive mark conversion"))?;
                        let ratio = Rational::from(m) / Rational::from(d);
                        let next = Pending::Descend {
                            frame: q.contact().clone(),
                            source: Arc::new(q.differential_coefficient().clone()),
                        };
                        b.reserve_slots(
                            self.stack
                                .len()
                                .checked_add(1)
                                .ok_or(Error::ResourceIncomplete("recursive stack count"))?,
                        )?;
                        self.stack.push(Arc::new(RecursiveLevel {
                            frame,
                            source,
                            contact: q,
                            presentation,
                            ratio,
                        }));
                        self.pending = Some(next);
                    }
                }
            }
        }
        Ok(())
    }
    fn empty(
        &mut self,
        frame: Arc<EtaleFrame>,
        source: Arc<MarkedIdeal>,
        presentation: Arc<RecursivePresentation>,
    ) -> Result<()> {
        if !self.stack.is_empty() {
            return Err(Error::Invalid(
                "recursive coefficient unexpectedly lost nonempty maximum",
            ));
        }
        self.pending = None;
        self.outcome = RecursiveOutcome::EmptyMarkedCosupport {
            frame,
            source,
            presentation,
        };
        Ok(())
    }
}
