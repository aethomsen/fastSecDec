//! One input preparation path for selected programs and shared recipe sets.
use super::*;

pub(super) fn run(
    root: &Path,
    input: &Path,
    workers: usize,
    overrides: crate::config::GenerationOverrides,
    recipes: Option<&[indexed::ProgramRecipe]>,
    observe: &mut impl FnMut(i32, &generation::GenerationProgress) -> ControlFlow<()>,
) -> CliResult<Response> {
    let loaded = input::load_observed_with_overrides(input, overrides, |_| Ok(()))?;
    let (options, generation, provenance, timings) = context(&loaded, workers)?;
    enum Native {
        Selected(native::Preparation),
        Programs(native::PreparedRecipeSet),
    }
    let native = if let Some(recipes) = recipes {
        Native::Programs(native::prepare_recipes_with_runtime(
            &loaded.integrand,
            &options,
            recipes,
            &loaded.runtime_parameters,
            &loaded.runtime_mass_constraints,
            root,
            |progress| observe(options.max_order, progress),
        )?)
    } else {
        Native::Selected(native::prepare_with_runtime(
            &loaded.integrand,
            &options,
            &loaded.runtime_parameters,
            &loaded.runtime_mass_constraints,
            root,
            |progress| observe(options.max_order, progress),
        )?)
    };
    Ok(match native {
        Native::Selected(native) => Response::Prepared(Box::new(Prepared {
            native,
            provenance,
            generation,
            timings,
        })),
        Native::Programs(native) => Response::PreparedPrograms(Box::new(PreparedPrograms {
            native,
            provenance,
            generation,
            timings,
        })),
    })
}

pub(super) fn context(
    loaded: &input::LoadedInput,
    workers: usize,
) -> CliResult<(
    generation::GenerationOptions,
    GenerationRecord,
    Provenance,
    GenerationTimings,
)> {
    let settings = &loaded.card.generation;
    let mut options = generation::GenerationOptions {
        source_sectors: settings.source_sectors.clone(),
        max_order: settings.order,
        mode: settings.mode,
        contour_jacobian: settings.contour_jacobian,
        subtraction: settings.subtraction,
        assume_no_threshold: settings.assume_no_threshold,
        program_recipe: settings.program_recipe(),
        coefficient_expansion: settings.coefficient_expansion.clone(),
        ..Default::default()
    };
    options.decomposition.max_sectors = settings.max_sectors;
    options.decomposition.max_support_pairs = settings.max_support_pairs;
    let generation = GenerationRecord {
        endpoint_retry: None,
        workers,
        mode: Some(options.mode),
        subtraction: Some(options.subtraction),
        source_chart_modes: None,
        formula_preparation: None,
        contraction_mode: loaded.loops.map(|_| settings.contraction_mode),
        requested_coefficient_expansion: options.coefficient_expansion.method,
        evaluator: Some(settings.evaluator),
    };
    let provenance = Provenance {
        name: loaded.label.clone(),
        sources: loaded.sources.clone(),
        dependencies: artifact::dependencies(),
        domain: format!("{:?}", loaded.integrand.domain()),
        assume_no_threshold: options.assume_no_threshold,
        dimension: loaded.card.integral.dimension.clone(),
        regulator: loaded.card.integral.regulator.clone(),
        measure: if loaded.loops.is_some() {
            "prod_l d^D k_l / (i*pi^(D/2)); propagators q^2-m^2+i0; no implicit scale factors"
        } else {
            "user-supplied direct density with the declared domain measure"
        }
        .into(),
        measure_multiplier: loaded.card.integral.measure_multiplier.clone(),
        max_order: options.max_order,
        integration: serde_json::to_value(&loaded.card.integration)?,
        family_preparation: loaded.family_preparation.clone(),
        model_parameter_defaults: loaded.model_parameter_defaults.clone(),
    };
    let timings = GenerationTimings {
        input_seconds: loaded.input_seconds,
        parametrization_seconds: loaded.parametrization_seconds,
        ..Default::default()
    };
    Ok((options, generation, provenance, timings))
}
