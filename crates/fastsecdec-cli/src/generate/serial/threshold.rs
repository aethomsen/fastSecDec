//! Compact coordinator for native threshold preparation and compilation.
use super::jobs::threshold::{self as worker, PreparedThreshold};
use super::*;
use fastsecdec::kernel::ThresholdPublicationPlan;

pub(super) fn generate(run: GenerationRun<'_>, dashboard: &mut Dashboard) -> CliResult<Artifact> {
    generate_with_retry(run, dashboard, None)
}
pub(super) fn generate_with_retry(
    run: GenerationRun<'_>,
    dashboard: &mut Dashboard,
    retry: Option<(&Path, &Path)>,
) -> CliResult<Artifact> {
    crate::artifact::paths(run.output)?;
    if run.workers == 0 {
        return Err("generation workers must be positive".into());
    }
    let path = fs::canonicalize(run.path)?;
    let output = std::path::absolute(run.output)?;
    let mut card: RunCard = toml::from_str(&fs::read_to_string(&path)?)?;
    run.overrides.apply(&mut card);
    if !card.generation.threshold_enabled() {
        return Err("threshold pipeline requires its explicit capability".into());
    }
    card.generation.evaluator.validate()?;
    dashboard.configure_generation(
        card.generation.mode,
        card.generation.coefficient_expansion.method,
    );
    let run_id = format!(
        "{}-{}",
        std::process::id(),
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let retry_seed = retry
        .map(|(old, overlay)| {
            super::threshold_retry::open(&path, &output, old, overlay, run.overrides, run.resume)
        })
        .transpose()?;
    let mut journal = Journal::open_with_family(
        &path,
        &output,
        run.resume,
        &run_id,
        run.overrides,
        RecipeFamily::single(ProgramRecipe::ThresholdV1),
    )?;
    let retry = journal.endpoint_retry(retry_seed.as_ref().map(|seed| seed.attempt.clone()))?;
    if journal.completed() {
        let artifact = Artifact::load_metadata(&output)?;
        artifact.verify_input_sources(&path)?;
        if !artifact.dependencies_compatible()
            || artifact.programs.as_ref().is_none_or(|p| {
                p.default_recipe != ProgramRecipe::ThresholdV1 || p.catalogue.recipes.len() != 1
            })
        {
            return Err("completed threshold artifact capability or dependencies changed".into());
        }
        return Ok(artifact);
    }
    let prior = journal.threshold_preparation()?;
    let mut runner = Runner::new(run.workers, run_id.clone(), journal.residency_lock())?;
    // Unique issued key forces current-process verification, even if an older
    // preparation response is durable. Old verification cannot be cached trust.
    let prepare_key = format!("threshold-prepare-{run_id}");
    let result = runner
        .run(
            &mut journal,
            vec![(
                prepare_key.clone(),
                if let Some(retry) = retry {
                    Request::RetryThreshold {
                        input: path.clone(),
                        workers: run.workers,
                        overrides: run.overrides,
                        attempt: run_id,
                        prior: prior.clone().map(Box::new),
                        retry: Box::new(retry),
                    }
                } else {
                    Request::PrepareThreshold {
                        input: path.clone(),
                        workers: run.workers,
                        overrides: run.overrides,
                        attempt: run_id,
                        prior: prior.clone().map(Box::new),
                    }
                },
            )],
            GenerationStage::Geometry,
            "Native threshold preparation and verification",
            dashboard,
        )?
        .pop()
        .ok_or("missing threshold preparation completion")?;
    let Response::PreparedThreshold(fresh) = result else {
        return Err("invalid threshold preparation response".into());
    };
    let mut timings = fresh.timings.clone();
    let anchor = if let Some(prior) = prior {
        prior
    } else {
        journal.anchor_threshold_preparation(&prepare_key)?;
        (*fresh).clone()
    };
    let plan = readmit(&journal.root, &anchor, &fresh)?;
    let directory = Path::new(&anchor.directory)
        .join(&anchor.native.work_directory)
        .to_str()
        .ok_or("non-UTF8 threshold staging path")?
        .to_owned();
    let mut requests = Vec::with_capacity(plan.job_count());
    for index in 0..plan.job_count() {
        requests.push((
            format!("threshold-compile-{index}"),
            Request::CompileThreshold {
                directory: directory.clone(),
                work: plan.work(index)?,
                output: journal.root.join(format!("threshold-record-{index}.fsd")),
            },
        ));
    }
    let started = Instant::now();
    let responses = runner.run(
        &mut journal,
        requests,
        GenerationStage::Compilation,
        "Compiling native threshold contributions",
        dashboard,
    )?;
    timings.compilation_seconds = started.elapsed().as_secs_f64();
    let mut compiled = responses
        .into_iter()
        .map(|response| match response {
            Response::CompiledThreshold(value) => Ok(value),
            _ => Err("invalid threshold compilation response".into()),
        })
        .collect::<CliResult<Vec<_>>>()?;
    // Cached and newly completed workers arrive in scheduler order. The native
    // closed inventory is in contribution order, with the setup carrier first.
    compiled.sort_by_key(|value| value.receipt.work().index());
    let staged = journal.root.join("complete.fsd.dat");
    let mut writer = plan.archive_writer(BufWriter::new(File::create(&staged)?))?;
    for compiled in compiled {
        plan.append_work_record(
            &mut writer,
            &mut File::open(&compiled.data)?,
            compiled.receipt,
        )?;
    }
    let (mut data, catalogue) = writer.finish()?;
    data.flush()?;
    data.get_ref().sync_all()?;
    drop(data);
    if dashboard.cancelled() {
        return Err("threshold generation cancelled before publication; use --resume".into());
    }
    let selected = catalogue.recipe(ProgramRecipe::ThresholdV1)?;
    let count = selected.sector_count();
    if let Some(reference) = run.reference {
        reference.validate_identity(&selected.content_id)?;
    }
    dashboard.generation_saving(runner.elapsed());
    let mut artifact = Artifact::from_program_archive(
        &staged,
        catalogue,
        ProgramRecipe::ThresholdV1,
        fresh.provenance,
    )?;
    artifact.reference = run.reference.map(|r| r.settings.clone());
    artifact.relocate_sources(
        path.parent().unwrap_or_else(|| Path::new(".")),
        output.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    timings.total_seconds = runner.elapsed();
    artifact.generation_timings = Some(timings.clone());
    let mut generation = fresh.generation;
    generation.workers = run.workers;
    artifact.set_program_generation(BTreeMap::from([(
        ProgramRecipe::ThresholdV1,
        ProgramGeneration {
            generation,
            timings: timings.clone(),
        },
    )]))?;
    artifact.save_staged(&output)?;
    journal.complete()?;
    let artifact = Artifact::load_metadata(&output)?;
    dashboard.generation(&GenerationSnapshot {
        stage: GenerationStage::Complete,
        completed: count,
        total: Some(count),
        sectors: count,
        kernels: count,
        elapsed_seconds: runner.elapsed(),
        timings,
        coefficient_expansion: None,
        formula_preparation: None,
        detail: format!(
            "Saved {} · complete threshold inventory · all native processes released",
            crate::artifact::relative_display(&output)
        ),
    })?;
    journal.cleanup()?;
    Ok(artifact)
}
fn readmit(
    root: &Path,
    anchor: &PreparedThreshold,
    fresh: &PreparedThreshold,
) -> CliResult<ThresholdPublicationPlan> {
    if anchor.native.publication.source_identity != fresh.native.publication.source_identity
        || anchor.native.publication.prepared_identity != fresh.native.publication.prepared_identity
    {
        return Err("reverified threshold preparation differs from compile anchor".into());
    }
    let directory = worker::rooted(root, &anchor.directory)?;
    Ok(ThresholdPublicationPlan::from_trusted_preparer(
        &worker::rooted(&directory, &anchor.native.work_directory)?,
        anchor.native.publication.clone(),
        &fresh.native.publication.source_identity,
        &fresh.native.publication.prepared_identity,
        worker::TRANSPORT_BYTES,
    )?)
}
