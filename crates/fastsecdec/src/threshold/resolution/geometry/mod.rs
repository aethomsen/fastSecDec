//! Constructive quotient normalization and initial relative boundary geometry.
//!
//! These operations certify localized ideal equality, all divisor intersections
//! and a monomial center. They do not yet construct blowup charts, exceptional
//! history transitions or a real integration atlas.
mod boundary;
pub(crate) mod generators;
mod monomial;
mod normalization;

pub use boundary::{
    BoundaryId, InitialDivisor, IntersectionCertificate, SncProduction, SncProgress,
    VerifiedRelativeSnc, verify_initial_relative_snc,
};
pub use monomial::{
    MonomialCandidate, MonomialProduction, MonomialProgress, MonomialWitness,
    ProducedMonomialCenter, produce_monomial_center,
};
pub use normalization::{
    NormalizationOutcome, NormalizationProgress, QuotientNormalizer, VerifiedQuotientNormalization,
};

#[cfg(test)]
mod tests;
