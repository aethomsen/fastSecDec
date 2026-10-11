//! Exact point admission for source-issued parameter predicates. This arithmetic
//! owner never restores a uniform geometry/continuation certificate.
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Mutex,
};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::{Q, Rational},
    evaluate::ExpressionEvaluator,
};
#[cfg(any(test, feature = "threshold-decomposition"))]
use symbolica::{
    evaluate::OptimizationSettings,
    poly::{PolyVariable, polynomial::MultivariatePolynomial},
};
mod semantic;
#[cfg(any(test, feature = "threshold-decomposition"))]
pub(crate) mod staging;
mod storage;
#[cfg(any(test, feature = "threshold-decomposition"))]
use std::sync::Arc;
pub(crate) use storage::Stored;
#[cfg(any(test, feature = "threshold-decomposition"))]
use storage::{AxisSlots, BoundSlots};
#[cfg(any(test, feature = "threshold-decomposition"))]
pub(crate) type Poly = MultivariatePolynomial<Q, u32>;
pub(crate) type Result<T> = std::result::Result<T, Error>;
#[derive(Debug)]
pub(crate) enum Error {
    KeySet,
    Outside { axis: usize },
    DegreeDrop { polynomial: usize },
    Selector { polynomial: usize },
    Predicate { index: usize, exceptional: bool },
    Invalid(&'static str),
    Resource(&'static str),
    Native(String),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::KeySet => f.write_str("parameter guard key set mismatch"),
            Self::Outside { axis } => {
                write!(f, "parameter lies outside open chamber at raw axis {axis}")
            }
            Self::DegreeDrop { polynomial } => write!(
                f,
                "parameter bound denominator vanishes for polynomial {polynomial}"
            ),
            Self::Selector { polynomial } => write!(
                f,
                "parameter bound selector failed for polynomial {polynomial}"
            ),
            Self::Predicate { index, exceptional } => write!(
                f,
                "parameter {} predicate {index} failed",
                if *exceptional { "nonzero" } else { "positive" }
            ),
            Self::Invalid(reason) => write!(f, "invalid parameter guard: {reason}"),
            Self::Resource(reason) => write!(f, "parameter guard resource limit: {reason}"),
            Self::Native(reason) => write!(f, "native parameter guard: {reason}"),
        }
    }
}
impl std::error::Error for Error {}
fn native(e: impl std::fmt::Display) -> Error {
    Error::Native(e.to_string())
}
#[derive(Clone, Debug)]
pub(crate) struct Limits {
    pub variables: usize,
    pub polynomials: usize,
    #[cfg(any(test, feature = "threshold-decomposition"))]
    pub terms: usize,
    pub degree: u32,
    pub rational_bits: u64,
    pub program_bytes: usize,
    pub instructions: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            variables: 64,
            polynomials: 4096,
            #[cfg(any(test, feature = "threshold-decomposition"))]
            terms: 20_000,
            degree: 256,
            rational_bits: 4096,
            program_bytes: 16 * 1024 * 1024,
            instructions: 1_000_000,
        }
    }
}
#[cfg(any(test, feature = "threshold-decomposition"))]
pub(crate) struct LinearBound {
    pub source_polynomial: usize,
    pub positive_selector: bool,
    pub numerator: Poly,
    pub denominator: Poly,
}
#[cfg(any(test, feature = "threshold-decomposition"))]
pub(crate) struct Axis {
    pub lower: Option<LinearBound>,
    pub upper: Option<LinearBound>,
}
#[cfg(any(test, feature = "threshold-decomposition"))]
pub(crate) struct Source {
    pub public_parameters: Vec<Symbol>,
    pub variables: Arc<Vec<PolyVariable>>,
    pub prefix_to_public: Vec<usize>,
    pub axes: Vec<Axis>,
    pub positive: Vec<Poly>,
    pub nonzero: Vec<Poly>,
}
/// Immutable shared instructions with serialized caller-side bind scratch.
/// No threads, symbolic reconstruction or optimization run during admission.
pub(crate) struct ExactParameterGuard {
    stored: Stored,
    exact: Mutex<ExpressionEvaluator<Rational>>,
    limits: Limits,
}
impl ExactParameterGuard {
    #[cfg(any(test, feature = "threshold-decomposition"))]
    pub(crate) fn prepare(
        source: Source,
        settings: OptimizationSettings,
        limits: Limits,
    ) -> Result<Self> {
        let n = source.public_parameters.len();
        if n == 0 || n > limits.variables || source.variables.len() > limits.variables {
            return Err(Error::Resource("guard variable allowance"));
        }
        if source.axes.len() != n
            || source.prefix_to_public.len() != n
            || source
                .prefix_to_public
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != (0..n).collect::<BTreeSet<_>>()
            || source
                .public_parameters
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != n
            || source.variables.len() < n
        {
            return Err(Error::Invalid("guard parameter layout"));
        }
        let mut raw = Vec::with_capacity(source.variables.len());
        for variable in source.variables.iter() {
            let PolyVariable::Symbol(symbol) = variable else {
                return Err(Error::Invalid("guard variable must be a native symbol"));
            };
            raw.push(*symbol);
        }
        if raw.iter().copied().collect::<BTreeSet<_>>().len() != raw.len() {
            return Err(Error::Invalid("duplicate native guard variable"));
        }
        let mut expressions = Vec::new();
        let mut total_terms = 0usize;
        let mut axes = Vec::with_capacity(n);
        let mut add = |p: &Poly, prefix: usize| -> Result<usize> {
            storage::polynomial(p, &source.variables, prefix, &limits)?;
            total_terms = total_terms
                .checked_add(p.coefficients.len())
                .ok_or(Error::Resource("guard aggregate term overflow"))?;
            if total_terms > limits.terms {
                return Err(Error::Resource("guard aggregate term allowance"));
            }
            if expressions.len() >= limits.polynomials {
                return Err(Error::Resource("guard polynomial allowance"));
            }
            let index = expressions.len();
            expressions.push(p.to_expression());
            Ok(index)
        };
        for (axis, bounds) in source.axes.iter().enumerate() {
            let mut lower = None;
            let mut upper = None;
            for (source, target) in [(&bounds.lower, &mut lower), (&bounds.upper, &mut upper)] {
                if let Some(bound) = source {
                    *target = Some(BoundSlots {
                        numerator: add(&bound.numerator, axis)?,
                        denominator: add(&bound.denominator, axis)?,
                        source_polynomial: bound.source_polynomial,
                        positive_selector: bound.positive_selector,
                    });
                }
            }
            axes.push(AxisSlots { lower, upper });
        }
        let positive = source
            .positive
            .iter()
            .map(|p| add(p, n))
            .collect::<Result<Vec<_>>>()?;
        let nonzero = source
            .nonzero
            .iter()
            .map(|p| add(p, n))
            .collect::<Result<Vec<_>>>()?;
        // A fully unbounded parameter chamber still uses one harmless constant
        // output, keeping a single native program representation.
        if expressions.is_empty() {
            expressions.push(Atom::one());
        }
        let inputs = raw[..n].iter().copied().map(Atom::var).collect::<Vec<_>>();
        let program = Atom::evaluator_multiple(&expressions, &inputs)
            .optimization_settings(settings)
            .build()
            .map_err(native)?;
        storage::program(&program, n, expressions.len(), &limits)?;
        let bytes = super::program::encode(&program).map_err(native)?;
        let stored = Stored {
            version: 1,
            public_parameters: source.public_parameters,
            raw_parameters: raw[..n].to_vec(),
            prefix_to_public: source.prefix_to_public,
            axes,
            positive,
            nonzero,
            expressions,
            program: bytes,
        };
        Self::restore(stored, limits)
    }
    /// Restore checked predicate arithmetic only. The containing source-issued
    /// template record must already pass its digest/source/lineage admission.
    pub(crate) fn restore(stored: Stored, limits: Limits) -> Result<Self> {
        stored.validate(&limits)?;
        let program = super::program::decode(&stored.program).map_err(native)?;
        storage::program(
            &program,
            stored.public_parameters.len(),
            stored.expressions.len(),
            &limits,
        )?;
        let exact = program.map_to_ring(&Q).map_err(native)?;
        Ok(Self {
            stored,
            exact: Mutex::new(exact),
            limits,
        })
    }
    pub(crate) fn public_parameters(&self) -> &[Symbol] {
        &self.stored.public_parameters
    }
    pub(crate) fn prefix_to_public(&self) -> &[usize] {
        &self.stored.prefix_to_public
    }
    pub(crate) fn stored(&self) -> &Stored {
        &self.stored
    }
    /// Return native-prefix values. Callers commit their point/evaluator/offset
    /// mutation only after this and all other existing bind checks succeed.
    pub(crate) fn admit(&self, values: &BTreeMap<Symbol, Rational>) -> Result<Vec<Rational>> {
        if values.len() != self.stored.public_parameters.len()
            || self
                .stored
                .public_parameters
                .iter()
                .any(|s| !values.contains_key(s))
        {
            return Err(Error::KeySet);
        }
        let ordered = self
            .stored
            .prefix_to_public
            .iter()
            .map(|i| values[&self.stored.public_parameters[*i]].clone())
            .collect::<Vec<_>>();
        for q in &ordered {
            storage::rational(q, &self.limits)?;
        }
        let mut output = vec![Rational::zero(); self.stored.expressions.len()];
        self.exact
            .lock()
            .map_err(|_| Error::Native("poisoned guard scratch".into()))?
            .try_evaluate_in_ring(&ordered, &mut output, &Q)
            .map_err(native)?;
        for q in &output {
            storage::rational(q, &self.limits)?;
        }
        for (axis, bounds) in self.stored.axes.iter().enumerate() {
            for (bound, lower) in [(&bounds.lower, true), (&bounds.upper, false)] {
                if let Some(bound) = bound {
                    let denominator = &output[bound.denominator];
                    if denominator.is_zero() {
                        return Err(Error::DegreeDrop {
                            polynomial: bound.source_polynomial,
                        });
                    }
                    let root = &output[bound.numerator] / denominator;
                    storage::rational(&root, &self.limits)?;
                    if bound.positive_selector && root <= Rational::zero() {
                        return Err(Error::Selector {
                            polynomial: bound.source_polynomial,
                        });
                    }
                    if (lower && ordered[axis] <= root) || (!lower && ordered[axis] >= root) {
                        return Err(Error::Outside { axis });
                    }
                }
            }
        }
        for (indices, exceptional) in [(&self.stored.positive, false), (&self.stored.nonzero, true)]
        {
            for (index, slot) in indices.iter().enumerate() {
                if (exceptional && output[*slot].is_zero())
                    || (!exceptional && output[*slot] <= Rational::zero())
                {
                    return Err(Error::Predicate { index, exceptional });
                }
            }
        }
        Ok(ordered)
    }
}

#[cfg(test)]
mod tests;
