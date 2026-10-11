//! Original marked-generator exceptional powers on the actual current SNC ledger.
use super::super::super::*;
use super::super::{
    helpers::{Result, unit},
    *,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct AccumulatedExceptional {
    history: Arc<ResolutionHistory>,
    powers: Vec<usize>,
    unit: Poly,
    product: Poly,
}
impl AccumulatedExceptional {
    pub(super) fn prepare(
        previous: &RelativeRecursiveChart,
        current: &RelativeRecursiveChart,
        b: &mut Budget,
    ) -> Result<Self> {
        Self::first(previous, b)?.advance(current, b)
    }
    pub(super) fn first(chart: &RelativeRecursiveChart, b: &mut Budget) -> Result<Self> {
        let local = chart.geometry().frame().local();
        let ledger = chart.history().ledger();
        b.reserve_slots(ledger.divisors().len())?;
        let mut powers = vec![0; ledger.divisors().len()];
        let mut multiplier = local.ring().one();
        let product = if let Some(e) = chart.geometry().exceptional() {
            let exponent = chart.parent().source().mark();
            let p = b.power(e, exponent)?;
            if unit(local, e, b)? {
                multiplier = p.clone();
            } else {
                let id = chart
                    .geometry()
                    .born()
                    .ok_or(Error::Invalid("original exceptional birth missing"))?
                    .0;
                let index = ledger
                    .divisors()
                    .iter()
                    .position(|d| d.id == id && d.equation == *e)
                    .ok_or(Error::Invalid("first exceptional ledger association"))?;
                powers[index] = exponent;
            }
            p
        } else {
            local.ring().one()
        };
        Self::checked(chart.history().clone(), powers, multiplier, product, b)
    }
    fn checked(
        history: Arc<ResolutionHistory>,
        powers: Vec<usize>,
        multiplier: Poly,
        expected: Poly,
        b: &mut Budget,
    ) -> Result<Self> {
        let ledger = history.ledger();
        let local = ledger.frame().local();
        if powers.len() != ledger.divisors().len() || !unit(local, &multiplier, b)? {
            return Err(Error::Invalid("accumulated exceptional unit/shape"));
        }
        let mut product = multiplier.clone();
        for (d, p) in ledger.divisors().iter().zip(&powers) {
            let f = b.power(&d.equation, *p)?;
            product = b.mul(&product, &f)?;
        }
        if !local.zero(&(&expected - &product), b)? {
            return Err(Error::Invalid("accumulated exceptional ledger identity"));
        }
        Ok(Self {
            history,
            powers,
            unit: multiplier,
            product,
        })
    }
    pub(super) fn restrict(&self, open: &LocalizedCoverHistory, b: &mut Budget) -> Result<Self> {
        if !Arc::ptr_eq(open.open().source(), &self.history)
            || !open.history().same_root(&self.history)
            || open.history().births() != self.history.births()
        {
            return Err(Error::Invalid("exceptional restriction history"));
        }
        let extension = open.open().extension();
        let local = open.history().ledger().frame().local();
        b.reserve_slots(open.history().ledger().divisors().len())?;
        let mut powers = Vec::new();
        for d in open.history().ledger().divisors() {
            let index = self
                .history
                .ledger()
                .divisors()
                .iter()
                .position(|old| old.id == d.id)
                .ok_or(Error::Invalid("exceptional restriction birth association"))?;
            let pulled = extension.pull(&self.history.ledger().divisors()[index].equation, b)?;
            if !local.zero(&(&pulled - &d.equation), b)? {
                return Err(Error::Invalid("exceptional restriction equation"));
            }
            powers.push(self.powers[index]);
        }
        if powers.len() != self.powers.len() {
            return Err(Error::Invalid("exceptional restriction complete ledger"));
        }
        Self::checked(
            open.history().clone(),
            powers,
            extension.pull(&self.unit, b)?,
            extension.pull(&self.product, b)?,
            b,
        )
    }
    pub(super) fn advance(&self, current: &RelativeRecursiveChart, b: &mut Budget) -> Result<Self> {
        if !Arc::ptr_eq(current.geometry().center().history(), &self.history) {
            return Err(Error::Invalid("exceptional physical transition source"));
        }
        let local = current.geometry().frame().local();
        let ledger = current.history().ledger();
        b.reserve_slots(ledger.divisors().len())?;
        let mut powers = vec![0usize; ledger.divisors().len()];
        let mut multiplier = current.geometry().pull(&self.unit, b)?;
        let mut newborn_power = current.parent().source().mark();
        for (old, exponent) in self.history.ledger().divisors().iter().zip(&self.powers) {
            if *exponent == 0 {
                continue;
            }
            let total = current.geometry().pull(&old.equation, b)?;
            if unit(local, &total, b)? {
                let p = b.power(&total, *exponent)?;
                multiplier = b.mul(&multiplier, &p)?;
                continue;
            }
            let receipt = current
                .geometry()
                .divisors()
                .iter()
                .find(|d| d.id == old.id)
                .ok_or(Error::Invalid(
                    "original exceptional strict transform missing",
                ))?;
            if !local.zero(&(&total - &receipt.total), b)? {
                return Err(Error::Invalid("original exceptional pullback association"));
            }
            let e = match current.geometry().exceptional() {
                Some(e) => b.power(e, receipt.exceptional_power)?,
                None => {
                    if receipt.exceptional_power != 0 {
                        return Err(Error::Invalid("exceptional factor without pivot"));
                    }
                    local.ring().one()
                }
            };
            let strict = b.mul(&receipt.unit, &receipt.strict_equation)?;
            let expected = b.mul(&strict, &e)?;
            if !local.zero(&(&total - &expected), b)? || !unit(local, &receipt.unit, b)? {
                return Err(Error::Invalid("original exceptional strict recombination"));
            }
            let p = b.power(&receipt.unit, *exponent)?;
            multiplier = b.mul(&multiplier, &p)?;
            if receipt.absent || unit(local, &receipt.strict_equation, b)? {
                if !unit(local, &receipt.strict_equation, b)? {
                    return Err(Error::Invalid("absent original exceptional not unit"));
                }
                let p = b.power(&receipt.strict_equation, *exponent)?;
                multiplier = b.mul(&multiplier, &p)?;
            } else {
                let index = ledger
                    .divisors()
                    .iter()
                    .position(|d| d.id == old.id && d.equation == receipt.strict_equation)
                    .ok_or(Error::Invalid(
                        "original exceptional current ledger association",
                    ))?;
                powers[index] =
                    powers[index]
                        .checked_add(*exponent)
                        .ok_or(Error::ResourceIncomplete(
                            "exceptional multiplicity overflow",
                        ))?;
            }
            newborn_power = newborn_power
                .checked_add(exponent.checked_mul(receipt.exceptional_power).ok_or(
                    Error::ResourceIncomplete("exceptional multiplicity overflow"),
                )?)
                .ok_or(Error::ResourceIncomplete(
                    "exceptional multiplicity overflow",
                ))?;
        }
        let mut expected = current.geometry().pull(&self.product, b)?;
        if let Some(e) = current.geometry().exceptional() {
            let p = b.power(e, current.parent().source().mark())?;
            expected = b.mul(&expected, &p)?;
            if unit(local, e, b)? {
                let p = b.power(e, newborn_power)?;
                multiplier = b.mul(&multiplier, &p)?;
            } else {
                let id = current
                    .geometry()
                    .born()
                    .ok_or(Error::Invalid("current exceptional birth missing"))?
                    .0;
                let index = ledger
                    .divisors()
                    .iter()
                    .position(|d| d.id == id && d.equation == *e)
                    .ok_or(Error::Invalid("current exceptional ledger association"))?;
                powers[index] =
                    powers[index]
                        .checked_add(newborn_power)
                        .ok_or(Error::ResourceIncomplete(
                            "exceptional multiplicity overflow",
                        ))?;
            }
        } else if newborn_power != current.parent().source().mark() {
            return Err(Error::Invalid(
                "accumulated exceptional missing current pivot",
            ));
        }
        Self::checked(current.history().clone(), powers, multiplier, expected, b)
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn powers(&self) -> &[usize] {
        &self.powers
    }
    pub fn unit(&self) -> &Poly {
        &self.unit
    }
    pub fn product(&self) -> &Poly {
        &self.product
    }
}
