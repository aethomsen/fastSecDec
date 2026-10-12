//! Exact meromorphic prefactor admission and a shared regular witness.
pub mod constant;
mod rational;
pub use constant::{ConstantCoefficientLimits, ConstantPolynomialPrefactor};
pub(crate) use rational::preflight_expression;
pub use rational::{Error, Limits, RationalPrefactor, SharedWitness};
use std::ops::ControlFlow;
use symbolica::{
    atom::{Atom, AtomCore, AtomView, Symbol},
    domains::float::Complex,
    prelude::Rational,
};
type Result<T> = std::result::Result<T, Error>;

/// c^(a+b*epsilon), c>0 exact rational. The real logarithm gives an entire,
/// nonvanishing function of epsilon. No new excluded witness points arise.
#[derive(Clone, Debug)]
pub struct PositiveScale {
    original: Atom,
    base: Rational,
    exponent: Atom,
    constant: Rational,
    slope: Rational,
}
impl PositiveScale {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn base(&self) -> &Rational {
        &self.base
    }
    pub fn exponent(&self) -> &Atom {
        &self.exponent
    }
    pub fn constant(&self) -> &Rational {
        &self.constant
    }
    pub fn slope(&self) -> &Rational {
        &self.slope
    }
}

#[derive(Clone, Debug)]
pub struct GammaPower {
    argument: Atom,
    constant: Rational,
    slope: Rational,
    power: i64,
}
impl GammaPower {
    pub fn argument(&self) -> &Atom {
        &self.argument
    }
    pub fn constant(&self) -> &Rational {
        &self.constant
    }
    pub fn slope(&self) -> &Rational {
        &self.slope
    }
    pub fn power(&self) -> i64 {
        self.power
    }
    fn value_at(&self, epsilon: &Rational) -> Rational {
        &self.constant + &(&self.slope * epsilon)
    }
    fn has_pole_at(&self, epsilon: &Rational) -> bool {
        let q = self.value_at(epsilon);
        self.power > 0 && q <= Rational::zero() && q.is_integer()
    }
    fn pole_count_bound(&self, a: &Rational, b: &Rational) -> Result<usize> {
        if self.power <= 0 || self.slope.is_zero() {
            return Ok(0);
        }
        let span = self.slope.abs() * (b - a);
        span.ceil()
            .to_i64()
            .and_then(|n| usize::try_from(n).ok())
            .and_then(|n| n.checked_add(1))
            .ok_or(Error::ResourceIncomplete("Gamma pole-family count"))
    }
    /// Exact positive distance to the nearest Gamma pole, in epsilon units.
    /// Reciprocal Gamma is entire; constant nonpole arguments are inert.
    fn pole_distance(&self, epsilon: &Rational) -> Result<Option<Rational>> {
        if self.power <= 0 || self.slope.is_zero() {
            return Ok(None);
        }
        let q = self.value_at(epsilon);
        let distance = if q > Rational::zero() {
            q
        } else {
            // Native Rational::floor truncates toward zero; use the
            // nonnegative distance variable for the nearest integer pair.
            let absolute = q.abs();
            let low = Rational::from(absolute.floor());
            let down = &absolute - &low;
            let up = Rational::one() - &down;
            std::cmp::min(down, up)
        };
        if distance <= Rational::zero() {
            return Err(Error::Invalid("Gamma witness is a pole"));
        }
        Ok(Some(distance / self.slope.abs()))
    }
}
/// Exact residual classes. Constant-polynomial balls certify definedness only;
/// the original coefficient Atoms remain the arithmetic source.
#[derive(Clone, Debug)]
pub enum PrefactorResidual {
    Rational(RationalPrefactor),
    ConstantPolynomial(ConstantPolynomialPrefactor),
}
impl PrefactorResidual {
    pub fn original(&self) -> &Atom {
        match self {
            Self::Rational(p) => p.original(),
            Self::ConstantPolynomial(p) => p.original(),
        }
    }
    pub fn regulator(&self) -> Symbol {
        match self {
            Self::Rational(p) => p.regulator(),
            Self::ConstantPolynomial(p) => p.regulator(),
        }
    }
}
#[derive(Clone, Debug)]
pub struct MeromorphicPrefactor {
    original: Atom,
    residual: PrefactorResidual,
    gamma: Vec<GammaPower>,
    scales: Vec<PositiveScale>,
}
impl MeromorphicPrefactor {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn residual(&self) -> &PrefactorResidual {
        &self.residual
    }
    pub fn rational(&self) -> Option<&RationalPrefactor> {
        match &self.residual {
            PrefactorResidual::Rational(p) => Some(p),
            PrefactorResidual::ConstantPolynomial(_) => None,
        }
    }
    pub fn gamma_factors(&self) -> &[GammaPower] {
        &self.gamma
    }
    pub fn positive_scales(&self) -> &[PositiveScale] {
        &self.scales
    }
    pub fn admit(original: Atom, regulator: Symbol, limits: Limits) -> Result<Self> {
        Self::admit_with_constants(
            original,
            regulator,
            limits,
            ConstantCoefficientLimits::default(),
        )
    }
    /// Native-only constant proof resources. Persisted common Limits are unchanged.
    pub fn admit_with_constants(
        original: Atom,
        regulator: Symbol,
        limits: Limits,
        constants: ConstantCoefficientLimits,
    ) -> Result<Self> {
        constants.validate()?;
        if limits.integer_power < 0 {
            return Err(Error::Invalid("negative prefactor power limit"));
        }
        // Bound the complete product, including all Gamma arguments, before
        // collecting factors or invoking any native algebra conversion.
        let mut nodes = 0usize;
        let mut oversized = false;
        original.visitor(&mut |_| {
            nodes = nodes.saturating_add(1);
            oversized |= nodes > limits.source_nodes;
            !oversized
        });
        if oversized {
            return Err(Error::ResourceIncomplete("meromorphic source nodes"));
        }
        let factors = match original.as_view() {
            AtomView::Mul(m) => m.iter().collect::<Vec<_>>(),
            other => vec![other],
        };
        if factors.len() > limits.terms {
            return Err(Error::ResourceIncomplete("prefactor factor count"));
        }
        let mut rational = Atom::one();
        let mut gamma = Vec::new();
        let mut scales = Vec::new();
        for factor in factors {
            let (base, power) = if let AtomView::Pow(p) = factor {
                (p.get_base(), Some(p.get_exp()))
            } else {
                (factor, None)
            };
            if let Some(exponent) = power
                && let Ok(value) = Rational::try_from(base)
                && value > Rational::zero()
            {
                let exponent = exponent.to_owned();
                RationalPrefactor::admit(exponent.clone(), regulator, limits)?;
                let (constant, slopes) =
                    crate::generation::endpoint_power_with_regulators(&exponent, &[regulator])
                        .map_err(|_| {
                            Error::Unsupported(
                                "positive scale exponent must be rational affine in epsilon",
                            )
                        })?;
                scales.push(PositiveScale {
                    original: factor.to_owned(),
                    base: value,
                    exponent,
                    constant,
                    slope: slopes[0].clone(),
                });
                continue;
            }
            let AtomView::Fun(f) = base else {
                rational *= factor.to_owned();
                continue;
            };
            if f.get_symbol() != symbolica::transcendental::gamma() {
                rational *= factor.to_owned();
                continue;
            }
            if f.get_nargs() != 1 {
                return Err(Error::Invalid("native Gamma arity"));
            }
            let power = match power {
                None => 1,
                Some(q) => {
                    let exact = Complex::<Rational>::try_from(q).map_err(|_| {
                        Error::Unsupported("Gamma exponent must be an exact integer")
                    })?;
                    if !exact.im.is_zero() || !exact.re.is_integer() {
                        return Err(Error::Unsupported(
                            "Gamma exponent must be an exact integer",
                        ));
                    }
                    exact
                        .re
                        .numerator_ref()
                        .to_i64()
                        .filter(|n| *n >= -limits.integer_power && *n <= limits.integer_power)
                        .ok_or(Error::ResourceIncomplete("Gamma exponent"))?
                }
            };
            let argument = f.iter().next().unwrap().to_owned();
            // Bound native conversion/differentiation work before invoking the
            // shared exact affine recognizer on the Gamma argument.
            RationalPrefactor::admit(argument.clone(), regulator, limits)?;
            let (constant, slopes) =
                crate::generation::endpoint_power_with_regulators(&argument, &[regulator])
                    .map_err(|_| {
                        Error::Unsupported("Gamma argument must be rational affine in epsilon")
                    })?;
            let slope = slopes[0].clone();
            if slope.is_zero() && constant <= Rational::zero() && constant.is_integer() {
                return Err(Error::Unsupported(
                    "identically singular constant Gamma argument",
                ));
            }
            gamma.push(GammaPower {
                argument,
                constant,
                slope,
                power,
            });
        }
        let residual = match RationalPrefactor::admit(rational.clone(), regulator, limits) {
            Ok(p) => PrefactorResidual::Rational(p),
            Err(Error::Unsupported(_)) => PrefactorResidual::ConstantPolynomial(
                ConstantPolynomialPrefactor::admit(rational, regulator, limits, constants)?,
            ),
            Err(e) => return Err(e),
        };
        // Atom normalization already owns the product identity. Retain every
        // original Gamma atom and exponent; never replace it with a value at
        // the witness or remove an apparent pole in isolation.
        Ok(Self {
            original,
            residual,
            gamma,
            scales,
        })
    }
}

#[derive(Clone, Debug)]
pub struct GammaWitness {
    term: usize,
    factor: usize,
    argument_value: Rational,
    pole_distance: Option<Rational>,
}
impl GammaWitness {
    pub fn term(&self) -> usize {
        self.term
    }
    pub fn factor(&self) -> usize {
        self.factor
    }
    pub fn argument_value(&self) -> &Rational {
        &self.argument_value
    }
    pub fn pole_distance(&self) -> Option<&Rational> {
        self.pole_distance.as_ref()
    }
}
#[derive(Clone, Debug)]
pub struct MeromorphicWitness {
    prefactors: Vec<MeromorphicPrefactor>,
    rational: SharedWitness,
    rational_term_indices: Vec<usize>,
    gamma: Vec<GammaWitness>,
}
impl MeromorphicWitness {
    pub fn prefactors(&self) -> &[MeromorphicPrefactor] {
        &self.prefactors
    }
    pub fn rational_witness(&self) -> &SharedWitness {
        &self.rational
    }
    /// Original term indices for the actual rational subset in rational_witness.
    pub fn rational_term_indices(&self) -> &[usize] {
        &self.rational_term_indices
    }
    pub fn gamma_witnesses(&self) -> &[GammaWitness] {
        &self.gamma
    }
    pub fn construct(
        prefactors: Vec<MeromorphicPrefactor>,
        lower: Option<&Rational>,
        upper: Option<&Rational>,
        limits: Limits,
        observer: impl FnMut(usize) -> ControlFlow<()>,
    ) -> Result<Self> {
        if prefactors
            .windows(2)
            .any(|pair| pair[0].residual.regulator() != pair[1].residual.regulator())
        {
            return Err(Error::Invalid("prefactor regulator mismatch"));
        }
        let rational_term_indices = prefactors
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.rational().map(|_| i))
            .collect::<Vec<_>>();
        let (a, b) = rational::inner_interval(lower, upper)?;
        let mut excluded = 0usize;
        for gamma in prefactors.iter().flat_map(|p| &p.gamma) {
            excluded = excluded
                .checked_add(gamma.pole_count_bound(&a, &b)?)
                .ok_or(Error::ResourceIncomplete("total Gamma pole count"))?;
        }
        let rational = SharedWitness::construct_with(
            &prefactors
                .iter()
                .filter_map(|p| p.rational().cloned())
                .collect::<Vec<_>>(),
            lower,
            upper,
            limits,
            excluded,
            |epsilon| {
                prefactors
                    .iter()
                    .flat_map(|p| &p.gamma)
                    .all(|g| !g.has_pole_at(epsilon))
            },
            observer,
        )?;
        let mut gamma = Vec::new();
        for (term, prefactor) in prefactors.iter().enumerate() {
            for (factor, g) in prefactor.gamma.iter().enumerate() {
                gamma.push(GammaWitness {
                    term,
                    factor,
                    argument_value: g.value_at(rational.epsilon()),
                    pole_distance: g.pole_distance(rational.epsilon())?,
                });
            }
        }
        Ok(Self {
            prefactors,
            rational,
            rational_term_indices,
            gamma,
        })
    }
}
