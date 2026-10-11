//! Induced lower-J transition. Its initial empty boundary belongs to the
//! ORIGINAL auxiliary J, not to the current transformed contact support.
use super::super::*;
use super::{
    coefficient::{
        CarriedCompanionChart, CarriedCompanionSupport, SupportedMarkedTransform,
        controlled_supported,
    },
    helpers::{Result, unit},
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct EmbeddedChildCycle {
    center: Arc<CompanionCenter>,
    initial_history: Arc<ResolutionHistory>,
    original_tree: Arc<super::tower::OriginalRecursionTree>,
}
impl EmbeddedChildCycle {
    pub fn new(center: Arc<CompanionCenter>, b: &mut Budget) -> Result<Arc<Self>> {
        let q = center.coefficient().contact();
        let child = center.child();
        if !Arc::ptr_eq(child.frame(), q.contact())
            || child.source().ideal() != center.coefficient().coefficient().coefficient().ideal()
            || child.source().mark() != center.coefficient().coefficient().coefficient().mark()
        {
            return Err(Error::Invalid("initial auxiliary J construction owner"));
        }
        let retained_origin = child
            .lift_receipt()
            .and_then(|r| r.level().presentation().origin());
        let initial_history = if let Some(origin) = retained_origin {
            if !Arc::ptr_eq(origin.source(), child.source())
                || !Arc::ptr_eq(origin.frame(), child.frame())
            {
                return Err(Error::Invalid("initial auxiliary power origin"));
            }
            origin.history().clone()
        } else {
            let ledger = match verify_initial_relative_snc(q.contact().clone(), vec![], b)? {
                SncProduction::Verified(l) => l,
                SncProduction::Incomplete { reason, .. } => {
                    return Err(Error::ResourceIncomplete(reason));
                }
                SncProduction::Unresolved { .. } => {
                    return Err(Error::Invalid("initial empty lower boundary"));
                }
            };
            ResolutionHistory::initial(ledger)?
        };
        let original_tree =
            super::tower::OriginalRecursionTree::new(center.clone(), initial_history.clone(), b)?;
        Ok(Arc::new(Self {
            center,
            initial_history,
            original_tree,
        }))
    }
    pub fn center(&self) -> &Arc<CompanionCenter> {
        &self.center
    }
    pub fn initial_history(&self) -> &Arc<ResolutionHistory> {
        &self.initial_history
    }
    pub fn original_tree(&self) -> &Arc<super::tower::OriginalRecursionTree> {
        &self.original_tree
    }
}
#[derive(Clone, Debug)]
pub struct InducedChildChart {
    origin: Arc<EmbeddedChildCycle>,
    ambient: Arc<CarriedCompanionChart>,
    support: Arc<CarriedCompanionSupport>,
    center_pullback: Arc<SupportedMarkedTransform>,
    history: Arc<ResolutionHistory>,
}
impl InducedChildChart {
    pub fn origin(&self) -> &Arc<EmbeddedChildCycle> {
        &self.origin
    }
    pub fn ambient(&self) -> &Arc<CarriedCompanionChart> {
        &self.ambient
    }
    pub fn support(&self) -> &Arc<CarriedCompanionSupport> {
        &self.support
    }
    pub fn center_pullback(&self) -> &Arc<SupportedMarkedTransform> {
        &self.center_pullback
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
}
pub fn induce_child_chart(
    origin: Arc<EmbeddedChildCycle>,
    ambient: Arc<CarriedCompanionChart>,
    index: usize,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<InducedChildChart>> {
    if !Arc::ptr_eq(origin.center(), ambient.center()) {
        return Err(Error::Invalid("induced lower cycle center owner"));
    }
    let support = ambient
        .opens()
        .get(index)
        .ok_or(Error::Invalid("induced support open"))?
        .clone();
    // The source belongs to the original coefficient construction. The actual
    // admissible physical center may differ from its earlier local candidate.
    let embedding = ambient.support().embedding();
    let lower = embedding
        .extension()
        .ideal(ambient.chart().geometry().center().ideal(), b)?;
    let source = Arc::new(MarkedIdeal::new(lower, 1, b)?);
    let center_pullback = controlled_supported(
        ambient.support().clone(),
        support.incidence_sum().open().clone(),
        source,
        &format!("{namespace}_center"),
        b,
    )?;
    let history = origin.initial_history().advanced_induced(
        &origin,
        &ambient,
        &support,
        &center_pullback,
        b,
    )?;
    Ok(Arc::new(InducedChildChart {
        origin,
        ambient,
        support,
        center_pullback,
        history,
    }))
}

#[derive(Clone, Debug)]
pub struct CarriedMonomialCenter {
    induced: Arc<InducedChildChart>,
    factor: Arc<FactorLeaf>,
    child: Arc<ProducedMonomialCenter>,
    ideal: Ideal,
    normals: Vec<Poly>,
    clearings: Vec<UnitClearing>,
}
impl CarriedMonomialCenter {
    pub fn induced(&self) -> &Arc<InducedChildChart> {
        &self.induced
    }
    pub fn factor(&self) -> &Arc<FactorLeaf> {
        &self.factor
    }
    pub fn child(&self) -> &Arc<ProducedMonomialCenter> {
        &self.child
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
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.induced.ambient().chart().geometry().frame()
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        self.induced.ambient().chart().parent().target()
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        self.induced.ambient().chart().history()
    }
}
#[derive(Clone, Debug)]
pub enum InducedCenterProduction {
    Center(Arc<CarriedMonomialCenter>),
    Empty {
        induced: Arc<InducedChildChart>,
        proof: MonomialProduction,
    },
    NeedsLowerRecursion {
        induced: Arc<InducedChildChart>,
        factor: Arc<FactorLeaf>,
    },
    NeedsLocalization {
        induced: Arc<InducedChildChart>,
        factor: Arc<FactorLeaf>,
    },
}
pub fn produce_induced_monomial_center(
    induced: Arc<InducedChildChart>,
    factor: Arc<FactorLeaf>,
    b: &mut Budget,
) -> Result<InducedCenterProduction> {
    let source = induced.support().incidence_sum().target();
    let data = factor.data();
    if !Arc::ptr_eq(data.source(), source) {
        return Err(Error::Invalid("induced factor source J owner"));
    }
    if !Arc::ptr_eq(data.history(), induced.history()) {
        if !data.history().same_root(induced.history())
            || !data
                .history()
                .chart_path()
                .starts_with(induced.history().chart_path())
            || data.history().births() != induced.history().births()
        {
            return Err(Error::Invalid(
                "induced factor history reset or foreign ancestry",
            ));
        }
        return Ok(InducedCenterProduction::NeedsLocalization { induced, factor });
    }
    let local = data.history().ledger().frame().local();
    let residual = Ideal::new(local.ring().clone(), data.quotients().to_vec(), b)?;
    if !local
        .ideal()
        .sum(&residual, b)?
        .contains(&local.ring().one(), local.unit_relations(), b)?
    {
        return Ok(InducedCenterProduction::NeedsLowerRecursion { induced, factor });
    }
    let proof = produce_monomial_center(
        data.history().ledger().clone(),
        MonomialWitness {
            source: source.clone(),
            powers: data.powers().to_vec(),
            quotients: data.quotients().to_vec(),
        },
        b,
    )?;
    let child = match proof {
        MonomialProduction::Center(c) => Arc::new(*c),
        p @ MonomialProduction::EmptyCosupport { .. } => {
            return Ok(InducedCenterProduction::Empty { induced, proof: p });
        }
        MonomialProduction::Incomplete { reason, .. } => {
            return Err(Error::ResourceIncomplete(reason));
        }
    };
    let open = induced.support().incidence_sum().open();
    let support = induced.ambient().support();
    let child_normals = child
        .indices()
        .iter()
        .map(|i| child.ledger().divisors()[*i].equation.clone())
        .collect::<Vec<_>>();
    let Some((ideal, normals, clearings)) = lift_supported_center(
        SupportedCenterLift {
            frame: induced.ambient().chart().geometry().frame(),
            parent: induced.ambient().chart().parent().target(),
            companion: induced.ambient().companion().target(),
            support,
            open,
            child: child.center(),
            child_normals: &child_normals,
        },
        b,
    )?
    else {
        return Ok(InducedCenterProduction::NeedsLocalization { induced, factor });
    };
    Ok(InducedCenterProduction::Center(Arc::new(
        CarriedMonomialCenter {
            induced,
            factor,
            child,
            ideal,
            normals,
            clearings,
        },
    )))
}

pub(super) struct SupportedCenterLift<'a> {
    pub frame: &'a Arc<EtaleFrame>,
    pub parent: &'a Arc<MarkedIdeal>,
    pub companion: &'a Arc<MarkedIdeal>,
    pub support: &'a Arc<super::support::StrictContactSupport>,
    pub open: &'a Arc<super::support::StrictSupportOpen>,
    pub child: &'a Ideal,
    pub child_normals: &'a [Poly],
}
pub(super) type LiftResult = (Ideal, Vec<Poly>, Vec<UnitClearing>);
pub(super) fn lift_supported_center(
    input: SupportedCenterLift<'_>,
    b: &mut Budget,
) -> Result<Option<LiftResult>> {
    let SupportedCenterLift {
        frame,
        parent,
        companion,
        support,
        open,
        child,
        child_normals,
    } = input;
    if !Arc::ptr_eq(frame, support.geometry().frame())
        || !support.opens().iter().any(|o| Arc::ptr_eq(o, open))
    {
        return Err(Error::Invalid("supported center lift embedding owner"));
    }
    if !unit(frame.local(), open.relative_minor(), b)? {
        return Ok(None);
    }
    let normals = || {
        open.chosen_equations()
            .iter()
            .map(|i| support.clearings()[*i].numerator.clone())
            .collect()
    };
    let lifted = super::super::recursive::lift_embedded_geometry(
        frame,
        parent,
        open.frame(),
        open.extension(),
        normals(),
        child,
        child_normals,
        b,
    )?;
    let _ = super::super::recursive::lift_embedded_geometry(
        frame,
        companion,
        open.frame(),
        open.extension(),
        normals(),
        child,
        child_normals,
        b,
    )?;
    Ok(Some(lifted))
}
