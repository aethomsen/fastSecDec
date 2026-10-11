//! Exact parameter-prefix geometry; integration bounds are not lowered here.
//! Live and restored point admission share the same native exact checker.
use super::{AliasRole, CellMap, GcadError, LinearAxis, Result, linear::root::lower_axes};
use crate::kernel::parameter_guard::{self, ExactParameterGuard};
use std::{collections::BTreeMap, sync::Arc};
use symbolica::atom::Symbol;
use symbolica::{domains::rational::Rational, evaluate::OptimizationSettings};
use symgcad::algebra::{Algebra, Poly};

mod chamber;
pub use chamber::{ParameterChamber, ParameterInputLayout};

pub struct ParameterPrefix {
    source: Arc<CellMap>,
    axes: Vec<LinearAxis>,
    pub(super) positive: Vec<Poly>,
    pub(super) nonzero: Vec<Poly>,
    guard: Option<Arc<ExactParameterGuard>>,
}
impl ParameterPrefix {
    pub(super) fn prepare(source: Arc<CellMap>) -> Result<Arc<Self>> {
        let mut prefix = Self::geometry(source)?;
        prefix.guard = prefix.prepare_guard()?;
        Ok(Arc::new(prefix))
    }
    fn geometry(source: Arc<CellMap>) -> Result<Self> {
        let raw = source.owner.native_result();
        let algebra = Algebra::new(&raw.order).map_err(|e| GcadError::Invalid(e.to_string()))?;
        let parameter_count = source
            .axes
            .iter()
            .take_while(|a| a.role == AliasRole::RuntimeParameter)
            .count();
        let axes = lower_axes(&source, 0..parameter_count)?;
        let parameter_only =
            |p: &Poly| (parameter_count..raw.order.len()).all(|i| p.degree(i) == 0);
        let parameter_positive = raw
            .normalized_constraints
            .iter()
            .map(|s| {
                algebra
                    .parse(s)
                    .map_err(|e| GcadError::Invalid(e.to_string()))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(parameter_only)
            .collect();
        let parameter_nonzero = raw
            .exceptional_polynomials
            .iter()
            .map(|index| {
                raw.polynomials
                    .get(*index)
                    .ok_or_else(|| GcadError::Invalid("missing exceptional polynomial".into()))
                    .and_then(|s| {
                        algebra
                            .parse(s)
                            .map_err(|e| GcadError::Invalid(e.to_string()))
                    })
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .filter(parameter_only)
            .collect();
        Ok(Self {
            source,
            axes,
            positive: parameter_positive,
            nonzero: parameter_nonzero,
            guard: None,
        })
    }
    fn prepare_guard(&self) -> Result<Option<Arc<ExactParameterGuard>>> {
        if self.axes.is_empty() {
            return Ok(None);
        }
        let algebra = Algebra::new(&self.source.owner.native_result().order)
            .map_err(|e| GcadError::Invalid(e.to_string()))?;
        let bound = |root: &Arc<super::linear::LinearRoot>| parameter_guard::LinearBound {
            source_polynomial: root.polynomial,
            positive_selector: root.selector == symgcad::output::RootIndexDomain::Positive,
            numerator: root.numerator.clone(),
            denominator: root.denominator.clone(),
        };
        let public = self.runtime_parameters().to_vec();
        let prefix_to_public =
            self.raw_parameters()
                .map(|p| {
                    public.iter().position(|s| *s == p).ok_or_else(|| {
                        GcadError::Invalid("parameter prefix lost public symbol".into())
                    })
                })
                .collect::<Result<Vec<_>>>()?;
        let guard = ExactParameterGuard::prepare(
            parameter_guard::Source {
                public_parameters: public,
                variables: algebra.variables,
                prefix_to_public,
                axes: self
                    .axes
                    .iter()
                    .map(|axis| parameter_guard::Axis {
                        lower: axis.lower.as_ref().map(bound),
                        upper: axis.upper.as_ref().map(bound),
                    })
                    .collect(),
                positive: self.positive.clone(),
                nonzero: self.nonzero.clone(),
            },
            OptimizationSettings::default().cores(1),
            parameter_guard::Limits::default(),
        )
        .map_err(guard_error)?;
        Ok(Some(Arc::new(guard)))
    }
    pub(crate) fn guard(&self) -> Option<&Arc<ExactParameterGuard>> {
        self.guard.as_ref()
    }
    pub fn admit(
        self: &Arc<Self>,
        values: &BTreeMap<Symbol, Rational>,
    ) -> Result<AdmittedParameterFiber> {
        let ordered = if let Some(guard) = &self.guard {
            guard.admit(values).map_err(guard_error)?
        } else if values.is_empty() {
            Vec::new()
        } else {
            return Err(GcadError::Invalid(
                "fixed map has no runtime parameters".into(),
            ));
        };
        Ok(AdmittedParameterFiber {
            prefix: self.clone(),
            values: values.clone(),
            ordered,
        })
    }
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    pub fn axes(&self) -> &[LinearAxis] {
        &self.axes
    }
    /// Original public runtime input order, distinct from native lifting order.
    pub fn runtime_parameters(&self) -> &[Symbol] {
        &self.source.owner.request().kinematics().runtime_parameters
    }
    /// Parameter symbols in the native leading-axis order.
    pub fn raw_parameters(&self) -> impl ExactSizeIterator<Item = Symbol> + '_ {
        self.source.axes[..self.axes.len()].iter().map(|a| a.symbol)
    }
}

impl std::fmt::Debug for ParameterPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ParameterPrefix")
            .field("source", &self.source)
            .field("axes", &self.axes)
            .field("positive", &self.positive)
            .field("nonzero", &self.nonzero)
            .finish_non_exhaustive()
    }
}
#[derive(Clone, Debug)]
pub struct AdmittedParameterFiber {
    prefix: Arc<ParameterPrefix>,
    values: BTreeMap<Symbol, Rational>,
    ordered: Vec<Rational>,
}
impl AdmittedParameterFiber {
    pub fn prefix(&self) -> &Arc<ParameterPrefix> {
        &self.prefix
    }
    pub fn values(&self) -> &BTreeMap<Symbol, Rational> {
        &self.values
    }
    pub fn raw_values(&self) -> &[Rational] {
        &self.ordered
    }
}
fn guard_error(error: parameter_guard::Error) -> GcadError {
    match error {
        parameter_guard::Error::Resource(reason) => {
            GcadError::Unsupported(format!("parameter admission resource incomplete: {reason}"))
        }
        parameter_guard::Error::KeySet => GcadError::Invalid(
            "parameter admission needs exactly the request's runtime parameter values".into(),
        ),
        parameter_guard::Error::Outside { axis } => GcadError::Invalid(format!(
            "parameter axis {axis} lies outside its open chamber"
        )),
        parameter_guard::Error::DegreeDrop { polynomial } => GcadError::Invalid(format!(
            "linear root polynomial {polynomial} changes degree at this point"
        )),
        parameter_guard::Error::Selector { polynomial } => GcadError::Invalid(format!(
            "positive-root selector {polynomial} has no strictly positive linear root"
        )),
        parameter_guard::Error::Predicate { index, exceptional } => GcadError::Invalid(format!(
            "parameter point violates {} predicate {index}",
            if exceptional {
                "exceptional nonzero"
            } else {
                "strict positive"
            }
        )),
        parameter_guard::Error::Invalid(reason) => GcadError::Invalid(reason.into()),
        parameter_guard::Error::Native(reason) => {
            GcadError::Invalid(format!("native exact parameter admission failed: {reason}"))
        }
    }
}
