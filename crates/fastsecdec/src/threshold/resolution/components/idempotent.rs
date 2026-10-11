use super::super::contact::clear_units;
use super::super::geometry::generators::VerifiedGeneratorPresentation;
use super::super::prepared_membership::PreparedLocalZero;
use super::super::{
    Budget, CartierDivision, Error, Ideal, MarkedIdeal, OpenCoverCertificate, Poly, UnitClearing,
    VerifiedCartierQuotient, VerifiedOpenCover, divide_cartier,
};
use super::elimination::{
    EliminationEvidence, auxiliary_ring, native_basis, produce_annihilator, remap,
};
use super::{RegularAlgebra, RegularOrigin, VerifiedAnnihilator};
use std::sync::Arc;
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Default)]
pub struct ComponentProgress {
    pub stage: &'static str,
    pub annihilator: Option<Arc<VerifiedAnnihilator>>,
    pub complement: Option<Arc<VerifiedAnnihilator>>,
    pub idempotent: Option<Poly>,
    pub checked_products: usize,
    pub checked_boundary_quotients: usize,
    pub operations: usize,
    pub ideal_slots: usize,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentPattern {
    NowhereIdenticallyZero,
    EverywhereIdenticallyZero,
    Mixed,
}
#[derive(Clone, Debug)]
pub struct ComponentOpen {
    clearing: UnitClearing,
    empty_in_ambient: bool,
}
impl ComponentOpen {
    pub fn clearing(&self) -> &UnitClearing {
        &self.clearing
    }
    pub fn empty_in_ambient(&self) -> bool {
        self.empty_in_ambient
    }
}
#[derive(Clone, Debug)]
pub struct VerifiedComponentSplit {
    owner: Arc<RegularAlgebra>,
    source: Arc<Ideal>,
    annihilator: Arc<VerifiedAnnihilator>,
    complement: Arc<VerifiedAnnihilator>,
    idempotent: Poly,
    idempotent_evidence: EliminationEvidence,
    idempotent_presentations: [Arc<VerifiedGeneratorPresentation>; 2],
    pattern: ComponentPattern,
    opens: [ComponentOpen; 2],
    cover: VerifiedOpenCover,
    overlap_quotient: Option<VerifiedCartierQuotient>,
    division_quotients: Vec<VerifiedCartierQuotient>,
    progress: ComponentProgress,
}
impl VerifiedComponentSplit {
    pub fn owner(&self) -> &Arc<RegularAlgebra> {
        &self.owner
    }
    pub fn source(&self) -> &Arc<Ideal> {
        &self.source
    }
    pub fn annihilator(&self) -> &Arc<VerifiedAnnihilator> {
        &self.annihilator
    }
    pub fn complement(&self) -> &Arc<VerifiedAnnihilator> {
        &self.complement
    }
    pub fn idempotent(&self) -> &Poly {
        &self.idempotent
    }
    pub fn idempotent_evidence(&self) -> &EliminationEvidence {
        &self.idempotent_evidence
    }
    /// Equivalent discovery inputs of the retained complete annihilators.
    pub fn idempotent_source_presentations(&self) -> [&Arc<MarkedIdeal>; 2] {
        [
            self.idempotent_presentations[0].selected(),
            self.idempotent_presentations[1].selected(),
        ]
    }
    pub fn pattern(&self) -> ComponentPattern {
        self.pattern
    }
    /// Ordered as the identical-zero component open, then its complement. These
    /// are algebraic overlapping ambient opens, never real integration cells.
    pub fn opens(&self) -> &[ComponentOpen; 2] {
        &self.opens
    }
    pub fn cover(&self) -> &VerifiedOpenCover {
        &self.cover
    }
    pub fn overlap_quotient(&self) -> Option<&VerifiedCartierQuotient> {
        self.overlap_quotient.as_ref()
    }
    pub fn division_quotients(&self) -> &[VerifiedCartierQuotient] {
        &self.division_quotients
    }
    pub fn progress(&self) -> &ComponentProgress {
        &self.progress
    }
    /// Checks the unique idempotent against immutable native complete
    /// annihilator receipts, not merely e*source=0 and e^2=e.
    pub fn verify_idempotent_candidate(&self, e: &Poly, budget: &mut Budget) -> Result<bool> {
        verify_idempotent(&self.owner, &self.annihilator, &self.complement, e, budget)
    }
}
#[derive(Clone, Debug)]
pub enum ComponentProduction {
    Complete(Box<VerifiedComponentSplit>),
    Incomplete {
        owner: Arc<RegularAlgebra>,
        source: Arc<Ideal>,
        reason: &'static str,
        progress: ComponentProgress,
    },
}
fn bump(counter: &mut usize) -> Result<()> {
    *counter = counter
        .checked_add(1)
        .ok_or(Error::ResourceIncomplete("component diagnostic counter"))?;
    Ok(())
}
fn verify_idempotent(
    owner: &RegularAlgebra,
    ann: &VerifiedAnnihilator,
    complement: &VerifiedAnnihilator,
    e: &Poly,
    budget: &mut Budget,
) -> Result<bool> {
    let local = owner.local();
    local.supports(e)?;
    budget.poly(e)?;
    let opposite = &local.ring().one() - e;
    budget.poly(&opposite)?;
    if !local.zero(&(&budget.mul(e, e)? - e), budget)? {
        return Ok(false);
    }
    let left = local.ideal().sum(ann.annihilator(), budget)?;
    let right = local.ideal().sum(complement.annihilator(), budget)?;
    if !left.contains(e, local.unit_relations(), budget)?
        || !right.contains(&opposite, local.unit_relations(), budget)?
    {
        return Ok(false);
    }
    let mut zero = PreparedLocalZero::new(local, budget)?;
    for f in ann.source().generators() {
        let product = budget.mul(e, f)?;
        if !zero.zero(&product, budget)? {
            return Ok(false);
        }
    }
    Ok(true)
}
struct IdempotentDiscovery {
    idempotent: Poly,
    evidence: EliminationEvidence,
    presentations: [Arc<VerifiedGeneratorPresentation>; 2],
}
fn discover_idempotent(
    owner: &RegularAlgebra,
    ann: &VerifiedAnnihilator,
    complement: &VerifiedAnnihilator,
    namespace: &str,
    budget: &mut Budget,
) -> Result<IdempotentDiscovery> {
    let local = owner.local();
    budget.reserve_slots(ann.annihilator().generators().len())?;
    budget.reserve_slots(complement.annihilator().generators().len())?;
    let ap = VerifiedGeneratorPresentation::prepare(
        local.clone(),
        Arc::new(MarkedIdeal::new((**ann.annihilator()).clone(), 1, budget)?),
        budget,
    )?;
    let cp = VerifiedGeneratorPresentation::prepare(
        local.clone(),
        Arc::new(MarkedIdeal::new(
            (**complement.annihilator()).clone(),
            1,
            budget,
        )?),
        budget,
    )?;
    let a = ap.selected().ideal();
    let c = cp.selected().ideal();
    let n = a
        .generators()
        .len()
        .checked_add(c.generators().len())
        .ok_or(Error::ResourceIncomplete("idempotent variable count"))?;
    let count = n
        .checked_add(1)
        .ok_or(Error::ResourceIncomplete("idempotent variable count"))?;
    let extended = auxiliary_ring(local.ring(), count, namespace, budget)?;
    let e = extended.coordinate(n)?;
    let mut left = e.clone();
    let mut right = &extended.one() - &e;
    for (i, f) in a.generators().iter().enumerate() {
        let f = remap(f, &extended, budget)?;
        left = left - budget.mul(&extended.coordinate(i)?, &f)?;
        budget.poly(&left)?;
    }
    for (i, f) in c.generators().iter().enumerate() {
        let f = remap(f, &extended, budget)?;
        right = right - budget.mul(&extended.coordinate(a.generators().len() + i)?, &f)?;
        budget.poly(&right)?;
    }
    let nr = local
        .ideal()
        .generators()
        .len()
        .checked_add(local.unit_relations().len())
        .and_then(|v| v.checked_add(2))
        .ok_or(Error::ResourceIncomplete("idempotent relation count"))?;
    budget.reserve_slots(nr)?;
    let mut equations = local
        .ideal()
        .generators()
        .iter()
        .chain(local.unit_relations())
        .map(|f| remap(f, &extended, budget))
        .collect::<Result<Vec<_>>>()?;
    equations.extend([left, right]);
    let evidence = native_basis(extended, count, equations, budget)?;
    budget.charge(1)?;
    let reduced = e.reduce(evidence.basis().generators());
    budget.poly(&reduced)?;
    if (0..count).any(|i| reduced.contains(i)) {
        return Err(Error::Invalid(
            "native component idempotent auxiliary elimination",
        ));
    }
    let e = remap(&reduced, local.ring(), budget)?;
    if !verify_idempotent(owner, ann, complement, &e, budget)? {
        return Err(Error::Invalid("idempotent original quotient identities"));
    }
    Ok(IdempotentDiscovery {
        idempotent: e,
        evidence,
        presentations: [ap, cp],
    })
}
pub fn produce_component_split(
    owner: Arc<RegularAlgebra>,
    source: Arc<Ideal>,
    namespace: &str,
    budget: &mut Budget,
) -> Result<ComponentProduction> {
    if source.ring() != owner.local().ring() {
        return Err(Error::Invalid("component source ring"));
    }
    for f in source.generators() {
        owner.local().supports(f)?;
    }
    let mut progress = ComponentProgress::default();
    let result = run(
        owner.clone(),
        source.clone(),
        namespace,
        budget,
        &mut progress,
    );
    progress.operations = budget.operations();
    progress.ideal_slots = budget.ideal_slots();
    match result {
        Ok(mut split) => {
            split.progress = progress;
            Ok(ComponentProduction::Complete(Box::new(split)))
        }
        Err(Error::ResourceIncomplete(reason)) => Ok(ComponentProduction::Incomplete {
            owner,
            source,
            reason,
            progress,
        }),
        Err(error) => Err(error),
    }
}
fn run(
    owner: Arc<RegularAlgebra>,
    source: Arc<Ideal>,
    namespace: &str,
    budget: &mut Budget,
    progress: &mut ComponentProgress,
) -> Result<VerifiedComponentSplit> {
    progress.stage = "native source annihilator";
    let annihilator = produce_annihilator(
        owner.clone(),
        source.clone(),
        &format!("{namespace}::ann"),
        budget,
    )?;
    progress.annihilator = Some(annihilator.clone());
    progress.stage = "native complementary annihilator";
    let complement = produce_annihilator(
        owner.clone(),
        annihilator.annihilator().clone(),
        &format!("{namespace}::complement"),
        budget,
    )?;
    progress.complement = Some(complement.clone());
    let local = owner.local();
    progress.stage = "original quotient complementary identities";
    let sum = annihilator
        .annihilator()
        .sum(complement.annihilator(), budget)?;
    if !local.ideal().sum(&sum, budget)?.contains(
        &local.ring().one(),
        local.unit_relations(),
        budget,
    )? {
        return Err(Error::Invalid(
            "regular component annihilators not complementary",
        ));
    }
    let products = annihilator
        .annihilator()
        .generators()
        .len()
        .checked_mul(complement.annihilator().generators().len())
        .ok_or(Error::ResourceIncomplete("complement product count"))?;
    budget.count(products)?;
    let mut zero = PreparedLocalZero::new(local, budget)?;
    for a in annihilator.annihilator().generators() {
        for c in complement.annihilator().generators() {
            let product = budget.mul(a, c)?;
            if !zero.zero(&product, budget)? {
                return Err(Error::Invalid("complement product in original quotient"));
            }
            bump(&mut progress.checked_products)?;
        }
    }
    progress.stage = "native idempotent discovery";
    let IdempotentDiscovery {
        idempotent,
        evidence: idempotent_evidence,
        presentations: idempotent_presentations,
    } = discover_idempotent(
        &owner,
        &annihilator,
        &complement,
        &format!("{namespace}::idempotent"),
        budget,
    )?;
    progress.idempotent = Some(idempotent.clone());
    let opposite = &local.ring().one() - &idempotent;
    budget.poly(&opposite)?;
    let pattern = if local.zero(&idempotent, budget)? {
        ComponentPattern::NowhereIdenticallyZero
    } else if local.zero(&opposite, budget)? {
        ComponentPattern::EverywhereIdenticallyZero
    } else {
        ComponentPattern::Mixed
    };
    progress.stage = "original ambient open cover";
    let ambient = owner.ambient_local();
    let left = clear_units(ambient, &idempotent, budget)?;
    let right = clear_units(ambient, &opposite, budget)?;
    let cover = OpenCoverCertificate {
        algebra: ambient.clone(),
        support: Ideal::new(ambient.ring().clone(), vec![], budget)?,
        opens: vec![left.numerator.clone(), right.numerator.clone()],
    }
    .verify(budget)?;
    let opens = [
        ComponentOpen {
            empty_in_ambient: ambient.zero(&left.numerator, budget)?,
            clearing: left,
        },
        ComponentOpen {
            empty_in_ambient: ambient.zero(&right.numerator, budget)?,
            clearing: right,
        },
    ];
    progress.stage = "boundary divisibility in original ambient quotient";
    let mut overlap_quotient = None;
    let mut division_quotients = Vec::new();
    if let RegularOrigin::Boundary {
        ledger, divisor, ..
    } = owner.origin()
    {
        budget.reserve_slots(source.generators().len())?;
        for f in source.generators() {
            let dividend = budget.mul(&idempotent, f)?;
            let CartierDivision::Quotient(q) = divide_cartier(
                ledger.clone(),
                *divisor,
                dividend,
                &format!("{namespace}::source_quotient"),
                budget,
            )?
            else {
                return Err(Error::Invalid("component divide-open recombination"));
            };
            division_quotients.push(q);
            bump(&mut progress.checked_boundary_quotients)?;
        }
        let overlap = budget.mul(&idempotent, &opposite)?;
        let CartierDivision::Quotient(q) = divide_cartier(
            ledger.clone(),
            *divisor,
            overlap,
            &format!("{namespace}::overlap_quotient"),
            budget,
        )?
        else {
            return Err(Error::Invalid("component overlap does not avoid boundary"));
        };
        overlap_quotient = Some(q);
        bump(&mut progress.checked_boundary_quotients)?;
    }
    progress.stage = "complete";
    Ok(VerifiedComponentSplit {
        owner,
        source,
        annihilator,
        complement,
        idempotent,
        idempotent_evidence,
        idempotent_presentations,
        pattern,
        opens,
        cover,
        overlap_quotient,
        division_quotients,
        progress: ComponentProgress::default(),
    })
}
