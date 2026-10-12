//! Native preparation of a static affine coefficient shift. This module adds
//! no arithmetic at restore: both exact evaluator programs are retained.
use super::Error;
use symbolica::domains::rational::Rational;

// Admission bounds on the exact expression before native optimizer allocation.
// They are not a total allocator/RSS guarantee.
const MAX_SHIFT_TERMS: usize = 65_536;
const MAX_SHIFT_SCALAR_BITS: u64 = 8192;

pub(super) fn preflight(degree: usize, lower: &Rational, upper: &Rational) -> Result<(), Error> {
    let terms = degree
        .checked_add(1)
        .and_then(|n| degree.checked_add(2).and_then(|m| n.checked_mul(m)))
        .map(|n| n / 2);
    if terms.is_none_or(|n| n > MAX_SHIFT_TERMS) {
        return Err(Error::Unsupported(
            "affine coefficient shift exceeds preparation term allowance".into(),
        ));
    }
    let width = upper - lower;
    let bits = [lower, &width]
        .into_iter()
        .map(|q| {
            q.numerator_ref()
                .significant_bits()
                .max(q.denominator_ref().significant_bits())
        })
        .max()
        .unwrap_or(0);
    // Each native scalar is binomial(i,k)*a^(i-k)*w^k. Its binomial
    // numerator has at most i+1 bits; no sum over unrelated c_i is hidden.
    let scalar_bits = bits
        .checked_add(1)
        .and_then(|n| u64::try_from(degree).ok().and_then(|d| n.checked_mul(d)))
        .and_then(|n| n.checked_add(1));
    if scalar_bits.is_none_or(|n| n > MAX_SHIFT_SCALAR_BITS) {
        return Err(Error::Unsupported(
            "affine coefficient shift exceeds preparation scalar allowance".into(),
        ));
    }
    Ok(())
}

#[cfg(feature = "threshold-decomposition")]
pub(super) fn prepare(
    polynomial: &symbolica::atom::Atom,
    variable: symbolica::atom::Symbol,
    inputs: &[symbolica::atom::Atom],
    lower: &Rational,
    upper: &Rational,
    settings: &symbolica::evaluate::OptimizationSettings,
) -> Result<(super::program::Exact, super::program::Exact), Error> {
    use symbolica::atom::{Atom, AtomCore};
    let degree = inputs.len() - 2;
    preflight(degree, lower, upper)?;
    let coordinate = Atom::var(variable);
    let image = Atom::num(lower.clone()) + Atom::num(upper - lower) * &coordinate;
    let normalized = polynomial
        .replace(coordinate.clone())
        .with(image.clone())
        .expand();
    // This is an exact coordinate change. The default expression field enables
    // statistical zero testing and can discard small but nonzero coefficients.
    let field = symbolica::domains::atom::AtomField {
        statistical_zero_test: false,
        ..Default::default()
    };
    let coefficients = normalized.to_polynomial_in_vars_with_field::<u32>(variable, &field);
    let shifted = coefficients
        .to_univariate_from_univariate(0)
        .coefficients()
        .to_vec();
    if shifted.len() != degree + 1 {
        return Err(Error::Invalid("native affine coefficient inventory".into()));
    }
    let recombined: Atom = shifted
        .iter()
        .enumerate()
        .map(|(i, coefficient)| coefficient * coordinate.pow(i as i64))
        .sum();
    if !(recombined - &normalized).expand().is_zero() {
        return Err(Error::Invalid(
            "native affine coefficient recombination".into(),
        ));
    }
    let build = |outputs: &[Atom], parameters: &[Atom]| {
        Atom::evaluator_multiple(outputs, parameters)
            .optimization_settings(settings.clone())
            .build()
            .map_err(|e| Error::Native(e.to_string()))
    };
    let shift = build(&shifted, &inputs[1..])?;
    // Runtime supplies q_i here, so optimization cannot reconstruct a physical
    // P evaluation through the original c_i and round the affine map first.
    let body = build(
        &[polynomial.clone(), polynomial.derivative(variable), image],
        inputs,
    )?;
    Ok((body, shift))
}
