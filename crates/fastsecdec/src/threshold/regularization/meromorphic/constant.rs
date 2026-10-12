//! Exact epsilon polynomials with separately certified constant coefficients.
//! Native AtomField owns polynomial conversion; balls never replace exact Atoms.
mod ball;
#[cfg(test)]
mod tests;

use super::{Error, Limits, Result};
pub use ball::{ConstantCoefficient, DenominatorWitness, RadicalWitness};
use std::collections::HashSet;
use symbolica::{
    atom::{Atom, AtomCore, AtomView, Symbol},
    domains::{atom::AtomField, float::Complex, rational::Rational},
};

/// Admission resources only. These settings do not change the exact residual
/// or its numerical evaluator, and are separate from persisted endpoint limits.
#[derive(Clone, Copy, Debug)]
pub struct ConstantCoefficientLimits {
    pub precision_bits: u32,
}
impl Default for ConstantCoefficientLimits {
    fn default() -> Self {
        Self {
            precision_bits: 256,
        }
    }
}
impl ConstantCoefficientLimits {
    pub(super) fn validate(self) -> Result<()> {
        if !(2..=16384).contains(&self.precision_bits) {
            return Err(Error::Invalid(
                "constant coefficient precision outside 2..=16384",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub struct ConstantPolynomialPrefactor {
    original: Atom,
    regulator: Symbol,
    degree: u16,
    coefficients: Vec<(u16, ConstantCoefficient)>,
    source_constants: Vec<ConstantCoefficient>,
}
impl ConstantPolynomialPrefactor {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn regulator(&self) -> Symbol {
        self.regulator
    }
    pub fn degree(&self) -> u16 {
        self.degree
    }
    pub fn coefficients(&self) -> &[(u16, ConstantCoefficient)] {
        &self.coefficients
    }
    /// Original maximal constant subexpressions, checked before native epsilon
    /// polynomial conversion can cancel or combine their coefficients.
    pub fn source_constants(&self) -> &[ConstantCoefficient] {
        &self.source_constants
    }

    pub fn admit(
        original: Atom,
        regulator: Symbol,
        limits: Limits,
        constants: ConstantCoefficientLimits,
    ) -> Result<Self> {
        constants.validate()?;
        if limits.integer_power < 0 || limits.degree > u16::MAX / 2 {
            return Err(Error::Invalid(
                "invalid constant-polynomial prefactor limits",
            ));
        }
        let bound = preflight(original.as_view(), regulator, limits, 0, &mut 0)?;
        let mut occupied = original
            .get_all_symbols(true)
            .into_iter()
            .collect::<HashSet<_>>();
        occupied.insert(regulator);
        let mut original_constants = HashSet::new();
        collect_constants(original.as_view(), regulator, &mut original_constants);
        // Native Atom ordering gives deterministic receipt/display order.
        let mut original_constants = original_constants.into_iter().collect::<Vec<_>>();
        original_constants.sort();
        let mut source_constants = Vec::with_capacity(original_constants.len());
        for expression in original_constants {
            source_constants.push(ball::certify(expression, &occupied, limits, constants)?);
        }
        let field = AtomField {
            statistical_zero_test: false,
            ..AtomField::new()
        };
        let polynomial = original.to_polynomial_in_vars_with_field::<u16>(regulator, &field);
        if polynomial.nvars() != 1
            || polynomial.variables()[0] != regulator
            || polynomial.degree(0) > limits.degree
            || usize::from(polynomial.degree(0)) > bound.degree
            || polynomial.nterms() > limits.terms
            || polynomial.nterms() > bound.terms
        {
            return Err(Error::ResourceIncomplete(
                "native constant-polynomial size/map",
            ));
        }
        let reconstructed = polynomial.to_expression_with_coeff_map(|_, c, out| *out = c.clone());
        // The same bounded native polynomial ring checks exact recombination.
        // Do not expand the bulk constant coefficient expressions.
        let difference =
            (&original - reconstructed).to_polynomial_in_vars_with_field::<u16>(regulator, &field);
        if !difference.is_zero() {
            return Err(Error::Invalid("native constant-polynomial recombination"));
        }
        let mut coefficients = Vec::with_capacity(polynomial.nterms());
        for term in &polynomial {
            if term
                .coefficient
                .get_all_symbols(true)
                .iter()
                .any(|s| *s != Symbol::PI)
            {
                return Err(Error::Unsupported(
                    "constant prefactor coefficient has a free role",
                ));
            }
            let coefficient =
                ball::certify(term.coefficient.clone(), &occupied, limits, constants)?;
            coefficients.push((term.exponents[0], coefficient));
        }
        Ok(Self {
            original,
            regulator,
            degree: polynomial.degree(0),
            coefficients,
            source_constants,
        })
    }
}

fn collect_constants(atom: AtomView<'_>, eps: Symbol, out: &mut HashSet<Atom>) {
    if !atom.get_all_symbols(true).contains(&eps) {
        out.insert(atom.to_owned());
        return;
    }
    match atom {
        AtomView::Add(sum) => {
            for a in sum.iter() {
                collect_constants(a, eps, out);
            }
        }
        AtomView::Mul(product) => {
            for a in product.iter() {
                collect_constants(a, eps, out);
            }
        }
        AtomView::Pow(power) => collect_constants(power.get_base(), eps, out),
        _ => {}
    }
}

#[derive(Clone, Copy)]
struct Bound {
    degree: usize,
    terms: usize,
}
fn preflight(
    node: AtomView<'_>,
    eps: Symbol,
    limits: Limits,
    depth: usize,
    nodes: &mut usize,
) -> Result<Bound> {
    *nodes = nodes
        .checked_add(1)
        .ok_or(Error::ResourceIncomplete("constant source nodes"))?;
    if *nodes > limits.source_nodes || depth > limits.source_depth {
        return Err(Error::ResourceIncomplete("constant source depth/nodes"));
    }
    let constant = Bound {
        degree: 0,
        terms: 1,
    };
    let checked = |b: Bound| {
        if b.degree > usize::from(limits.degree) || b.terms > limits.terms {
            Err(Error::ResourceIncomplete(
                "constant-polynomial preconversion size",
            ))
        } else {
            Ok(b)
        }
    };
    let add = |a: usize, b: usize| {
        a.checked_add(b).ok_or(Error::ResourceIncomplete(
            "constant-polynomial size overflow",
        ))
    };
    let mul = |a: usize, b: usize| {
        a.checked_mul(b).ok_or(Error::ResourceIncomplete(
            "constant-polynomial size overflow",
        ))
    };
    checked(match node {
        AtomView::Num(_) => {
            Complex::<Rational>::try_from(node)
                .map_err(|_| Error::Unsupported("constant coefficient is not exact Q(i)"))?;
            constant
        }
        AtomView::Var(v) if v.get_symbol() == eps => Bound {
            degree: 1,
            terms: 1,
        },
        AtomView::Var(v) if v.get_symbol() == Symbol::PI => constant,
        AtomView::Var(_) => {
            return Err(Error::Unsupported(
                "constant prefactor contains undeclared variable",
            ));
        }
        AtomView::Add(sum) => {
            let mut b = Bound {
                degree: 0,
                terms: 0,
            };
            for child in sum.iter() {
                let c = preflight(child, eps, limits, depth + 1, nodes)?;
                b = checked(Bound {
                    degree: b.degree.max(c.degree),
                    terms: add(b.terms, c.terms)?,
                })?;
            }
            // A regulator-independent expression is one native field coefficient.
            if b.degree == 0 { constant } else { b }
        }
        AtomView::Mul(product) => {
            let mut b = constant;
            for child in product.iter() {
                let c = preflight(child, eps, limits, depth + 1, nodes)?;
                b = checked(Bound {
                    degree: add(b.degree, c.degree)?,
                    terms: mul(b.terms, c.terms)?,
                })?;
            }
            b
        }
        AtomView::Pow(power) => {
            let b = preflight(power.get_base(), eps, limits, depth + 1, nodes)?;
            let q = Rational::try_from(power.get_exp())
                .map_err(|_| Error::Unsupported("nonexact coefficient power"))?;
            if q == (1, 2) && b.degree == 0 {
                constant
            } else {
                let k = q
                    .numerator_ref()
                    .to_i64()
                    .filter(|_| q.is_integer())
                    .ok_or(Error::Unsupported("prefactor power outside constant class"))?;
                if k.unsigned_abs() > limits.integer_power as u64 {
                    return Err(Error::ResourceIncomplete(
                        "constant coefficient integer power",
                    ));
                }
                if b.degree == 0 {
                    constant
                } else {
                    let k = u32::try_from(k).map_err(|_| {
                        Error::Unsupported("epsilon denominator outside constant-polynomial class")
                    })?;
                    Bound {
                        degree: mul(b.degree, k as usize)?,
                        terms: b.terms.checked_pow(k).ok_or(Error::ResourceIncomplete(
                            "constant-polynomial power support",
                        ))?,
                    }
                }
            }
        }
        _ => {
            return Err(Error::Unsupported(
                "function outside constant coefficient class",
            ));
        }
    })
}
