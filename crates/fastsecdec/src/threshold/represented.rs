//! Threshold-only faithful interpretation of finite represented coefficients.
//! No geometry, branch, endpoint or convergence certificate is constructed.
pub mod family;
pub mod graph;
#[cfg(test)]
mod tests;
use crate::parametric::{ParametricIntegrand, ParametricTerm, PolynomialFactor};
use std::{ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, AtomView},
    coefficient::Coefficient,
    domains::float::{Complex, Float},
    prelude::Rational,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NumericalMeaning {
    /// Coefficients are the finite values actually represented by the caller.
    /// This does not turn a rounded measurement into an exact physical value.
    RepresentedValues,
    /// Interval/error semantics cannot be replaced by a single rational point.
    UncertaintyBounds,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub nodes: usize,
    pub depth: usize,
    pub precision_bits: u32,
    pub rational_bits: u64,
    pub converted_literals: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            input_bytes: 16 * 1024 * 1024,
            output_bytes: 64 * 1024 * 1024,
            nodes: 100_000,
            depth: 128,
            precision_bits: 4096,
            rational_bits: 32768,
            converted_literals: 16384,
        }
    }
}
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum Error {
    #[error("unsupported represented input: {0}")]
    Unsupported(&'static str),
    #[error("represented input resource limit: {0}")]
    ResourceIncomplete(&'static str),
    #[error("represented input preparation cancelled")]
    Cancelled,
    #[error("invalid represented input: {0}")]
    Invalid(String),
}
type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Location {
    Prefactor { term: usize },
    MonomialPower { term: usize, axis: usize },
    FactorPolynomial { term: usize, factor: usize },
    FactorExponent { term: usize, factor: usize },
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LiteralConversion<L = Location> {
    pub location: L,
    pub original: Atom,
    pub exact: Atom,
    pub precision_bits: [u32; 2],
    pub binary_exponents: [Option<i32>; 2],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactRepresentedInput {
    original: Arc<ParametricIntegrand>,
    exact: Arc<ParametricIntegrand>,
    conversions: Vec<LiteralConversion>,
    meaning: NumericalMeaning,
    limits: Limits,
}
impl ExactRepresentedInput {
    pub fn original(&self) -> &Arc<ParametricIntegrand> {
        &self.original
    }
    pub fn exact(&self) -> &Arc<ParametricIntegrand> {
        &self.exact
    }
    pub fn conversions(&self) -> &[LiteralConversion] {
        &self.conversions
    }
    pub fn limits(&self) -> Limits {
        self.limits
    }
    pub fn meaning(&self) -> NumericalMeaning {
        self.meaning
    }
    /// This owner retains both complete native inputs. Adapters must bind this
    /// provenance into request identity; `exact()` alone is not source identity.
    pub fn prepare(
        original: Arc<ParametricIntegrand>,
        meaning: NumericalMeaning,
        limits: Limits,
        mut observer: impl FnMut(usize) -> ControlFlow<()>,
    ) -> Result<Self> {
        let mut work = Work::new(meaning, limits)?;
        let mut terms = Vec::new();
        for (term_index, term) in original.terms().iter().enumerate() {
            if observer(term_index).is_break() {
                return Err(Error::Cancelled);
            }
            let prefactor =
                work.atom(term.prefactor(), Location::Prefactor { term: term_index })?;
            let powers = term
                .monomial_powers()
                .iter()
                .enumerate()
                .map(|(axis, a)| {
                    work.atom(
                        a,
                        Location::MonomialPower {
                            term: term_index,
                            axis,
                        },
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let factors = term
                .factors()
                .iter()
                .enumerate()
                .map(|(factor, f)| {
                    let polynomial = work.atom(
                        f.polynomial(),
                        Location::FactorPolynomial {
                            term: term_index,
                            factor,
                        },
                    )?;
                    let exponent = work.atom(
                        f.exponent(),
                        Location::FactorExponent {
                            term: term_index,
                            factor,
                        },
                    )?;
                    Ok(PolynomialFactor::new(polynomial, exponent, f.role())
                        .with_semantics(f.semantics()))
                })
                .collect::<Result<Vec<_>>>()?;
            terms.push(ParametricTerm::new(prefactor, powers, factors));
        }
        let exact = Arc::new(
            ParametricIntegrand::new(
                original.parameters().to_vec(),
                original.regulator(),
                original.domain(),
                terms,
            )
            .map_err(|e| Error::Invalid(e.to_string()))?,
        );
        if exact.terms().len() != original.terms().len() {
            return Err(Error::Invalid(
                "native represented-input term inventory changed".into(),
            ));
        }
        // Native import may reorder sums when Symbol allocation order changes.
        // Provenance occurrences retain multiplicity but have canonical ordering.
        work.conversions.sort_by_cached_key(|row| {
            (
                row.location.clone(),
                row.original.to_canonical_string(),
                row.exact.to_canonical_string(),
                row.precision_bits,
                row.binary_exponents,
            )
        });
        Ok(Self {
            original,
            exact,
            conversions: work.conversions,
            meaning,
            limits,
        })
    }
}
struct Work<L = Location> {
    limits: Limits,
    nodes: usize,
    input_bytes: usize,
    output_bytes: usize,
    conversions: Vec<LiteralConversion<L>>,
}
impl<L: Clone> Work<L> {
    fn new(meaning: NumericalMeaning, limits: Limits) -> Result<Self> {
        if meaning != NumericalMeaning::RepresentedValues {
            return Err(Error::Unsupported(
                "uncertainty-bearing values require an explicit interval/error contract",
            ));
        }
        if limits.precision_bits == 0 || limits.rational_bits == 0 {
            return Err(Error::Invalid("nonpositive coefficient limits".into()));
        }
        Ok(Self {
            limits,
            nodes: 0,
            input_bytes: 0,
            output_bytes: 0,
            conversions: Vec::new(),
        })
    }
    fn atom(&mut self, original: &Atom, location: L) -> Result<Atom> {
        self.atom_checked(original, location, false, || Ok(()))
    }
    // One bounded traversal and one native replacement for both old scalar
    // conversion and the explicit graph coefficient capability. The optional
    // native admission runs on the original before any coefficient replacement.
    fn atom_checked(
        &mut self,
        original: &Atom,
        location: L,
        refuse_float_arguments: bool,
        admit_original: impl FnOnce() -> Result<()>,
    ) -> Result<Atom> {
        self.input_bytes = self
            .input_bytes
            .checked_add(original.as_view().get_byte_size())
            .filter(|n| *n <= self.limits.input_bytes)
            .ok_or(Error::ResourceIncomplete("represented input bytes"))?;
        // Bound syntax before native replacement or cloning decoded coefficients.
        let mut stack = vec![(original.as_view(), 0usize, false)];
        while let Some((node, depth, in_function)) = stack.pop() {
            self.nodes = self
                .nodes
                .checked_add(1)
                .filter(|n| *n <= self.limits.nodes)
                .ok_or(Error::ResourceIncomplete("represented input nodes"))?;
            if depth > self.limits.depth {
                return Err(Error::ResourceIncomplete("represented input depth"));
            }
            if refuse_float_arguments
                && in_function
                && let AtomView::Num(value) = node
                && matches!(value.get_coeff_view().to_owned(), Coefficient::Float(_))
            {
                return Err(Error::Unsupported(
                    "graph payload Float under a function is unsupported, including scalar f(0.1) and shorthand arguments",
                ));
            }
            let remaining = self.limits.nodes.saturating_sub(self.nodes);
            let mut push = |child| -> Result<()> {
                if stack.len() >= remaining {
                    return Err(Error::ResourceIncomplete(
                        "represented input traversal frontier",
                    ));
                }
                stack.push((
                    child,
                    depth + 1,
                    in_function || matches!(node, AtomView::Fun(_)),
                ));
                Ok(())
            };
            match node {
                AtomView::Add(a) => {
                    for child in a.iter() {
                        push(child)?;
                    }
                }
                AtomView::Mul(a) => {
                    for child in a.iter() {
                        push(child)?;
                    }
                }
                AtomView::Pow(p) => {
                    push(p.get_base())?;
                    push(p.get_exp())?;
                }
                AtomView::Fun(f) => {
                    // Preserve native/opaque heads. Conversion makes no
                    // analytic claim; later geometry/endpoint admission owns
                    // the function class and no uncertainty wrapper is erased.
                    for child in f.iter() {
                        push(child)?;
                    }
                }
                _ => {}
            }
            if stack.len() > self.limits.nodes.saturating_sub(self.nodes) {
                return Err(Error::ResourceIncomplete(
                    "represented input traversal frontier",
                ));
            }
        }
        admit_original()?;
        let mut error = None;
        let exact = original.replace_map_bottom_up(|node, _, out| {
            if error.is_some() {
                return;
            }
            let AtomView::Num(n) = node else { return };
            let result = (|| -> Result<()> {
                match n.get_coeff_view().to_owned() {
                    Coefficient::Complex(q) => {
                        check_bits(&q.re, self.limits)?;
                        check_bits(&q.im, self.limits)?;
                    }
                    Coefficient::Float(q) => {
                        if self.conversions.len() >= self.limits.converted_literals {
                            return Err(Error::ResourceIncomplete("represented literal count"));
                        }
                        let real = exact_float(&q.re, self.limits)?;
                        let imag = exact_float(&q.im, self.limits)?;
                        let value = Atom::num(Complex::new(real, imag));
                        self.conversions.push(LiteralConversion {
                            location: location.clone(),
                            original: node.to_owned(),
                            exact: value.clone(),
                            precision_bits: [q.re.prec(), q.im.prec()],
                            binary_exponents: [q.re.as_raw().get_exp(), q.im.as_raw().get_exp()],
                        });
                        **out = value;
                    }
                    Coefficient::Indeterminate | Coefficient::Infinity(_) => {
                        return Err(Error::Unsupported("nonfinite represented coefficient"));
                    }
                    _ => return Err(Error::Unsupported(
                        "coefficient field outside exact complex rationals and represented Float",
                    )),
                }
                Ok(())
            })();
            if let Err(e) = result {
                error = Some(e);
            }
        });
        if let Some(e) = error {
            return Err(e);
        }
        self.output_bytes = self
            .output_bytes
            .checked_add(exact.as_view().get_byte_size())
            .filter(|n| *n <= self.limits.output_bytes)
            .ok_or(Error::ResourceIncomplete("represented output bytes"))?;
        Ok(exact)
    }
}
fn check_bits(q: &Rational, limits: Limits) -> Result<()> {
    if q.numerator_ref().significant_bits() > limits.rational_bits
        || q.denominator_ref().significant_bits() > limits.rational_bits
    {
        return Err(Error::ResourceIncomplete("represented rational bit limit"));
    }
    Ok(())
}
fn exact_float(f: &Float, limits: Limits) -> Result<Rational> {
    if !f.is_finite() {
        return Err(Error::Unsupported("nonfinite represented Float"));
    }
    if f.prec() > limits.precision_bits {
        return Err(Error::ResourceIncomplete("represented Float precision"));
    }
    // A p-bit significand with MPFR-style binary exponent e needs at most
    // p+|e|+2 bits in either integer of its exact ratio. Check before conversion.
    let bound = u64::from(f.prec())
        .checked_add(
            f.as_raw()
                .get_exp()
                .map_or(0, |e| u64::from(e.unsigned_abs())),
        )
        .and_then(|n| n.checked_add(2))
        .ok_or(Error::ResourceIncomplete("represented Float exponent"))?;
    if bound > limits.rational_bits {
        return Err(Error::ResourceIncomplete(
            "represented Float ratio allocation bound",
        ));
    }
    let exact = f
        .try_to_rational()
        .ok_or(Error::Unsupported("nonfinite native Float ratio"))?;
    check_bits(&exact, limits)?;
    Ok(exact)
}

impl Limits {
    pub(crate) fn is_within(self, cap: Self) -> bool {
        self.input_bytes <= cap.input_bytes
            && self.output_bytes <= cap.output_bytes
            && self.nodes <= cap.nodes
            && self.depth <= cap.depth
            && self.precision_bits <= cap.precision_bits
            && self.rational_bits <= cap.rational_bits
            && self.converted_literals <= cap.converted_literals
    }
}
