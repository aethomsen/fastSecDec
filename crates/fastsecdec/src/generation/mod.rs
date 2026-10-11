//! Direct monomial substitution, exact endpoint subtraction, and Laurent vectors.
//!
//! Symbolica owns all polynomial manipulation, derivatives and series expansions.
//! This module owns the integral-specific order of these operations.
mod assembly;
pub(crate) use assembly::normalize_exact_coefficient;
mod selection;
mod session;
pub use selection::{GenerationSourceScope, SourceSectorSelection};
pub use session::{GenerationSession, GenerationSessionState};
mod coefficient_first;
mod coefficients;
mod conditioning;
mod context;
mod domain;
mod family;
pub use family::{
    RecipeFamily, RecipeFamilyCompletion, RecipeFamilyDispatch, RecipeFamilyError, RecipeFamilyJob,
    RecipeFamilyJobId, RecipeFamilyJobProgress, RecipeFamilyJobStage, RecipeFamilyOutput,
    RecipeFamilySession, RecipeFamilySessionError, RecipeFamilySnapshot,
};
mod geometry;
pub(crate) mod identity;
pub use identity::source_identity;
mod laurent;
mod mapping;
mod metadata;
mod metadata_display;
pub(crate) mod numerical_dual;
mod program;
pub use crate::status::GeometryReuseStatus;
pub use context::{GenerationContext, GenerationEvent};
pub use fastsecdec_sectors::{
    DecompositionPhase, DecompositionProgress, GeometryCompletion, GeometryDispatch, GeometryJob,
    GeometryJobId, GeometryWorkError, SectorError,
};
pub use metadata_display::MetadataView;
#[cfg(test)]
pub(crate) mod profiling;
pub mod streaming;
mod subtraction;
#[cfg(feature = "threshold-decomposition")]
mod threshold;
#[cfg(feature = "threshold-decomposition")]
pub(crate) use subtraction::endpoints::endpoint_power_with_regulators;
#[cfg(feature = "threshold-decomposition")]
pub(crate) use threshold::subtract as threshold_subtract;
mod support;
mod symmetry;
mod types;
mod work;
#[cfg(feature = "threshold-decomposition")]
pub(crate) use mapping::add_scaled_powers;
pub(crate) use mapping::coordinates_from_parts;
pub use work::{SymbolicCompletion, SymbolicDispatch, SymbolicJob, SymbolicJobId, SymbolicStage};

pub use metadata::{
    BranchPolicy, ChartRecord, CoordinateMap, DomainAssessment, EndpointPower, FactorAssessment,
    FactorCertificate, GenerationMetadata, PreSubtractionMetadata, PreSubtractionTerm,
};
pub use types::{
    CoefficientExpansionMethod, CoefficientExpansionOptions, CoefficientExpansionStage,
    CoefficientRequestCounts, ConditioningBasis, EndpointCancellationSource, EndpointProfileRow,
    GeneratedIntegral, GeneratedSector, GenerationError, GenerationMode, GenerationOptions,
    GenerationPhase, GenerationProgress, SubtractionStrategy,
};

use crate::parametric::ParametricIntegrand;
use context::emit;
use fastsecdec_sectors::PolynomialSupport;
use geometry::{Geometry, GeometrySource};
use std::{collections::BTreeMap, ops::ControlFlow, time::Instant};
#[cfg(test)]
use symbolica::atom::Atom;

/// Test-only access to the real Laurent stage, without manufacturing a partial
/// generated integral or its domain/chart metadata.
#[cfg(test)]
pub(crate) struct CapturedLaurent {
    pub template: Atom,
    pub coefficients: BTreeMap<i32, symbolica::atom::AliasedAtom>,
}

#[cfg(test)]
pub(crate) fn captured_coefficients(
    expression: &Atom,
    parameters: &[symbolica::atom::Symbol],
    regulator: symbolica::atom::Symbol,
    maximum: i32,
) -> Result<CapturedLaurent, GenerationError> {
    let mut cache = laurent::TemplateCache::default();
    let coefficients = laurent::expand(expression, parameters, regulator, maximum, &mut cache)?;
    Ok(CapturedLaurent {
        template: cache.only_template().clone(),
        coefficients,
    })
}

/// Test-only replay of already admitted, source-bound mapped inputs.
/// Tuple order is (prefactor, regular factor, coordinate powers).
#[cfg(test)]
pub(crate) fn captured_subtraction(
    terms: Vec<(Atom, Atom, Vec<Atom>)>,
    parameters: &[symbolica::atom::Symbol],
    regulator: symbolica::atom::Symbol,
    options: &GenerationOptions,
) -> Result<(Atom, usize, Vec<Vec<usize>>), GenerationError> {
    subtraction::subtract(
        terms
            .into_iter()
            .map(|(prefactor, regular, powers)| mapping::MappedTerm {
                powers,
                prefactor,
                regular,
            })
            .collect(),
        parameters,
        regulator,
        options,
    )
}

pub fn generate(
    input: &ParametricIntegrand,
    options: &GenerationOptions,
    mut progress: impl FnMut(&GenerationProgress) -> ControlFlow<()>,
) -> Result<GeneratedIntegral, GenerationError> {
    generate_inner(
        input,
        options,
        GeometrySource::Uncached,
        None,
        |event| match event {
            GenerationEvent::Progress(status) => progress(status),
            GenerationEvent::GeometryReuse(_) => ControlFlow::Continue(()),
        },
    )
}

fn generate_inner(
    input: &ParametricIntegrand,
    options: &GenerationOptions,
    geometry_source: GeometrySource<'_, '_>,
    mut symbolic_dispatch: Option<&mut SymbolicDispatch<'_>>,
    mut progress: impl FnMut(&GenerationEvent) -> ControlFlow<()>,
) -> Result<GeneratedIntegral, GenerationError> {
    let started = Instant::now();
    let domain = domain::check_options(input, options)?;
    emit(
        &mut progress,
        GenerationProgress::PhaseTiming {
            phase: GenerationPhase::Domain,
            seconds: started.elapsed().as_secs_f64(),
        },
    )?;
    let started = Instant::now();
    let mut source_supports = support::SupportCache::new(input.parameters());
    let mut supports = Vec::new();
    for term in input.terms() {
        for factor in term.factors() {
            if domain::is_geometry_factor(factor, options.contour_enabled()) {
                let support = source_supports.get(factor)?;
                if !supports.contains(support) {
                    supports.push(support.clone());
                }
            }
        }
    }
    if supports.is_empty() {
        supports.push(PolynomialSupport::new(vec![vec![
            0;
            input.parameters().len()
        ]])?);
    }
    let mut decomposition = if input.terms().is_empty() {
        None
    } else {
        Some(Geometry::compute(
            input.domain(),
            &supports,
            &options.decomposition,
            geometry_source,
            &mut progress,
        )?)
    };
    emit(
        &mut progress,
        GenerationProgress::PhaseTiming {
            phase: GenerationPhase::Geometry,
            seconds: started.elapsed().as_secs_f64(),
        },
    )?;
    let mut templates = laurent::TemplateCache::default();
    let mut registry = symmetry::SymmetryRegistry::default();
    let mut representatives = BTreeMap::new();
    let mut charts = Vec::new();
    let maps = decomposition
        .iter_mut()
        .flat_map(Geometry::maps)
        .map(|map| map.into_owned())
        .collect::<Vec<_>>();
    let (maps, source_scope) = selection::select_maps(maps, options.source_sectors.as_deref())?;
    let total = maps.len();
    let parameters =
        mapping::target_parameters(input, maps.first().map_or(0, |map| map.dimension()));
    if options.mode == GenerationMode::NumericalDual {
        return numerical_dual::generate(
            input,
            options,
            numerical_dual::PreparedMaps {
                domain,
                maps,
                parameters,
            },
            &source_supports,
            symbolic_dispatch,
            &mut progress,
        )?
        .preserve_empty_recipe(options.program_recipe, options.contour_jacobian)?
        .with_source_scope(source_scope);
    }
    let mut maps = maps.into_iter();
    let mut prepared_charts = if let Some(dispatch) = symbolic_dispatch.as_deref_mut() {
        let mapped = work::map_dispatched(
            input,
            options,
            maps.by_ref().collect(),
            &parameters,
            &source_supports,
            dispatch,
            &mut progress,
        )?;
        Some(work::symmetry_dispatched(mapped, dispatch, &mut progress)?.into_iter())
    } else {
        None
    };
    for index in 0..total {
        let prepared = if let Some(charts) = &mut prepared_charts {
            charts
                .next()
                .ok_or_else(|| GenerationError::Invariant("missing mapped chart".into()))?
        } else {
            emit(
                &mut progress,
                GenerationProgress::Factorization {
                    sector: index,
                    total,
                },
            )?;
            let map = maps
                .next()
                .ok_or_else(|| GenerationError::Invariant("missing geometry map".into()))?;
            let chart = work::map_chart(
                input,
                options,
                map,
                parameters.clone(),
                &mut source_supports,
                &mut progress,
            )?;
            let started = Instant::now();
            let prepared = work::prepare_symmetry(index, chart, total, &mut progress)?;
            emit(
                &mut progress,
                GenerationProgress::PhaseTiming {
                    phase: GenerationPhase::Symmetry,
                    seconds: started.elapsed().as_secs_f64(),
                },
            )?;
            prepared
        };
        let work::PreparedChart { chart, symmetry } = prepared;
        let work::MappedChart {
            program,
            map,
            parameters,
            coordinates,
            mapped,
            pre_subtraction,
            contour,
        } = chart;
        emit(
            &mut progress,
            GenerationProgress::Symmetry {
                completed: index,
                total,
            },
        )?;
        let started = Instant::now();
        #[cfg(test)]
        symmetry::profile::chart(index);
        let matched = registry.register_prepared(index, symmetry)?;
        #[cfg(test)]
        symmetry::profile::trace(
            "Registration",
            "end",
            serde_json::json!({"representative": matched.representative, "matched": matched.representative != index}),
        );
        debug_assert_eq!(matched.permutation.len(), parameters.len());
        charts.push(ChartRecord {
            source_index: index,
            representative: matched.representative,
            representative_permutation: matched.permutation.clone(),
            kernel_sector: None,
            coordinates,
            geometry: map.clone(),
            pre_subtraction,
            contour,
        });
        if matched.representative == index {
            representatives.insert(index, (map, parameters, mapped, 1usize, program));
        } else {
            let representative = representatives
                .get_mut(&matched.representative)
                .ok_or_else(|| {
                    GenerationError::Invariant("missing symmetry representative".into())
                })?;
            representative.3 += 1;
        }
        emit(
            &mut progress,
            GenerationProgress::PhaseTiming {
                phase: GenerationPhase::Symmetry,
                seconds: started.elapsed().as_secs_f64(),
            },
        )?;
        emit(
            &mut progress,
            GenerationProgress::Symmetry {
                completed: index + 1,
                total,
            },
        )?;
    }
    let total = representatives.len();
    let expanded = if let Some(dispatch) = symbolic_dispatch {
        work::expand_dispatched(
            representatives,
            input.regulator(),
            options,
            dispatch,
            &mut progress,
        )?
    } else {
        let mut expanded = Vec::with_capacity(total);
        for (index, (representative_index, (map, parameters, mapped, multiplicity, program))) in
            representatives.into_iter().enumerate()
        {
            #[cfg(test)]
            {
                laurent::profiling::context(index, representative_index, multiplicity);
                laurent::profiling::before_subtraction(
                    &mapped,
                    &parameters,
                    input.regulator(),
                    options.max_order,
                )?;
            }
            let output = coefficients::expand(
                mapped,
                coefficients::Representative {
                    parameters: &parameters,
                    regulator: input.regulator(),
                    index,
                    total,
                },
                options,
                &mut templates,
                &mut progress,
            )?;
            emit(
                &mut progress,
                GenerationProgress::PhaseTiming {
                    phase: output.phase,
                    seconds: output.phase_started.elapsed().as_secs_f64(),
                },
            )?;
            expanded.push((
                representative_index,
                map,
                parameters,
                multiplicity,
                output,
                program,
            ));
        }
        expanded
    };
    let mut assembly = assembly::Assembly::new(options.max_order);
    for (representative_index, map, parameters, multiplicity, output, program) in expanded {
        assembly.push(
            representative_index,
            map,
            parameters,
            multiplicity,
            output,
            program,
        )?;
    }
    #[cfg(test)]
    laurent::profiling::reject_uncaptured_result()?;
    let result = assembly
        .finish(domain, charts, options.max_order)
        .preserve_empty_recipe(options.program_recipe, options.contour_jacobian)?
        .with_source_scope(source_scope)?;
    emit(
        &mut progress,
        GenerationProgress::Complete {
            sectors: result.sectors.len(),
            orders: result.orders.clone(),
        },
    )?;
    Ok(result)
}
// Test-only native named-coefficient program controls.
#[cfg(test)]
type CapturedNamedCoefficients = (
    std::collections::BTreeMap<i32, symbolica::atom::AliasedAtom>,
    serde_json::Value,
);

#[cfg(test)]
pub(crate) fn captured_named_coefficients(
    terms: Vec<(Atom, Atom, Vec<Atom>)>,
    parameters: &[symbolica::atom::Symbol],
    regulator: symbolica::atom::Symbol,
    options: &GenerationOptions,
) -> Result<CapturedNamedCoefficients, GenerationError> {
    captured_named_coefficients_with_faces(terms, parameters, regulator, options, false)
}

#[cfg(test)]
pub(crate) fn captured_named_coefficients_with_faces(
    terms: Vec<(Atom, Atom, Vec<Atom>)>,
    parameters: &[symbolica::atom::Symbol],
    regulator: symbolica::atom::Symbol,
    options: &GenerationOptions,
    interleaved: bool,
) -> Result<CapturedNamedCoefficients, GenerationError> {
    let terms = terms
        .into_iter()
        .map(|(prefactor, regular, powers)| mapping::MappedTerm {
            prefactor,
            regular,
            powers,
        })
        .collect::<Vec<_>>();
    let (coefficients, attempts, statistics) = if interleaved {
        subtraction::series_first::expand_named_with_resolution(
            &terms,
            parameters,
            regulator,
            options,
            subtraction::series_first::Resolution::InterleavedFaces,
        )?
    } else {
        subtraction::series_first::expand_named(&terms, parameters, regulator, options)?
    };
    let attempts = attempts
        .into_iter()
        .map(|attempt| {
            serde_json::json!({
                "route":attempt.route,"width":attempt.width,"absolute_bound":attempt.absolute_bound,
                "pieces":attempt.pieces,"seconds":attempt.seconds,
            })
        })
        .collect::<Vec<_>>();
    Ok((
        coefficients,
        serde_json::json!({"coefficient_representation":"native_named", "resolution":if interleaved {"interleaved_faces"} else {"original"}, "attempts":attempts,"statistics":statistics}),
    ))
}

#[cfg(feature = "threshold-decomposition")]
pub(crate) use threshold::expand_vector as threshold_expand_vector;

mod constant;
pub(crate) use constant::coordinate_independent;
