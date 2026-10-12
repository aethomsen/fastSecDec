//! Generation-side construction from the actual exact section owner. The
//! detached arithmetic module has no dependency on GCAD or source proofs.
use super::{RegularSection, Result, native};
use crate::generation::identity::CanonicalAtom;
pub use crate::kernel::algebraic::{Number, RootProgram, Scope, attempt};
use std::sync::Arc;
use symbolica::evaluate::OptimizationSettings;
impl RootProgram {
    pub fn prepare(section: &RegularSection, settings: OptimizationSettings) -> Result<Arc<Self>> {
        Self::prepare_section(section, settings, 1)
    }
    /// Normalize a certified, static rational bracket to [0,1]. This preserves
    /// the original physical branch and does not add a density Jacobian.
    pub fn prepare_affine(
        section: &RegularSection,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        Self::prepare_section(section, settings, 2)
    }
    /// Static affine normalization with native coefficient shifting once per
    /// root request. Saved V1/V2 arithmetic remains independently supported.
    pub fn prepare_affine_shifted(
        section: &RegularSection,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        Self::prepare_section(section, settings, 3)
    }
    fn prepare_section(
        section: &RegularSection,
        settings: OptimizationSettings,
        version: u32,
    ) -> Result<Arc<Self>> {
        let source = section.source();
        let raw = source.decomposition().native_result();
        Self::prepare_payload(
            section.certificate(),
            &source
                .decomposition()
                .request()
                .source_identity()
                .map_err(native)?,
            raw,
            "fastsecdec-regular-section-branch-v1",
            settings,
            version,
        )
    }
    /// A genuine positive-projective first-axis constant section. This does not
    /// convert its geometric authority to an affine CellMap.
    pub fn prepare_positive_constant(
        section: &crate::threshold::maps::normalized::constant::PositiveConstantSection,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        let source = section.source();
        Self::prepare_payload(
            section.certificate(),
            &crate::threshold::maps::normalized::program::source_digest(source)?,
            source.decomposition().native_result(),
            "fastsecdec-positive-constant-section-v1",
            settings,
            3,
        )
    }
    fn prepare_payload(
        section: &super::certificate::SectionCertificate,
        source_identity: &str,
        raw: &symgcad::output::SolveResult,
        label: &str,
        settings: OptimizationSettings,
        version: u32,
    ) -> Result<Arc<Self>> {
        let selector_domain = match section.bound().index_domain {
            symgcad::output::RootIndexDomain::Real => 0u8,
            symgcad::output::RootIndexDomain::Positive => 1,
            symgcad::output::RootIndexDomain::RealDescending => 2,
        };
        let contract = serde_json::to_vec(&(
            label,
            source_identity,
            CanonicalAtom(section.equation()),
            section
                .coefficients()
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            &raw.order,
            &raw.normalized_constraints,
            selector_domain,
            section.bound().index,
            section.axis(),
            section.bracket().lower.to_string(),
            section.bracket().upper.to_string(),
            section.bracket().derivative_margin.to_string(),
            section.orientation(),
        ))
        .map_err(native)?;
        let prepare = match version {
            1 => Self::prepare_from_certificate,
            2 => Self::prepare_affine_from_certificate,
            3 => Self::prepare_shifted_from_certificate,
            _ => unreachable!("private root recipe"),
        };
        prepare(
            section.coefficients().len() - 1,
            section.bracket().lower.clone(),
            section.bracket().upper.clone(),
            contract,
            settings,
        )
        .map_err(Into::into)
    }
}
