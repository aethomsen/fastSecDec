//! Serializable partial resource overrides, shared by native, CLI and HEPKit
//! callers. Mathematical roles, margins and certificate algorithms stay owned
//! by their existing native issuers. Omitted fields preserve caller settings.
use super::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Resources for fresh endpoint preparation, independent of GCAD solve limits.
/// Geometry/discovery clocks do not bound continuation, vector staging or
/// compilation. Native operations check clocks cooperatively; hard wall/RSS
/// supervision remains the caller's responsibility.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct EndpointPreparationResources {
    pub geometry: GeometryResources,
    pub discovery: DiscoveryResources,
    pub prefactors: PrefactorResources,
    pub max_charts: Option<usize>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GeometryResources {
    pub max_degree: Option<usize>,
    pub max_terms: Option<usize>,
    pub max_rational_bits: Option<u64>,
    pub max_refinements: Option<usize>,
    pub wall_time_secs: Option<f64>,
    pub certificates: CertificateResources,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiscoveryResources {
    pub max_dyadic_level: Option<u32>,
    pub max_endpoint_queries: Option<usize>,
    pub max_family_attempts: Option<usize>,
    pub max_margin_halvings: Option<u32>,
    pub wall_time_secs: Option<f64>,
}

/// Partial overrides of symGCAD's native CertificateOptions. Its native
/// certificate discovery/replay remains responsible for admission of limits;
/// no owner ceiling or proof algorithm is reimplemented here.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CertificateResources {
    pub max_terms: Option<usize>,
    pub polya_order: Option<u32>,
    pub bernstein_depth: Option<usize>,
    pub bernstein_nodes: Option<usize>,
    pub bernstein_coefficients: Option<usize>,
    pub work_limit: Option<usize>,
    pub univariate_sturm_degree: Option<usize>,
    pub square_residual_candidates: Option<usize>,
    pub weighted_square_candidates: Option<usize>,
    pub quadratic_component_candidates: Option<usize>,
    pub quadratic_no_real_roots_candidates: Option<usize>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PrefactorResources {
    pub source_nodes: Option<usize>,
    pub source_depth: Option<usize>,
    pub degree: Option<u16>,
    pub terms: Option<usize>,
    pub integer_power: Option<i64>,
    pub witness_candidates: Option<usize>,
}

fn invalid(message: &str) -> Error {
    Error::RegularFamily(regularization::secant::Error::Invalid(message.into()))
}
fn wall(seconds: f64) -> Result<Duration> {
    let value = Duration::try_from_secs_f64(seconds).map_err(|_| {
        invalid("endpoint wall allowance must be finite, positive and representable")
    })?;
    if value.is_zero() {
        return Err(invalid("endpoint wall allowance must be positive"));
    }
    Ok(value)
}
macro_rules! overlay {
    ($target:expr, $source:expr; $($field:ident),+ $(,)?) => {
        $(if let Some(value) = $source.$field { $target.$field = value; })+
    };
}
impl EndpointPreparationResources {
    pub(super) fn apply_restored(
        &self,
        options: &mut PreparationOptions,
        route: regular::Route,
    ) -> Result<()> {
        if route != regular::Route::LegacyFiber {
            return self.apply(options);
        }
        // Legacy recovery deliberately retained no regular-family capability.
        // Refuse limits for absent operations instead of dropping an override.
        let g = &self.geometry;
        let d = &self.discovery;
        let c = &g.certificates;
        if options.regular_family.is_some()
            || g.max_terms.is_some()
            || g.max_rational_bits.is_some()
            || g.max_refinements.is_some()
            || g.wall_time_secs.is_some()
            || d.max_dyadic_level.is_some()
            || d.max_endpoint_queries.is_some()
            || d.max_family_attempts.is_some()
            || d.max_margin_halvings.is_some()
            || d.wall_time_secs.is_some()
            || c.max_terms.is_some()
            || c.polya_order.is_some()
            || c.bernstein_depth.is_some()
            || c.bernstein_nodes.is_some()
            || c.bernstein_coefficients.is_some()
            || c.work_limit.is_some()
            || c.univariate_sturm_degree.is_some()
            || c.square_residual_candidates.is_some()
            || c.weighted_square_candidates.is_some()
            || c.quadratic_component_candidates.is_some()
            || c.quadratic_no_real_roots_candidates.is_some()
        {
            return Err(Error::Unsupported(
                "legacy endpoint retry supports only chart count, degree and prefactor limits",
            ));
        }
        let (_, legacy) =
            self.updated(&RegularFamilyPreparation::default(), options.regularization)?;
        options.regularization = legacy;
        Ok(())
    }

    /// Validate scalar conversions/discovery bounds without performing native
    /// algebra. Certificate/prefactor resource admission also runs in the
    /// corresponding native operation when that operation is required.
    pub fn validate(&self) -> Result<()> {
        self.updated(
            &RegularFamilyPreparation::default(),
            regularization::Limits::default(),
        )?;
        Ok(())
    }
    fn updated(
        &self,
        settings: &RegularFamilyPreparation,
        mut legacy: regularization::Limits,
    ) -> Result<(RegularFamilyPreparation, regularization::Limits)> {
        let mut settings = settings.clone();
        let g = &mut settings.limits.geometry;
        overlay!(g, self.geometry; max_degree, max_terms, max_rational_bits, max_refinements);
        if let Some(seconds) = self.geometry.wall_time_secs {
            g.wall_time = wall(seconds)?;
        }
        let c = &mut g.certificates;
        overlay!(c, self.geometry.certificates;
            max_terms, polya_order, bernstein_depth, bernstein_nodes,
            bernstein_coefficients, work_limit, univariate_sturm_degree,
            square_residual_candidates, weighted_square_candidates,
            quadratic_component_candidates, quadratic_no_real_roots_candidates);
        let d = &mut settings.discovery;
        overlay!(d, self.discovery;
            max_dyadic_level, max_endpoint_queries, max_family_attempts, max_margin_halvings);
        if let Some(seconds) = self.discovery.wall_time_secs {
            d.wall_time = wall(seconds)?;
        }
        if let Some(max_charts) = self.max_charts {
            legacy.max_charts = max_charts;
        }
        if let Some(degree) = self.geometry.max_degree {
            legacy.max_degree = u32::try_from(degree)
                .map_err(|_| invalid("legacy endpoint degree exceeds native u32 range"))?;
        }
        for p in [&mut settings.limits.density, &mut legacy.prefactors] {
            overlay!(p, self.prefactors;
                source_nodes, source_depth, degree, terms, integer_power, witness_candidates);
            if p.integer_power < 0 || p.degree > u16::MAX / 2 {
                return Err(invalid(
                    "endpoint prefactor limit exceeds native exponent range",
                ));
            }
        }
        super::regular::check_settings(&settings)?;
        Ok((settings, legacy))
    }
    pub(super) fn apply(&self, options: &mut PreparationOptions) -> Result<()> {
        let settings = options.regular_family.as_ref().ok_or(Error::Unsupported(
            "enable endpoint routes before applying endpoint resources",
        ))?;
        let (settings, legacy) = self.updated(settings, options.regularization)?;
        // Validation precedes either write, including when an override fails
        // after earlier fields in the candidate have already been updated.
        options.regular_family = Some(settings);
        options.regularization = legacy;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
