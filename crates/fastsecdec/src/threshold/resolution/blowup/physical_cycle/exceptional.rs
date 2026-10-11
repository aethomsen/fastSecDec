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
        let local = current.geometry().frame().local();
        let ledger = current.history().ledger();
        b.reserve_slots(ledger.divisors().len())?;
        let mut powers = vec![0usize; ledger.divisors().len()];
        let mut multiplier = local.ring().one();
        let mut new_power = current.parent().source().mark();
        let mut expected = local.ring().one();
        if let Some(old) = previous.geometry().exceptional() {
            let total = current.geometry().pull(old, b)?;
            let exponent = previous.parent().source().mark();
            expected = b.power(&total, exponent)?;
            if unit(local, &total, b)? {
                multiplier = expected.clone();
            } else {
                let id = previous
                    .geometry()
                    .born()
                    .ok_or(Error::Invalid("original exceptional birth missing"))?
                    .0;
                let receipt = current
                    .geometry()
                    .divisors()
                    .iter()
                    .find(|d| d.id == id)
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
                let strict_product = b.mul(&receipt.unit, &receipt.strict_equation)?;
                let recombined = b.mul(&strict_product, &e)?;
                if !local.zero(&(&total - &recombined), b)? || !unit(local, &receipt.unit, b)? {
                    return Err(Error::Invalid("original exceptional strict recombination"));
                }
                multiplier = b.power(&receipt.unit, exponent)?;
                if receipt.absent {
                    if !unit(local, &receipt.strict_equation, b)? {
                        return Err(Error::Invalid("absent original exceptional not unit"));
                    }
                    let p = b.power(&receipt.strict_equation, exponent)?;
                    multiplier = b.mul(&multiplier, &p)?;
                } else {
                    let index = ledger
                        .divisors()
                        .iter()
                        .position(|d| d.id == id && d.equation == receipt.strict_equation)
                        .ok_or(Error::Invalid(
                            "original exceptional current ledger association",
                        ))?;
                    powers[index] = exponent;
                }
                new_power = new_power
                    .checked_add(exponent.checked_mul(receipt.exceptional_power).ok_or(
                        Error::ResourceIncomplete("exceptional multiplicity overflow"),
                    )?)
                    .ok_or(Error::ResourceIncomplete(
                        "exceptional multiplicity overflow",
                    ))?;
            }
        }
        if let Some(e) = current.geometry().exceptional() {
            let p = b.power(e, current.parent().source().mark())?;
            expected = b.mul(&expected, &p)?;
            if unit(local, e, b)? {
                let p = b.power(e, new_power)?;
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
                        .checked_add(new_power)
                        .ok_or(Error::ResourceIncomplete(
                            "exceptional multiplicity overflow",
                        ))?;
            }
        } else if new_power != current.parent().source().mark() {
            return Err(Error::Invalid(
                "accumulated exceptional missing current pivot",
            ));
        }
        if !unit(local, &multiplier, b)? {
            return Err(Error::Invalid(
                "accumulated exceptional multiplier not unit",
            ));
        }
        let mut product = multiplier.clone();
        for (d, p) in ledger.divisors().iter().zip(&powers) {
            let factor = b.power(&d.equation, *p)?;
            product = b.mul(&product, &factor)?;
        }
        if !local.zero(&(&expected - &product), b)? {
            return Err(Error::Invalid("accumulated exceptional ledger identity"));
        }
        Ok(Self {
            history: current.history().clone(),
            powers,
            unit: multiplier,
            product,
        })
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
