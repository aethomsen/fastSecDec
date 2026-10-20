//! Faithful fixed-point conversion before native propagator-family arithmetic.
//! This owner provides neither geometry authority nor a generated integral.
mod identity;
mod payload;
pub use payload::GraphPayloadPolicy;

use super::{Error, Limits, LiteralConversion, NumericalMeaning, Result, Work};
use crate::{
    Atom, AtomCore, EdgeId, FeynmanDiagram, Kinematics, Symbol, input::GraphIntegral,
    parametric::ParametricIntegrand,
};
use std::{collections::BTreeMap, ops::ControlFlow, sync::Arc};

/// Unprepared native inputs. No family, parameterization or scalar arithmetic is
/// performed by storing this value. All fields are validated during preparation.
#[derive(Clone, Debug)]
pub struct GraphPoint {
    pub diagram: Arc<FeynmanDiagram>,
    pub kinematics: Arc<Kinematics>,
    pub scalar_values: BTreeMap<Symbol, Atom>,
    pub auxiliary_momenta: Vec<Atom>,
    pub powers: BTreeMap<EdgeId, u32>,
    pub measure_multiplier: Atom,
    pub coordinates: Vec<Symbol>,
    pub regulator: Symbol,
    pub dimension: Atom,
}

/// Canonical native source-role association, independent of callback order.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GraphLocation {
    /// All native dot/metric keys owning this identical original value.
    KinematicValue {
        keys: Vec<String>,
    },
    ScalarBinding {
        symbol: String,
    },
    Measure {},
    /// Scanned for exactness only; a Float here is explicitly unsupported.
    ExactPayload {
        role: String,
    },
    /// Applied primitive/global payload under an explicit numerical policy.
    Payload {
        role: String,
    },
    /// Observation only: never an additional factor in the converted graph.
    OriginalAggregate {},
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    ExactPayload,
    Kinematics,
    ScalarBindings,
    Measure,
    Parameterization,
    Complete,
    Payload,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub stage: Stage,
    pub converted_literals: usize,
}

/// Original point and exact native parameterization, with conversion evidence.
///
/// Float values are supported in kinematic assumptions, closed scalar bindings
/// and the measure. By default graph numerator/weight/projector payloads must
/// be exact. The existing generation PreparationOptions can explicitly opt into
/// GraphPayloadPolicy::PrimitiveFragmentsV1; this chooses a different numerical
/// interpretation of the stored local fragments, retaining the original rounded
/// aggregate as provenance. Uncertainty-bearing values are never exactified.
/// The original native graph/model/routing owner is shared, not reconstructed.
#[derive(Clone, Debug)]
pub struct ExactRepresentedGraphInput {
    original: Arc<GraphPoint>,
    payload_policy: Option<GraphPayloadPolicy>,
    exact_diagram: Arc<FeynmanDiagram>,
    exact_kinematics: Arc<Kinematics>,
    exact_scalar_values: BTreeMap<Symbol, Atom>,
    exact_measure: Atom,
    exact: Arc<ParametricIntegrand>,
    conversions: Vec<LiteralConversion<GraphLocation>>,
    meaning: NumericalMeaning,
    limits: Limits,
    source_identity: String,
    source_witness: String,
}

impl PartialEq for ExactRepresentedGraphInput {
    fn eq(&self, other: &Self) -> bool {
        self.source_witness == other.source_witness
            && self.exact == other.exact
            && self.limits == other.limits
    }
}
impl Eq for ExactRepresentedGraphInput {}

fn require_exact(work: &mut Work<GraphLocation>, expression: &Atom, role: String) -> Result<()> {
    let before = work.conversions.len();
    let _ = work.atom(expression, GraphLocation::ExactPayload { role })?;
    if work.conversions.len() != before {
        return Err(Error::Unsupported(
            "Float outside kinematic values, scalar bindings and measure",
        ));
    }
    Ok(())
}

impl ExactRepresentedGraphInput {
    pub fn original(&self) -> &Arc<GraphPoint> {
        &self.original
    }
    /// The explicit primitive interpretation, absent on the legacy path.
    pub fn payload_policy(&self) -> Option<GraphPayloadPolicy> {
        self.payload_policy
    }
    /// Effective native graph. The unchanged original remains available above.
    pub fn exact_diagram(&self) -> &Arc<FeynmanDiagram> {
        &self.exact_diagram
    }
    pub fn exact_kinematics(&self) -> &Arc<Kinematics> {
        &self.exact_kinematics
    }
    pub fn exact_scalar_values(&self) -> &BTreeMap<Symbol, Atom> {
        &self.exact_scalar_values
    }
    pub fn exact_measure(&self) -> &Atom {
        &self.exact_measure
    }
    pub fn exact(&self) -> &Arc<ParametricIntegrand> {
        &self.exact
    }
    pub fn conversions(&self) -> &[LiteralConversion<GraphLocation>] {
        &self.conversions
    }
    pub fn meaning(&self) -> NumericalMeaning {
        self.meaning
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    /// Original numerical meaning and source roles, not just the exact density.
    pub fn source_identity(&self) -> &str {
        &self.source_identity
    }

    // Full native/canonical association retained for replay equality, not a
    // graph parser, geometry certificate, or hash-only authorization.
    pub(crate) fn source_witness(&self) -> &str {
        &self.source_witness
    }

    pub fn prepare(
        original: Arc<GraphPoint>,
        meaning: NumericalMeaning,
        limits: Limits,
        observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        Self::prepare_with_payloads(original, meaning, limits, None, observer)
    }

    pub(crate) fn prepare_with_payloads(
        original: Arc<GraphPoint>,
        meaning: NumericalMeaning,
        limits: Limits,
        payload_policy: Option<GraphPayloadPolicy>,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        let mut work = Work::<GraphLocation>::new(meaning, limits)?;
        let mut observe = |stage, work: &Work<GraphLocation>| {
            if observer(Progress {
                stage,
                converted_literals: work.conversions.len(),
            })
            .is_break()
            {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        };
        observe(Stage::ExactPayload, &work)?;
        let exact_diagram = if payload_policy.is_some() {
            observe(Stage::Payload, &work)?;
            let graph = payload::convert(&original.diagram, &mut work, |work| {
                observe(Stage::Payload, work)
            })?;
            require_exact(
                &mut work,
                &original.dimension,
                "integration_dimension".into(),
            )?;
            Arc::new(graph)
        } else {
            // Scan every local fragment as well as aggregate/global factors before
            // native validation can contract or combine any of these expressions.
            for (role, expression) in [
                ("overall_factor", original.diagram.overall_factor()),
                ("numerator", original.diagram.numerator().expression()),
                (
                    "numerator_prefactor",
                    original.diagram.numerator_prefactor().expression(),
                ),
                ("projector", original.diagram.projector().expression()),
                ("integration_dimension", &original.dimension),
            ] {
                require_exact(&mut work, expression, role.into())?;
            }
            for (id, vertex) in original.diagram.vertices() {
                require_exact(
                    &mut work,
                    vertex.numerator.expression(),
                    format!("vertex_numerator:{}", id.0),
                )?;
            }
            for (id, _, edge) in original.diagram.edges() {
                require_exact(
                    &mut work,
                    edge.numerator.expression(),
                    format!("edge_numerator:{}", id.0),
                )?;
            }
            original.diagram.clone()
        };
        for momentum in original
            .kinematics
            .momenta()
            .chain(&original.auxiliary_momenta)
        {
            require_exact(&mut work, momentum, "formal_momentum".into())?;
        }
        observe(Stage::Kinematics, &work)?;
        let mut associations = BTreeMap::<&Atom, Vec<String>>::new();
        for (index, (key, value)) in original.kinematics.scalar_assumptions().enumerate() {
            if index >= limits.nodes {
                return Err(Error::ResourceIncomplete("kinematic assumption inventory"));
            }
            require_exact(&mut work, key, "kinematic_key".into())?;
            associations
                .entry(value)
                .or_default()
                .push(key.to_canonical_string());
        }
        for keys in associations.values_mut() {
            keys.sort();
        }
        let exact_kinematics = Arc::new(original.kinematics.try_map_scalar_values(|value| {
            work.atom(
                value,
                GraphLocation::KinematicValue {
                    keys: associations[value].clone(),
                },
            )
        })?);
        observe(Stage::ScalarBindings, &work)?;
        let exact_scalar_values = original
            .scalar_values
            .iter()
            .map(|(symbol, value)| {
                work.atom(
                    value,
                    GraphLocation::ScalarBinding {
                        symbol: Atom::var(*symbol).to_canonical_string(),
                    },
                )
                .map(|value| (*symbol, value))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;
        observe(Stage::Measure, &work)?;
        let exact_measure = work.atom(&original.measure_multiplier, GraphLocation::Measure {})?;
        observe(Stage::Parameterization, &work)?;
        let graph = GraphIntegral::new_with_scalar_values(
            exact_diagram.clone(),
            &exact_kinematics,
            &exact_scalar_values,
        )
        .and_then(|graph| graph.with_auxiliary_external_momenta(&original.auxiliary_momenta))
        .and_then(|graph| graph.with_powers(&original.powers))
        .map_err(|error| Error::Invalid(error.to_string()))?
        .with_measure_multiplier(exact_measure.clone());
        let exact = Arc::new(
            ParametricIntegrand::from_graph(
                &graph,
                original.coordinates.clone(),
                original.regulator,
                original.dimension.clone(),
            )
            .map_err(|error| Error::Invalid(error.to_string()))?,
        );
        work.conversions.sort_by_cached_key(|row| {
            (
                row.location.clone(),
                row.original.to_canonical_string(),
                row.exact.to_canonical_string(),
                row.precision_bits,
                row.binary_exponents,
            )
        });
        let mut result = Self {
            original,
            payload_policy,
            exact_diagram,
            exact_kinematics,
            exact_scalar_values,
            exact_measure,
            exact,
            conversions: work.conversions,
            meaning,
            limits,
            source_identity: String::new(),
            source_witness: String::new(),
        };
        result.source_witness = identity::witness(&result)?;
        result.source_identity = blake3::hash(result.source_witness.as_bytes())
            .to_hex()
            .to_string();
        if observer(Progress {
            stage: Stage::Complete,
            converted_literals: result.conversions.len(),
        })
        .is_break()
        {
            return Err(Error::Cancelled);
        }
        Ok(result)
    }
}
