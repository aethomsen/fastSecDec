//! File-backed synchronous jobs. This module runs only in recyclable children.
mod preparation;
pub(super) mod threshold;
use crate::{
    CliResult,
    artifact::{self, GenerationRecord, Provenance},
    input,
};
use fastsecdec::{
    generation::{self, streaming as native},
    kernel::{
        CompilationSettings, PrecisionPolicy,
        indexed::{self, RecordReceipt},
    },
    status::{GenerationSnapshot, GenerationStage, GenerationTimings},
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    ops::ControlFlow,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Job {
    pub root: PathBuf,
    pub response: PathBuf,
    pub request: Request,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Request {
    PrepareThreshold {
        input: PathBuf,
        workers: usize,
        overrides: crate::config::GenerationOverrides,
        attempt: String,
        prior: Option<Box<threshold::PreparedThreshold>>,
    },
    RetryThreshold {
        input: PathBuf,
        workers: usize,
        overrides: crate::config::GenerationOverrides,
        attempt: String,
        prior: Option<Box<threshold::PreparedThreshold>>,
        retry: Box<threshold::retry::RetryAttempt>,
    },
    CompileThreshold {
        directory: String,
        work: fastsecdec::kernel::ThresholdCompilationWork,
        output: PathBuf,
    },
    Prepare {
        input: PathBuf,
        workers: usize,
        #[serde(default)]
        overrides: crate::config::GenerationOverrides,
    },
    PreparePrograms {
        input: PathBuf,
        workers: usize,
        overrides: crate::config::GenerationOverrides,
        recipes: Vec<indexed::ProgramRecipe>,
    },
    PrepareChartSource {
        preparation: PathBuf,
        source_identity: String,
        dimension: usize,
        map: native::MapJob,
    },
    Discover {
        preparation: PathBuf,
        program_recipe: indexed::ProgramRecipe,
        source_id: String,
        dimension: usize,
        index: usize,
    },
    DiscoverPrepared {
        preparation: PathBuf,
        program_recipe: indexed::ProgramRecipe,
        source_id: String,
        source: native::PreparedChartSource,
    },
    Symmetry {
        preparation: PathBuf,
        chart: native::DiscoveredSector,
        candidates: Vec<native::DiscoveredSector>,
    },
    Formula {
        preparation: PathBuf,
        chart: native::DiscoveredSector,
    },
    Sector {
        job: native::SectorJob,
        max_order: i32,
        evaluator: CompilationSettings,
        output: PathBuf,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Prepared {
    pub native: native::Preparation,
    pub provenance: Provenance,
    pub generation: GenerationRecord,
    pub timings: GenerationTimings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct PreparedPrograms {
    pub native: native::PreparedRecipeSet,
    pub provenance: Provenance,
    pub generation: GenerationRecord,
    pub timings: GenerationTimings,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Compiled {
    pub program_recipe: indexed::ProgramRecipe,
    pub source_id: String,
    pub source_index: usize,
    pub data: PathBuf,
    pub receipts: Vec<RecordReceipt>,
    pub source_chart_modes: std::collections::BTreeMap<usize, generation::GenerationMode>,
    pub compilation_seconds: f64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) enum Response {
    PreparedThreshold(Box<threshold::PreparedThreshold>),
    CompiledThreshold(Box<threshold::CompiledThreshold>),
    Prepared(Box<Prepared>),
    PreparedPrograms(Box<PreparedPrograms>),
    ChartSource(native::PreparedChartSource),
    Discovered(native::DiscoveredSector),
    Symmetry(native::SymmetryAssignment),
    Formula(native::FormulaRecord),
    Compiled(Compiled),
}

pub(crate) fn read_response(path: &Path) -> CliResult<Response> {
    Ok(serde_json::from_reader(File::open(path)?)?)
}

fn read_prepared(path: &Path, recipe: indexed::ProgramRecipe) -> CliResult<Prepared> {
    let prepared: CliResult<Prepared> = match read_response(path)? {
        Response::Prepared(value) => Ok(*value),
        Response::PreparedPrograms(value) => {
            let native = value
                .native
                .recipes
                .into_iter()
                .find(|candidate| candidate.program_recipe == recipe)
                .ok_or("requested recipe is absent from prepared source")?;
            Ok(Prepared {
                native,
                provenance: value.provenance,
                generation: value.generation,
                timings: value.timings,
            })
        }
        _ => Err("generation preparation receipt has the wrong job type".into()),
    };
    let prepared = prepared?;
    if prepared.native.program_recipe != recipe {
        return Err("generation preparation belongs to a different recipe".into());
    }
    Ok(prepared)
}

fn read_programs(path: &Path) -> CliResult<PreparedPrograms> {
    match read_response(path)? {
        Response::PreparedPrograms(value) => Ok(*value),
        _ => Err("generation recipe-set receipt has the wrong job type".into()),
    }
}

pub(crate) fn execute(
    path: &Path,
    emit: &mut dyn FnMut(GenerationSnapshot) -> std::io::Result<()>,
) -> CliResult<()> {
    let job: Job = serde_json::from_reader(File::open(path)?)?;
    fs::create_dir_all(&job.root)?;
    let started = Instant::now();
    let mut snapshot = GenerationSnapshot {
        stage: GenerationStage::Input,
        completed: 0,
        total: None,
        sectors: 0,
        kernels: 0,
        elapsed_seconds: 0.,
        timings: Default::default(),
        coefficient_expansion: None,
        formula_preparation: None,
        detail: String::new(),
    };
    // A native subtraction callback also serves as a cancellation poll and can
    // run far more frequently than a dashboard refresh. Coalesce those updates
    // before serializing so bounded control-channel backpressure cannot turn
    // algebra into a wait for the coordinator to consume identical snapshots.
    // The separate parent watchdog remains active between displayed updates.
    let mut progress_gate = ProgressGate::default();
    // Progress is bounded metadata. It never contains a native sector object.
    let mut observe = |order, progress: &generation::GenerationProgress| {
        snapshot.observe_generation(order, progress);
        let elapsed = started.elapsed();
        snapshot.elapsed_seconds = elapsed.as_secs_f64();
        if !progress_gate.accept(snapshot.stage, elapsed) {
            return ControlFlow::Continue(());
        }
        snapshot.detail = snapshot.detail.chars().take(4096).collect();
        if emit(snapshot.clone()).is_err() {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let mut response = match job.request {
        Request::RetryThreshold {
            input,
            workers,
            overrides,
            attempt,
            prior,
            retry,
        } => threshold::prepare_with_retry(
            &job.root,
            &input,
            workers,
            overrides,
            &attempt,
            prior.as_deref(),
            Some(&retry),
            emit,
        )?,
        Request::PrepareThreshold {
            input,
            workers,
            overrides,
            attempt,
            prior,
        } => threshold::prepare(
            &job.root,
            &input,
            workers,
            overrides,
            &attempt,
            prior.as_deref(),
            emit,
        )?,
        Request::CompileThreshold {
            directory,
            work,
            output,
        } => threshold::compile(&job.root, &directory, work, &output, emit)?,
        Request::Prepare {
            input,
            workers,
            overrides,
        } => preparation::run(&job.root, &input, workers, overrides, None, &mut observe)?,
        Request::PreparePrograms {
            input,
            workers,
            overrides,
            recipes,
        } => preparation::run(
            &job.root,
            &input,
            workers,
            overrides,
            Some(&recipes),
            &mut observe,
        )?,
        Request::PrepareChartSource {
            preparation,
            source_identity,
            dimension,
            map,
        } => {
            let prepared = read_programs(&preparation)?;
            if prepared.native.source_identity != source_identity
                || prepared
                    .native
                    .recipes
                    .iter()
                    .any(|recipe| recipe.dimension != dimension)
            {
                return Err(
                    "chart-source request differs from its prepared physical source".into(),
                );
            }
            Response::ChartSource(native::prepare_chart_source(
                &job.root,
                &prepared.native,
                &map,
                |progress| observe(prepared.provenance.max_order, progress),
            )?)
        }
        Request::DiscoverPrepared {
            preparation,
            program_recipe,
            source_id,
            source,
        } => {
            let prepared = read_prepared(&preparation, program_recipe)?;
            if prepared.native.source.blake3 != source_id {
                return Err(
                    "recipe discovery request differs from its prepared execution source".into(),
                );
            }
            Response::Discovered(native::discover_prepared(
                &job.root,
                &prepared.native,
                &source,
                |progress| observe(prepared.provenance.max_order, progress),
            )?)
        }
        Request::Discover {
            preparation,
            program_recipe,
            source_id,
            dimension,
            index,
        } => {
            let prepared = read_prepared(&preparation, program_recipe)?;
            if program_recipe != prepared.native.program_recipe
                || source_id != prepared.native.source.blake3
                || dimension != prepared.native.dimension
            {
                return Err("discovery request differs from its prepared source or recipe".into());
            }
            let map = prepared
                .native
                .charts
                .get(index)
                .ok_or("unknown discovery chart")?;
            Response::Discovered(native::discover(&job.root, &prepared.native, map, |p| {
                observe(prepared.provenance.max_order, p)
            })?)
        }
        Request::Symmetry {
            preparation,
            chart,
            candidates,
        } => {
            let prepared = read_prepared(&preparation, chart.program_recipe)?;
            Response::Symmetry(native::compare_symmetry(
                &job.root,
                &prepared.native,
                &chart,
                &candidates,
                |p| observe(prepared.provenance.max_order, p),
            )?)
        }
        Request::Formula { preparation, chart } => {
            let prepared = read_prepared(&preparation, chart.program_recipe)?;
            Response::Formula(native::build_formula(
                &job.root,
                &prepared.native,
                &chart,
                |p| observe(prepared.provenance.max_order, p),
            )?)
        }
        Request::Sector {
            job: sector,
            max_order,
            evaluator,
            output,
        } => {
            let unit = native::generate_sector(&job.root, &sector, |p| observe(max_order, p))?;
            let modes = unit
                .generated
                .metadata()
                .charts()
                .iter()
                .map(|chart| {
                    let mode = chart
                        .kernel_sector()
                        .map_or(generation::GenerationMode::Symbolic, |id| {
                            unit.generated.sectors()[id].generation_mode()
                        });
                    (unit.source_indices[chart.source_index()], mode)
                })
                .collect();
            let compile_started = Instant::now();
            let kernels = unit
                .generated
                .compile_with_settings_parameters_and_dispatch(
                    PrecisionPolicy::default(),
                    &unit.runtime_parameters,
                    evaluator,
                    &mut |jobs| jobs.map(|job| job.run()).collect(),
                    |progress| {
                        snapshot.observe_compilation(progress);
                        snapshot.elapsed_seconds = started.elapsed().as_secs_f64();
                        if emit(snapshot.clone()).is_err() {
                            ControlFlow::Break(())
                        } else {
                            ControlFlow::Continue(())
                        }
                    },
                )?
                .with_runtime_mass_constraints(unit.runtime_mass_constraints)?;
            let compilation_seconds = compile_started.elapsed().as_secs_f64();
            drop(unit.generated);
            let temporary = output.with_extension("writing");
            let result = (|| -> CliResult<Vec<RecordReceipt>> {
                let mut data = BufWriter::new(File::create(&temporary)?);
                let receipts = indexed::write_unit(&mut data, &kernels, unit.source_indices)?;
                data.flush()?;
                data.get_ref().sync_all()?;
                drop(data);
                fs::rename(&temporary, &output)?;
                File::open(output.parent().ok_or("sector output has no directory")?)?.sync_all()?;
                Ok(receipts)
            })();
            if result.is_err() {
                let _ = fs::remove_file(&temporary);
            }
            Response::Compiled(Compiled {
                program_recipe: sector.program_recipe,
                source_id: sector.source.blake3.clone(),
                source_index: sector.index,
                data: output,
                receipts: result?,
                source_chart_modes: modes,
                compilation_seconds,
            })
        }
    };
    let timings = match &mut response {
        Response::Prepared(value) => Some(&mut value.timings),
        Response::PreparedPrograms(value) => Some(&mut value.timings),
        _ => None,
    };
    if let Some(timings) = timings {
        *timings = GenerationTimings {
            input_seconds: timings.input_seconds,
            parametrization_seconds: timings.parametrization_seconds,
            ..snapshot.timings
        };
    }
    // A receipt is visible only after every heavy output it references is durable.
    artifact::atomic_write(&job.response, &serde_json::to_vec(&response)?)?;
    File::open(job.response.parent().ok_or("receipt has no directory")?)?.sync_all()?;
    Ok(())
}

#[derive(Default)]
struct ProgressGate {
    last: Option<(GenerationStage, Duration)>,
}

impl ProgressGate {
    fn accept(&mut self, stage: GenerationStage, now: Duration) -> bool {
        if let Some((previous_stage, previous_time)) = self.last
            && previous_stage == stage
            && now.saturating_sub(previous_time) < Duration::from_millis(50)
        {
            return false;
        }
        self.last = Some((stage, now));
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequent_native_polls_are_coalesced_but_stage_changes_are_immediate() {
        let mut gate = ProgressGate::default();
        let now = Duration::from_secs(1);
        assert!(gate.accept(GenerationStage::Mapping, now));
        for elapsed in 0..50 {
            assert!(!gate.accept(
                GenerationStage::Mapping,
                now + Duration::from_millis(elapsed)
            ));
        }
        assert!(gate.accept(GenerationStage::Mapping, now + Duration::from_millis(50)));
        assert!(gate.accept(
            GenerationStage::FormulaPreparation,
            now + Duration::from_millis(51)
        ));
    }
}
