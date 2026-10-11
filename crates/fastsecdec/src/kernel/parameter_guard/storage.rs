//! Native transport declarations and bounded structural admission. The outer
//! existing artifact envelope owns State import, byte caps and content hashes.
use super::*;
#[cfg(any(test, feature = "threshold-decomposition"))]
use symbolica::poly::{LexOrder, MonomialOrder};
use symbolica::{domains::float::Complex, evaluate::Instruction, state::StateMap};
#[derive(Clone, Debug, bincode::Encode, bincode::Decode, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct BoundSlots {
    pub numerator: usize,
    pub denominator: usize,
    pub source_polynomial: usize,
    pub positive_selector: bool,
}
#[derive(Clone, Debug, bincode::Encode, bincode::Decode, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AxisSlots {
    pub lower: Option<BoundSlots>,
    pub upper: Option<BoundSlots>,
}
/// Embeddable in existing State-aware native records. It is not a geometry
/// proof, a public guard constructor, or a second native program codec.
#[derive(Clone, bincode::Encode, bincode::Decode)]
#[bincode(decode_context = "StateMap")]
pub(crate) struct Stored {
    pub(super) version: u32,
    pub(super) public_parameters: Vec<Symbol>,
    pub(super) raw_parameters: Vec<Symbol>,
    pub(super) prefix_to_public: Vec<usize>,
    pub(super) axes: Vec<AxisSlots>,
    pub(super) positive: Vec<usize>,
    pub(super) nonzero: Vec<usize>,
    pub(super) expressions: Vec<Atom>,
    pub(super) program: Vec<u8>,
}
impl Stored {
    pub(super) fn validate(&self, limits: &Limits) -> Result<()> {
        let n = self.public_parameters.len();
        let outputs = self.expressions.len();
        if self.version != 1 {
            return Err(Error::Invalid("guard storage version"));
        }
        if n == 0
            || n > limits.variables
            || outputs == 0
            || outputs > limits.polynomials
            || self.program.len() > limits.program_bytes
        {
            return Err(Error::Resource("guard record allowance"));
        }
        if self.raw_parameters.len() != n
            || self.axes.len() != n
            || self.prefix_to_public.len() != n
            || self
                .public_parameters
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != n
            || self
                .raw_parameters
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                .len()
                != n
            || self
                .prefix_to_public
                .iter()
                .copied()
                .collect::<BTreeSet<_>>()
                != (0..n).collect::<BTreeSet<_>>()
        {
            return Err(Error::Invalid("restored guard parameter layout"));
        }
        let mut slots = BTreeSet::new();
        for axis in &self.axes {
            for bound in [&axis.lower, &axis.upper].into_iter().flatten() {
                for slot in [bound.numerator, bound.denominator] {
                    if slot >= outputs || !slots.insert(slot) {
                        return Err(Error::Invalid("guard bound output association"));
                    }
                }
            }
        }
        for slot in self.positive.iter().chain(&self.nonzero) {
            if *slot >= outputs || !slots.insert(*slot) {
                return Err(Error::Invalid("guard predicate output association"));
            }
        }
        if slots.is_empty() {
            if self.expressions != [Atom::one()] {
                return Err(Error::Invalid("unguarded native constant output"));
            }
        } else if slots.len() != outputs {
            return Err(Error::Invalid("unassociated guard output"));
        }
        Ok(())
    }
    /// Callers extend their existing native State inventory before encoding.
    pub(crate) fn visit_atoms(&self, visit: &mut impl FnMut(&Atom)) {
        for atom in &self.expressions {
            visit(atom);
        }
    }
    pub(crate) fn symbols(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.public_parameters
            .iter()
            .chain(&self.raw_parameters)
            .copied()
    }
}
pub(super) fn rational(value: &Rational, limits: &Limits) -> Result<()> {
    if value.numerator_ref().significant_bits() > limits.rational_bits
        || value.denominator_ref().significant_bits() > limits.rational_bits
    {
        return Err(Error::Resource("guard rational bit allowance"));
    }
    Ok(())
}
#[cfg(any(test, feature = "threshold-decomposition"))]
pub(super) fn polynomial(
    p: &Poly,
    variables: &Arc<Vec<PolyVariable>>,
    prefix: usize,
    limits: &Limits,
) -> Result<()> {
    let n = variables.len();
    if n == 0
        || prefix > n
        || p.variables() != variables
        || p.coefficients.len().checked_mul(n) != Some(p.exponents.len())
    {
        return Err(Error::Invalid("native guard polynomial layout"));
    }
    if p.coefficients.len() > limits.terms {
        return Err(Error::Resource("guard polynomial term allowance"));
    }
    for c in &p.coefficients {
        rational(c, limits)?;
        if c.is_zero() {
            return Err(Error::Invalid("zero native polynomial coefficient"));
        }
    }
    let mut previous = None;
    for exponents in p.exponents.chunks_exact(n) {
        if exponents[prefix..].iter().any(|e| *e != 0) {
            return Err(Error::Invalid("guard polynomial depends on a later axis"));
        }
        if exponents
            .iter()
            .try_fold(0u32, |sum, e| sum.checked_add(*e))
            .is_none_or(|degree| degree > limits.degree)
        {
            return Err(Error::Resource("guard polynomial degree allowance"));
        }
        if previous.is_some_and(|old| LexOrder::cmp(exponents, old) != std::cmp::Ordering::Greater)
        {
            return Err(Error::Invalid("native polynomial monomial order"));
        }
        previous = Some(exponents);
    }
    Ok(())
}
/// Arithmetic-subset check on an already decoded trusted native program, not a
/// replacement hostile native-IR validator. No instructions are interpreted here.
pub(super) fn program(
    p: &ExpressionEvaluator<Complex<Rational>>,
    inputs: usize,
    outputs: usize,
    limits: &Limits,
) -> Result<()> {
    if p.get_input_len() != inputs || p.get_output_len() != outputs {
        return Err(Error::Invalid("guard native program shape"));
    }
    let view = p.export_instructions();
    if view.instructions.len() > limits.instructions
        || view.temporary_count > limits.instructions
        || view.constants.len() > limits.instructions
    {
        return Err(Error::Resource("guard native program allowance"));
    }
    if !view.constant_functions.is_empty() || !view.sub_evaluators.is_empty() {
        return Err(Error::Invalid("guard native program contains functions"));
    }
    for q in &view.constants {
        if !q.im.is_zero() {
            return Err(Error::Invalid("complex guard constant"));
        }
        rational(&q.re, limits)?;
    }
    for instruction in &view.instructions {
        match instruction {
            Instruction::Add(..) | Instruction::Mul(..) | Instruction::Assign(..) => {}
            Instruction::Pow(_, _, exponent, _)
                if *exponent >= 0 && (*exponent as u64) <= u64::from(limits.degree) => {}
            _ => {
                return Err(Error::Invalid(
                    "guard program requires polynomial arithmetic",
                ));
            }
        }
    }
    Ok(())
}
