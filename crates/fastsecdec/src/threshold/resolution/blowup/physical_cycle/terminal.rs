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
        let records = prove_original_terminal(
            OriginalTerminalInput {
                source: carry.chart().parent().target(),
                history: carry.chart().history(),
                exceptional: carry.exceptional(),
                generators: carry.generators(),
                jacobian: carry.original_coordinate_jacobian(),
            },
            &factors,
            namespace,
            b,
        )?;
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
pub(super) struct OriginalTerminalInput<'a> {
    pub source: &'a Arc<MarkedIdeal>,
    pub history: &'a Arc<ResolutionHistory>,
    pub exceptional: &'a super::AccumulatedExceptional,
    pub generators: &'a [super::OriginalGeneratorPullback],
    pub jacobian: &'a Poly,
}
pub(super) fn prove_original_terminal(
    input: OriginalTerminalInput<'_>,
    factors: &CompletedComponentFactors,
    namespace: &str,
    b: &mut Budget,
) -> Result<Vec<OriginalTerminalRecord>> {
    let root = factors
        .nodes()
        .get(&Vec::new())
        .ok_or(Error::Invalid("original terminal missing root"))?;
    if !Arc::ptr_eq(root.data().source(), input.source)
        || !Arc::ptr_eq(root.data().history(), input.history)
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
        // Retain every original keyed polynomial, including its coefficient.
        // A normalized current ideal may use different basis rows; native
        // Cartier/unit division supplies the individual residual quotient.
        let mut original_factors = Vec::new();
        b.reserve_slots(input.generators.len())?;
        let mut powers = Vec::new();
        b.reserve_slots(data.powers().len())?;
        for (d, p) in data.history().ledger().divisors().iter().zip(data.powers()) {
            let old = input
                .exceptional
                .history()
                .ledger()
                .divisors()
                .iter()
                .position(|q| q.id == d.id)
                .ok_or(Error::Invalid(
                    "terminal boundary original history association",
                ))?;
            powers.push(p.checked_add(input.exceptional.powers()[old]).ok_or(
                Error::ResourceIncomplete("original terminal power overflow"),
            )?);
        }
        if powers.len() != input.exceptional.powers().len() {
            return Err(Error::Invalid("terminal localization boundary inventory"));
        }
        let exceptional_unit = pull_path(factors, path, input.exceptional.unit(), b)?;
        for original in input.generators {
            let mut q = pull_path(factors, path, original.current_quotient(), b)?;
            for (di, (d, power)) in data
                .history()
                .ledger()
                .divisors()
                .iter()
                .zip(data.powers())
                .enumerate()
            {
                for j in 0..*power {
                    q = super::super::super::companion::divide_regular_in_localization(
                        local,
                        &d.equation,
                        &q,
                        &format!("{namespace}_key{}_boundary{di}_{j}", original_factors.len()),
                        b,
                    )?
                    .0;
                }
            }
            q = super::super::super::companion::divide_regular_in_localization(
                local,
                principal.residual_generator(),
                &q,
                &format!("{namespace}_key{}_residual", original_factors.len()),
                b,
            )?
            .0;
            let total = pull_path(factors, path, original.total(), b)?;
            let multiplier = b.mul(&exceptional_unit, &q)?;
            if !unit(local, &multiplier, b)? {
                break;
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
        if original_factors.len() != input.generators.len() {
            records.push(OriginalTerminalRecord::IndividualFactorsUnresolved {
                path: path.clone(),
                principal,
            });
            continue;
        }
        let original_coordinate_jacobian = pull_path(factors, path, input.jacobian, b)?;
        records.push(OriginalTerminalRecord::Terminal {
            path: path.clone(),
            proof: Arc::new(OriginalDivisorTerminal {
                principal,
                factors: original_factors,
                original_coordinate_jacobian,
            }),
        });
    }
    Ok(records)
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
