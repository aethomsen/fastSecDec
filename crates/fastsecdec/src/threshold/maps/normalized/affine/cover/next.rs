//! Checked rational normalization of the actual next selected section.
//! Uniform implicit certificates form a sufficient class, not a general AJ solver.
mod admission;
mod arithmetic;
pub mod program;
use super::*;
use crate::threshold::maps::normalized::BracketProposal;
use crate::{generation::identity::CanonicalAtom, kernel::algebraic::RootProgram};
use symbolica::{
    domains::float::{FloatLike, RealBall},
    evaluate::OptimizationSettings,
};
use symgcad::domain::DomainCertificate;

#[derive(Clone)]
pub struct NextSectionProposal {
    pub unit_powers: Vec<u32>,
    pub root_symbol: Symbol,
    pub centre: Atom,
    pub scale: Atom,
    pub equation: Atom,
    /// Full substituted original P = presentation * equation. Presentation
    /// retains source/content factors and must be nonzero on the open chart.
    pub presentation: Atom,
    pub bracket: BracketProposal,
    pub unit_margin: Rational,
}
pub struct EndpointReceipt {
    original: Atom,
    representative: Atom,
    quotients: Vec<Atom>,
    certificate: Option<DomainCertificate>,
}
impl EndpointReceipt {
    pub fn original(&self) -> &Atom {
        &self.original
    }
    pub fn representative(&self) -> &Atom {
        &self.representative
    }
    pub fn relation_quotients(&self) -> &[Atom] {
        &self.quotients
    }
    pub fn certificate(&self) -> Option<&DomainCertificate> {
        self.certificate.as_ref()
    }
}
pub struct RationalReceipt {
    numerator: Atom,
    denominator: Atom,
    powers: Vec<u32>,
    numerator_unit: Option<UnitReceipt>,
    denominator_unit: UnitReceipt,
}
impl RationalReceipt {
    /// The admitted rational map. Native cancellation is intentional here:
    /// runtime uses this certified denominator, never the proposal's syntax.
    pub fn expression(&self) -> Atom {
        &self.numerator / &self.denominator
    }
    pub fn numerator(&self) -> &Atom {
        &self.numerator
    }
    pub fn denominator(&self) -> &Atom {
        &self.denominator
    }
    pub fn coordinate_powers(&self) -> &[u32] {
        &self.powers
    }
    pub fn numerator_unit(&self) -> Option<&UnitReceipt> {
        self.numerator_unit.as_ref()
    }
    pub fn denominator_unit(&self) -> &UnitReceipt {
        &self.denominator_unit
    }
}
/// Retains the full original prefix/cover and authentic branch selector. This
/// issues a boundary section, not the next integration interval or density.
pub struct RestrictedNextSection {
    parent: Arc<RestrictedRootEquation>,
    witness: InteriorWitness,
    proposal: NextSectionProposal,
    powers: Vec<Atom>,
    measure: Atom,
    relations: Vec<(Symbol, Atom)>,
    centre: RationalReceipt,
    scale: RationalReceipt,
    presentation: RationalReceipt,
    identity_quotients: Vec<Atom>,
    coefficients: Vec<Atom>,
    orientation: i8,
    total_denominator: UnitReceipt,
    derivative: UnitReceipt,
    endpoints: Vec<EndpointReceipt>,
    witness_slots: Vec<RealBall>,
    normalized_witness: RealBall,
}
impl RestrictedNextSection {
    pub fn parent(&self) -> &Arc<RestrictedRootEquation> {
        &self.parent
    }
    pub fn witness(&self) -> &InteriorWitness {
        &self.witness
    }
    pub fn proposal(&self) -> &NextSectionProposal {
        &self.proposal
    }
    pub fn power_images(&self) -> &[Atom] {
        &self.powers
    }
    pub fn power_measure(&self) -> &Atom {
        &self.measure
    }
    pub fn relations(&self) -> &[(Symbol, Atom)] {
        &self.relations
    }
    pub fn centre_receipt(&self) -> &RationalReceipt {
        &self.centre
    }
    pub fn centre_expression(&self) -> Atom {
        self.centre.expression()
    }
    pub fn scale_receipt(&self) -> &RationalReceipt {
        &self.scale
    }
    pub fn scale_expression(&self) -> Atom {
        self.scale.expression()
    }
    pub fn presentation_receipt(&self) -> &RationalReceipt {
        &self.presentation
    }
    pub fn identity_quotients(&self) -> &[Atom] {
        &self.identity_quotients
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.coefficients
    }
    pub fn orientation(&self) -> i8 {
        self.orientation
    }
    pub fn derivative_certificate(&self) -> &UnitReceipt {
        &self.derivative
    }
    pub fn endpoint_certificates(&self) -> &[EndpointReceipt] {
        &self.endpoints
    }
    pub fn transformed_denominator(&self) -> &UnitReceipt {
        &self.total_denominator
    }
    pub fn witness_slots(&self) -> &[RealBall] {
        &self.witness_slots
    }
    pub fn normalized_witness(&self) -> &RealBall {
        &self.normalized_witness
    }
    pub fn prepare(
        parent: Arc<RestrictedRootEquation>,
        witness: InteriorWitness,
        proposal: NextSectionProposal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        admission::prepare(parent, witness, proposal, b)
    }
    pub fn root_program(&self, settings: OptimizationSettings) -> Result<Arc<RootProgram>> {
        let cover = self.parent.leaf().cover();
        let raw = cover.parent().source().decomposition().native_result();
        let mut digest = blake3::Hasher::new();
        serde_json::to_writer(
            &mut digest,
            &(
                "fastsecdec-restricted-next-section-math-v1",
                cover
                    .parent()
                    .source()
                    .decomposition()
                    .request()
                    .source_identity()
                    .map_err(native)?,
                &raw.order,
                &raw.normalized_constraints,
                cover.parent().source().cell_index(),
                cover.parent().source().native_axes(),
                cover.next_bound(),
                CanonicalAtom(cover.original_polynomial()),
                cover
                    .parent()
                    .images()
                    .iter()
                    .map(CanonicalAtom)
                    .collect::<Vec<_>>(),
                cover
                    .slots()
                    .iter()
                    .map(|(s, b)| (s.to_string(), b.each_ref().map(ToString::to_string)))
                    .collect::<Vec<_>>(),
            ),
        )
        .map_err(native)?;
        serde_json::to_writer(
            &mut digest,
            &(
                self.parent.leaf().index(),
                self.parent
                    .leaf()
                    .restriction()
                    .unit_images()
                    .iter()
                    .map(CanonicalAtom)
                    .collect::<Vec<_>>(),
                self.relations
                    .iter()
                    .map(|(s, p)| (s.to_string(), CanonicalAtom(p)))
                    .collect::<Vec<_>>(),
                &self.proposal.unit_powers,
                CanonicalAtom(&self.centre.expression()),
                CanonicalAtom(&self.scale.expression()),
                CanonicalAtom(&self.presentation.expression()),
                CanonicalAtom(&self.proposal.equation),
                self.proposal.bracket.lower.to_string(),
                self.proposal.bracket.upper.to_string(),
                self.orientation,
            ),
        )
        .map_err(native)?;
        let contract = serde_json::to_vec(&(
            "fastsecdec-restricted-next-section-v1",
            digest.finalize().to_hex().to_string(),
        ))
        .map_err(native)?;
        // Exact binomials avoid unnecessary translation of small root coefficients
        // on broad brackets. Dense polynomials retain V3: translating a dense
        // polynomial can improve its conditioning, even for exact static bounds.
        // This numerical recipe choice changes neither the admitted branch nor
        // its certificates or requested native refinement tolerance.
        let bracket = &self.proposal.bracket;
        let unshifted = self.coefficients[1..self.coefficients.len() - 1]
            .iter()
            .all(Atom::is_zero)
            && self.endpoints.iter().all(|r| r.certificate.is_some())
            && [&bracket.lower, &bracket.upper].into_iter().all(|q| {
                let value = q.to_f64();
                value.is_finite() && Rational::try_from(value).ok().as_ref() == Some(q)
            });
        let prepare = if unshifted {
            RootProgram::prepare_from_certificate
        } else {
            RootProgram::prepare_shifted_from_certificate
        };
        prepare(
            self.coefficients.len() - 1,
            bracket.lower.clone(),
            bracket.upper.clone(),
            contract,
            settings,
        )
        .map_err(Into::into)
    }
}

#[cfg(test)]
mod tests;
