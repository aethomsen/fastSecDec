//! Each original generator, not merely its generated ideal, has SNC support.
use super::super::super::*;
use super::{
    super::{
        helpers::{Result, unit},
        *,
    },
    AmbientCycleCarry,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct OriginalFactorTerminal {
    source: Poly,
    total: Poly,
    boundary_powers: Vec<usize>,
    residual: Poly,
    unit: Poly,
}
impl OriginalFactorTerminal {
    pub fn source(&self) -> &Poly {
        &self.source
    }
    pub fn total(&self) -> &Poly {
        &self.total
    }
    pub fn boundary_powers(&self) -> &[usize] {
        &self.boundary_powers
    }
    pub fn residual(&self) -> &Poly {
        &self.residual
    }
    pub fn unit(&self) -> &Poly {
        &self.unit
    }
}
#[derive(Clone, Debug)]
pub struct OriginalDivisorTerminal {
    principal: Arc<LocalPrincipalization>,
    factors: Vec<OriginalFactorTerminal>,
    original_coordinate_jacobian: Poly,
}
impl OriginalDivisorTerminal {
    pub fn principal(&self) -> &Arc<LocalPrincipalization> {
        &self.principal
    }
    pub fn factors(&self) -> &[OriginalFactorTerminal] {
        &self.factors
    }
    pub fn original_coordinate_jacobian(&self) -> &Poly {
        &self.original_coordinate_jacobian
    }
}
#[derive(Clone, Debug)]
pub enum OriginalTerminalRecord {
    Terminal {
        path: FactorPath,
        proof: Arc<OriginalDivisorTerminal>,
    },
    FurtherResolution {
        path: FactorPath,
        factor: Arc<FactorLeaf>,
    },
    IndividualFactorsUnresolved {
        path: FactorPath,
        principal: Arc<LocalPrincipalization>,
    },
    ZeroIdeal {
        path: FactorPath,
        evidence: Arc<VerifiedComponentSplit>,
    },
}
#[derive(Clone, Debug)]
pub struct OriginalTerminalInventory {
    carry: Arc<AmbientCycleCarry>,
    factors: Arc<CompletedComponentFactors>,
    records: Vec<OriginalTerminalRecord>,
}
impl OriginalTerminalInventory {
    pub fn prove(
        carry: Arc<AmbientCycleCarry>,
        factors: Arc<CompletedComponentFactors>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let root = factors
            .nodes()
            .get(&Vec::new())
            .ok_or(Error::Invalid("original terminal missing root"))?;
        if !Arc::ptr_eq(root.data().source(), carry.chart().parent().target())
            || !Arc::ptr_eq(root.data().history(), carry.chart().history())
        {
            return Err(Error::Invalid("original terminal actual source/history"));
        }
        b.reserve_slots(factors.nodes().len())?;
        let mut records = Vec::new();
        for (path, node) in factors.nodes() {
            if let Some(zero) = node.zero_ideal() {
                records.push(OriginalTerminalRecord::ZeroIdeal {
                    path: path.clone(),
                    evidence: zero.clone(),
                });
                continue;
            }
            let Some(leaf) = node.leaf() else { continue };
            let principal = match certify_local_principalization(
                leaf.clone(),
                &format!("{namespace}_{path:?}"),
                b,
            )? {
                PrincipalizationProduction::FurtherResolution { factor } => {
                    records.push(OriginalTerminalRecord::FurtherResolution {
                        path: path.clone(),
                        factor,
                    });
                    continue;
                }
                PrincipalizationProduction::Principal(principal) => principal,
            };
            let data = leaf.data();
            let local = data.history().ledger().frame().local();
            // Principalization of an ideal does not say that EACH labelled
            // polynomial factor has SNC support: every individual residual ratio
            // must be a unit. Retain this sufficient-path refusal explicitly.
            let mut individual_units = true;
            for q in principal.residual_quotients() {
                individual_units &= unit(local, q, b)?;
            }
            if !individual_units {
                records.push(OriginalTerminalRecord::IndividualFactorsUnresolved {
                    path: path.clone(),
                    principal,
                });
                continue;
            }
            let mut original_factors = Vec::new();
            b.reserve_slots(carry.generators().len())?;
            let mut powers = Vec::new();
            b.reserve_slots(data.powers().len())?;
            for (d, p) in data.history().ledger().divisors().iter().zip(data.powers()) {
                let old = carry
                    .exceptional()
                    .history()
                    .ledger()
                    .divisors()
                    .iter()
                    .position(|q| q.id == d.id)
                    .ok_or(Error::Invalid(
                        "terminal boundary original history association",
                    ))?;
                powers.push(p.checked_add(carry.exceptional().powers()[old]).ok_or(
                    Error::ResourceIncomplete("original terminal power overflow"),
                )?);
            }
            if powers.len() != carry.exceptional().powers().len() {
                return Err(Error::Invalid("terminal localization boundary inventory"));
            }
            let exceptional_unit = pull_path(&factors, path, carry.exceptional().unit(), b)?;
            for original in carry.generators() {
                let index = root
                    .data()
                    .source()
                    .ideal()
                    .generators()
                    .iter()
                    .position(|p| p == original.current_quotient())
                    .ok_or(Error::Invalid("terminal original generator association"))?;
                let q = principal
                    .residual_quotients()
                    .get(index)
                    .ok_or(Error::Invalid("terminal residual generator index"))?;
                let total = pull_path(&factors, path, original.total(), b)?;
                let multiplier = b.mul(&exceptional_unit, q)?;
                if !unit(local, &multiplier, b)? {
                    return Err(Error::Invalid("original terminal nonunit multiplier"));
                }
                let mut expected = b.mul(&multiplier, principal.residual_generator())?;
                for (d, p) in data.history().ledger().divisors().iter().zip(&powers) {
                    let f = b.power(&d.equation, *p)?;
                    expected = b.mul(&expected, &f)?;
                }
                if !local.zero(&(&total - &expected), b)? {
                    return Err(Error::Invalid("original factor SNC identity"));
                }
                original_factors.push(OriginalFactorTerminal {
                    source: original.source().clone(),
                    total,
                    boundary_powers: powers.clone(),
                    residual: principal.residual_generator().clone(),
                    unit: multiplier,
                });
            }
            let original_coordinate_jacobian =
                pull_path(&factors, path, carry.original_coordinate_jacobian(), b)?;
            records.push(OriginalTerminalRecord::Terminal {
                path: path.clone(),
                proof: Arc::new(OriginalDivisorTerminal {
                    principal,
                    factors: original_factors,
                    original_coordinate_jacobian,
                }),
            });
        }
        Ok(Arc::new(Self {
            carry,
            factors,
            records,
        }))
    }
    pub fn carry(&self) -> &Arc<AmbientCycleCarry> {
        &self.carry
    }
    pub fn factor_inventory(&self) -> &Arc<CompletedComponentFactors> {
        &self.factors
    }
    pub fn records(&self) -> &[OriginalTerminalRecord] {
        &self.records
    }
    /// Only local labelled-divisor geometry. No real atlas or density authority.
    pub fn all_local_factors_snc(&self) -> bool {
        !self.records.is_empty()
            && self
                .records
                .iter()
                .all(|r| matches!(r, OriginalTerminalRecord::Terminal { .. }))
    }
}
fn pull_path(
    factors: &CompletedComponentFactors,
    path: &[usize],
    p: &Poly,
    b: &mut Budget,
) -> Result<Poly> {
    let mut out = p.clone();
    let mut prefix = Vec::new();
    b.reserve_slots(path.len())?;
    for side in path {
        let source = factors
            .nodes()
            .get(&prefix)
            .ok_or(Error::Invalid("terminal source factor path"))?;
        prefix.push(*side);
        let node = factors
            .nodes()
            .get(&prefix)
            .ok_or(Error::Invalid("terminal target factor path"))?;
        let restriction = node
            .parent()
            .ok_or(Error::Invalid("terminal missing factor localization"))?;
        if !Arc::ptr_eq(restriction.open().source(), source.data().history())
            || !Arc::ptr_eq(restriction.history(), node.data().history())
        {
            return Err(Error::Invalid("terminal factor localization owner"));
        }
        out = restriction.open().extension().pull(&out, b)?;
    }
    Ok(out)
}
