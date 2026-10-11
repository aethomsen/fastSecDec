//! Caller-driven, verified native GCAD geometry.
//!
//! This boundary preserves the original density and signed exponents. A verified
//! generic open-cell decomposition is not an endpoint-normalization certificate
//! or authorization to publish a complete integral.

mod anchor;
mod request;
mod source_identity;
pub mod staging;
mod verified;
pub use anchor::{PositiveProjectiveQuotient, ProjectiveFactorAuthority};

pub use request::{
    AliasRole, DomainOrigin, GcadKinematics, GcadRequest, PreparedDomain, RequestIdentity,
    SignedFactor, SourceProvenance, SymbolAlias,
};
pub use symgcad::{Limits, Problem, SolverOptions};
pub use verified::{NativeDecomposition, VerifiedCell, VerifiedDecomposition};

/// Errors at the native geometry admission and verification boundary.
#[derive(Debug, thiserror::Error)]
pub enum GcadError {
    #[error(transparent)]
    Represented(#[from] crate::threshold::represented::Error),
    #[error("invalid GCAD request: {0}")]
    Invalid(String),
    #[error("unsupported GCAD input: {0}")]
    Unsupported(String),
    #[error("native GCAD solve failed: {0}")]
    Solve(String),
    #[error("native GCAD result is not complete_generic: {0:?}")]
    Incomplete(symgcad::output::Status),
    #[error("GCAD evidence does not match its original request")]
    RequestMismatch,
    #[error("native GCAD verification failed: {0}")]
    Verification(String),
}

pub type Result<T> = std::result::Result<T, GcadError>;

#[cfg(test)]
mod anchor_tests;
#[cfg(test)]
mod tests;
