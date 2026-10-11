mod artifact;
mod config;
mod contour_cli;
mod contour_pilot;
mod diagnostics;
mod display;
mod driver;
mod generate;
mod generation_report;
mod input;
mod inspect;
mod integration_report;
mod isolated;
mod loading;
mod math_display;
#[cfg(test)]
mod numerical_dual_benchmark;
mod process;
mod reference;
mod results;
mod serial_cli;
mod status_policy;
mod symanzik_export;
mod terminal_policy;
mod threshold_cli;

use clap::{Args, Parser, Subcommand};
use config::IntegrationInput;
use fastsecdec::AtomCore;
use std::{
    io::IsTerminal,
    path::PathBuf,
    process::{Command, ExitCode},
};
type CliResult<T> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct ReportedFailure;
impl std::fmt::Display for ReportedFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("diagnostic evaluations failed")
    }
}
impl std::error::Error for ReportedFailure {}

#[derive(Parser)]
#[command(
    name = "fastsecdec",
    version,
    about = "Native Feynman-parameter generation and numerical sector integration"
)]
struct Cli {
    #[arg(long, global = true, help = "Write the final report as JSON")]
    json: bool,
    #[arg(
        long,
        global = true,
        help = "Disable the live dashboard and report colors"
    )]
    plain: bool,
    #[arg(
        long,
        global = true,
        help = "Stream status snapshots as JSON lines to stderr"
    )]
    status_json: bool,
    /// Refresh interval for dashboard, plain and JSON status; cancellation polls independently.
    #[arg(long, global = true, default_value_t = 1000)]
    status_interval_ms: u64,
    #[command(subcommand)]
    command: Action,
}

impl Cli {
    fn validate_generation_settings(&self) -> CliResult<()> {
        let input = match &self.command {
            Action::Generate { input, .. } => input,
            Action::Run {
                input, integration, ..
            } if !integration.resume => input,
            _ => return Ok(()),
        };
        let card: config::RunCard = toml::from_str(&std::fs::read_to_string(input)?)?;
        card.generation.evaluator.validate()?;
        if card.generation.evaluator.verbose && (!self.plain || self.json || self.status_json) {
            return Err(
                "generation.evaluator.verbose=true requires --plain and cannot be combined with --json or --status-json; native optimizer logs are human-readable"
                    .into(),
            );
        }
        Ok(())
    }
}

#[derive(Subcommand)]
enum Action {
    /// Export native graph U/F and an exact positive-ray sign problem, without sectors or integration.
    ExportSymanzik {
        input: PathBuf,
        /// New output directory containing symanzik.json and problem.toml.
        #[arg(short, long)]
        output: PathBuf,
        /// Exact rational substitutions in a TOML [parameters] table; floats are rejected.
        #[arg(long)]
        point: Option<PathBuf>,
        /// Explicit inequality in the exported k0,k1,... aliases (repeatable).
        #[arg(long = "kinematic-constraint")]
        constraints: Vec<String>,
    },
    /// Generate portable O2 kernels from a native TOML run card.
    Generate {
        input: PathBuf,
        /// Generate all contour recipes; the saved default remains undeformed.
        #[arg(long, conflicts_with = "recipe")]
        contour: bool,
        /// Resolve real thresholds using verified GCAD cells and symbolic subtraction.
        #[arg(long, conflicts_with = "contour")]
        threshold_decomposition: bool,
        /// Generate one native recipe, such as dynamic-sign-aware-v1.
        #[arg(long, value_parser = parse_program_recipe, conflicts_with = "contour")]
        recipe: Option<fastsecdec::kernel::ProgramRecipe>,
        /// Contour determinant construction, independent of endpoint generation mode.
        #[arg(long, value_parser = parse_contour_jacobian)]
        contour_jacobian: Option<fastsecdec::contour::ContourJacobian>,
        /// Artifact basename, such as output/integral.fsd, without .json or .dat.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Caller-owned workers for geometry, symbolic generation, and compilation.
        #[arg(long = "workers", visible_alias = "geometry-workers", default_value_t = default_generation_workers())]
        geometry_workers: std::num::NonZeroUsize,
        /// Complete, persist and release one sector per worker process.
        #[arg(long)]
        serial: bool,
        /// Reuse verified, durable receipts from an interrupted serial generation.
        #[arg(long)]
        resume: bool,
    },
    /// Generate and integrate a native TOML run card.
    Run {
        input: PathBuf,
        /// Generate a threshold-decomposition recipe before integration.
        #[arg(long)]
        threshold_decomposition: bool,
        /// Artifact basename, such as output/integral.fsd, without .json or .dat.
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Caller-owned generation workers; resumed artifacts need no generation.
        #[arg(long = "generation-workers", visible_alias = "geometry-workers", default_value_t = default_generation_workers())]
        geometry_workers: std::num::NonZeroUsize,
        #[command(flatten)]
        integration: IntegrationArgs,
    },
    /// Integrate a saved portable artifact, optionally resuming a checkpoint.
    Integrate {
        artifact: PathBuf,
        #[command(flatten)]
        integration: IntegrationArgs,
    },
    /// View a saved numerical result without loading graphs or compiled kernels.
    ShowResult {
        path: PathBuf,
        #[command(flatten)]
        view: results::ViewArgs,
    },
    /// Export an explicitly selected estimate or original stored reference.
    ExportReference {
        path: PathBuf,
        #[arg(long, value_enum)]
        source: results::ReferenceSource,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Inspect native input or an existing portable artifact.
    Inspect {
        path: PathBuf,
        /// Select an explicitly stored mathematical recipe (for example fixed-v1).
        #[arg(long, value_parser = parse_program_recipe)]
        recipe: Option<fastsecdec::kernel::indexed::ProgramRecipe>,
        /// Verify artifact integrity and retained metadata; binary checks require --deep.
        #[arg(long)]
        validate_artifact: bool,
        /// Inspect one compiled sector by its zero-based ID from the overview.
        #[arg(long)]
        sector: Option<usize>,
        /// Restore the binary evaluators and inspect all retained metadata (can be expensive).
        #[arg(long)]
        deep: bool,
        /// Show retained expressions; implies --deep and can be expensive.
        #[arg(long)]
        expressions: bool,
    },
    /// Measure repeated kernel evaluations with reproducible interior points.
    Benchmark {
        artifact: PathBuf,
        #[command(flatten)]
        contour: contour_cli::ContourArgs,
        #[arg(long, default_value_t = 100_000)]
        points: usize,
        #[arg(long, default_value_t = 5)]
        repetitions: usize,
    },
    /// Sample bounded coordinate faces from inside, including face intersections.
    CheckBoundaries {
        artifact: PathBuf,
        #[command(flatten)]
        contour: contour_cli::ContourArgs,
        #[arg(long, value_delimiter = ',', default_value = "3,6,9,12,15")]
        exponents: Vec<i32>,
        #[arg(long, default_value_t = 2)]
        max_codimension: usize,
        #[arg(long, default_value_t = 10_000)]
        max_probes: usize,
        /// Sampled growth threshold per approached axis; not an integrability test.
        #[arg(long, default_value_t = 0.5)]
        growth_tolerance: f64,
        /// Decreasing scales in (0,1), each relative to the original distances.
        #[arg(long, value_delimiter = ',')]
        retry_scales: Vec<f64>,
    },
    #[command(name = "__generate-worker", hide = true)]
    GenerationWorker {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        lease_id: u64,
    },
    #[command(name = "__integrate-worker", hide = true)]
    IntegrationWorker {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        lease_id: u64,
    },
    #[command(name = "__setup-worker", hide = true)]
    SetupWorker {
        #[arg(long)]
        run_id: String,
        #[arg(long)]
        lease_id: u64,
    },
}

impl Action {
    fn validate_generation_output(&self) -> CliResult<()> {
        if let Self::Generate { input, output, .. } | Self::Run { input, output, .. } = self {
            let output = output
                .clone()
                .unwrap_or_else(|| input::artifact_path(input));
            artifact::paths(&output)?;
        }
        Ok(())
    }
}

#[derive(Args, Default)]
struct IntegrationArgs {
    #[command(flatten)]
    contour: contour_cli::ContourArgs,
    /// Minimum sampling seconds per resident sector; workers remain parallel.
    #[arg(long = "serial", value_name = "SECONDS")]
    serial_seconds: Option<f64>,
    /// Grow points per refinement; false keeps points fixed and appends replicas.
    #[arg(long, action = clap::ArgAction::Set)]
    double_points: Option<bool>,
    /// Verify saved artifact integrity and retained metadata during loading (can be expensive).
    #[arg(long)]
    validate_artifact: bool,
    /// TOML runtime settings overlay; explicit command-line flags take precedence.
    #[arg(long)]
    integration_settings: Option<PathBuf>,
    /// TOML file with [parameters] containing this integration point.
    #[arg(long)]
    parameters: Option<PathBuf>,
    /// Override one runtime scalar with NAME=VALUE; may be repeated.
    #[arg(long = "parameter", value_name = "NAME=VALUE")]
    parameter: Vec<String>,
    /// Clear a stored/card subset and integrate the complete parent integral.
    #[arg(long, conflicts_with_all = ["sectors", "exact_contributions"])]
    full_integral: bool,
    /// Compiled kernel IDs, comma-separated; use none for an explicit empty subset.
    #[arg(long, requires = "exact_contributions")]
    sectors: Option<String>,
    /// Include or exclude the complete folded exact offset for selected scope.
    #[arg(long, requires = "sectors", value_parser = ["include", "exclude"])]
    exact_contributions: Option<String>,
    #[arg(long)]
    method: Option<String>,
    #[arg(long)]
    points: Option<u64>,
    #[arg(long)]
    shifts: Option<u32>,
    /// Explicit published generating vector; the default remains kuo33002.
    #[arg(long, value_parser = ["kuo33002", "kuo38005", "kuo39101", "hkkn-alpha3"])]
    lattice: Option<String>,
    #[arg(long)]
    seed: Option<u64>,
    #[arg(long)]
    workers: Option<usize>,
    /// Points per evaluator call (default 256); independent of statistical batches.
    #[arg(long, visible_alias = "batch-size")]
    evaluation_batch_size: Option<std::num::NonZeroUsize>,
    #[arg(long)]
    absolute_tolerance: Option<f64>,
    #[arg(long)]
    relative_tolerance: Option<f64>,
    /// Target one epsilon coefficient using the full complex mean and RMS error.
    #[arg(long, allow_hyphen_values = true)]
    target_order: Option<i32>,
    /// Maximum number of successively refined production allocations.
    #[arg(long)]
    max_rounds: Option<usize>,
    #[arg(long)]
    checkpoint: Option<PathBuf>,
    #[arg(long)]
    resume: bool,
    /// Compare with a versioned reference JSON file; relative paths use cwd.
    #[arg(long)]
    reference: Option<PathBuf>,
    /// Save the accepted numerical result as a native versioned document.
    #[arg(long)]
    save_result: Option<PathBuf>,
}

impl IntegrationArgs {
    fn serial_residence(&self, settings: &IntegrationInput) -> CliResult<Option<f64>> {
        let mut seconds = settings.serial_seconds;
        if let Some(path) = &self.integration_settings {
            let overlay = config::read_integration_overlay(path)?;
            if let Some(value) = overlay.get("serial_seconds") {
                seconds = Some(serde_json::from_value(value.clone())?);
            }
        }
        Ok(self.serial_seconds.or(seconds))
    }

    fn apply(&self, settings: &mut IntegrationInput) -> CliResult<()> {
        settings.parameters =
            config::canonical_parameter_names(std::mem::take(&mut settings.parameters))?;
        if let Some(path) = &self.integration_settings {
            settings.apply_overlay(path)?;
        }
        let mut parameter_values = std::collections::BTreeMap::new();
        if let Some(path) = &self.parameters {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Point {
                parameters: std::collections::BTreeMap<String, toml::Value>,
            }
            let point: Point = toml::from_str(&std::fs::read_to_string(path)?)?;
            parameter_values = config::canonical_parameter_names(point.parameters)?;
            settings.parameters.clear();
        }
        for value in &self.parameter {
            let (name, value) = value
                .split_once('=')
                .ok_or("--parameter requires NAME=VALUE")?;
            if name.trim().is_empty() {
                return Err("--parameter requires a nonempty name".into());
            }
            parameter_values.insert(
                fastsecdec::Atom::var(input::symbol(name.trim())?).to_canonical_string(),
                toml::Value::String(value.to_string()),
            );
        }
        let mut supplied = std::collections::BTreeMap::new();
        for (name, value) in parameter_values {
            let symbol = input::symbol(&name)?;
            if supplied
                .insert(symbol, input::value_expression(&value)?)
                .is_some()
            {
                return Err(format!("duplicate runtime parameter {name}").into());
            }
        }
        let supplied = feynkit_model::resolve_scalar_bindings(supplied)?;
        for (symbol, expression) in supplied {
            let value = expression
                .evaluate(&std::collections::HashMap::<fastsecdec::Atom, f64>::new())
                .map_err(|error| {
                    format!(
                        "runtime parameter {} is not a real numeric expression: {error}",
                        fastsecdec::Atom::var(symbol).to_canonical_string()
                    )
                })?;
            if !value.is_finite() {
                return Err("runtime parameter values must be finite".into());
            }
            settings
                .parameters
                .insert(fastsecdec::Atom::var(symbol).to_canonical_string(), value);
        }

        if self.full_integral {
            settings.scope = fastsecdec::results::ResultScope::FullIntegral;
        }
        if let Some(ids) = &self.sectors {
            let sector_ids = if ids == "none" {
                Vec::new()
            } else {
                ids.split(',')
                    .map(str::parse)
                    .collect::<Result<Vec<u64>, _>>()?
            };
            let exact_policy = match self.exact_contributions.as_deref() {
                Some("include") => fastsecdec::results::ExactContributionPolicy::IncludeAll,
                Some("exclude") => fastsecdec::results::ExactContributionPolicy::ExcludeAll,
                _ => {
                    return Err(
                        "sector selection requires an explicit exact-contributions policy".into(),
                    );
                }
            };
            settings.scope = fastsecdec::results::ResultScope::SelectedSectors {
                sector_ids,
                exact_policy,
            };
        }
        if let Some(value) = &self.method {
            settings.method = value.clone();
        }
        if let Some(value) = self.points {
            settings.points = value;
        }
        if let Some(value) = self.shifts {
            settings.shifts = value;
        }
        if let Some(value) = &self.lattice {
            settings.lattice = value.clone();
        }
        if let Some(value) = self.seed {
            settings.seed = value;
        }
        if let Some(value) = self.workers {
            settings.workers = value;
        }
        if let Some(value) = self.evaluation_batch_size {
            settings.evaluation_batch_size = value.get();
        }
        if let Some(value) = self.absolute_tolerance {
            settings.absolute_tolerance = value;
        }
        if let Some(value) = self.relative_tolerance {
            settings.relative_tolerance = value;
        }
        if let Some(order) = self.target_order {
            settings.accuracy_target = fastsecdec::integration::AccuracyTarget::LaurentOrder(order);
        }
        if let Some(rounds) = self.max_rounds {
            settings.max_rounds = Some(rounds);
        }
        if let Some(seconds) = self.serial_seconds {
            settings.serial_seconds = Some(seconds);
        }
        if let Some(double_points) = self.double_points {
            settings.double_points = double_points;
        }
        if settings.evaluation_batch_size == 0 {
            return Err("evaluation_batch_size must be greater than zero".into());
        }
        self.contour.apply_integration(settings)?;
        settings.validate_execution()
    }
}

fn default_generation_workers() -> std::num::NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(std::num::NonZeroUsize::MIN)
}

fn bind_parameters(
    kernels: &mut fastsecdec::kernel::KernelSet,
    settings: &IntegrationInput,
) -> CliResult<std::collections::BTreeMap<fastsecdec::Symbol, f64>> {
    let values = settings
        .parameters
        .iter()
        .map(|(name, value)| Ok((input::symbol(name)?, *value)))
        .collect::<CliResult<std::collections::BTreeMap<_, _>>>()?;
    kernels.set_contour_diagnostics(settings.contour_diagnostics)?;
    kernels.bind_parameters_with_contour(&values, &settings.contour)?;
    kernels.set_stability_settings(&settings.stability)?;
    Ok(values)
}

fn main() -> ExitCode {
    // This documented Symbolica display setting never changes licensing. A
    // fresh process avoids changing global state after threads may exist.
    if std::env::var_os("SYMBOLICA_HIDE_BANNER").is_none() {
        let result = std::env::current_exe().and_then(|executable| {
            let mut command = Command::new(executable);
            command
                .args(std::env::args_os().skip(1))
                .env("SYMBOLICA_HIDE_BANNER", "1");
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                Err::<std::process::ExitStatus, _>(command.exec())
            }
            #[cfg(not(unix))]
            {
                command.status()
            }
        });
        return match result {
            Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
            Err(error) => {
                eprintln!("fastsecdec: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let cli = Cli::parse();
    // Check the public artifact name before run-card/reference I/O, terminal
    // setup or generation. In particular this also covers `run --resume`.
    let preflight = cli
        .command
        .validate_generation_output()
        .and_then(|()| cli.validate_generation_settings());
    let json = cli.json;
    let color =
        terminal_policy::ColorPolicy::for_stream(cli.plain, std::io::stderr().is_terminal());
    match preflight.and_then(|()| run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if error.is::<ReportedFailure>() {
                return ExitCode::FAILURE;
            }
            if json {
                println!(
                    "{}",
                    serde_json::json!({"error":{"message":error.to_string()}})
                );
            } else if color.enabled() {
                eprintln!("\x1b[31;1mfastsecdec\x1b[0m: {error}");
            } else {
                eprintln!("fastsecdec: {error}");
            }
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> CliResult<()> {
    let render_json = cli.json;
    let make_dashboard = || {
        display::Dashboard::with_status_interval(
            !cli.plain && !cli.json,
            cli.status_json,
            cli.status_interval_ms,
        )
    };
    match cli.command {
        Action::Generate {
            input,
            contour,
            threshold_decomposition,
            recipe,
            contour_jacobian,
            output,
            geometry_workers,
            serial,
            resume,
        } => {
            let reference = reference::from_card(&input, None)?;
            let output = output.unwrap_or_else(|| input::artifact_path(&input));
            let mut dashboard = make_dashboard()?;
            let card: config::RunCard = toml::from_str(&std::fs::read_to_string(&input)?)?;
            if serial || card.generation.serial || resume {
                let artifact = generate::serial::generate_with_overrides(
                    &input,
                    &output,
                    &mut dashboard,
                    reference.as_ref(),
                    geometry_workers.get(),
                    resume,
                    config::GenerationOverrides {
                        contour,
                        threshold_decomposition,
                        recipe,
                        contour_jacobian,
                    },
                )?;
                drop(dashboard);
                return generation_report::print_indexed(
                    &output,
                    &artifact,
                    cli.plain,
                    render_json,
                );
            }
            let (artifact, kernels) = generate::generate_with_overrides(
                &input,
                &output,
                &mut dashboard,
                reference.as_ref(),
                geometry_workers.get(),
                config::GenerationOverrides {
                    contour,
                    threshold_decomposition,
                    recipe,
                    contour_jacobian,
                },
            )?;
            if kernels.runtime_parameters().is_empty()
                && let Some(reference) = &reference
            {
                reference.validate_identity(kernels.content_id())?;
            }
            drop(dashboard);
            generation_report::print(&output, &artifact, &kernels, cli.plain, render_json)?;
        }
        Action::Run {
            input,
            threshold_decomposition,
            output,
            geometry_workers,
            integration,
        } => {
            let reference = reference::from_card(&input, integration.reference.as_deref())?;
            let output = output.unwrap_or_else(|| input::artifact_path(&input));
            let mut dashboard = make_dashboard()?;
            let resident = if integration.resume {
                None
            } else {
                let mut card: config::RunCard = toml::from_str(&std::fs::read_to_string(&input)?)?;
                let (mut overrides, mut requested_recipe) =
                    integration.contour.generation_request(
                        &card.integration,
                        integration.integration_settings.as_deref(),
                    )?;
                overrides.threshold_decomposition = threshold_decomposition;
                overrides.apply(&mut card);
                if card.generation.threshold_enabled()
                    && requested_recipe == fastsecdec::kernel::ProgramRecipe::UndeformedV1
                {
                    requested_recipe = fastsecdec::kernel::ProgramRecipe::ThresholdV1;
                }
                card.generation.validate_resident_recipe(requested_recipe)?;
                if card.generation.serial || integration.serial_seconds.is_some() {
                    generate::serial::generate_with_overrides(
                        &input,
                        &output,
                        &mut dashboard,
                        reference.as_ref(),
                        geometry_workers.get(),
                        false,
                        overrides,
                    )?;
                    None
                } else if integration.serial_residence(&card.integration)?.is_some() {
                    dashboard.configure_generation(
                        card.generation.mode,
                        card.generation.coefficient_expansion.method,
                    );
                    isolated::generate(
                        &input,
                        &output,
                        geometry_workers.get(),
                        integration.reference.as_deref(),
                        overrides,
                        &mut dashboard,
                    )?;
                    None
                } else {
                    Some(generate::generate_with_resident_recipe(
                        &input,
                        &output,
                        &mut dashboard,
                        reference.as_ref(),
                        geometry_workers.get(),
                        overrides,
                        Some(requested_recipe),
                    )?)
                }
            };
            integrate_artifact(
                &output,
                &integration,
                dashboard,
                resident,
                integration.resume.then_some(input.as_path()),
                reference,
                cli.plain,
                render_json,
            )?;
        }
        Action::Integrate {
            artifact: path,
            integration,
        } => {
            integrate_artifact(
                &path,
                &integration,
                make_dashboard()?,
                None,
                None,
                None,
                cli.plain,
                render_json,
            )?;
        }
        Action::ShowResult { path, view } => results::show(&path, &view, render_json)?,
        Action::ExportSymanzik {
            input,
            output,
            point,
            constraints,
        } => {
            report(
                &symanzik_export::export(&input, &output, point.as_deref(), &constraints)?,
                render_json,
            )?;
        }
        Action::ExportReference {
            path,
            source,
            output,
        } => {
            results::export_reference(&path, &output, source)?;
            report(
                &serde_json::json!({"reference":artifact::relative_path(&output, std::path::Path::new("."))?}),
                render_json,
            )?;
        }
        Action::Inspect {
            path,
            recipe,
            validate_artifact,
            deep,
            expressions,
            sector,
        } => {
            if path
                .extension()
                .is_some_and(|extension| extension == "toml")
            {
                if validate_artifact {
                    return Err("--validate-artifact requires a generated artifact basename, not a run card".into());
                }
                if sector.is_some() {
                    return Err(
                        "--sector requires a generated artifact basename, not a run card".into(),
                    );
                }
                if recipe.is_some() {
                    return Err("--recipe requires a generated artifact basename".into());
                }
                let loaded = input::load(&path)?;
                let mut value = serde_json::json!({"name":loaded.label,"loops":loaded.loops,"parameters":loaded.propagators,
                    "domain":format!("{:?}",loaded.integrand.domain()),"terms":loaded.integrand.terms().len(),
                    "independent_externals":loaded.independent_externals,"dependent_externals":loaded.dependent_externals});
                if let Some(preparation) = loaded.family_preparation {
                    value["family_preparation"] = serde_json::to_value(preparation)?;
                    value["active_parameters"] = loaded.integrand.parameters().len().into();
                }
                if expressions {
                    value["density"] =
                        serde_json::Value::String(loaded.integrand.density().to_canonical_string());
                }
                report(&value, render_json)?;
            } else {
                inspect::artifact(
                    &path,
                    recipe,
                    fastsecdec::kernel::KernelLoadOptions {
                        validate: validate_artifact,
                    },
                    deep || expressions,
                    expressions,
                    sector,
                    cli.plain,
                    render_json,
                )?;
            }
        }
        Action::Benchmark {
            artifact,
            contour,
            points,
            repetitions,
        } => {
            let (artifact, mut kernels) = contour.load_diagnostics(&artifact)?;
            let mut dashboard = display::Dashboard::new(false, cli.status_json)?;
            let contour = contour.prepare_diagnostics(&artifact, &mut kernels, &mut dashboard)?;
            let benchmark = fastsecdec::diagnostics::benchmark(
                &mut kernels,
                &fastsecdec::diagnostics::BenchmarkOptions {
                    points,
                    repetitions,
                    ..Default::default()
                },
                |progress| diagnostics::observe(&mut dashboard, cli.status_json, progress),
            )?;
            let failed =
                benchmark.stop == fastsecdec::diagnostics::DiagnosticStop::EvaluationFailure;
            let mut result = serde_json::to_value(benchmark)?;
            if let Some(contour) = contour {
                contour.attach(&mut kernels, &mut result, "warmup_and_measurements")?;
            }
            result["loading_seconds"] = artifact.loading_seconds.into();
            result["generation_timings"] = serde_json::to_value(artifact.generation_timings)?;
            report(&result, render_json)?;
            if failed {
                return Err(ReportedFailure.into());
            }
        }
        Action::CheckBoundaries {
            artifact,
            contour,
            exponents,
            max_codimension,
            max_probes,
            growth_tolerance,
            retry_scales,
        } => {
            let (artifact, mut kernels) = contour.load_diagnostics(&artifact)?;
            let mut dashboard = display::Dashboard::new(false, cli.status_json)?;
            let contour = contour.prepare_diagnostics(&artifact, &mut kernels, &mut dashboard)?;
            let boundaries = fastsecdec::diagnostics::scan_boundaries(
                &mut kernels,
                &fastsecdec::diagnostics::BoundaryScanOptions {
                    sampling: fastsecdec::diagnostics::BoundaryOptions {
                        exponents,
                        max_codimension,
                        max_probes,
                        ..Default::default()
                    },
                    growth: fastsecdec::diagnostics::BoundaryGrowthOptions {
                        max_power_per_axis: growth_tolerance,
                        ..Default::default()
                    },
                    retry_scales,
                },
                |progress| diagnostics::observe_scan(&mut dashboard, cli.status_json, progress),
            )?;
            let failed = boundaries.diagnostics.failures;
            let mut result = serde_json::to_value(&boundaries)?;
            if let Some(contour) = contour {
                contour.attach(&mut kernels, &mut result, "boundary_probes")?;
            }
            if render_json {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                diagnostics::display_scan(&boundaries, cli.plain);
            }
            if failed > 0 {
                return Err(ReportedFailure.into());
            }
        }
        Action::GenerationWorker { run_id, lease_id } => {
            process::child::serve(run_id, lease_id, |path: PathBuf, emit| {
                generate::serial::jobs::execute(&path, emit)
                    .map_err(|error| std::io::Error::other(error.to_string()))
            })?;
        }
        Action::IntegrationWorker { run_id, lease_id } => {
            driver::serial::worker(run_id, lease_id)?;
        }
        Action::SetupWorker { run_id, lease_id } => isolated::worker(run_id, lease_id)?,
    }
    Ok(())
}

/// Decide residency from compact metadata before any numerical-sector decoding.
#[allow(clippy::too_many_arguments)]
fn integrate_artifact(
    path: &std::path::Path,
    args: &IntegrationArgs,
    mut dashboard: display::Dashboard,
    resident: Option<(artifact::Artifact, fastsecdec::kernel::KernelSet)>,
    verify_input: Option<&std::path::Path>,
    prepared_reference: Option<reference::PreparedReference>,
    plain: bool,
    json: bool,
) -> CliResult<()> {
    let options = fastsecdec::kernel::KernelLoadOptions {
        validate: args.validate_artifact,
    };
    let (mut artifact, resident) = match resident {
        Some((artifact, kernels)) => (artifact, Some(kernels)),
        None => (
            artifact::Artifact::load_metadata_with_options(path, options)?,
            None,
        ),
    };
    if let Some(input) = verify_input {
        artifact.verify_input_sources(input)?;
    }
    let reference = match prepared_reference {
        Some(value) => Some(value),
        None => reference::prepare(artifact.resolved_reference(), args.reference.as_deref())?,
    };
    let mut settings: IntegrationInput = if let Some(input) = verify_input {
        // Scheduling is excluded from mathematical artifact identity. A run
        // still obeys its current card; checkpoint admission decides which
        // integration changes are compatible with accepted sampling work.
        let card: config::RunCard = toml::from_str(&std::fs::read_to_string(input)?)?;
        card.integration
    } else {
        serde_json::from_value(artifact.provenance.integration.clone())?
    };
    let checkpoint = args
        .checkpoint
        .clone()
        .unwrap_or_else(|| path.with_extension("checkpoint.json"));
    if args.resume {
        settings.restore_historical_policy(&checkpoint)?;
    }
    let contour = args
        .contour
        .resolve(&settings, args.integration_settings.as_deref())?;
    contour_cli::select_program(&mut artifact, &contour)?;
    if let Some(result) = &args.save_result {
        results::check_destination(result, &artifact, path, &checkpoint, reference.as_ref())?;
    }
    let (result, manifest) = if args.serial_residence(&settings)?.is_some() {
        args.apply(&mut settings)?;
        drop(resident);
        serial_cli::integrate(
            path,
            &artifact,
            &mut settings,
            &checkpoint,
            args.resume,
            options,
            reference.as_ref(),
            &mut dashboard,
        )?
    } else {
        let mut kernels = if let Some(kernels) = resident {
            kernels
        } else {
            let (restored, kernels) = loading::load(
                path,
                options,
                artifact.selected_recipe(),
                &mut dashboard,
                |_| Ok(()),
            )?;
            if restored.content_id != artifact.content_id
                || restored.kernel_content_id != artifact.kernel_content_id
            {
                return Err("artifact manifest changed while starting integration; restart with its completed version".into());
            }
            artifact = restored;
            kernels
        };
        // Import the saved native symbol attributes before parameter parsing.
        args.apply(&mut settings)?;
        bind_parameters(&mut kernels, &settings)?;
        let manifest = fastsecdec::results::KernelResultManifest::from_kernels(&kernels);
        settings.scope = manifest.canonical_scope(&settings.scope)?;
        let pilot_started = std::time::Instant::now();
        if kernels.contour_capable()
            && settings.contour.validation.policy != fastsecdec::contour::ContourValidation::Off
        {
            dashboard.begin_loading();
        }
        let pilot = contour_pilot::run(
            &mut kernels,
            &settings.contour,
            settings.seed,
            Some(&settings.scope),
            |pilot| {
                dashboard.loading(&loading::Snapshot {
                    phase: loading::Phase::ContourValidation,
                    completed: Some(pilot.completed),
                    total: Some(pilot.total),
                    elapsed_seconds: pilot_started.elapsed().as_secs_f64(),
                    primary_evaluators: None,
                })?;
                if dashboard.cancelled() {
                    return Err("contour preflight cancelled".into());
                }
                Ok(())
            },
        )?;
        if let Some(reference) = &reference {
            reference.validate_identity(kernels.content_id())?;
        }
        let mut initial_diagnostics = fastsecdec::status::EvaluationDiagnostics::default();
        if settings.contour_diagnostics == fastsecdec::contour::ContourDiagnosticsMode::Aggregate
            && let Some(report) = kernels.take_contour_runtime_report()?
        {
            initial_diagnostics
                .record_contour_runtime(fastsecdec::status::IntegrationStage::Pilot, &report)?;
        }
        let result = driver::integrate_with_pilot(
            &artifact,
            &kernels,
            &settings,
            &checkpoint,
            args.resume,
            &mut dashboard,
            pilot
                .as_ref()
                .map(|report| contour_pilot::provenance(&kernels, settings.seed, report)),
            initial_diagnostics,
        )?;
        (result, manifest)
    };
    drop(dashboard);
    let saved =
        results::assemble_manifest(&artifact, manifest, &settings, &result, reference.as_ref())?;
    if let Some(path) = &args.save_result {
        results::save(path, &saved)?;
    }
    let comparison = reference
        .as_ref()
        .map(|reference| reference.report_saved(&saved))
        .transpose()?;
    integration_report::print(&result, comparison.as_ref(), plain, json)?;
    if result.failed() {
        return Err(ReportedFailure.into());
    }
    Ok(())
}

fn report(value: &serde_json::Value, json: bool) -> CliResult<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("╭─ FastSecDec ─────────────────────────────────────────────────╮");
        if let Some(object) = value.as_object() {
            for (key, value) in object {
                println!(
                    "  {key:20} {}",
                    if let Some(text) = value.as_str() {
                        text.to_owned()
                    } else {
                        value.to_string()
                    }
                );
            }
        } else {
            println!("{}", serde_json::to_string_pretty(value)?);
        }
        println!("╰─────────────────────────────────────────────────────────────╯");
    }
    Ok(())
}

fn parse_program_recipe(value: &str) -> Result<fastsecdec::kernel::indexed::ProgramRecipe, String> {
    serde_json::from_value(serde_json::Value::String(value.to_owned()))
        .map_err(|error| error.to_string())
}

fn parse_contour_jacobian(value: &str) -> Result<fastsecdec::contour::ContourJacobian, String> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| "contour Jacobian must be symbolic or dual".into())
}

#[cfg(test)]
mod jacobian_cli_tests {
    use super::*;

    #[test]
    fn threshold_switch_is_generation_only_and_forwards_from_run() {
        for action in ["generate", "run"] {
            let cli = Cli::try_parse_from([
                "fastsecdec",
                action,
                "input.toml",
                "--threshold-decomposition",
            ])
            .unwrap();
            assert!(matches!(
                cli.command,
                Action::Generate {
                    threshold_decomposition: true,
                    ..
                } | Action::Run {
                    threshold_decomposition: true,
                    ..
                }
            ));
        }
        assert!(
            Cli::try_parse_from([
                "fastsecdec",
                "generate",
                "input.toml",
                "--contour",
                "--threshold-decomposition"
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "fastsecdec",
                "integrate",
                "input.fsd",
                "--threshold-decomposition"
            ])
            .is_err()
        );
    }

    #[test]
    fn contour_jacobian_is_a_generation_only_choice() {
        let cli = Cli::try_parse_from([
            "fastsecdec",
            "generate",
            "input.toml",
            "--contour-jacobian",
            "dual",
        ])
        .unwrap();
        assert!(matches!(
            cli.command,
            Action::Generate {
                contour_jacobian: Some(fastsecdec::contour::ContourJacobian::Dual),
                ..
            }
        ));
        assert!(
            Cli::try_parse_from([
                "fastsecdec",
                "generate",
                "input.toml",
                "--contour-jacobian",
                "auto",
            ])
            .is_err()
        );
        assert!(
            Cli::try_parse_from([
                "fastsecdec",
                "integrate",
                "input.fsd",
                "--contour-jacobian",
                "dual",
            ])
            .is_err()
        );
    }
}
