//! Parametric integral data backed by Symbolica atoms and native HEPKit U/F.

pub(crate) mod homogeneity;
mod integrand;
mod numerator;
mod preparation;
mod scalar;

pub use fastsecdec_sectors::ParametricDomain;
pub use integrand::{
    FactorRole, FactorSemantics, ParametricError, ParametricIntegrand, ParametricTerm,
    PolynomialFactor, polynomial_support,
};
pub use preparation::{
    FamilyPreparationFallback, FamilyPreparationPolicy, FamilyPreparationReport,
    FamilyPreparationStatus, PreparedFamily, prepare_family,
};
pub use scalar::ScalarParametricIntegral;
