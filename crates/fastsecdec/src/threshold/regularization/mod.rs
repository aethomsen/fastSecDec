//! First certified rational interval bridge, not a general resolver.
mod atlas;
mod bound;
pub mod meromorphic;
pub mod projective_family;
pub mod projective_pair_family;
pub mod secant;
pub mod sign_definite;
pub use bound::BoundContinuation;
mod normalize;

use super::{
    gcad::{DomainOrigin, GcadError, VerifiedDecomposition},
    maps::{CellMap, ParameterAdmission},
    phase::CausalCell,
};
use crate::{
    generation::{GenerationOptions, endpoint_power_with_regulators},
    parametric::FactorRole,
};
use std::{collections::BTreeMap, ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol, SymbolAttribute, SymbolBuilder},
    domains::{atom::AtomField, rational::Rational},
    evaluate::{FunctionMap, FunctionRegistrationOptions, InliningPolicy},
    id::{Pattern, Replacement},
    wrap_symbol,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid rational endpoint bridge: {0}")]
    Invalid(String),
    #[error("unsupported rational endpoint bridge: {0}")]
    Unsupported(String),
    #[error("rational endpoint bridge cancelled")]
    Cancelled,
    #[error("rational endpoint bridge resource limit: {0}")]
    ResourceIncomplete(&'static str),
    #[error(transparent)]
    Geometry(#[from] GcadError),
    #[error(transparent)]
    Generation(#[from] crate::generation::GenerationError),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Strict bounds for a common open real-epsilon interval. The intersection is
/// established before finite chart seams are omitted or continuation starts.
#[derive(Clone, Debug, Default)]
pub struct ConvergenceStrip {
    lower: Option<Rational>,
    upper: Option<Rational>,
}
impl ConvergenceStrip {
    pub fn lower(&self) -> Option<&Rational> {
        self.lower.as_ref()
    }
    pub fn upper(&self) -> Option<&Rational> {
        self.upper.as_ref()
    }
    pub(crate) fn admit(&mut self, power: &Atom, eps: Symbol) -> Result<()> {
        let (constant, slopes) = endpoint_power_with_regulators(power, &[eps])
            .map_err(|e| unsupported(&e.to_string()))?;
        let slope = &slopes[0];
        if slope.is_zero() {
            if constant <= -1 {
                return Err(unsupported(
                    "no common epsilon convergence strip for an unregulated endpoint",
                ));
            }
            return Ok(());
        }
        let boundary = (Rational::from(-1) - constant) / slope;
        if slope > &Rational::zero() {
            if self.lower.as_ref().is_none_or(|l| l < &boundary) {
                self.lower = Some(boundary);
            }
        } else if self.upper.as_ref().is_none_or(|u| u > &boundary) {
            self.upper = Some(boundary);
        }
        if let (Some(l), Some(u)) = (&self.lower, &self.upper)
            && l >= u
        {
            return Err(unsupported(
                "cell family has no common epsilon convergence strip",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_charts: usize,
    pub max_degree: u32,
    pub prefactors: meromorphic::Limits,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_charts: 128,
            max_degree: 256,
            prefactors: meromorphic::Limits::default(),
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Progress {
    Cell(usize),
    Chart(usize),
    Factor {
        chart: usize,
        term: usize,
        factor: usize,
    },
    Continue(usize),
    PrefactorWitness(usize),
}
fn poll(progress: Progress, observer: &mut impl FnMut(Progress) -> ControlFlow<()>) -> Result<()> {
    if observer(progress).is_break() {
        Err(Error::Cancelled)
    } else {
        Ok(())
    }
}
fn substitute(expression: &Atom, values: &BTreeMap<Symbol, Rational>) -> Atom {
    expression.replace_multiple(values.iter().map(|(s, v)| {
        Replacement::new(
            Pattern::Literal(Atom::var(*s)),
            Pattern::Literal(Atom::num(v.clone())),
        )
    }))
}
fn exact(expression: &Atom) -> Result<Rational> {
    Rational::try_from(expression.cancel().as_view())
        .map_err(|e| unsupported(&format!("expected exact rational fiber expression: {e}")))
}
fn invalid(message: &str) -> Error {
    Error::Invalid(message.into())
}
fn unsupported(message: &str) -> Error {
    Error::Unsupported(message.into())
}

#[derive(Clone, Debug)]
pub struct ClosedUnit {
    pub(super) original_factor: Option<usize>,
    pub(super) multiplicity: u32,
    pub(super) residual: Atom,
    pub(super) lower_value: Rational,
    pub(super) upper_value: Rational,
}
impl ClosedUnit {
    pub fn factor_index(&self) -> Option<usize> {
        self.original_factor
    }
    pub fn multiplicity(&self) -> u32 {
        self.multiplicity
    }
    pub fn residual(&self) -> &Atom {
        &self.residual
    }
    pub fn endpoint_values(&self) -> (&Rational, &Rational) {
        (&self.lower_value, &self.upper_value)
    }
}
#[derive(Clone, Debug)]
struct NormalizedTerm {
    prefactor: Atom,
    regular: Atom,
    power: Atom,
    units: Vec<ClosedUnit>,
}
#[derive(Debug)]
pub struct IntervalChart {
    admission: ParameterAdmission,
    image: Atom,
    measure: Atom,
    orientation: i8,
    bounds: (Rational, Rational),
    terms: Vec<NormalizedTerm>,
}
impl IntervalChart {
    pub(crate) fn normalized_term(&self, index: usize) -> Option<(&Atom, &Atom, &Atom)> {
        self.terms
            .get(index)
            .map(|t| (&t.prefactor, &t.regular, &t.power))
    }
    pub fn admission(&self) -> &ParameterAdmission {
        &self.admission
    }
    pub fn image(&self) -> &Atom {
        &self.image
    }
    pub fn positive_measure(&self) -> &Atom {
        &self.measure
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn interval(&self) -> (&Rational, &Rational) {
        (&self.bounds.0, &self.bounds.1)
    }
    pub fn units(&self, term: usize) -> Option<&[ClosedUnit]> {
        self.terms.get(term).map(|t| t.units.as_slice())
    }
}
/// The constructor owns the coverage, unit and strip proofs. No raw tuple or
/// supplied boolean can create this capability.
#[derive(Debug)]
pub struct RegularizedFiber {
    owner: Arc<VerifiedDecomposition>,
    parameters: BTreeMap<Symbol, Rational>,
    unit: Symbol,
    numerator: Symbol,
    numerator_bodies: Vec<Atom>,
    charts: Vec<IntervalChart>,
    strip: ConvergenceStrip,
    prefactor_witness: meromorphic::MeromorphicWitness,
}
impl RegularizedFiber {
    pub fn admit(
        owner: Arc<VerifiedDecomposition>,
        parameters: BTreeMap<Symbol, Rational>,
        unit: Symbol,
        limits: Limits,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        atlas::admit(owner, parameters, unit, limits, &mut observer)
    }
    pub fn decomposition(&self) -> &Arc<VerifiedDecomposition> {
        &self.owner
    }
    pub fn parameters(&self) -> &BTreeMap<Symbol, Rational> {
        &self.parameters
    }
    pub fn charts(&self) -> &[IntervalChart] {
        &self.charts
    }
    pub fn convergence_strip(&self) -> &ConvergenceStrip {
        &self.strip
    }
    pub fn prefactor_witness(&self) -> &meromorphic::MeromorphicWitness {
        &self.prefactor_witness
    }
    pub fn continue_symbolically(
        &self,
        options: &GenerationOptions,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<ContinuedFiber<'_>> {
        if options.mode != crate::generation::GenerationMode::Symbolic
            || options.contour_enabled()
            || options.contour_jacobian != crate::contour::ContourJacobian::Symbolic
            || options.source_sectors.is_some()
        {
            return Err(unsupported(
                "continuation requires symbolic endpoints without a contour or source-sector selection",
            ));
        }
        let mut expressions = Vec::with_capacity(self.charts.len());
        let mut profiles = Vec::with_capacity(self.charts.len());
        for (index, chart) in self.charts.iter().enumerate() {
            poll(Progress::Continue(index), &mut observer)?;
            let continued = crate::generation::threshold_subtract(
                chart
                    .terms
                    .iter()
                    .map(|term| {
                        (
                            term.prefactor.clone(),
                            term.regular.clone(),
                            vec![term.power.clone()],
                        )
                    })
                    .collect(),
                &[self.unit],
                &[self.owner.request().input().regulator()],
                options,
            )
            .map_err(Error::Generation)?;
            profiles.push(continued.endpoint_profiles);
            expressions.push(continued.expression);
        }
        // Never discard individual cell poles. This native sum is the object
        // passed to a later Laurent pass and coherent evaluator.
        let expression = expressions.iter().cloned().sum();
        Ok(ContinuedFiber {
            certificate: self,
            expression,
            chart_expressions: expressions,
            profiles,
            functions: self.functions(options.max_subtractions_per_axis, false, &mut observer)?,
            derivative_order: options.max_subtractions_per_axis,
            generation_options: options.clone(),
        })
    }
    fn definitions(
        &self,
        degree: usize,
        bind_parameters: bool,
        observer: &mut impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<Vec<BoundDefinition>> {
        let source = self.owner.request().domain().coordinates()[0];
        let mut definitions = Vec::new();
        for (term, body) in self.numerator_bodies.iter().enumerate() {
            poll(Progress::Continue(term), observer)?;
            let body = if bind_parameters {
                let value = substitute(body, &self.parameters);
                bound::no_physical_parameters(&value, self)?;
                value
            } else {
                body.clone()
            };
            definitions.push(BoundDefinition {
                head: self.numerator,
                tags: vec![Atom::num(term)],
                formals: vec![source],
                body: body.clone(),
                derivative_order: None,
            });
            let mut partial = body;
            for order in 1..=degree {
                poll(Progress::Continue(term), observer)?;
                partial = partial.derivative(source);
                definitions.push(BoundDefinition {
                    head: Symbol::DERIVATIVE,
                    tags: vec![
                        Atom::Zero,
                        Atom::num(order),
                        Atom::var(self.numerator),
                        Atom::num(term),
                    ],
                    formals: vec![source],
                    body: partial.clone(),
                    derivative_order: Some(order),
                });
            }
        }
        Ok(definitions)
    }
    fn functions(
        &self,
        degree: usize,
        bind_parameters: bool,
        observer: &mut impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<FunctionMap> {
        register_definitions(&self.definitions(degree, bind_parameters, observer)?)
    }
}
/// Crate-private native definition view for the existing Atom-table staging
/// codec. This record alone conveys no continuation or publication authority.
#[derive(Clone, Debug)]
pub(crate) struct BoundDefinition {
    head: Symbol,
    tags: Vec<Atom>,
    formals: Vec<Symbol>,
    body: Atom,
    derivative_order: Option<usize>,
}
impl BoundDefinition {
    pub(crate) fn head(&self) -> Symbol {
        self.head
    }
    pub(crate) fn tags(&self) -> &[Atom] {
        &self.tags
    }
    pub(crate) fn formals(&self) -> &[Symbol] {
        &self.formals
    }
    pub(crate) fn body(&self) -> &Atom {
        &self.body
    }
    pub(crate) fn derivative_order(&self) -> Option<usize> {
        self.derivative_order
    }
}
fn register_definitions(definitions: &[BoundDefinition]) -> Result<FunctionMap> {
    let mut functions = FunctionMap::new();
    for definition in definitions {
        let options = if definition.derivative_order().is_some() {
            FunctionRegistrationOptions::new().inlining(InliningPolicy::Always)
        } else {
            FunctionRegistrationOptions::new()
        };
        functions
            .add_tagged_function_with_options(
                definition.head(),
                definition.tags().to_vec(),
                definition.formals().to_vec(),
                definition.body().clone(),
                options,
            )
            .map_err(|e| invalid(&e.to_string()))?;
    }
    Ok(functions)
}

pub struct ContinuedFiber<'a> {
    generation_options: GenerationOptions,
    certificate: &'a RegularizedFiber,
    expression: Atom,
    chart_expressions: Vec<Atom>,
    profiles: Vec<Vec<crate::generation::EndpointProfileRow>>,
    functions: FunctionMap,
    derivative_order: usize,
}
impl ContinuedFiber<'_> {
    pub fn certificate(&self) -> &RegularizedFiber {
        self.certificate
    }
    pub fn expression(&self) -> &Atom {
        &self.expression
    }
    pub fn chart_expressions(&self) -> &[Atom] {
        &self.chart_expressions
    }
    pub fn profiles(&self) -> &[Vec<crate::generation::EndpointProfileRow>] {
        &self.profiles
    }
    /// Low-level native compiler input, not an independently rebound program.
    /// Keep this certificate alive and supply exactly its admitted parameters.
    /// No unchecked caller value, raw function map, or bare expression carries
    /// the coverage/unit/strip proof. A later owned compiler must enforce this.
    pub fn functions(&self) -> &FunctionMap {
        &self.functions
    }
}

#[cfg(test)]
mod tests;

fn prefactor_error(error: meromorphic::Error) -> Error {
    match error {
        meromorphic::Error::Cancelled => Error::Cancelled,
        meromorphic::Error::ResourceIncomplete(s) => Error::ResourceIncomplete(s),
        meromorphic::Error::Unsupported(s) => unsupported(s),
        meromorphic::Error::Invalid(s) => invalid(s),
        meromorphic::Error::Native(s) => unsupported(&format!("native prefactor admission: {s}")),
    }
}
