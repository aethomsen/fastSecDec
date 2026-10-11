//! Run-card adaptation of native threshold settings. Algebra, solving,
//! continuation and publication authority remain in the Rust library.
use crate::CliResult;
use fastsecdec::threshold::{
    ThresholdDecompositionOptions, ThresholdStrategy,
    gcad::{GcadKinematics, Limits, SolverOptions},
    resolution,
};

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Input {
    pub strategy: ThresholdStrategy,
    /// Native polynomial expressions required to be strictly positive.
    pub kinematic_constraints: Vec<String>,
    pub threshold_cells: Option<Vec<usize>>,
    pub solver: SolverOptions,
    #[serde(deserialize_with = "deserialize_gcad_limits")]
    pub gcad_limits: Limits,
    pub resolution_limits: resolution::Limits,
}

// Native solver defaults target its own CLI. Start nested overrides from this
// caller-driven library's one-job defaults without duplicating owner fields.
fn deserialize_gcad_limits<'de, D>(deserializer: D) -> Result<Limits, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let overrides = serde_json::Map::<String, serde_json::Value>::deserialize(deserializer)?;
    let mut defaults = serde_json::to_value(ThresholdDecompositionOptions::default().gcad_limits)
        .map_err(serde::de::Error::custom)?;
    defaults
        .as_object_mut()
        .expect("native limits object")
        .extend(overrides);
    serde_json::from_value(defaults).map_err(serde::de::Error::custom)
}

impl Default for Input {
    fn default() -> Self {
        let native = ThresholdDecompositionOptions::default();
        Self {
            strategy: native.strategy,
            kinematic_constraints: Vec::new(),
            threshold_cells: native.threshold_cells,
            solver: native.solver,
            gcad_limits: native.gcad_limits,
            resolution_limits: native.resolution_limits,
        }
    }
}

impl Input {
    pub(crate) fn native(
        &self,
        runtime_parameters: &[fastsecdec::Symbol],
    ) -> CliResult<ThresholdDecompositionOptions> {
        Ok(ThresholdDecompositionOptions {
            strategy: self.strategy,
            kinematics: GcadKinematics {
                runtime_parameters: runtime_parameters.to_vec(),
                strict_positive: self
                    .kinematic_constraints
                    .iter()
                    .map(|text| crate::input::expression(text))
                    .collect::<CliResult<_>>()?,
                ..Default::default()
            },
            threshold_cells: self.threshold_cells.clone(),
            solver: self.solver.clone(),
            gcad_limits: self.gcad_limits.clone(),
            resolution_limits: self.resolution_limits.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastsecdec::{Atom, kernel::ProgramRecipe};

    #[test]
    fn native_options_remain_opt_in_and_preserve_input_symbols_and_defaults() {
        let source = "[threshold_decomposition]\nstrategy='sector_first'\nkinematic_constraints=['p','1-p']\nthreshold_cells=[2,0]\n[threshold_decomposition.gcad_limits]\nmemory_mib=1024\n[threshold_decomposition.resolution_limits]\nmax_mark=7\n";
        let card: crate::config::RunCard = toml::from_str(source).unwrap();
        assert!(!card.generation.threshold_enabled());
        let p = crate::input::symbol("p").unwrap();
        let options = card.threshold_decomposition.native(&[p]).unwrap();
        assert_eq!(options.strategy, ThresholdStrategy::SectorFirst);
        assert_eq!(options.threshold_cells, Some(vec![2, 0]));
        assert_eq!(options.kinematics.runtime_parameters, [p]);
        assert_eq!(
            options.kinematics.strict_positive,
            [Atom::var(p), Atom::num(1) - Atom::var(p)]
        );
        assert_eq!(options.gcad_limits.workers, 1);
        assert_eq!(options.gcad_limits.memory_mib, 1024);
        assert_eq!(options.resolution_limits.max_mark, 7);
        assert_eq!(
            options.resolution_limits.max_terms,
            resolution::Limits::default().max_terms
        );
        for source in [
            "[threshold_decomposition]\nunknown=true",
            "[threshold_decomposition.gcad_limits]\nmemory_mb=5",
            "[threshold_decomposition.resolution_limits]\nmax_marker=3",
            "[threshold_decomposition.solver]\nunknown=true",
        ] {
            assert!(toml::from_str::<crate::config::RunCard>(source).is_err());
        }
    }

    #[test]
    fn threshold_recipe_rejects_contour_and_numerical_endpoint_choices() {
        for source in ["threshold_decomposition=true", "recipe='threshold-v1'"] {
            let input: crate::config::GenerationInput = toml::from_str(source).unwrap();
            input.validate_capabilities().unwrap();
            assert_eq!(input.program_recipe(), ProgramRecipe::ThresholdV1);
        }
        for source in [
            "threshold_decomposition=true\ncontour=true",
            "threshold_decomposition=true\nrecipe='fixed-v1'",
            "threshold_decomposition=true\nmode='numerical_dual'",
            "threshold_decomposition=true\ncontour_jacobian='dual'",
        ] {
            let input: crate::config::GenerationInput = toml::from_str(source).unwrap();
            assert!(input.validate_capabilities().is_err());
        }
        let mut card: crate::config::RunCard = toml::from_str("").unwrap();
        let overrides = crate::config::GenerationOverrides {
            threshold_decomposition: true,
            ..Default::default()
        };
        let restored: crate::config::GenerationOverrides =
            serde_json::from_value(serde_json::to_value(overrides).unwrap()).unwrap();
        restored.apply(&mut card);
        assert_eq!(card.generation.program_recipe(), ProgramRecipe::ThresholdV1);
        assert!(
            card.generation
                .validate_resident_recipe(ProgramRecipe::UndeformedV1)
                .is_err()
        );
    }

    #[test]
    fn maintained_sextic_preserves_native_cube_and_explicit_lifting_order() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/no_deformation/regular_sextic.toml");
        let loaded = crate::input::load(&path).unwrap();
        assert!(loaded.card.generation.threshold_enabled());
        assert!(loaded.runtime_parameters.is_empty());
        assert_eq!(
            loaded.integrand.domain(),
            fastsecdec::parametric::ParametricDomain::UnitCube
        );
        assert_eq!(loaded.integrand.parameters().len(), 2);
        let options = loaded.card.threshold_decomposition.native(&[]).unwrap();
        assert_eq!(options.solver.order, ["v0", "v1"]);
        let request = options
            .gcad_first_request(
                &loaded.integrand,
                &fastsecdec::generation::GenerationOptions::default(),
            )
            .unwrap();
        assert_eq!(request.domain().coordinates().len(), 2);
        assert!(matches!(
            request.domain().origin(),
            fastsecdec::threshold::gcad::DomainOrigin::NativeUnitCube
        ));
        assert_eq!(
            request.prepared_terms()[0].factors()[0].semantics(),
            fastsecdec::parametric::FactorSemantics::Causal
        );
    }

    #[test]
    fn maintained_bubble_uses_native_graph_and_explicit_generation_kinematics() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/no_deformation/threshold_bubble.toml");
        let loaded = crate::input::load(&path).unwrap();
        assert_eq!(loaded.loops, Some(1));
        assert_eq!(loaded.propagators, 2);
        assert!(loaded.runtime_parameters.is_empty());
        assert!(loaded.card.integration.parameters.is_empty());
        let options = loaded
            .card
            .threshold_decomposition
            .native(&loaded.runtime_parameters)
            .unwrap();
        let request = options
            .gcad_first_request(
                &loaded.integrand,
                &fastsecdec::generation::GenerationOptions::default(),
            )
            .unwrap();
        assert_eq!(request.domain().coordinates().len(), 1);
        assert!(request.kinematics().runtime_parameters.is_empty());
        let x = Atom::var(request.domain().coordinates()[0]);
        let factors = request.prepared_terms()[0].factors();
        let causal = factors
            .iter()
            .filter(|f| f.semantics() == fastsecdec::parametric::FactorSemantics::Causal)
            .collect::<Vec<_>>();
        assert_eq!(causal.len(), 1);
        use fastsecdec::AtomCore;
        assert!(
            (causal[0].polynomial() - (Atom::num(3) - Atom::num(16) * &x * (Atom::one() - &x)))
                .expand()
                .is_zero()
        );
    }
}
