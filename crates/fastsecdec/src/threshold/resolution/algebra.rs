use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::rational::{Q, RationalField},
    poly::{MonomialOrder, PolyVariable, polynomial::MultivariatePolynomial},
};

pub type Poly = MultivariatePolynomial<RationalField, u16>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Error {
    Invalid(&'static str),
    ResourceIncomplete(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

/// Preflight limits for checker-owned construction. Native Groebner internals
/// are not interruptible through this API; callers still own hard process caps.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Limits {
    pub max_mark: usize,
    pub max_generators: usize,
    pub max_terms: usize,
    pub max_degree_per_axis: u16,
    pub max_operations: usize,
    pub max_total_ideal_slots: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            max_mark: 64,
            max_generators: 4096,
            max_terms: 65536,
            max_degree_per_axis: 1024,
            max_operations: 100_000,
            max_total_ideal_slots: 100_000,
        }
    }
}
pub struct Budget {
    pub limits: Limits,
    operations: usize,
    ideal_slots: usize,
}
impl Budget {
    pub fn new(limits: Limits) -> Self {
        Self {
            limits,
            operations: 0,
            ideal_slots: 0,
        }
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn ideal_slots(&self) -> usize {
        self.ideal_slots
    }
    /// Reserve checker-owned vector slots before cloning/constructing ideals.
    /// This does not purport to count allocations inside the native F4 engine.
    pub(crate) fn reserve_slots(&mut self, n: usize) -> Result<()> {
        self.count(n)?;
        let total = self
            .ideal_slots
            .checked_add(n)
            .ok_or(Error::ResourceIncomplete(
                "ideal allocation counter overflow",
            ))?;
        if total > self.limits.max_total_ideal_slots {
            return Err(Error::ResourceIncomplete("total ideal allocation budget"));
        }
        self.ideal_slots = total;
        Ok(())
    }
    pub(crate) fn charge(&mut self, n: usize) -> Result<()> {
        let total = self
            .operations
            .checked_add(n)
            .ok_or(Error::ResourceIncomplete("operation count overflow"))?;
        if total > self.limits.max_operations {
            return Err(Error::ResourceIncomplete("operation budget"));
        }
        self.operations = total;
        Ok(())
    }
    pub(crate) fn count(&self, n: usize) -> Result<()> {
        if n > self.limits.max_generators {
            Err(Error::ResourceIncomplete("generator budget"))
        } else {
            Ok(())
        }
    }
    pub(crate) fn poly<O: MonomialOrder>(
        &self,
        p: &MultivariatePolynomial<RationalField, u16, O>,
    ) -> Result<()> {
        if p.nterms() > self.limits.max_terms {
            return Err(Error::ResourceIncomplete("polynomial term budget"));
        }
        if (0..p.nvars()).any(|i| p.degree(i) > self.limits.max_degree_per_axis) {
            return Err(Error::ResourceIncomplete("polynomial degree budget"));
        }
        Ok(())
    }
    pub(crate) fn mul(&mut self, a: &Poly, b: &Poly) -> Result<Poly> {
        self.poly(a)?;
        self.poly(b)?;
        let terms = a
            .nterms()
            .checked_mul(b.nterms())
            .ok_or(Error::ResourceIncomplete("term product overflow"))?;
        if terms > self.limits.max_terms {
            return Err(Error::ResourceIncomplete("term product bound"));
        }
        for i in 0..a.nvars() {
            if a.degree(i)
                .checked_add(b.degree(i))
                .is_none_or(|d| d > self.limits.max_degree_per_axis)
            {
                return Err(Error::ResourceIncomplete("degree product bound"));
            }
        }
        self.charge(1)?;
        let p = a * b;
        self.poly(&p)?;
        Ok(p)
    }
    pub(crate) fn power(&mut self, p: &Poly, n: usize) -> Result<Poly> {
        if n > self.limits.max_mark {
            return Err(Error::ResourceIncomplete("power budget"));
        }
        let exponent =
            u32::try_from(n).map_err(|_| Error::ResourceIncomplete("power conversion"))?;
        if p.nterms()
            .checked_pow(exponent)
            .is_none_or(|v| v > self.limits.max_terms)
        {
            return Err(Error::ResourceIncomplete("power term bound"));
        }
        for i in 0..p.nvars() {
            if usize::from(p.degree(i))
                .checked_mul(n)
                .is_none_or(|v| v > usize::from(self.limits.max_degree_per_axis))
            {
                return Err(Error::ResourceIncomplete("power degree bound"));
            }
        }
        self.charge(1)?;
        let result = p.pow(n);
        self.poly(&result)?;
        Ok(result)
    }
    pub(crate) fn substitute(&mut self, p: &Poly, axis: usize, image: &Poly) -> Result<Poly> {
        self.poly(p)?;
        self.poly(image)?;
        let mut terms = 0usize;
        for exponents in p.exponents_iter() {
            let n = image
                .nterms()
                .checked_pow(u32::from(exponents[axis]))
                .ok_or(Error::ResourceIncomplete("substitution term power"))?;
            terms = terms
                .checked_add(n)
                .ok_or(Error::ResourceIncomplete("substitution term sum"))?;
            if terms > self.limits.max_terms {
                return Err(Error::ResourceIncomplete("substitution term bound"));
            }
        }
        for j in 0..p.nvars() {
            let old = if j == axis {
                0
            } else {
                usize::from(p.degree(j))
            };
            let added = usize::from(p.degree(axis))
                .checked_mul(usize::from(image.degree(j)))
                .ok_or(Error::ResourceIncomplete("substitution degree product"))?;
            if old
                .checked_add(added)
                .is_none_or(|v| v > usize::from(self.limits.max_degree_per_axis))
            {
                return Err(Error::ResourceIncomplete("substitution degree bound"));
            }
        }
        self.charge(1)?;
        let out = p.replace_with_poly(axis, image);
        self.poly(&out)?;
        Ok(out)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ring {
    variables: Arc<Vec<PolyVariable>>,
    parameters: BTreeSet<usize>,
}
impl Ring {
    pub fn new(symbols: Vec<Symbol>, parameters: Vec<usize>) -> Result<Self> {
        if symbols.is_empty()
            || symbols.iter().collect::<BTreeSet<_>>().len() != symbols.len()
            || parameters.iter().any(|i| *i >= symbols.len())
            || parameters.iter().collect::<BTreeSet<_>>().len() != parameters.len()
        {
            return Err(Error::Invalid("ring variables/parameter roles"));
        }
        Ok(Self {
            variables: Arc::new(symbols.into_iter().map(PolyVariable::from).collect()),
            parameters: parameters.into_iter().collect(),
        })
    }
    pub fn atom(&self, a: &Atom) -> Result<Poly> {
        let p: Poly = a
            .try_to_polynomial(&Q, self.variables.clone())
            .map_err(|_| Error::Invalid("exact Q-polynomial required"))?;
        self.check(&p)?;
        Ok(p)
    }
    pub fn coordinate(&self, i: usize) -> Result<Poly> {
        let v = self.variables.get(i).ok_or(Error::Invalid("axis index"))?;
        self.atom(&v.to_atom())
    }
    pub fn one(&self) -> Poly {
        MultivariatePolynomial::new(&Q, None, self.variables.clone()).one()
    }
    pub fn check(&self, p: &Poly) -> Result<()> {
        if p.variables() != &self.variables {
            return Err(Error::Invalid("polynomial variable map"));
        }
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.variables.len()
    }
    pub fn is_empty(&self) -> bool {
        self.variables.is_empty()
    }
    pub fn is_parameter(&self, i: usize) -> bool {
        self.parameters.contains(&i)
    }
    pub(crate) fn supports(&self, p: &Poly, axes: &[usize]) -> Result<()> {
        self.check(p)?;
        if (0..self.len()).any(|i| p.degree(i) != 0 && !self.is_parameter(i) && !axes.contains(&i))
        {
            return Err(Error::Invalid("polynomial depends on foreign coordinate"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Ideal {
    ring: Arc<Ring>,
    generators: Vec<Poly>,
}
impl Ideal {
    pub fn new(ring: Arc<Ring>, generators: Vec<Poly>, budget: &mut Budget) -> Result<Self> {
        budget.count(generators.len())?;
        for p in &generators {
            ring.check(p)?;
            budget.poly(p)?;
        }
        let mut kept = Vec::new();
        // Native F4 currently assumes nonzero generators. Removing literal
        // native zeros is also the canonical representation of the zero ideal.
        for p in generators {
            if !p.is_zero() && !kept.contains(&p) {
                kept.push(p);
            }
        }
        Ok(Self {
            ring,
            generators: kept,
        })
    }
    pub fn ring(&self) -> &Arc<Ring> {
        &self.ring
    }
    pub fn generators(&self) -> &[Poly] {
        &self.generators
    }
    pub fn sum(&self, other: &Self, budget: &mut Budget) -> Result<Self> {
        if self.ring != other.ring {
            return Err(Error::Invalid("ideal ring mismatch"));
        }
        budget.reserve_slots(
            self.generators
                .len()
                .checked_add(other.generators.len())
                .ok_or(Error::ResourceIncomplete("ideal sum count"))?,
        )?;
        Self::new(
            self.ring.clone(),
            self.generators
                .iter()
                .chain(&other.generators)
                .cloned()
                .collect(),
            budget,
        )
    }
    pub fn product(&self, other: &Self, budget: &mut Budget) -> Result<Self> {
        if self.ring != other.ring {
            return Err(Error::Invalid("ideal ring mismatch"));
        }
        budget.reserve_slots(
            self.generators
                .len()
                .checked_mul(other.generators.len())
                .ok_or(Error::ResourceIncomplete("ideal product count"))?,
        )?;
        let mut out = Vec::new();
        for a in &self.generators {
            for b in &other.generators {
                out.push(budget.mul(a, b)?);
            }
        }
        Self::new(self.ring.clone(), out, budget)
    }
    pub fn power(&self, n: usize, budget: &mut Budget) -> Result<Self> {
        if n > budget.limits.max_mark {
            return Err(Error::ResourceIncomplete("marked ideal power"));
        }
        let n32 =
            u32::try_from(n).map_err(|_| Error::ResourceIncomplete("ideal power conversion"))?;
        budget.count(
            self.generators
                .len()
                .checked_pow(n32)
                .ok_or(Error::ResourceIncomplete("ideal power count"))?,
        )?;
        let mut out = Self::new(self.ring.clone(), vec![self.ring.one()], budget)?;
        for _ in 0..n {
            out = out.product(self, budget)?;
        }
        Ok(out)
    }
    pub(crate) fn contains(
        &self,
        p: &Poly,
        relations: &[Poly],
        budget: &mut Budget,
    ) -> Result<bool> {
        self.ring.check(p)?;
        budget.poly(p)?;
        super::prepared_membership::PreparedMembership::new(self, relations, budget)?
            .contains(p, budget)
    }
}
