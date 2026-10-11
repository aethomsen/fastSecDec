//! Relative next-center adaptation, retaining the actual recursive owner.
use super::super::*;
use super::{
    graph::{GraphCoordinateOpen, graph_coordinates},
    helpers::*,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub enum RecursiveCenterOrigin {
    DispatchedContact(Arc<DispatchedContactCenter>),
    LocalizedComponent(Arc<LocalizedComponentCenter>),
    BoundaryFree {
        center: Arc<RecursiveCenter>,
        history: Arc<ResolutionHistory>,
    },
    Companion(Arc<CompanionCenter>),
    CarriedMonomial(Arc<super::induced::CarriedMonomialCenter>),
    EmbeddedMonomial(Arc<super::embedded_center::EmbeddedMonomialCenter>),
    EmbeddedProblem(Arc<super::problem_center::EmbeddedProblemCenter>),
    OriginalProblem(Arc<super::original_problem::OriginalProblemCenter>),
    SupportedProblem(Arc<super::continued_problem::ContinuedSupportedCenter>),
}
#[derive(Clone, Debug)]
pub struct CheckedRecursiveCenter {
    origin: RecursiveCenterOrigin,
    history: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    ideal: Ideal,
    normals: Vec<Poly>,
    contained: Vec<bool>,
    normal_presentation: Option<Arc<super::normal_presentation::CenterNormalPresentation>>,
}
impl CheckedRecursiveCenter {
    pub fn origin(&self) -> &RecursiveCenterOrigin {
        &self.origin
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn ideal(&self) -> &Ideal {
        &self.ideal
    }
    pub fn normals(&self) -> &[Poly] {
        &self.normals
    }
    pub fn contained_boundaries(&self) -> &[bool] {
        &self.contained
    }
    pub fn normal_presentation(
        &self,
    ) -> Option<&Arc<super::normal_presentation::CenterNormalPresentation>> {
        self.normal_presentation.as_ref()
    }
    /// Optional same-center presentation. Existing rank/transversality/cover
    /// checks remain required; native normalization is not a smoothness proof.
    pub fn with_simplified_normals(self: &Arc<Self>, b: &mut Budget) -> Result<Arc<Self>> {
        if self.normal_presentation.is_some() {
            return Ok(self.clone());
        }
        let receipt = super::normal_presentation::prepare(self.clone(), b)?;
        b.reserve_slots(receipt.selected().ideal().generators().len())?;
        Ok(Arc::new(Self {
            origin: self.origin.clone(),
            history: self.history.clone(),
            source: self.source.clone(),
            ideal: self.ideal.clone(),
            normals: receipt.selected().ideal().generators().to_vec(),
            contained: self.contained.clone(),
            normal_presentation: Some(receipt),
        }))
    }
    pub fn new(origin: RecursiveCenterOrigin, b: &mut Budget) -> Result<Arc<Self>> {
        let (frame, history, source, ideal, normals) = match &origin {
            RecursiveCenterOrigin::DispatchedContact(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::LocalizedComponent(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::BoundaryFree { center, history } => (
                center.frame(),
                history,
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::Companion(center) => (
                center.frame(),
                center.coefficient().source().restriction().history(),
                center.parent_source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::CarriedMonomial(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::EmbeddedMonomial(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::EmbeddedProblem(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::OriginalProblem(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
            RecursiveCenterOrigin::SupportedProblem(center) => (
                center.frame(),
                center.history(),
                center.source(),
                center.ideal(),
                center.normals(),
            ),
        };
        if !Arc::ptr_eq(frame, history.ledger().frame())
            || source.ideal().ring() != frame.local().ring()
            || ideal.ring() != frame.local().ring()
            || normals.is_empty()
        {
            return Err(Error::Invalid("recursive center geometry owners"));
        }
        let local = frame.local();
        let generated = Ideal::new(local.ring().clone(), normals.to_vec(), b)?;
        let actual = local.ideal().sum(ideal, b)?;
        let supplied = local.ideal().sum(&generated, b)?;
        for f in actual.generators() {
            if !supplied.contains(f, local.unit_relations(), b)? {
                return Err(Error::Invalid("center normals miss original ideal"));
            }
        }
        for f in supplied.generators() {
            if !actual.contains(f, local.unit_relations(), b)? {
                return Err(Error::Invalid("center normals enlarge original ideal"));
            }
        }
        let mut layer = source.ideal().clone();
        for order in 0..source.mark() {
            for f in layer.generators() {
                if !actual.contains(f, local.unit_relations(), b)? {
                    return Err(Error::Invalid("recursive center outside parent cosupport"));
                }
            }
            if order + 1 < source.mark() {
                layer = super::super::differential::relative_differential_ideal(frame, &layer, b)?;
            }
        }
        b.reserve_slots(history.ledger().divisors().len())?;
        let contained = history
            .ledger()
            .divisors()
            .iter()
            .map(|d| actual.contains(&d.equation, local.unit_relations(), b))
            .collect::<Result<Vec<_>>>()?;
        Ok(Arc::new(Self {
            history: history.clone(),
            source: source.clone(),
            ideal: ideal.clone(),
            normals: normals.to_vec(),
            contained,
            normal_presentation: None,
            origin,
        }))
    }
}

#[derive(Clone, Debug)]
pub struct RecursiveAdaptedOpen {
    pub(super) geometry: Arc<GraphCoordinateOpen>,
    pub(super) incidence: Vec<usize>,
    pub(super) center_axes: Vec<usize>,
    pub(super) identity_side: Option<usize>,
}
impl RecursiveAdaptedOpen {
    pub fn source_open(&self) -> &Poly {
        &self.geometry.source_open
    }
    pub fn columns(&self) -> &[usize] {
        &self.geometry.columns
    }
    pub fn functions(&self) -> &[Poly] {
        &self.geometry.functions
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        &self.geometry.graph
    }
    pub fn center_axes(&self) -> &[usize] {
        &self.center_axes
    }
    pub fn incidence(&self) -> &[usize] {
        &self.incidence
    }
    pub fn identity_side(&self) -> Option<usize> {
        self.identity_side
    }
}
#[derive(Clone, Debug)]
pub struct RecursiveAdaptation {
    center: Arc<CheckedRecursiveCenter>,
    opens: Vec<Arc<RecursiveAdaptedOpen>>,
    empty: Vec<EmptyAdaptedOpen>,
    center_proof: VerifiedOpenCover,
    proof: VerifiedOpenCover,
}
impl RecursiveAdaptation {
    pub fn center(&self) -> &Arc<CheckedRecursiveCenter> {
        &self.center
    }
    pub fn opens(&self) -> &[Arc<RecursiveAdaptedOpen>] {
        &self.opens
    }
    pub fn empty(&self) -> &[EmptyAdaptedOpen] {
        &self.empty
    }
    pub fn proof(&self) -> &VerifiedOpenCover {
        &self.proof
    }
    pub fn center_proof(&self) -> &VerifiedOpenCover {
        &self.center_proof
    }
}
pub fn adapt_recursive_center(
    center: Arc<CheckedRecursiveCenter>,
    namespace: &str,
    b: &mut Budget,
) -> Result<Arc<RecursiveAdaptation>> {
    let ledger = center.history.ledger();
    let frame = ledger.frame();
    let local = frame.local();
    let n = ledger.divisors().len();
    let r = center.normals.len();
    let shift =
        u32::try_from(n).map_err(|_| Error::ResourceIncomplete("center incidence count"))?;
    let count = 1usize
        .checked_shl(shift)
        .ok_or(Error::ResourceIncomplete("center incidence count"))?;
    b.reserve_slots(count)?;
    let mut opens = Vec::new();
    let mut empty = Vec::new();
    for mask in 0..count {
        let incidence = (0..n).filter(|i| mask & (1 << i) != 0).collect::<Vec<_>>();
        let locus = center.ideal.sum(
            &Ideal::new(
                local.ring().clone(),
                incidence
                    .iter()
                    .map(|i| ledger.divisors()[*i].equation.clone())
                    .collect(),
                b,
            )?,
            b,
        )?;
        let locus = local.ideal().sum(&locus, b)?;
        let locus_empty = locus.contains(&local.ring().one(), local.unit_relations(), b)?;
        let mut functions = center.normals.clone();
        functions.extend(
            incidence
                .iter()
                .filter(|i| !center.contained[**i])
                .map(|i| ledger.divisors()[*i].equation.clone()),
        );
        if functions.len() > frame.free_axes().len() {
            if !locus_empty {
                return Err(Error::ResourceIncomplete(
                    "center needs boundary/component refinement",
                ));
            }
            // Retain the exact empty intersection even without a candidate minor.
            empty.push(EmptyAdaptedOpen {
                incidence: incidence.clone(),
                columns: vec![],
                ideal: locus,
                guards: local.guards().to_vec(),
            });
            continue;
        }
        let extra_units = ledger
            .divisors()
            .iter()
            .enumerate()
            .filter(|(i, _)| !incidence.contains(i))
            .map(|(_, d)| d.equation.clone())
            .collect::<Vec<_>>();
        for columns in combinations(frame.free_axes().len(), functions.len(), b)? {
            let name = format!(
                "{namespace}_inc{mask}_cols{}",
                columns
                    .iter()
                    .map(usize::to_string)
                    .collect::<Vec<_>>()
                    .join("_")
            );
            match graph_coordinates(frame.clone(), &functions, &columns, &extra_units, &name, b)? {
                Ok(geometry) => {
                    b.reserve_slots(1)?;
                    let center_axes = geometry.coordinate_axes[..r].to_vec();
                    opens.push(Arc::new(RecursiveAdaptedOpen {
                        geometry,
                        incidence: incidence.clone(),
                        center_axes,
                        identity_side: None,
                    }));
                }
                Err(e) => empty.push(EmptyAdaptedOpen {
                    incidence: incidence.clone(),
                    columns,
                    ideal: e.ideal,
                    guards: e.guards,
                }),
            }
        }
    }
    let center_proof = OpenCoverCertificate {
        algebra: local.clone(),
        support: center.ideal.clone(),
        opens: opens.iter().map(|o| o.source_open().clone()).collect(),
    }
    .verify(b);
    let center_proof = match center_proof {
        Ok(v) => v,
        Err(Error::Invalid(_)) => {
            return Err(Error::ResourceIncomplete(
                "center rank/normal-crossing cover incomplete",
            ));
        }
        Err(e) => return Err(e),
    };
    for (i, f) in center.normals.iter().enumerate() {
        match graph_coordinates(
            frame.clone(),
            &[],
            &[],
            std::slice::from_ref(f),
            &format!("{namespace}_outside{i}"),
            b,
        )? {
            Ok(geometry) => {
                b.reserve_slots(1)?;
                opens.push(Arc::new(RecursiveAdaptedOpen {
                    geometry,
                    incidence: vec![],
                    center_axes: vec![],
                    identity_side: Some(i),
                }));
            }
            Err(e) => empty.push(EmptyAdaptedOpen {
                incidence: vec![],
                columns: vec![],
                ideal: e.ideal,
                guards: e.guards,
            }),
        }
    }
    let proof = OpenCoverCertificate {
        algebra: local.clone(),
        support: Ideal::new(local.ring().clone(), vec![], b)?,
        opens: opens.iter().map(|o| o.source_open().clone()).collect(),
    }
    .verify(b)?;
    Ok(Arc::new(RecursiveAdaptation {
        center,
        opens,
        empty,
        center_proof,
        proof,
    }))
}
