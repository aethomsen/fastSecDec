//! One caller-owned immutable ideal and lazy checked native membership basis.
//! Its lifetime ends with the verifier batch; there is no global cache.
use super::{Budget, Error, Ideal, LocalizedAlgebra, Poly};
use symbolica::{
    domains::rational::RationalField,
    poly::{GrevLexOrder, polynomial::MultivariatePolynomial},
};

type OrderedPoly = MultivariatePolynomial<RationalField, u16, GrevLexOrder>;
type Result<T> = std::result::Result<T, Error>;

/// The local owner fixes both the allowed support and quotient relations.
pub(super) struct PreparedLocalZero<'a> {
    local: &'a LocalizedAlgebra,
    membership: PreparedMembership<'a>,
}
impl<'a> PreparedLocalZero<'a> {
    pub(super) fn new(local: &'a LocalizedAlgebra, budget: &mut Budget) -> Result<Self> {
        Ok(Self {
            local,
            membership: PreparedMembership::new(local.ideal(), local.unit_relations(), budget)?,
        })
    }
    pub(super) fn zero(&mut self, p: &Poly, budget: &mut Budget) -> Result<bool> {
        self.local.supports(p)?;
        self.membership.contains(p, budget)
    }
    pub(super) fn is_for(&self, local: &LocalizedAlgebra) -> bool {
        std::ptr::eq(self.local, local)
    }
}

pub(super) struct PreparedMembership<'a> {
    // The borrow keeps the exact source and quotient relations associated.
    _source: &'a Ideal,
    _relations: &'a [Poly],
    source_slots: usize,
    combined: Ideal,
    basis: Option<Vec<OrderedPoly>>,
}

impl<'a> PreparedMembership<'a> {
    pub(super) fn new(
        source: &'a Ideal,
        relations: &'a [Poly],
        budget: &mut Budget,
    ) -> Result<Self> {
        let source_slots = source
            .generators()
            .len()
            .checked_add(relations.len())
            .ok_or(Error::ResourceIncomplete("localized ideal count"))?;
        budget.reserve_slots(source_slots)?;
        let combined = Ideal::new(
            source.ring().clone(),
            source
                .generators()
                .iter()
                .chain(relations)
                .cloned()
                .collect(),
            budget,
        )?;
        Ok(Self {
            _source: source,
            _relations: relations,
            source_slots,
            combined,
            basis: None,
        })
    }

    pub(super) fn contains(&mut self, query: &Poly, budget: &mut Budget) -> Result<bool> {
        self.combined.ring().check(query)?;
        budget.poly(query)?;
        // Limits are caller-owned and mutable; admission under an earlier
        // budget does not exempt an existing source or basis from tighter caps.
        budget.count(self.source_slots)?;
        for p in self.combined.generators() {
            budget.poly(p)?;
        }
        // Native division proves membership when its remainder is zero, even
        // for non-Groebner original generators. Nonzero still needs the basis.
        budget.charge(1)?;
        let direct = query.reduce(self.combined.generators());
        budget.poly(&direct)?;
        if direct.is_zero() {
            return Ok(true);
        }
        if self.basis.is_none() {
            budget.reserve_slots(self.combined.generators().len())?;
            let generators = self
                .combined
                .generators()
                .iter()
                .map(|p| p.reorder::<GrevLexOrder>())
                .collect::<Vec<_>>();
            // This is the existing exact native generation AND verification.
            // Assignment occurs only after every check succeeds, so failure
            // cannot leave a partly checked basis behind for the next query.
            self.basis = Some(super::native_basis::checked_ordered(&generators, budget)?);
        }
        let basis = self.basis.as_ref().expect("checked basis initialized");
        budget.count(basis.len())?;
        for p in basis {
            budget.poly(p)?;
        }
        budget.charge(1)?;
        let remainder = query.reorder::<GrevLexOrder>().reduce(basis);
        budget.poly(&remainder)?;
        Ok(remainder.is_zero())
    }
}

#[cfg(test)]
#[path = "prepared_membership/tests.rs"]
mod tests;
