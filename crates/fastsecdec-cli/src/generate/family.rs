//! CLI-owned parallel dispatch around the native bounded family session.
use super::*;
use crate::artifact::{InspectionIndex, ProgramGeneration};
use fastsecdec::{
    generation::{
        RecipeFamilyJobProgress, RecipeFamilyJobStage, RecipeFamilySession,
        RecipeFamilySessionError,
    },
    kernel::ProgramRecipe,
    status::GenerationTimings,
};
use std::collections::BTreeMap;

#[allow(clippy::too_many_arguments)]
pub(super) fn generate(
    loaded: input::LoadedInput,
    options: GenerationOptions,
    path: &Path,
    output: &Path,
    dashboard: &mut Dashboard,
    reference: Option<&crate::reference::PreparedReference>,
    workers: usize,
    resident_recipe: Option<ProgramRecipe>,
    mut status: GenerationSnapshot,
    started: Instant,
) -> CliResult<(Artifact, KernelSet)> {
    let family = loaded.card.generation.recipe_family();
    let resident_recipe = resident_recipe.unwrap_or(family.default_recipe());
    family.validate_resident(Some(resident_recipe))?;
    let provenance = super::provenance(&loaded, &options)?;
    let evaluator = loaded.card.generation.evaluator;
    let record = GenerationRecord {
        endpoint_retry: None,
        workers,
        mode: Some(options.mode),
        subtraction: Some(options.subtraction),
        source_chart_modes: None,
        formula_preparation: None,
        contraction_mode: loaded
            .loops
            .map(|_| loaded.card.generation.contraction_mode),
        requested_coefficient_expansion: options.coefficient_expansion.method,
        evaluator: Some(evaluator),
    };
    let staging = tempfile::tempdir()?;
    let staged = tempfile::NamedTempFile::new()?;
    let mut session = RecipeFamilySession::new(
        loaded.integrand,
        options.clone(),
        family.clone(),
        staging.path().to_owned(),
        staged.as_file().try_clone()?,
    )
    .with_runtime_inputs(loaded.runtime_parameters, loaded.runtime_mass_constraints)?
    .with_resident_recipe(Some(resident_recipe))?
    .with_evaluator(Default::default(), evaluator)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()?;
    let cancelled = dashboard.cancellation_handle();
    let mut display_error = None;
    let mut observations = BTreeMap::new();
    let mut active_recipe = None;
    let mut recipe_started = Instant::now();
    let mut completed_recipes = 0;
    let worker_snapshot = status.clone();
    {
        let ui = RefCell::new((&mut *dashboard, &mut status, &mut display_error));
        let mut geometry_dispatch =
            |jobs: &mut dyn ExactSizeIterator<Item = generation::GeometryJob>| {
                dispatch::run(
                    &pool,
                    jobs,
                    &cancelled,
                    |job| format!("geometry {:?}", job.id()),
                    |job, observe| {
                        Ok(job.run(|p| {
                            observe(format!(
                                "{:?} · {}/{} constraints",
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
                        dashboard.generation_workers(progress);
                        publish_generation(dashboard, status, error)
                    },
                )
                .map_err(generation::SectorError::Geometry)
            };
        let mut dispatch = |jobs: &mut dyn ExactSizeIterator<
            Item = generation::RecipeFamilyJob,
        >| {
            let mut jobs = jobs.peekable();
            let stage = jobs.peek().map(|job| job.id().stage);
            dispatch::run(
                &pool,
                &mut jobs,
                &cancelled,
                |job| format!("{:?} {}", job.id().stage, job.id().index),
                |job, observe| {
                    let mut progress = worker_snapshot.clone();
                    job.run(|event| {
                        match event {
                            RecipeFamilyJobProgress::Generation(event) => {
                                progress.observe_generation(options.max_order, event)
                            }
                            RecipeFamilyJobProgress::Compilation(event) => {
                                progress.observe_compilation(event)
                            }
                        }
                        observe(progress.detail.clone())
                    })
                    .map_err(|error| error.to_string())
                },
                |progress| {
                    let mut ui = ui.borrow_mut();
                    let (dashboard, status, error) = &mut *ui;
                    status.stage = match stage {
                        Some(RecipeFamilyJobStage::Formula) => GenerationStage::FormulaPreparation,
                        Some(RecipeFamilyJobStage::Sector) => GenerationStage::Compilation,
                        _ => GenerationStage::Mapping,
                    };
                    status.elapsed_seconds = started.elapsed().as_secs_f64();
                    status.detail = format!("{workers} workers · {} running", progress.running());
                    dashboard.generation_workers(progress);
                    publish_generation(dashboard, status, error)
                },
            )
            .map_err(RecipeFamilySessionError::State)
        };
        let outcome = session.step_with_dispatch(
            usize::MAX,
            workers,
            &mut geometry_dispatch,
            &mut dispatch,
            |snapshot| {
                if snapshot.recipe != active_recipe {
                    active_recipe = snapshot.recipe;
                    recipe_started = Instant::now();
                }
                if snapshot.completed_recipes > completed_recipes {
                    completed_recipes = snapshot.completed_recipes;
                    let mut generation = record.clone();
                    generation.formula_preparation = snapshot.generation.formula_preparation;
                    observations.insert(
                        snapshot.recipe.unwrap(),
                        ProgramGeneration {
                            generation,
                            timings: GenerationTimings {
                                total_seconds: recipe_started.elapsed().as_secs_f64(),
                                ..Default::default()
                            },
                        },
                    );
                }
                let mut ui = ui.borrow_mut();
                let (dashboard, status, error) = &mut *ui;
                **status = snapshot.generation.clone();
                status.timings.input_seconds = loaded.input_seconds;
                status.timings.parametrization_seconds = loaded.parametrization_seconds;
                status.elapsed_seconds = started.elapsed().as_secs_f64();
                if let Some(recipe) = snapshot.recipe {
                    status.detail = format!(
                        "{} · {}/{} recipes · {}",
                        recipe.name(),
                        snapshot.completed_recipes,
                        snapshot.total_recipes,
                        status.detail
                    );
                }
                publish_generation(dashboard, status, error)
            },
        );
        if let Some(error) = ui.borrow_mut().2.take() {
            return Err(error.into());
        }
        outcome?;
    }
    if dashboard.cancelled() || !session.is_complete() {
        return Err("generation cancelled".into());
    }
    let result = session
        .take_result()
        .ok_or("native family has no completed output")?;
    result.writer.sync_all()?;
    let kernels = result
        .resident
        .ok_or("native family lost requested resident")?;
    if let Some(reference) = reference.filter(|_| kernels.runtime_parameters().is_empty()) {
        reference.validate_identity(kernels.content_id())?;
    }
    let mut artifact = Artifact::from_program_archive(
        staged.path(),
        result.catalogue,
        family.default_recipe(),
        provenance,
    )?;
    artifact
        .programs
        .as_mut()
        .unwrap()
        .inspection
        .insert(resident_recipe, InspectionIndex::from_kernels(&kernels));
    artifact.set_program_generation(observations)?;
    artifact.reference = reference.map(|reference| reference.settings.clone());
    artifact.relocate_sources(
        path.parent().unwrap_or_else(|| Path::new(".")),
        output.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    status.timings.total_seconds = started.elapsed().as_secs_f64();
    artifact.generation_timings = Some(status.timings.clone());
    dashboard.generation_coordinator();
    dashboard.generation_saving(started.elapsed().as_secs_f64());
    artifact.save_staged(output)?;
    let artifact = Artifact::load_metadata(output)?;
    status.stage = GenerationStage::Complete;
    status.sectors = kernels
        .generation_metadata()
        .map_or(0, |metadata| metadata.charts().len());
    status.kernels = kernels.sectors().len();
    status.completed = status.kernels;
    status.total = Some(status.kernels);
    status.detail = format!(
        "Saved all {} recipes to {} · reporting {}",
        family.recipes().len(),
        crate::artifact::relative_display(output),
        resident_recipe.name(),
    );
    dashboard.generation(&status)?;
    Ok((artifact, kernels))
}
