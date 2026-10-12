//! CLI process orchestration for bounded-memory generation and recovery.
mod family;
#[cfg(test)]
mod family_tests;
pub(crate) mod jobs;
mod journal;
mod pipeline;
#[cfg(test)]
mod program_recovery_tests;
#[cfg(test)]
mod recovery_tests;
mod runner;
mod threshold;
mod threshold_retry;

pub(crate) fn retry_endpoint(
    run: GenerationRun<'_>,
    old_output: &Path,
    overlay: &Path,
    dashboard: &mut Dashboard,
) -> CliResult<Artifact> {
    threshold::generate_with_retry(run, dashboard, Some((old_output, overlay)))
}
#[cfg(test)]
mod threshold_tests;

use crate::{
    CliResult,
    artifact::{Artifact, ProgramGeneration},
    config::RunCard,
    display::Dashboard,
    reference::PreparedReference,
};
use family::RecipeFamily;
#[cfg(test)]
use fastsecdec::generation::streaming as native;
use fastsecdec::{
    kernel::indexed::{ProgramArchiveWriter, ProgramRecipe},
    status::{GenerationSnapshot, GenerationStage, GenerationTimings},
};
use jobs::{Prepared, Request, Response};
use journal::Journal;
use runner::Runner;
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufWriter, Write},
    path::Path,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

pub(crate) struct GenerationRun<'a> {
    pub path: &'a Path,
    pub output: &'a Path,
    pub reference: Option<&'a PreparedReference>,
    pub workers: usize,
    pub resume: bool,
    pub overrides: crate::config::GenerationOverrides,
}

/// Resolve public capabilities once and retain them in the durable journal.
pub(crate) fn generate_with_overrides(
    path: &Path,
    output: &Path,
    dashboard: &mut Dashboard,
    reference: Option<&PreparedReference>,
    workers: usize,
    resume: bool,
    overrides: crate::config::GenerationOverrides,
) -> CliResult<Artifact> {
    let mut card: RunCard = toml::from_str(&fs::read_to_string(path)?)?;
    overrides.apply(&mut card);
    if card.generation.threshold_enabled() {
        return threshold::generate(
            GenerationRun {
                path,
                output,
                reference,
                workers,
                resume,
                overrides,
            },
            dashboard,
        );
    }
    let family = card.generation.recipe_family();
    generate_family(
        GenerationRun {
            path,
            output,
            reference,
            workers,
            resume,
            overrides,
        },
        family.recipes(),
        family.default_recipe(),
        dashboard,
    )
}

/// All heavy source/recipe work runs in recyclable CLI child processes. This
/// coordinator retains compact receipts and stream-copies native records only.
pub(crate) fn generate_family(
    run: GenerationRun<'_>,
    recipes: &[ProgramRecipe],
    default_recipe: ProgramRecipe,
    dashboard: &mut Dashboard,
) -> CliResult<Artifact> {
    let family = RecipeFamily::new(recipes.iter().copied(), default_recipe)?;
    crate::artifact::paths(run.output)?;
    if run.workers == 0 {
        return Err("generation workers must be positive".into());
    }
    let path = fs::canonicalize(run.path)?;
    let output = std::path::absolute(run.output)?;
    let mut card: RunCard = toml::from_str(&fs::read_to_string(&path)?)?;
    run.overrides.apply(&mut card);
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
    // Persist the complete family, including its default, before this shortcut.
    let mut journal = Journal::open_with_family(
        &path,
        &output,
        run.resume,
        &run_id,
        run.overrides,
        family.clone(),
    )?;
    if journal.completed() {
        let artifact = Artifact::load_metadata(&output)?;
        artifact.verify_input_sources(&path)?;
        if !artifact.dependencies_compatible() {
            return Err("completed generation dependencies changed".into());
        }
        let stored = artifact
            .programs
            .as_ref()
            .ok_or("completed family has no recipe directory")?;
        if stored.default_recipe != family.default_recipe()
            || stored
                .catalogue
                .recipes
                .iter()
                .map(|r| r.recipe)
                .ne(family.recipes().iter().copied())
        {
            return Err("completed artifact differs from its requested recipe family".into());
        }
        return Ok(artifact);
    }
    let mut runner = Runner::new(run.workers, run_id, journal.residency_lock())?;
    let prepared = runner
        .run(
            &mut journal,
            vec![(
                "prepare".into(),
                Request::PreparePrograms {
                    input: path.clone(),
                    workers: run.workers,
                    overrides: run.overrides,
                    recipes: family.recipes().to_vec(),
                },
            )],
            GenerationStage::Geometry,
            "Shared native source and geometry",
            dashboard,
        )?
        .pop()
        .ok_or("missing recipe preparation result")?;
    let Response::PreparedPrograms(prepared) = prepared else {
        return Err("invalid recipe preparation completion".into());
    };
    let preparation_path = journal.response_path("prepare");
    let mut timings = prepared.timings.clone();
    let phase = Instant::now();
    let source_jobs = prepared
        .native
        .recipes
        .first()
        .ok_or("empty prepared family")?
        .charts
        .iter()
        .map(|map| {
            (
                format!("source-{}", map.index),
                Request::PrepareChartSource {
                    preparation: preparation_path.clone(),
                    source_identity: prepared.native.source_identity.clone(),
                    dimension: prepared.native.recipes[0].dimension,
                    map: map.clone(),
                },
            )
        })
        .collect();
    let mut sources = runner
        .run(
            &mut journal,
            source_jobs,
            GenerationStage::Mapping,
            "Extracting shared native chart sources",
            dashboard,
        )?
        .into_iter()
        .map(|r| match r {
            Response::ChartSource(source) => Ok(source),
            _ => Err("invalid chart-source completion".into()),
        })
        .collect::<CliResult<Vec<_>>>()?;
    sources.sort_by_key(|source| source.index);
    timings.mapping_seconds = phase.elapsed().as_secs_f64();
    let staged = journal.root.join("complete.fsd.dat");
    let mut writer = ProgramArchiveWriter::new(
        BufWriter::new(File::create(&staged)?),
        prepared.native.source_identity.clone(),
        family.recipes().iter().copied(),
    )?;
    let mut observations = BTreeMap::new();
    for native in prepared.native.recipes {
        let recipe = native.program_recipe;
        let mut selected = Prepared {
            native,
            provenance: prepared.provenance.clone(),
            generation: prepared.generation.clone(),
            timings: Default::default(),
        };
        selected.generation.workers = run.workers;
        let phase = Instant::now();
        let discovery_jobs = sources
            .iter()
            .map(|source| {
                (
                    format!("{}-discover-{}", recipe.name(), source.index),
                    Request::DiscoverPrepared {
                        preparation: preparation_path.clone(),
                        program_recipe: recipe,
                        source_id: selected.native.source.blake3.clone(),
                        source: source.clone(),
                    },
                )
            })
            .collect();
        let mut charts = runner
            .run(
                &mut journal,
                discovery_jobs,
                GenerationStage::Mapping,
                &format!("Applying {} to shared charts", recipe.name()),
                dashboard,
            )?
            .into_iter()
            .map(|r| match r {
                Response::Discovered(chart) => Ok(chart),
                _ => Err("invalid recipe discovery completion".into()),
            })
            .collect::<CliResult<Vec<_>>>()?;
        charts.sort_by_key(|chart| chart.index);
        selected.timings.mapping_seconds = phase.elapsed().as_secs_f64();
        let compiled = pipeline::run(
            &mut runner,
            &mut journal,
            &preparation_path,
            &mut selected,
            charts,
            card.generation.evaluator,
            dashboard,
        )?;
        // Completed evaluator objects never cross IPC or accumulate here.
        for unit in compiled {
            if dashboard.cancelled() {
                return Err("generation cancelled before publication; use --resume".into());
            }
            let mut data = File::open(unit.data)?;
            for receipt in unit.receipts {
                writer.append_record(recipe, &mut data, receipt)?;
            }
        }
        add_recipe_timings(&mut timings, &selected.timings);
        observations.insert(
            recipe,
            ProgramGeneration {
                generation: selected.generation,
                timings: selected.timings,
            },
        );
    }
    dashboard.generation_saving(runner.elapsed());
    let (mut data, catalogue) = writer.finish()?;
    data.flush()?;
    data.get_ref().sync_all()?;
    drop(data);
    if dashboard.cancelled() {
        return Err("generation cancelled before publication; use --resume".into());
    }
    let selected = catalogue.recipe(default_recipe)?;
    let sector_count = selected.sector_count();
    if let Some(reference) = run
        .reference
        .filter(|_| selected.runtime_parameters.is_empty())
    {
        reference.validate_identity(&selected.content_id)?;
    }
    let mut artifact =
        Artifact::from_program_archive(&staged, catalogue, default_recipe, prepared.provenance)?;
    artifact.reference = run.reference.map(|r| r.settings.clone());
    artifact.relocate_sources(
        path.parent().unwrap_or_else(|| Path::new(".")),
        output.parent().unwrap_or_else(|| Path::new(".")),
    )?;
    timings.total_seconds = runner.elapsed();
    artifact.generation_timings = Some(timings.clone());
    artifact.set_program_generation(observations)?;
    artifact.save_staged(&output)?;
    journal.complete()?;
    let artifact = Artifact::load_metadata(&output)?;
    dashboard.generation(&GenerationSnapshot {
        stage: GenerationStage::Complete,
        completed: sector_count,
        total: Some(sector_count),
        sectors: sector_count,
        kernels: sector_count,
        elapsed_seconds: runner.elapsed(),
        timings,
        coefficient_expansion: None,
        formula_preparation: artifact
            .generation
            .as_ref()
            .and_then(|g| g.formula_preparation),
        detail: format!(
            "Saved {} · {} recipes · all sector processes released",
            crate::artifact::relative_display(&output),
            family.recipes().len()
        ),
    })?;
    journal.cleanup()?;
    Ok(artifact)
}

fn add_recipe_timings(total: &mut GenerationTimings, recipe: &GenerationTimings) {
    total.mapping_seconds += recipe.mapping_seconds;
    total.symmetry_seconds += recipe.symmetry_seconds;
    total.coefficient_expansion_seconds += recipe.coefficient_expansion_seconds;
    if let Some(seconds) = recipe.formula_preparation_seconds {
        *total.formula_preparation_seconds.get_or_insert(0.) += seconds;
    }
}
