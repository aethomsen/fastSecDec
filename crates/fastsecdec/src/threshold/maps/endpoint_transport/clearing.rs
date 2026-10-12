//! Bounded native QQ rational clearing. No closed-face unit inference.
use super::{Error, Result, Work};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, AtomView, Symbol},
    domains::{
        integer::Z,
        rational::{Q, Rational},
        rational_polynomial::{FromNumeratorAndDenominator, RationalPolynomial},
    },
    id::{Pattern, Replacement},
    poly::PolyVariable,
};
use symgcad::algebra::Poly;

pub(super) fn pull(a: &Atom, source: &[Symbol], images: &[Atom]) -> Atom {
    a.replace_multiple(source.iter().zip(images).map(|(s, v)| {
        Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(v.clone()))
    }))
}
#[derive(Debug)]
pub(super) struct Ring {
    symbols: Vec<Symbol>,
    variables: Arc<Vec<PolyVariable>>,
    occupied: BTreeSet<Symbol>,
}
impl Ring {
    pub fn new(symbols: Vec<Symbol>) -> Result<Self> {
        if symbols.iter().collect::<BTreeSet<_>>().len() != symbols.len() {
            return Err(Error::Invalid("duplicate polynomial role".into()));
        }
        let variables = Arc::new(symbols.iter().copied().map(PolyVariable::from).collect());
        let occupied = symbols.iter().copied().collect();
        Ok(Self {
            symbols,
            variables,
            occupied,
        })
    }
    pub fn reserve(&mut self, symbols: impl Iterator<Item = Symbol>) {
        self.occupied.extend(symbols);
    }
    fn preflight(&self, a: &Atom, work: &mut Work<'_>) -> Result<()> {
        work.check()?;
        if !a
            .get_all_symbols(true)
            .is_subset(&self.symbols.iter().copied().collect())
        {
            return Err(Error::Unsupported(
                "rational transport contains a foreign polynomial role".into(),
            ));
        }
        use crate::threshold::regularization::meromorphic::{Limits, preflight_expression};
        preflight_expression(
            a,
            Limits {
                degree: u16::try_from(work.limits.algebra.max_degree).map_err(super::native)?,
                terms: work.limits.algebra.max_terms,
                source_nodes: work.limits.algebra.max_terms.saturating_mul(8),
                integer_power: work.limits.algebra.max_degree as i64,
                ..Default::default()
            },
        )
        .map_err(|e| Error::ResourceIncomplete(format!("transport rational preflight: {e:?}")))?;
        Ok(())
    }
    fn bounded(&self, p: &Poly, work: &mut Work<'_>) -> Result<()> {
        work.check()?;
        if p.variables() != &self.variables {
            return Err(Error::Invalid(
                "native polynomial changed role inventory".into(),
            ));
        }
        if p.nterms() > work.limits.algebra.max_terms
            || (0..p.nvars()).any(|i| p.degree(i) as usize > work.limits.algebra.max_degree)
        {
            return Err(Error::ResourceIncomplete(
                "transport polynomial allowance".into(),
            ));
        }
        for c in &p.coefficients {
            work.budget.rational(c)?;
        }
        Ok(())
    }
    pub fn polynomial(&self, a: &Atom, work: &mut Work<'_>) -> Result<Poly> {
        self.preflight(a, work)?;
        let p = a
            .try_to_polynomial(&Q, self.variables.clone())
            .map_err(super::native)?;
        self.bounded(&p, work)?;
        Ok(p)
    }
    /// Resource admission only. Repeated known inverse bases share a scratch
    /// slot, so native maximum degrees bound ONE common denominator. The
    /// original expression, not this scratch polynomial, enters the native QQ
    /// fraction conversion below. General meromorphic preflight is unchanged.
    fn inverse_preflight(&self, a: &Atom, units: &[Symbol], work: &mut Work<'_>) -> Result<()> {
        work.check()?;
        if !a
            .get_all_symbols(true)
            .is_subset(&self.symbols.iter().copied().collect())
        {
            return Err(Error::Unsupported("foreign rational transport role".into()));
        }
        let bases = units
            .iter()
            .flat_map(|s| [Atom::var(*s), Atom::one() - Atom::var(*s)])
            .collect::<Vec<_>>();
        let inverse = (0usize..)
            .map(|namespace| {
                (0..bases.len())
                    .map(|i| {
                        symbolica::symbol!(format!(
                            "fastsecdec::endpoint_inverse_{namespace}::i{i}"
                        ))
                    })
                    .collect::<Vec<_>>()
            })
            .find(|symbols| symbols.iter().all(|s| !self.occupied.contains(s)))
            .expect("finite reserved symbol inventory");
        let negative = bases.iter().map(|b| (-b).expand()).collect::<Vec<_>>();
        let mut replacements = Vec::new();
        let mut nodes = 0usize;
        let mut failure = None;
        a.visitor(&mut |node| {
            if failure.is_some() {
                return false;
            }
            if let Err(e) = work.check() {
                failure = Some(e);
                return false;
            }
            nodes = match nodes.checked_add(1) {
                Some(n) => n,
                None => {
                    failure = Some(Error::ResourceIncomplete(
                        "inverse preflight node overflow".into(),
                    ));
                    return false;
                }
            };
            if nodes > work.limits.algebra.max_terms.saturating_mul(8) {
                failure = Some(Error::ResourceIncomplete(
                    "inverse preflight node allowance".into(),
                ));
                return false;
            }
            match node {
                AtomView::Num(_) => match Rational::try_from(node) {
                    Ok(value) => {
                        if let Err(e) = work.budget.rational(&value) {
                            failure = Some(e);
                            return false;
                        }
                    }
                    Err(_) => {
                        failure = Some(Error::Unsupported("non-QQ transport coefficient".into()));
                        return false;
                    }
                },
                AtomView::Pow(p) => {
                    let Some(exponent) = Rational::try_from(p.get_exp())
                        .ok()
                        .filter(|r| r.is_integer())
                        .and_then(|r| r.numerator_ref().to_i64())
                    else {
                        failure = Some(Error::Unsupported(
                            "noninteger rational transport power".into(),
                        ));
                        return false;
                    };
                    if exponent.unsigned_abs() > work.limits.algebra.max_degree as u64 {
                        failure = Some(Error::ResourceIncomplete(
                            "inverse preflight power allowance".into(),
                        ));
                        return false;
                    }
                    if exponent < 0 {
                        let base = p.get_base().to_owned();
                        let found = bases
                            .iter()
                            .position(|b| b == &base)
                            .map(|i| (i, false))
                            .or_else(|| {
                                negative.iter().position(|b| b == &base).map(|i| (i, true))
                            });
                        let Some((i, negated)) = found else {
                            failure = Some(Error::Unsupported(
                                "inverse base is not an original unit or complement".into(),
                            ));
                            return false;
                        };
                        let Ok(power) = u32::try_from(exponent.unsigned_abs()) else {
                            failure =
                                Some(Error::ResourceIncomplete("inverse exponent width".into()));
                            return false;
                        };
                        let sign = if negated && power % 2 == 1 { -1 } else { 1 };
                        replacements.push(Replacement::new(
                            Pattern::Literal(node.to_owned()),
                            Pattern::Literal(Atom::num(sign) * Atom::var(inverse[i]).pow(power)),
                        ));
                        return false;
                    }
                }
                AtomView::Fun(_) => {
                    failure = Some(Error::Unsupported(
                        "opaque function in rational transport".into(),
                    ));
                    return false;
                }
                _ => {}
            }
            true
        });
        if let Some(e) = failure {
            return Err(e);
        }
        let scratch = Ring::new(self.symbols.iter().chain(&inverse).copied().collect())?;
        let lifted = a.replace_multiple(replacements);
        // This runs the existing syntax degree/term/node bound BEFORE native
        // polynomial collection, then the usual exact QQ term/coefficient caps.
        let p = scratch.polynomial(&lifted, work)?;
        let overflow = || Error::ResourceIncomplete("common denominator clearing bound".into());
        let degrees = (0..inverse.len())
            .map(|i| p.degree(self.symbols.len() + i))
            .collect::<Vec<_>>();
        let denominator_degree = degrees
            .iter()
            .try_fold(0usize, |n, d| n.checked_add(*d as usize))
            .ok_or_else(overflow)?;
        let numerator_degree = (0..self.symbols.len())
            .try_fold(denominator_degree, |n, i| {
                n.checked_add(p.degree(i) as usize)
            })
            .ok_or_else(overflow)?;
        let upper_degree = degrees
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 1)
            .try_fold(0u32, |n, (_, d)| n.checked_add(*d))
            .ok_or_else(overflow)?;
        let terms = p
            .nterms()
            .checked_mul(2usize.checked_pow(upper_degree).ok_or_else(overflow)?)
            .ok_or_else(overflow)?;
        if denominator_degree > work.limits.algebra.max_degree
            || numerator_degree > work.limits.algebra.max_degree
            || terms > work.limits.algebra.max_terms
        {
            return Err(overflow());
        }
        work.check()
    }
    pub fn ratio(&self, a: &Atom, units: &[Symbol], work: &mut Work<'_>) -> Result<(Poly, Poly)> {
        self.inverse_preflight(a, units, work)?;
        let p: RationalPolynomial<Z, u32> = a
            .try_to_rational_polynomial(&Q, &Z, self.variables.clone())
            .map_err(super::native)?;
        let n = p.numerator.map_coeff(|c| Rational::from(c.clone()), Q);
        let d = p.denominator.map_coeff(|c| Rational::from(c.clone()), Q);
        self.bounded(&n, work)?;
        self.bounded(&d, work)?;
        Ok((n, d))
    }
}

/// D is a product of positive original unit coordinates and complements.
/// The retained parent keeps even geometry divisors canceled by native algebra.
#[derive(Debug)]
pub struct ClearedFactor {
    numerator: Atom,
    denominator: Atom,
    denominator_powers: Vec<[u32; 2]>,
}
impl ClearedFactor {
    pub fn numerator(&self) -> &Atom {
        &self.numerator
    }
    pub fn denominator(&self) -> &Atom {
        &self.denominator
    }
    /// Lower-unit and upper-complement powers in parent unit order.
    pub fn denominator_powers(&self) -> &[[u32; 2]] {
        &self.denominator_powers
    }
}
impl Ring {
    pub fn clear(
        &self,
        mapped: &Atom,
        units: &[Symbol],
        work: &mut Work<'_>,
    ) -> Result<ClearedFactor> {
        let (n, mut remainder) = self.ratio(mapped, units, work)?;
        if n.is_zero() {
            return Err(Error::Unsupported(
                "identically zero transported geometric factor".into(),
            ));
        }
        let original_d = remainder.clone();
        let mut denominator = Atom::one();
        let mut denominator_powers = Vec::with_capacity(units.len());
        for u in units {
            let mut powers = [0; 2];
            for (side, base) in [Atom::var(*u), Atom::one() - Atom::var(*u)]
                .iter()
                .enumerate()
            {
                let divisor = self.polynomial(base, work)?;
                loop {
                    work.check()?;
                    let (q, r) = remainder.quot_rem(&divisor, false);
                    if !r.is_zero() {
                        break;
                    }
                    if powers[side] as usize >= work.limits.algebra.max_degree {
                        return Err(Error::ResourceIncomplete(
                            "transport denominator power allowance".into(),
                        ));
                    }
                    if &q * &divisor != remainder {
                        return Err(Error::Invalid(
                            "native denominator division identity".into(),
                        ));
                    }
                    remainder = q;
                    powers[side] += 1;
                }
                denominator *= base.pow(powers[side]);
            }
            denominator_powers.push(powers);
        }
        let c = Rational::try_from(remainder.to_expression().as_view()).map_err(|_| {
            Error::Unsupported(
                "transport denominator is not an admitted unit/complement product".into(),
            )
        })?;
        if c.is_zero() {
            return Err(Error::Invalid("zero transport denominator".into()));
        }
        let d = self.polynomial(&denominator, work)?;
        if &d * &d.constant(c.clone()) != original_d {
            return Err(Error::Invalid(
                "complete denominator inventory identity".into(),
            ));
        }
        let natural_n = n.clone().div_coeff(&c);
        self.bounded(&natural_n, work)?;
        // Native rational conversion of the original expression and exact cross
        // multiplication replay the clearing identity without expression heuristics.
        if &natural_n * &original_d != &n * &d {
            return Err(Error::Invalid("native rational clearing identity".into()));
        }
        Ok(ClearedFactor {
            numerator: natural_n.to_expression(),
            denominator,
            denominator_powers,
        })
    }
}

/// An exact monomial times a rational residual; the residual has OPEN-domain
/// denominator authority only and may vanish or diverge on closed faces.
#[derive(Debug)]
pub struct ExtractedFactor {
    orders: Vec<symbolica::domains::integer::Integer>,
    numerator: Atom,
    denominator: Atom,
}
impl ExtractedFactor {
    pub fn orders(&self) -> &[symbolica::domains::integer::Integer] {
        &self.orders
    }
    pub fn numerator(&self) -> &Atom {
        &self.numerator
    }
    pub fn denominator(&self) -> &Atom {
        &self.denominator
    }
}
impl Ring {
    /// Canonicalize only the complete positive measure, after the once-only
    /// chart Jacobian. Original clearing/divisor records remain in the parent.
    /// Source factors with independent noninteger powers use `extract` unchanged.
    pub fn extract_measure(
        &self,
        n: &Atom,
        d: &Atom,
        units: &[Symbol],
        work: &mut Work<'_>,
    ) -> Result<ExtractedFactor> {
        let numerator = self.polynomial(n, work)?;
        let denominator = self.polynomial(d, work)?;
        if numerator.is_zero() || denominator.is_zero() {
            return Err(Error::Invalid(
                "zero full measure numerator/denominator".into(),
            ));
        }
        let before = self.extract(n, d, units, work)?.0;
        work.check()?;
        let reduced =
            RationalPolynomial::from_num_den(numerator.clone(), denominator.clone(), &Z, true);
        let mut rn = reduced
            .numerator
            .map_coeff(|c| Rational::from(c.clone()), Q);
        let mut rd = reduced
            .denominator
            .map_coeff(|c| Rational::from(c.clone()), Q);
        self.bounded(&rn, work)?;
        self.bounded(&rd, work)?;
        // Reuse the existing native preconversion product/degree bound before
        // collecting either cross-product; bounded inputs alone do not bound
        // multiplication workspace by the caller's term allowance.
        let left = self.polynomial(&(rn.to_expression() * denominator.to_expression()), work)?;
        let right = self.polynomial(&(rd.to_expression() * numerator.to_expression()), work)?;
        if left != right {
            return Err(Error::Invalid(
                "canonical full measure cross-product identity".into(),
            ));
        }
        // Native rational normalization may reverse both signs. This exact
        // interior value chooses a presentation only; closed positivity still
        // requires the downstream native margin certificates. Original measure
        // denominator ancestry already proves nonvanishing in the open cube.
        let point = self
            .symbols
            .iter()
            .map(|s| {
                if units.contains(s) {
                    Rational::from((1, 2))
                } else {
                    Rational::zero()
                }
            })
            .collect::<Vec<_>>();
        if self
            .symbols
            .iter()
            .enumerate()
            .any(|(i, s)| !units.contains(s) && rd.degree(i) != 0)
        {
            return Err(Error::Invalid(
                "measure denominator gained a root role".into(),
            ));
        }
        let orientation = rd.evaluate(Clone::clone, &point);
        if orientation.is_zero() {
            return Err(Error::Invalid(
                "canonical measure denominator vanishes at open centre".into(),
            ));
        }
        if orientation < 0 {
            rn = -rn;
            rd = -rd;
        }
        let after = self
            .extract(&rn.to_expression(), &rd.to_expression(), units, work)?
            .0;
        if before.orders != after.orders {
            return Err(Error::Invalid(
                "canonical full measure changed net endpoint orders".into(),
            ));
        }
        work.check()?;
        Ok(after)
    }
    pub fn extract(
        &self,
        n: &Atom,
        d: &Atom,
        units: &[Symbol],
        work: &mut Work<'_>,
    ) -> Result<(ExtractedFactor, Vec<u32>)> {
        let n = self.polynomial(n, work)?;
        let d = self.polynomial(d, work)?;
        if n.is_zero() || d.is_zero() {
            return Err(Error::Invalid("zero extracted geometric factor".into()));
        }
        let factor = |p: &Poly, work: &mut Work<'_>| -> Result<(Poly, Vec<u32>)> {
            let minima = units
                .iter()
                .map(|u| {
                    self.symbols
                        .iter()
                        .position(|s| s == u)
                        .map(|i| p.degree_bounds(i).0)
                        .ok_or_else(|| Error::Invalid("missing target-unit polynomial role".into()))
                })
                .collect::<Result<Vec<_>>>()?;
            let m = units
                .iter()
                .zip(&minima)
                .map(|(u, e)| Atom::var(*u).pow(*e))
                .product::<Atom>();
            let m = self.polynomial(&m, work)?;
            let (q, r) = p.quot_rem(&m, false);
            if !r.is_zero() || &q * &m != *p {
                return Err(Error::Invalid(
                    "native minimum/recombination identity".into(),
                ));
            }
            Ok((q, minima))
        };
        let (nr, nm) = factor(&n, work)?;
        let (dr, dm) = factor(&d, work)?;
        let orders = nm
            .iter()
            .zip(&dm)
            .map(|(a, b)| {
                symbolica::domains::integer::Integer::from(*a)
                    - symbolica::domains::integer::Integer::from(*b)
            })
            .collect();
        Ok((
            ExtractedFactor {
                orders,
                numerator: nr.to_expression(),
                denominator: dr.to_expression(),
            },
            nm,
        ))
    }
}

#[cfg(test)]
mod tests;
