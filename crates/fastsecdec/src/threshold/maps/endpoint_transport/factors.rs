//! Source-bound real residual obligations for every compatible relative face.
//!
//! This sufficient QQ class retains all source links and the actual selected
//! root owners. Even a complete factor portfolio grants no numerator, measure,
//! common-regulator-strip, subtraction or integration authority.
mod real;
mod restriction;
#[cfg(test)]
mod tests;
use super::{Error, FactorLink, Result, cover::IssuedEndpointChart};
use crate::threshold::{
    gcad::ProjectiveFactorAuthority,
    maps::regular::{Budget, Limits as AlgebraLimits},
};
pub use real::{MarginEvidence, MarginProof, ProofBox, RetainedMargin};
pub use restriction::{RelativeFace, RestrictedFactor};
use std::{collections::BTreeMap, ops::ControlFlow, sync::Arc};
use symbolica::{atom::Atom, domains::rational::Rational};

#[derive(Clone, Debug)]
pub struct Limits {
    pub algebra: AlgebraLimits,
    pub max_faces: usize,
    pub max_rows: usize,
    /// Used when the signed constant coefficient does not propose a margin.
    /// Failure to certify this candidate is only an unresolved obligation.
    pub fallback_margin: Rational,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            algebra: AlgebraLimits::default(),
            max_faces: 728,
            max_rows: 100_000,
            fallback_margin: Rational::from((1, 2)),
        }
    }
}
#[derive(Debug)]
pub struct ActiveFactor {
    identity: usize,
    sign: i8,
    links: Vec<(usize, usize)>,
}
impl ActiveFactor {
    pub fn identity(&self) -> usize {
        self.identity
    }
    pub fn expected_sign(&self) -> i8 {
        self.sign
    }
    /// Original preparation term and factor ordinals. Exponents, roles and
    /// inactive/lazy rows remain in the one retained chart/transport owner.
    pub fn source_links(&self) -> &[(usize, usize)] {
        &self.links
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedReason {
    ZeroDenominator,
    DenominatorMarginMiss,
    SignedNumeratorMarginMiss,
}
#[derive(Debug)]
pub enum FaceOutcome {
    SignedUnit {
        numerator: MarginProof,
        denominator: MarginProof,
    },
    /// Exact zero of this residual, with an independently proved unit D.
    /// The source factor's transported monomial orders remain separate.
    ZeroResidual {
        denominator: MarginProof,
    },
    Unresolved(UnresolvedReason),
}
#[derive(Debug)]
pub struct FactorRow {
    factor: usize,
    restriction: RestrictedFactor,
    outcome: FaceOutcome,
}
impl FactorRow {
    pub fn factor(&self) -> usize {
        self.factor
    }
    pub fn restriction(&self) -> &RestrictedFactor {
        &self.restriction
    }
    pub fn outcome(&self) -> &FaceOutcome {
        &self.outcome
    }
}
#[derive(Debug)]
pub struct FaceRecord {
    face: RelativeFace,
    proof_box: ProofBox,
    rows: Vec<FactorRow>,
}
impl FaceRecord {
    pub fn face(&self) -> &RelativeFace {
        &self.face
    }
    pub fn proof_box(&self) -> &ProofBox {
        &self.proof_box
    }
    pub fn rows(&self) -> &[FactorRow] {
        &self.rows
    }
}
/// One immutable issued chart and its exhaustive finite relative-face inventory.
/// A native bounded proof miss is retained; cancellation/resource/native failure
/// instead returns an error and publishes no partial portfolio.
#[derive(Debug)]
pub struct FactorFacePortfolio {
    chart: IssuedEndpointChart,
    active: Vec<ActiveFactor>,
    faces: Vec<FaceRecord>,
    expected_faces: usize,
}
impl FactorFacePortfolio {
    pub fn chart(&self) -> &IssuedEndpointChart {
        &self.chart
    }
    pub fn active_factors(&self) -> &[ActiveFactor] {
        &self.active
    }
    pub fn faces(&self) -> &[FaceRecord] {
        &self.faces
    }
    pub fn inventory_complete(&self) -> bool {
        self.faces.len() == self.expected_faces
            && self.faces.iter().enumerate().all(|(i, f)| {
                f.face.code() == i + 1
                    && f.rows.len() == self.active.len()
                    && f.rows.iter().enumerate().all(|(j, r)| r.factor == j)
            })
    }
    pub fn unresolved_rows(&self) -> usize {
        self.faces
            .iter()
            .flat_map(|f| &f.rows)
            .filter(|r| matches!(r.outcome, FaceOutcome::Unresolved(_)))
            .count()
    }
    /// Classified exact zeros still require physical refinement.
    pub fn zero_residual_rows(&self) -> usize {
        self.faces
            .iter()
            .flat_map(|f| &f.rows)
            .filter(|r| matches!(r.outcome, FaceOutcome::ZeroResidual { .. }))
            .count()
    }
    /// Every residual must be a signed real unit. An exact zero is accounted
    /// work but cannot satisfy factor regularity or endpoint normal form.
    pub fn factor_faces_complete(&self) -> bool {
        self.inventory_complete()
            && self
                .faces
                .iter()
                .flat_map(|f| &f.rows)
                .all(|r| matches!(r.outcome, FaceOutcome::SignedUnit { .. }))
    }
    pub fn prepare(
        chart: IssuedEndpointChart,
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let budget = Budget::new(limits.algebra.clone());
        let mut check = || {
            budget.check()?;
            if progress().is_break() {
                Err(Error::Cancelled)
            } else {
                Ok(())
            }
        };
        check()?;
        budget.rational(&limits.fallback_margin)?;
        if limits.fallback_margin <= 0 {
            return Err(Error::Invalid(
                "positive face margin candidate required".into(),
            ));
        }
        let count = restriction::face_count(chart.target_units().len(), limits.max_faces)?;
        let active = active_factors(&chart)?;
        let rows = count
            .checked_mul(active.len())
            .ok_or_else(|| Error::ResourceIncomplete("factor-face row overflow".into()))?;
        if rows > limits.max_rows {
            return Err(Error::ResourceIncomplete(
                "factor-face row allowance".into(),
            ));
        }
        let source = restriction::Source::new(&chart, &active, &budget)?;
        let mut faces = Vec::with_capacity(count);
        for code in 1..=count {
            check()?;
            let face = RelativeFace::new(chart.target_units(), code)?;
            let proof_box = ProofBox::new(&chart, &face, &budget)?;
            let mut rows = Vec::with_capacity(active.len());
            for (factor, a) in active.iter().enumerate() {
                check()?;
                let restriction = source.restrict(factor, &face, &budget)?;
                let outcome = classify(&proof_box, &restriction, a.sign, limits, &budget)?;
                rows.push(FactorRow {
                    factor,
                    restriction,
                    outcome,
                });
            }
            faces.push(FaceRecord {
                face,
                proof_box,
                rows,
            });
        }
        check()?;
        Ok(Arc::new(Self {
            chart,
            active,
            faces,
            expected_faces: count,
        }))
    }
}
fn classify(
    domain: &ProofBox,
    residual: &RestrictedFactor,
    sign: i8,
    limits: &Limits,
    budget: &Budget,
) -> Result<FaceOutcome> {
    if residual.denominator().is_zero() {
        return Ok(FaceOutcome::Unresolved(UnresolvedReason::ZeroDenominator));
    }
    let Some(denominator) =
        domain.propose(residual.denominator(), &limits.fallback_margin, budget)?
    else {
        return Ok(FaceOutcome::Unresolved(
            UnresolvedReason::DenominatorMarginMiss,
        ));
    };
    if residual.numerator().is_zero() {
        return Ok(FaceOutcome::ZeroResidual { denominator });
    }
    let signed = Atom::num(sign) * residual.numerator();
    Ok(
        match domain.propose(&signed, &limits.fallback_margin, budget)? {
            Some(numerator) => FaceOutcome::SignedUnit {
                numerator,
                denominator,
            },
            None => FaceOutcome::Unresolved(UnresolvedReason::SignedNumeratorMarginMiss),
        },
    )
}
pub(super) fn active_factors(chart: &IssuedEndpointChart) -> Result<Vec<ActiveFactor>> {
    let source = chart.parent_prefix().source();
    let owner = source.decomposition();
    let terms = owner.preparation().terms();
    let authorities = owner.factor_authorities();
    let links = chart.transport().term_links();
    if terms.len() != links.len() || terms.len() != authorities.len() {
        return Err(Error::Invalid(
            "full original factor inventory differs".into(),
        ));
    }
    let cell = &owner.native_result().cells[source.cell_index()];
    let mut active: BTreeMap<usize, ActiveFactor> = BTreeMap::new();
    for (i, ((term, links), authority)) in terms.iter().zip(links).zip(authorities).enumerate() {
        if term.factors().len() != links.factors().len() || term.factors().len() != authority.len()
        {
            return Err(Error::Invalid("original factor row differs".into()));
        }
        for (j, ((factor, link), authority)) in term
            .factors()
            .iter()
            .zip(links.factors())
            .zip(authority)
            .enumerate()
        {
            let identity = match (link, authority) {
                (FactorLink::Inactive, ProjectiveFactorAuthority::Inactive)
                    if factor.exponent().is_zero() =>
                {
                    continue;
                }
                (FactorLink::LazyPolynomial, ProjectiveFactorAuthority::Polynomial) => continue,
                (
                    FactorLink::Cleared(id),
                    ProjectiveFactorAuthority::Positive { .. }
                    | ProjectiveFactorAuthority::Split { .. },
                ) => *id,
                _ => {
                    return Err(Error::Invalid(
                        "source factor disposition/authority differs".into(),
                    ));
                }
            };
            if identity >= chart.chart().factors().len() {
                return Err(Error::Invalid("missing issued factor identity".into()));
            }
            let sign = match authority {
                ProjectiveFactorAuthority::Positive { .. } => 1,
                ProjectiveFactorAuthority::Split { index, .. } => *cell
                    .signs
                    .get(*index)
                    .ok_or_else(|| Error::Invalid("source split sign absent".into()))?,
                _ => unreachable!(),
            };
            if !matches!(sign, -1 | 1) {
                return Err(Error::Unsupported(
                    "nonzero original split sign required".into(),
                ));
            }
            let a = active.entry(identity).or_insert_with(|| ActiveFactor {
                identity,
                sign,
                links: Vec::new(),
            });
            if a.sign != sign {
                return Err(Error::Invalid(
                    "shared exact residual has conflicting source sign".into(),
                ));
            }
            a.links.push((i, j));
        }
    }
    Ok(active.into_values().collect())
}
