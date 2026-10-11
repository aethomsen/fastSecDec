use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    id::{Pattern, Replacement},
    prelude::{PolyVariable, Q, Rational},
};
use symgcad::{
    Limits, Problem, SolverOptions,
    algebra::{Algebra, Poly},
};

use super::{GcadError, Result};
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
    },
    threshold::projective::{AffineProjectivePreparation, FactorOrigin},
};

/// The role is retained independently of the chosen native lifting order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AliasRole {
    RuntimeParameter,
    IntegrationCoordinate,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolAlias {
    pub symbol: Symbol,
    pub name: String,
    pub role: AliasRole,
}

/// A prepared domain is a declared native geometry, not a proof of an original
/// projective/source integral's coverage. That proof belongs to later stages.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainOrigin {
    NativeUnitCube,
    AffineProjective { eliminated_index: usize },
    ExplicitPrepared { provenance: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedDomain {
    coordinates: Vec<Symbol>,
    strict_positive: Vec<Atom>,
    origin: DomainOrigin,
}

impl PreparedDomain {
    /// Declare an already prepared real semialgebraic domain in these exact
    /// integration coordinates. No pullback, Jacobian or coverage is invented.
    pub fn explicit(
        coordinates: Vec<Symbol>,
        strict_positive: Vec<Atom>,
        provenance: String,
    ) -> Result<Self> {
        if provenance.trim().is_empty() {
            return Err(GcadError::Invalid(
                "prepared-domain provenance is required".into(),
            ));
        }
        Ok(Self {
            coordinates,
            strict_positive,
            origin: DomainOrigin::ExplicitPrepared { provenance },
        })
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    pub fn strict_positive(&self) -> &[Atom] {
        &self.strict_positive
    }
    pub fn origin(&self) -> &DomainOrigin {
        &self.origin
    }
}

/// Exact specializations and the remaining real parameter fiber. Constraints
/// are strict-positive native atoms and must depend only on runtime parameters.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GcadKinematics {
    pub exact_values: BTreeMap<Symbol, Rational>,
    pub runtime_parameters: Vec<Symbol>,
    pub strict_positive: Vec<Atom>,
}

impl GcadKinematics {
    /// Apply only the declared exact bindings with Symbolica's simultaneous
    /// replacement. Use this for the complete density as well as its geometry;
    /// specializing a denominator alone does not specialize numerator or phase.
    pub fn specialize_exact(&self, expression: &Atom) -> Atom {
        expression.replace_multiple(self.exact_values.iter().map(|(symbol, value)| {
            Replacement::new(
                Pattern::Literal(Atom::var(*symbol)),
                Pattern::Literal(Atom::num(value.clone())),
            )
        }))
    }

    /// Preserve every finite IEEE value exactly using Numerica's native
    /// conversion, without guessing small rational kinematics.
    pub fn from_f64(values: &BTreeMap<Symbol, f64>) -> Result<Self> {
        let exact_values = values
            .iter()
            .map(|(s, v)| {
                Rational::try_from(*v)
                    .map(|r| (*s, r))
                    .map_err(|e| GcadError::Invalid(e.into()))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            exact_values,
            ..Self::default()
        })
    }
}

/// Original physical factor association. The exponent remains a complete Atom,
/// including epsilon and future auxiliary regulators; no phase is frozen here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignedFactor {
    pub term_index: usize,
    /// Index in the prepared term. `origin` identifies the original density factor.
    pub factor_index: usize,
    pub origin: FactorOrigin,
    pub split_index: usize,
    pub original_polynomial: Atom,
    pub prepared_polynomial: Atom,
    pub exponent: Atom,
    pub semantics: FactorSemantics,
}

/// Mutually exclusive source interpretation. These immutable native owners
/// record input meaning; they are not geometry or endpoint certificates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceProvenance {
    Exact,
    ParametricRepresented(Arc<crate::threshold::represented::ExactRepresentedInput>),
    PreparametricGraph(Arc<crate::threshold::represented::graph::ExactRepresentedGraphInput>),
    PreparametricFamily(Arc<crate::threshold::represented::family::ExactRepresentedFamilyInput>),
}

/// Exact, typed request association carried with unverified native evidence.
/// This includes density/domain/alias roles as well as native solver settings.
#[derive(Clone, Debug)]
pub struct RequestIdentity {
    input: Arc<ParametricIntegrand>,
    preparation: Option<Arc<AffineProjectivePreparation>>,
    provenance: SourceProvenance,
    domain: PreparedDomain,
    kinematics: GcadKinematics,
    aliases: Vec<SymbolAlias>,
    factors: Vec<SignedFactor>,
    problem_bytes: Vec<u8>,
}

impl PartialEq for RequestIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.provenance == other.provenance
            && self.preparation == other.preparation
            && self.domain == other.domain
            && self.kinematics == other.kinematics
            && self.aliases == other.aliases
            && self.factors == other.factors
            && self.problem_bytes == other.problem_bytes
            && self.input == other.input
    }
}
impl Eq for RequestIdentity {}

#[derive(Clone, Debug)]
pub struct GcadRequest {
    identity: RequestIdentity,
    problem: Problem,
}

impl GcadRequest {
    /// Native defaults adapted to a synchronous caller-owned library request.
    /// Hard memory/time limits still belong to the caller's generation worker.
    pub fn default_limits() -> Limits {
        Limits {
            workers: 1,
            ..Limits::default()
        }
    }

    pub fn unit_cube(
        input: &ParametricIntegrand,
        kinematics: GcadKinematics,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        if input.domain() != ParametricDomain::UnitCube {
            return Err(GcadError::Unsupported("unit-cube admission requires a native UnitCube input; projective preparation is a separate proof".into()));
        }
        let strict_positive = input
            .parameters()
            .iter()
            .flat_map(|s| [Atom::var(*s), Atom::num(1) - Atom::var(*s)])
            .collect();
        let domain = PreparedDomain {
            coordinates: input.parameters().to_vec(),
            strict_positive,
            origin: DomainOrigin::NativeUnitCube,
        };
        Self::prepared(input, domain, kinematics, solver, limits)
    }

    pub fn prepared(
        input: &ParametricIntegrand,
        domain: PreparedDomain,
        kinematics: GcadKinematics,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        if input.domain() == ParametricDomain::ProjectiveSimplex {
            return Err(GcadError::Unsupported("a projective density must first receive a proved native gauge/pullback with its measure".into()));
        }
        if domain.coordinates != input.parameters() {
            return Err(GcadError::Invalid(
                "prepared coordinates differ from native integration parameters".into(),
            ));
        }
        Self::from_preparation(
            Arc::new(input.clone()),
            domain,
            None,
            kinematics,
            solver,
            limits,
        )
    }

    /// Admit a proved affine delta-gauge preparation while retaining its original
    /// projective density, map, factor associations and simplex boundary. An owned
    /// preparation is moved; a supplied `Arc` is retained without copying terms.
    pub fn projective(
        preparation: impl Into<Arc<AffineProjectivePreparation>>,
        kinematics: GcadKinematics,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        let preparation = preparation.into();
        if preparation.coordinates().is_empty() {
            return Err(GcadError::Unsupported(
                "zero-dimensional projective preparation is exact-only and needs no GCAD decomposition".into(),
            ));
        }
        let domain = PreparedDomain {
            coordinates: preparation.coordinates().to_vec(),
            strict_positive: preparation.strict_positive().to_vec(),
            origin: DomainOrigin::AffineProjective {
                eliminated_index: preparation.eliminated_index(),
            },
        };
        Self::from_preparation(
            preparation.input_owner().clone(),
            domain,
            Some(preparation),
            kinematics,
            solver,
            limits,
        )
    }

    fn from_preparation(
        input: Arc<ParametricIntegrand>,
        domain: PreparedDomain,
        preparation: Option<Arc<AffineProjectivePreparation>>,
        kinematics: GcadKinematics,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        // An eliminated integration variable must not reappear as a kinematic
        // parameter or binding merely because it is absent from the target axes.
        let coordinates = input.parameters().iter().copied().collect::<BTreeSet<_>>();
        let parameters = kinematics
            .runtime_parameters
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        if parameters.len() != kinematics.runtime_parameters.len()
            || !coordinates.is_disjoint(&parameters)
            || parameters.contains(&input.regulator())
            || kinematics.exact_values.keys().any(|s| {
                coordinates.contains(s) || parameters.contains(s) || *s == input.regulator()
            })
        {
            return Err(GcadError::Invalid(
                "kinematic and integration/regulator symbol roles overlap or repeat".into(),
            ));
        }
        if limits.workers != 1 {
            return Err(GcadError::Unsupported("native GCAD is caller-driven; set limits.workers=1 and schedule external workers in the caller".into()));
        }
        if solver.search_time_secs.is_some()
            || solver.max_rollouts != SolverOptions::default().max_rollouts
        {
            return Err(GcadError::Unsupported("ordering search orchestration is not implicit in solver::solve; use a separately caller-owned search".into()));
        }
        let aliases = kinematics
            .runtime_parameters
            .iter()
            .map(|s| (*s, AliasRole::RuntimeParameter))
            .chain(
                domain
                    .coordinates
                    .iter()
                    .map(|s| (*s, AliasRole::IntegrationCoordinate)),
            )
            .enumerate()
            .map(|(i, (symbol, role))| SymbolAlias {
                symbol,
                name: format!("v{i}"),
                role,
            })
            .collect::<Vec<_>>();
        let names = aliases.iter().map(|a| a.name.clone()).collect::<Vec<_>>();
        let algebra = Algebra::new(&names).map_err(|e| GcadError::Invalid(e.to_string()))?;
        let variables = Arc::new(
            aliases
                .iter()
                .map(|a| PolyVariable::from(a.symbol))
                .collect::<Vec<_>>(),
        );
        let native_polynomial = |expression: &Atom| -> Result<Poly> {
            let exact = kinematics.specialize_exact(expression);
            let poly: Poly = exact
                .try_to_polynomial(&Q, variables.clone())
                .map_err(|e| {
                    GcadError::Unsupported(format!("exact rational geometry required: {e}"))
                })?;
            if poly.variables() != &variables {
                return Err(GcadError::Unsupported("geometry contains undeclared coefficients, functions or nonpolynomial variables".into()));
            }
            // Native formatting aliases the ordered polynomial axes; native
            // parsing also checks the supported degree and rational domain.
            algebra
                .parse(&algebra.format(&poly))
                .map_err(|e| GcadError::Unsupported(e.to_string()))
        };
        let mut constraints = Vec::new();
        for c in &kinematics.strict_positive {
            if input
                .parameters()
                .iter()
                .any(|s| c.contains(Atom::var(*s).as_view()))
            {
                return Err(GcadError::Invalid(
                    "kinematic chamber constraints may not involve integration coordinates".into(),
                ));
            }
            constraints.push(format!("({})>0", algebra.format(&native_polynomial(c)?)));
        }
        for c in &domain.strict_positive {
            constraints.push(format!("({})>0", algebra.format(&native_polynomial(c)?)));
        }
        let mut polynomials = Vec::<Poly>::new();
        let mut factors = Vec::new();
        let terms = preparation.as_ref().map_or(input.terms(), |p| p.terms());
        for (term_index, term) in terms.iter().enumerate() {
            for (factor_index, factor) in term.factors().iter().enumerate() {
                if factor.role() != FactorRole::Singularity {
                    continue;
                }
                let polynomial = native_polynomial(factor.polynomial())?;
                if polynomial.is_zero() {
                    return Err(GcadError::Invalid(
                        "specialization makes a singular factor identically zero".into(),
                    ));
                }
                let split_index = if let Some(i) = polynomials.iter().position(|p| p == &polynomial)
                {
                    i
                } else {
                    polynomials.push(polynomial);
                    polynomials.len() - 1
                };
                let origin = preparation
                    .as_ref()
                    .map_or(FactorOrigin::OriginalFactor { factor_index }, |p| {
                        p.factor_origins()[term_index][factor_index]
                    });
                let original_polynomial = match origin {
                    FactorOrigin::OriginalFactor { factor_index } => input.terms()[term_index]
                        .factors()[factor_index]
                        .polynomial()
                        .clone(),
                    FactorOrigin::OriginalMonomial { parameter_index } => {
                        Atom::var(input.parameters()[parameter_index])
                    }
                };
                factors.push(SignedFactor {
                    term_index,
                    factor_index,
                    origin,
                    split_index,
                    original_polynomial,
                    prepared_polynomial: factor.polynomial().clone(),
                    exponent: factor.exponent().clone(),
                    semantics: factor.semantics(),
                });
            }
        }
        let mut variable_groups = vec![];
        for role in [
            AliasRole::RuntimeParameter,
            AliasRole::IntegrationCoordinate,
        ] {
            let group = aliases
                .iter()
                .filter(|a| a.role == role)
                .map(|a| a.name.clone())
                .collect::<Vec<_>>();
            if !group.is_empty() {
                variable_groups.push(group);
            }
        }
        let problem = Problem {
            variables: names,
            constraints,
            split: polynomials.iter().map(|p| algebra.format(p)).collect(),
            variable_groups,
            solver,
            limits,
            ..Problem::default()
        };
        problem
            .validate()
            .map_err(|e| GcadError::Invalid(e.to_string()))?;
        let problem_bytes =
            serde_json::to_vec(&problem).map_err(|e| GcadError::Invalid(e.to_string()))?;
        let identity = RequestIdentity {
            provenance: SourceProvenance::Exact,
            input,
            preparation,
            domain,
            kinematics,
            aliases,
            factors,
            problem_bytes,
        };
        Ok(Self { identity, problem })
    }

    /// Prepare the represented exact view while retaining its original native source.
    pub fn represented_unit_cube(
        represented: Arc<crate::threshold::represented::ExactRepresentedInput>,
        kinematics: GcadKinematics,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        Self::unit_cube(represented.exact(), kinematics, solver, limits)?
            .retain_represented(represented)
    }

    pub fn represented_projective(
        represented: Arc<crate::threshold::represented::ExactRepresentedInput>,
        eliminated_index: usize,
        kinematics: GcadKinematics,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        let preparation =
            AffineProjectivePreparation::eliminate(represented.exact(), eliminated_index)
                .map_err(|e| GcadError::Invalid(e.to_string()))?;
        Self::projective(preparation, kinematics, solver, limits)?.retain_represented(represented)
    }

    pub(crate) fn retain_represented(
        mut self,
        represented: Arc<crate::threshold::represented::ExactRepresentedInput>,
    ) -> Result<Self> {
        if !matches!(self.identity.provenance, SourceProvenance::Exact)
            || self.input() != represented.exact().as_ref()
        {
            return Err(GcadError::RequestMismatch);
        }
        // No altered source/policy identity for the benign already-exact path.
        if represented.conversions().is_empty() {
            return Ok(self);
        }
        self.identity.input = represented.original().clone();
        for factor in &mut self.identity.factors {
            factor.original_polynomial = match factor.origin {
                FactorOrigin::OriginalFactor { factor_index } => {
                    self.identity.input.terms()[factor.term_index].factors()[factor_index]
                        .polynomial()
                        .clone()
                }
                FactorOrigin::OriginalMonomial { parameter_index } => {
                    Atom::var(self.identity.input.parameters()[parameter_index])
                }
            };
        }
        self.identity.provenance = SourceProvenance::ParametricRepresented(represented);
        Ok(self)
    }

    /// Prepare a fixed native graph point after faithful scalar conversion and
    /// native parameterization. No runtime parameter family is specialized here.
    pub fn preparametric_projective(
        represented: Arc<crate::threshold::represented::graph::ExactRepresentedGraphInput>,
        eliminated_index: usize,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        let preparation =
            AffineProjectivePreparation::eliminate(represented.exact(), eliminated_index)
                .map_err(|e| GcadError::Invalid(e.to_string()))?;
        Self::projective(preparation, GcadKinematics::default(), solver, limits)?
            .retain_preparametric(represented)
    }

    pub(crate) fn retain_preparametric(
        mut self,
        represented: Arc<crate::threshold::represented::graph::ExactRepresentedGraphInput>,
    ) -> Result<Self> {
        if !matches!(self.identity.provenance, SourceProvenance::Exact)
            || self.input() != represented.exact().as_ref()
            || self.kinematics() != &GcadKinematics::default()
            || self.projective_preparation().is_none()
        {
            return Err(GcadError::RequestMismatch);
        }
        self.identity.provenance = SourceProvenance::PreparametricGraph(represented);
        Ok(self)
    }

    /// Prepare the exact parameterization of an already exact native family at
    /// an explicitly represented point. Original family/point provenance is retained.
    pub fn preparametric_family_projective(
        represented: Arc<crate::threshold::represented::family::ExactRepresentedFamilyInput>,
        eliminated_index: usize,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        let preparation =
            AffineProjectivePreparation::eliminate(represented.exact(), eliminated_index)
                .map_err(|e| GcadError::Invalid(e.to_string()))?;
        Self::projective(preparation, GcadKinematics::default(), solver, limits)?
            .retain_preparametric_family(represented)
    }
    pub(crate) fn retain_preparametric_family(
        mut self,
        represented: Arc<crate::threshold::represented::family::ExactRepresentedFamilyInput>,
    ) -> Result<Self> {
        if !matches!(self.identity.provenance, SourceProvenance::Exact)
            || self.input() != represented.exact().as_ref()
            || self.kinematics() != &GcadKinematics::default()
            || self.projective_preparation().is_none()
        {
            return Err(GcadError::RequestMismatch);
        }
        self.identity.provenance = SourceProvenance::PreparametricFamily(represented);
        Ok(self)
    }
    /// Original native family/point and its faithful pre-specialization evidence.
    /// `input()` is the exact parameterization, not an invented Float density.
    pub fn preparametric_family_input(
        &self,
    ) -> Option<&crate::threshold::represented::family::ExactRepresentedFamilyInput> {
        match &self.identity.provenance {
            SourceProvenance::PreparametricFamily(owner) => Some(owner),
            _ => None,
        }
    }

    pub fn source_provenance(&self) -> &SourceProvenance {
        &self.identity.provenance
    }

    /// Original native graph point and its pre-arithmetic conversion evidence.
    /// `input()` is its exact parameterized density; no Float density is invented.
    pub fn preparametric_graph_input(
        &self,
    ) -> Option<&crate::threshold::represented::graph::ExactRepresentedGraphInput> {
        match &self.identity.provenance {
            SourceProvenance::PreparametricGraph(owner) => Some(owner),
            _ => None,
        }
    }

    pub fn represented_input(
        &self,
    ) -> Option<&crate::threshold::represented::ExactRepresentedInput> {
        match &self.identity.provenance {
            SourceProvenance::ParametricRepresented(owner) => Some(owner),
            _ => None,
        }
    }
    /// Exact density before geometric preparation. Parametric-represented sources
    /// retain their original density in `input()`; graph sources retain the
    /// original native point separately in `preparametric_graph_input()`.
    pub fn exact_input(&self) -> &ParametricIntegrand {
        self.represented_input().map_or(self.input(), |r| r.exact())
    }

    /// Native density before geometric preparation. For pre-parametric graph
    /// sources this is the exact parameterization; the original numerical graph
    /// remains available only through `preparametric_graph_input()`.
    pub fn input(&self) -> &ParametricIntegrand {
        &self.identity.input
    }
    pub fn projective_preparation(&self) -> Option<&AffineProjectivePreparation> {
        self.identity.preparation.as_deref()
    }
    /// Complete factorized density in `domain().coordinates()`, retaining term order.
    pub fn prepared_terms(&self) -> &[ParametricTerm] {
        self.projective_preparation()
            .map_or(self.exact_input().terms(), |p| p.terms())
    }
    pub fn domain(&self) -> &PreparedDomain {
        &self.identity.domain
    }
    pub fn kinematics(&self) -> &GcadKinematics {
        &self.identity.kinematics
    }
    pub fn aliases(&self) -> &[SymbolAlias] {
        &self.identity.aliases
    }
    pub fn signed_factors(&self) -> &[SignedFactor] {
        &self.identity.factors
    }
    pub fn problem(&self) -> &Problem {
        &self.problem
    }
    pub fn identity(&self) -> &RequestIdentity {
        &self.identity
    }
    pub(super) fn matches_native_problem(&self, p: &Problem) -> bool {
        serde_json::to_vec(p).is_ok_and(|v| v == self.identity.problem_bytes)
    }

    pub(super) fn preserves_parameter_fibers(&self, order: &[String]) -> bool {
        if order.len() != self.aliases().len()
            || order.iter().collect::<BTreeSet<_>>().len() != order.len()
        {
            return false;
        }
        let mut integration_seen = false;
        for name in order {
            match self
                .aliases()
                .iter()
                .find(|a| &a.name == name)
                .map(|a| a.role)
            {
                Some(AliasRole::RuntimeParameter) if !integration_seen => {}
                Some(AliasRole::IntegrationCoordinate) => integration_seen = true,
                _ => return false,
            }
        }
        true
    }
}
