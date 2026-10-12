pub(crate) mod dispatch;
mod family;
#[cfg(test)]
mod geometry_dispatch;
mod progress;
#[cfg(test)]
mod record_tests;
pub(crate) mod serial;

use progress::{observe_generation, publish_generation, worker_activity};

use std::{
    cell::RefCell, io::Write, ops::ControlFlow, path::Path, sync::atomic::Ordering, time::Instant,
};

use fastsecdec::{
    generation::{self, GenerationContext, GenerationEvent, GenerationOptions, GenerationProgress},
    kernel::KernelSet,
    status::{GenerationSnapshot, GenerationStage},
};

use crate::{
    CliResult,
    artifact::{Artifact, GenerationRecord, Provenance, dependencies},
    display::Dashboard,
    input,
};

#[cfg(test)]
pub fn generate(
    path: &Path,
    output: &Path,
    dashboard: &mut Dashboard,
    reference: Option<&crate::reference::PreparedReference>,
) -> CliResult<(Artifact, KernelSet)> {
    generate_with_workers(path, output, dashboard, reference, 1)
}

#[cfg(test)]
pub fn generate_with_workers(
    path: &Path,
    output: &Path,
    dashboard: &mut Dashboard,
    reference: Option<&crate::reference::PreparedReference>,
    workers: usize,
) -> CliResult<(Artifact, KernelSet)> {
    generate_with_overrides(
        path,
        output,
        dashboard,
        reference,
        workers,
        Default::default(),
    )
}

pub(crate) fn generate_with_overrides(
    path: &Path,
    output: &Path,
    dashboard: &mut Dashboard,
    reference: Option<&crate::reference::PreparedReference>,
    workers: usize,
    overrides: crate::config::GenerationOverrides,
) -> CliResult<(Artifact, KernelSet)> {
    generate_with_resident_recipe(path, output, dashboard, reference, workers, overrides, None)
}

/// The artifact default and the resident owner used by `run` are separate
/// choices. Family generation retains that owner while draining other recipes.
pub(crate) fn generate_with_resident_recipe(
    path: &Path,
    output: &Path,
    dashboard: &mut Dashboard,
    reference: Option<&crate::reference::PreparedReference>,
    workers: usize,
    overrides: crate::config::GenerationOverrides,
    resident_recipe: Option<fastsecdec::kernel::ProgramRecipe>,
) -> CliResult<(Artifact, KernelSet)> {
    crate::artifact::paths(output)?;
    if workers == 0 {
        return Err("generation workers must be positive".into());
    }
    let mut threshold_card: crate::config::RunCard =
        toml::from_str(&std::fs::read_to_string(path)?)?;
    overrides.apply(&mut threshold_card);
    if threshold_card.generation.threshold_enabled() {
        if let Some(recipe) = resident_recipe {
            threshold_card.generation.validate_resident_recipe(recipe)?;
        }
        serial::generate_with_overrides(
            path, output, dashboard, reference, workers, false, overrides,
        )?;
        return Artifact::load(output);
    }
    let started = Instant::now();
    let mut status = GenerationSnapshot {
        stage: GenerationStage::Input,
        completed: 0,
        total: None,
        sectors: 0,
        kernels: 0,
        elapsed_seconds: 0.0,
        timings: Default::default(),
        coefficient_expansion: None,
        formula_preparation: None,
        detail: format!("Reading {}", crate::artifact::relative_display(path)),
    };
    dashboard.generation(&status)?;
    let loaded = input::load_observed_with_overrides(path, overrides, |progress| {
        match progress {
            input::LoadProgress::Parsed(card) => {
                if let Some(recipe) = resident_recipe {
                    card.generation.validate_resident_recipe(recipe)?;
                }
                dashboard.configure_generation(
                    card.generation.mode,
                    card.generation.coefficient_expansion.method,
                );
            }
            input::LoadProgress::Parametrization => {
                status.stage = GenerationStage::Parametrization;
                status.completed = 0;
                status.total = None;
                status.detail = "Preparing the native integral".into();
            }
        }
        status.elapsed_seconds = started.elapsed().as_secs_f64();
        dashboard.generation(&status)?;
        if dashboard.cancelled() {
            Err("generation cancelled".into())
        } else {
            Ok(())
        }
    })?;
    let evaluator_settings = loaded.card.generation.evaluator;
    evaluator_settings.validate()?;
    status.timings.input_seconds = loaded.input_seconds;
    status.timings.parametrization_seconds = loaded.parametrization_seconds;
    status.stage = GenerationStage::Parametrization;
    status.completed = 1;
    status.total = Some(1);
    status.detail = format!(
        "{} · {} parameters · {} terms",
        loaded.label,
        loaded.integrand.parameters().len(),
        loaded.integrand.terms().len()
    );
    status.elapsed_seconds = started.elapsed().as_secs_f64();
    dashboard.generation(&status)?;
    let mut options = GenerationOptions {
        source_sectors: loaded.card.generation.source_sectors.clone(),
        max_order: loaded.card.generation.order,
        mode: loaded.card.generation.mode,
        contour_jacobian: loaded.card.generation.contour_jacobian,
        subtraction: loaded.card.generation.subtraction,
        assume_no_threshold: loaded.card.generation.assume_no_threshold,
        program_recipe: loaded.card.generation.program_recipe(),
        coefficient_expansion: loaded.card.generation.coefficient_expansion.clone(),
        ..GenerationOptions::default()
    };
    options.decomposition.max_sectors = loaded.card.generation.max_sectors;
    options.decomposition.max_support_pairs = loaded.card.generation.max_support_pairs;
    if loaded.card.generation.recipe_family().recipes().len() > 1 {
        return family::generate(
            loaded,
            options,
            path,
            output,
            dashboard,
            reference,
            workers,
            resident_recipe,
            status,
            started,
        );
    }
    let source_identity = generation::source_identity(
        &loaded.integrand,
        &loaded.runtime_parameters,
        &loaded.runtime_mass_constraints,
    )?;
    let mut display_error = None;
    let cancelled = dashboard.cancellation_handle();
    let generated = {
        let ui = RefCell::new((&mut *dashboard, &mut status, &mut display_error));
        let present = |progress: &GenerationProgress| {
            let mut ui = ui.borrow_mut();
            let (dashboard, status, error) = &mut *ui;
            observe_generation(
                dashboard,
                status,
                error,
                started,
                options.max_order,
                progress,
            )
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()?;
        let mut geometry_stage = 0;
        let mut geometry_dispatch =
            |jobs: &mut dyn ExactSizeIterator<Item = generation::GeometryJob>| {
                let label = if geometry_stage == 0 {
                    "charts"
                } else {
                    "cones"
                };
                geometry_stage += 1;
                dispatch::run(
                    &pool,
                    jobs,
                    &cancelled,
                    |job| format!("geometry {:?}", job.id()),
                    |job, observe| {
                        let id = job.id();
                        Ok(job.run(|p| {
                            observe(format!(
                                "{id:?} · {:?} · {}/{} constraints",
                                p.phase, p.completed_constraints, p.total_constraints
                            ))
                        }))
                    },
                    |progress| {
                        let mut ui = ui.borrow_mut();
                        let (dashboard, status, error) = &mut *ui;
                        status.stage = GenerationStage::Geometry;
                        status.completed = progress.completed;
                        status.total = Some(progress.total);
                        status.elapsed_seconds = started.elapsed().as_secs_f64();
                        status.detail = format!(
                            "{workers} workers · {label} · {} running · {} queued",
                            progress.running(),
                            progress
                                .total
                                .saturating_sub(progress.completed + progress.running())
                        );
                        dashboard.generation_workers(progress);
                        publish_generation(dashboard, status, error)
                    },
                )
                .map_err(|error| {
                    if error == "generation cancelled" {
                        generation::SectorError::Cancelled
                    } else {
                        generation::SectorError::Geometry(error)
                    }
                })
            };
        let mut symbolic_dispatch =
            |jobs: &mut dyn ExactSizeIterator<Item = generation::SymbolicJob>| {
                if jobs.len() == 0 {
                    return Ok(Vec::new());
                }
                // The first job identifies the homogeneous native stage without
                // pre-running or collecting its remaining lazy work.
                let mut jobs = jobs.peekable();
                let stage = match jobs.peek().map(|job| job.id().stage) {
                    Some(generation::SymbolicStage::FormulaPreparation) => {
                        GenerationStage::FormulaPreparation
                    }
                    Some(generation::SymbolicStage::Symmetry) => GenerationStage::Symmetry,
                    Some(generation::SymbolicStage::Coefficients) => {
                        GenerationStage::CoefficientExpansion
                    }
                    _ => GenerationStage::Mapping,
                };
                dispatch::run(
                &pool,
                &mut jobs,
                &cancelled,
                |job| match job.id().stage {
                    generation::SymbolicStage::FormulaPreparation => {
                        format!("Subtraction formula {}", job.id().index)
                    }
                    generation::SymbolicStage::Symmetry => {
                        format!("Finding equivalent sectors · sector {}", job.id().index)
                    }
                    stage => format!("{stage:?} · sector {}", job.id().index),
                },
                |job, observe| {
                    let id = job.id();
                    job.run(|progress| {
                        let mut snapshot = GenerationSnapshot {
                            stage,
                            completed: 0,
                            total: None,
                            sectors: 0,
                            kernels: 0,
                            elapsed_seconds: 0.0,
                            timings: Default::default(),
                            coefficient_expansion: None,
                            formula_preparation: None,
                            detail: String::new(),
                        };
                        snapshot.observe_generation(options.max_order, progress);
                        observe(worker_activity(id, &snapshot.detail))
                    })
                    .map_err(|error| error.to_string())
                },
                |progress| {
                    let mut ui = ui.borrow_mut();
                    let (dashboard, status, error) = &mut *ui;
                    status.stage = stage;
                    status.completed = progress.completed;
                    status.total = Some(progress.total);
                    status.elapsed_seconds = started.elapsed().as_secs_f64();
                    status.detail = format!(
                        "{workers} workers · {} running · {} queued",
                        progress.running(),
                        progress
                            .total
                            .saturating_sub(progress.completed + progress.running())
                    );
                    if stage == GenerationStage::Symmetry {
                        status.detail = format!("Preparing comparisons · {}", status.detail);
                    } else if stage == GenerationStage::FormulaPreparation {
                        status.coefficient_expansion = None;
                        if let Some(formulas) = &mut status.formula_preparation {
                            formulas.completed = progress.completed;
                            status.detail = format!(
                                "{} unique formulas · {} eligible sectors · {} shared uses · {}",
                                formulas.total, formulas.sectors, formulas.reused, status.detail
                            );
                        }
                    }
                    dashboard.generation_workers(progress);
                    publish_generation(dashboard, status, error)
                },
            )
            .map_err(|error| {
                if error == "generation cancelled" {
                    generation::GenerationError::Cancelled
                } else {
                    generation::GenerationError::Invariant(error)
                }
            })
            };
        GenerationContext::new(0).generate_with_all_dispatch(
            &loaded.integrand,
            &options,
            &mut geometry_dispatch,
            &mut symbolic_dispatch,
            || cancelled.load(Ordering::Relaxed),
            |event| match event {
                GenerationEvent::Progress(progress) => present(progress),
                GenerationEvent::GeometryReuse(_) => ControlFlow::Continue(()),
            },
        )
    };
    if let Some(error) = display_error {
        return Err(error.into());
    }
    let generated = generated?;
    let source_chart_modes = source_chart_modes(&generated);
    let compilation_started = Instant::now();
    let kernels = {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()?;
        let ui = RefCell::new((&mut *dashboard, &mut status, &mut display_error));
        let mut compile_dispatch =
            |jobs: &mut dyn ExactSizeIterator<Item = fastsecdec::kernel::CompilationJob>| {
                dispatch::run(
                    &pool,
                    jobs,
                    &cancelled,
                    |job| format!("SymJIT O2 · sector {}", job.index()),
                    |job, _| job.run().map_err(|error| error.to_string()),
                    |progress| {
                        let mut ui = ui.borrow_mut();
                        let (dashboard, status, error) = &mut *ui;
                        status.stage = GenerationStage::Compilation;
                        status.completed = progress.completed;
                        status.total = Some(progress.total);
                        status.kernels = progress.completed;
                        status.elapsed_seconds = started.elapsed().as_secs_f64();
                        status.detail = format!(
                            "SymJIT O2 · {workers} workers · {} running",
                            progress.running()
                        );
                        dashboard.generation_workers(progress);
                        publish_generation(dashboard, status, error)
                    },
                )
                .map_err(fastsecdec::kernel::KernelError::Compilation)
            };
        generated.compile_with_settings_parameters_and_dispatch(
            fastsecdec::kernel::PrecisionPolicy::default(),
            &loaded.runtime_parameters,
            evaluator_settings,
            &mut compile_dispatch,
            |progress| {
                let mut ui = ui.borrow_mut();
                let (dashboard, status, error) = &mut *ui;
                status.observe_compilation(progress);
                status.elapsed_seconds = started.elapsed().as_secs_f64();
                publish_generation(dashboard, status, error)
            },
        )
    };
    if let Some(error) = display_error {
        return Err(error.into());
    }
    let provenance = provenance(&loaded, &options)?;
    let mut kernels = kernels?.with_runtime_mass_constraints(loaded.runtime_mass_constraints)?;
    drop(generated);
    status.timings.compilation_seconds = compilation_started.elapsed().as_secs_f64();
    dashboard.generation_coordinator();
    status.completed = 0;
    status.total = None;
    dashboard.generation_saving(started.elapsed().as_secs_f64());
    status.detail = "Preparing portable artifact; all kernels compiled".into();
    status.elapsed_seconds = started.elapsed().as_secs_f64();
    dashboard.generation(&status)?;
    // The native owner partitions and adopts the same universal recipe archive
    // as serial workers without rebuilding any evaluator. Its retained bytes
    // are borrowed into a temporary staging file, not copied into another Vec.
    let catalogue = kernels.retain_program_archive(source_identity)?;
    if let Some(reference) = reference.filter(|_| kernels.runtime_parameters().is_empty()) {
        reference.validate_identity(kernels.content_id())?;
    }
    let mut staged = tempfile::NamedTempFile::new()?;
    staged.write_all(kernels.artifact_bytes()?)?;
    staged.as_file().sync_all()?;
    let mut artifact = Artifact::from_program_archive(
        staged.path(),
        catalogue,
        options.program_recipe,
        provenance,
    )?;
    artifact
        .programs
        .as_mut()
        .expect("new recipe archive")
        .inspection
        .insert(
            options.program_recipe,
            crate::artifact::InspectionIndex::from_kernels(&kernels),
        );
    artifact.reference = reference.map(|value| value.settings.clone());
    artifact.relocate_sources(
        path.parent().unwrap_or_else(|| Path::new(".")),
        output.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    let generation_record = GenerationRecord {
        endpoint_retry: None,
        workers,
        mode: Some(options.mode),
        subtraction: Some(options.subtraction),
        source_chart_modes: Some(source_chart_modes),
        formula_preparation: status.formula_preparation,
        contraction_mode: loaded
            .loops
            .map(|_| loaded.card.generation.contraction_mode),
        requested_coefficient_expansion: options.coefficient_expansion.method,
        evaluator: Some(evaluator_settings),
    };
    artifact.set_program_generation(std::collections::BTreeMap::from([(
        options.program_recipe,
        crate::artifact::ProgramGeneration {
            generation: generation_record,
            timings: status.timings.clone(),
        },
    )]))?;
    status.timings.total_seconds = started.elapsed().as_secs_f64();
    artifact.generation_timings = Some(status.timings.clone());
    status.detail = format!(
        "Saving portable artifact to {}",
        crate::artifact::relative_display(output)
    );
    status.elapsed_seconds = started.elapsed().as_secs_f64();
    dashboard.generation(&status)?;
    artifact.save_staged(output)?;
    // The manifest owns the generation-specific immutable data filename.
    // Return that published handle, without retaining a second archive buffer.
    let artifact = Artifact::load_metadata(output)?;
    status.stage = GenerationStage::Complete;
    status.kernels = kernels.sectors().len();
    status.completed = status.kernels;
    status.total = Some(status.kernels);
    status.elapsed_seconds = started.elapsed().as_secs_f64();
    status.detail = format!("Saved {}", crate::artifact::relative_display(output));
    dashboard.generation(&status)?;
    Ok((artifact, kernels))
}

fn provenance(loaded: &input::LoadedInput, options: &GenerationOptions) -> CliResult<Provenance> {
    Ok(Provenance {
        name: loaded.label.clone(),
        sources: loaded.sources.clone(),
        dependencies: dependencies(),
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
    })
}

fn source_chart_modes(
    generated: &generation::GeneratedIntegral,
) -> std::collections::BTreeMap<usize, generation::GenerationMode> {
    generated
        .metadata()
        .charts()
        .iter()
        .map(|chart| {
            // Zero-dimensional charts use exact symbolic admission and are
            // folded into the exact contribution before a kernel is created.
            let mode = chart
                .kernel_sector()
                .map_or(generation::GenerationMode::Symbolic, |sector| {
                    generated.sectors()[sector].generation_mode()
                });
            (chart.source_index(), mode)
        })
        .collect()
}
