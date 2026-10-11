use super::*;

#[derive(Clone, Debug)]
pub struct ControlledGenerator {
    pub source: Poly,
    pub total: Poly,
    pub quotient: Poly,
}
#[derive(Clone, Debug)]
pub struct CarriedFirstChart {
    center: Arc<RecursiveCenter>,
    contact: Arc<ContactQuotient>,
    original_coefficient: Arc<VerifiedQuotientNormalization>,
    extension: RingExtension,
    transform: RecursiveBlowupChart,
    history: Arc<ResolutionHistory>,
    parent: Arc<MarkedIdeal>,
    companion: Arc<MarkedIdeal>,
    companion_order: OrderProduction,
    companion_generators: Vec<ControlledGenerator>,
    strict_support: Poly,
    support_is_empty: bool,
    coefficient: Arc<MarkedIdeal>,
    coefficient_generators: Vec<ControlledGenerator>,
    coefficient_clearings: Vec<UnitClearing>,
}
impl CarriedFirstChart {
    pub fn center(&self) -> &Arc<RecursiveCenter> {
        &self.center
    }
    pub fn contact(&self) -> &Arc<ContactQuotient> {
        &self.contact
    }
    pub fn original_coefficient(&self) -> &Arc<VerifiedQuotientNormalization> {
        &self.original_coefficient
    }
    pub fn extension(&self) -> &RingExtension {
        &self.extension
    }
    pub fn transform(&self) -> &RecursiveBlowupChart {
        &self.transform
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn parent(&self) -> &Arc<MarkedIdeal> {
        &self.parent
    }
    pub fn companion(&self) -> &Arc<MarkedIdeal> {
        &self.companion
    }
    pub fn companion_order(&self) -> &OrderProduction {
        &self.companion_order
    }
    pub fn companion_generators(&self) -> &[ControlledGenerator] {
        &self.companion_generators
    }
    pub fn strict_support(&self) -> &Poly {
        &self.strict_support
    }
    pub fn support_is_empty(&self) -> bool {
        self.support_is_empty
    }
    pub fn coefficient(&self) -> &Arc<MarkedIdeal> {
        &self.coefficient
    }
    pub fn coefficient_generators(&self) -> &[ControlledGenerator] {
        &self.coefficient_generators
    }
    pub fn coefficient_clearings(&self) -> &[UnitClearing] {
        &self.coefficient_clearings
    }
    pub fn companion_resolved(&self) -> bool {
        match &self.companion_order {
            OrderProduction::UnitIdeal { .. } => true,
            OrderProduction::ContactCover(c) => c.algebraic_maximum_order() < self.companion.mark(),
            _ => false,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CarriedFirstBlowup {
    initial_history: Arc<ResolutionHistory>,
    center: Arc<RecursiveCenter>,
    charts: Vec<Arc<CarriedFirstChart>>,
}
impl CarriedFirstBlowup {
    pub fn initial_history(&self) -> &Arc<ResolutionHistory> {
        &self.initial_history
    }
    pub fn center(&self) -> &Arc<RecursiveCenter> {
        &self.center
    }
    pub fn charts(&self) -> &[Arc<CarriedFirstChart>] {
        &self.charts
    }
}
/// Carry a first boundary-free cycle using the actual previously checked
/// all-pivot blowup. This does not recompute its original coefficient ideal.
/// More general quotient division/support adaptation remains explicit refusal.
pub fn carry_first_blowup(
    history: Arc<ResolutionHistory>,
    blowup: FirstBlowup,
    b: &mut Budget,
) -> Result<CarriedFirstBlowup> {
    let (center, extension, charts) = match blowup {
        FirstBlowup::MarkedResolved {
            center,
            extension,
            charts,
        }
        | FirstBlowup::FurtherCyclesRequired {
            center,
            extension,
            charts,
        } => (center, extension, charts),
        FirstBlowup::GeneralAdaptationRequired { .. } => {
            return Err(Error::ResourceIncomplete(
                "general first-center adaptation required",
            ));
        }
    };
    if !Arc::ptr_eq(history.ledger().frame(), center.frame())
        || !history.ledger().divisors().is_empty()
        || history.stage() != 0
        || !history.chart_path().is_empty()
    {
        return Err(Error::Invalid("first carried cycle initial history owner"));
    }
    let level = center
        .lift_receipt()
        .ok_or(Error::Invalid("first carried cycle lacks contact edge"))?
        .level();
    if level.presentation().power().is_some() {
        return Err(Error::ResourceIncomplete(
            "principal presentation requires retained hierarchy transport",
        ));
    }
    let q = level.contact().clone();
    if !Arc::ptr_eq(q.source().frame(), center.frame()) {
        return Err(Error::Invalid("first carried cycle original contact frame"));
    }
    let normalizer = QuotientNormalizer::prepare(q.contact().local().clone(), b)?;
    let coefficient =
        match normalizer.normalize(Arc::new(q.differential_coefficient().clone()), b)? {
            NormalizationOutcome::Complete(c) => Arc::new(*c),
            NormalizationOutcome::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
    b.reserve_slots(charts.len())?;
    let mut out = Vec::new();
    for chart in charts {
        let source_axes = center.frame().free_axes();
        let position = source_axes
            .iter()
            .position(|i| *i == chart.pivot_source_axis())
            .ok_or(Error::Invalid("carried pivot source"))?;
        let e = extension
            .target()
            .coordinate(chart.transform().map().target().axes()[position])?;
        let (id, _) = history.next_transition()?;
        let ledger = match verify_initial_relative_snc(
            chart.target_frame().clone(),
            vec![InitialDivisor {
                id,
                equation: e.clone(),
            }],
            b,
        )? {
            SncProduction::Verified(l) => l,
            SncProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
            SncProduction::Unresolved { .. } => {
                return Err(Error::Invalid("checked coordinate exceptional not SNC"));
            }
        };
        let target_history =
            history.advanced_first_coordinate(&center, &extension, &chart, ledger, b)?;
        let pull = |p: &Poly, b: &mut Budget| -> Result<Poly> {
            let p = extension.pull(p, b)?;
            let adapted = chart.transform().center().frame().map().pull(&p, b)?;
            chart.transform().map().pull(&adapted, b)
        };
        let gmark = q.source().algebraic_maximum_order();
        let gfactor = b.power(&e, gmark)?;
        let mut ggenerators = Vec::new();
        b.reserve_slots(q.source().source().generators().len())?;
        for f in q.source().source().generators() {
            let total = pull(f, b)?;
            let quotient = controlled(&total, &gfactor, b)?;
            ggenerators.push(ControlledGenerator {
                source: f.clone(),
                total,
                quotient,
            });
        }
        let companion = Arc::new(MarkedIdeal::new(
            Ideal::new(
                extension.target().clone(),
                ggenerators.iter().map(|g| g.quotient.clone()).collect(),
                b,
            )?,
            gmark,
            b,
        )?);
        let order = produce_ordinary_contact_cover(
            chart.target_frame().clone(),
            Arc::new(companion.ideal().clone()),
            b,
        )?;
        if let OrderProduction::Incomplete { reason, .. } = &order {
            return Err(Error::ResourceIncomplete(reason));
        }
        let h = super::super::clear_units(
            center.frame().local(),
            &q.source().candidates()[q.candidate_index()].equation,
            b,
        )?;
        let htotal = pull(&h.numerator, b)?;
        let strict_support = controlled(&htotal, &e, b)?;
        // Smooth contact has order one along the admitted center; this exact
        // quotient is its strict hypersurface transform, including unit charts.
        let support_is_empty = Ideal::new(
            extension.target().clone(),
            vec![strict_support.clone()],
            b,
        )?
        .contains(&extension.target().one(), &[], b)?;
        let cmark = coefficient.normalized().mark();
        let cfactor = b.power(&e, cmark)?;
        let mut cgenerators = Vec::new();
        let mut clearings = Vec::new();
        b.reserve_slots(
            coefficient
                .normalized()
                .ideal()
                .generators()
                .len()
                .checked_mul(2)
                .ok_or(Error::ResourceIncomplete("carried coefficient records"))?,
        )?;
        for f in coefficient.normalized().ideal().generators() {
            let clearing = super::super::clear_units(q.contact().local(), f, b)?;
            if (extension.source().len()..clearing.numerator.nvars())
                .any(|i| clearing.numerator.degree(i) > 0 || clearing.denominator.degree(i) > 0)
            {
                return Err(Error::ResourceIncomplete(
                    "carried coefficient needs general support quotient lift",
                ));
            }
            let numerator = clearing
                .numerator
                .rearrange_with_growth(extension.source().one().variables())
                .map_err(|_| Error::Invalid("carried coefficient variable map"))?;
            let denominator = clearing
                .denominator
                .rearrange_with_growth(extension.source().one().variables())
                .map_err(|_| Error::Invalid("carried denominator variable map"))?;
            let denom = pull(&denominator, b)?;
            if !chart.transform().map().target().unit(&denom, b)? {
                return Err(Error::ResourceIncomplete(
                    "carried coefficient denominator requires support localization",
                ));
            }
            let total = pull(&numerator, b)?;
            let quotient = controlled(&total, &cfactor, b)?;
            cgenerators.push(ControlledGenerator {
                source: f.clone(),
                total,
                quotient,
            });
            clearings.push(clearing);
        }
        let target_coefficient = Arc::new(MarkedIdeal::new(
            Ideal::new(
                extension.target().clone(),
                cgenerators.iter().map(|c| c.quotient.clone()).collect(),
                b,
            )?,
            cmark,
            b,
        )?);
        let parent = Arc::new(chart.transform().target().clone());
        out.push(Arc::new(CarriedFirstChart {
            center: center.clone(),
            contact: q.clone(),
            original_coefficient: coefficient.clone(),
            extension: extension.clone(),
            transform: chart,
            history: target_history,
            parent,
            companion,
            companion_order: order,
            companion_generators: ggenerators,
            strict_support,
            support_is_empty,
            coefficient: target_coefficient,
            coefficient_generators: cgenerators,
            coefficient_clearings: clearings,
        }));
    }
    Ok(CarriedFirstBlowup {
        initial_history: history,
        center,
        charts: out,
    })
}
fn controlled(total: &Poly, factor: &Poly, b: &mut Budget) -> Result<Poly> {
    b.poly(total)?;
    b.poly(factor)?;
    b.charge(1)?;
    let (q, r) = total.quot_rem(factor, false);
    if !r.is_zero() {
        return Err(Error::ResourceIncomplete(
            "controlled lift requires quotient support division",
        ));
    }
    b.poly(&q)?;
    if b.mul(factor, &q)? != *total {
        return Err(Error::Invalid("carried controlled generator recombination"));
    }
    Ok(q)
}
#[derive(Clone, Debug)]
pub struct ResidualDrop {
    prior: Arc<CarriedFirstChart>,
    current: Arc<ComponentResidualOrder>,
}
impl ResidualDrop {
    pub fn prior(&self) -> &Arc<CarriedFirstChart> {
        &self.prior
    }
    pub fn current(&self) -> &Arc<ComponentResidualOrder> {
        &self.current
    }
}
/// A checked transition prerequisite. It does not itself reset any history or
/// discard the carried original C; the next cycle must retain this owner.
pub fn prove_first_residual_drop(
    prior: Arc<CarriedFirstChart>,
    current: Arc<ComponentResidualOrder>,
) -> Result<ResidualDrop> {
    let h = current.factor().data().history();
    if !Arc::ptr_eq(current.factor().data().source(), prior.parent())
        || !h.same_root(prior.history())
        || !h.chart_path().starts_with(prior.history().chart_path())
        || !prior.companion_resolved()
        || current.algebraic_maximum_on_cosupport() >= prior.companion().mark()
    {
        return Err(Error::Invalid("unproved residual maximum drop"));
    }
    Ok(ResidualDrop { prior, current })
}
