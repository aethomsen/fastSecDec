//! Caller-run admitted threshold preparation. The expensive native owners live
//! only in this call; the returned descriptors contain no global CAD proof.
mod configuration;
mod regular;
pub use regular::{DiscoveryLimits, DiscoveryProgress, RegularFamilyPreparation};
mod family;
mod graph;
pub use family::{prepare_family, resume_family, resume_family_evidence};
pub use graph::{prepare_graph, resume_graph, resume_graph_evidence};
#[cfg(test)]
mod tests;
use super::{
    gcad,
    options::{ThresholdDecompositionOptions, ThresholdStrategy},
    regularization, represented,
};
use crate::{
    generation::GenerationOptions,
    kernel::{
        CompilationSettings, PrecisionPolicy, ThresholdCompilationPlan,
        ThresholdPublicationDescriptor,
    },
    parametric::ParametricIntegrand,
};
use std::{collections::BTreeMap, ops::ControlFlow, path::Path, sync::Arc, time::Instant};
use symbolica::{atom::Symbol, prelude::Rational};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Geometry(#[from] gcad::GcadError),
    #[error(transparent)]
    Evidence(#[from] gcad::staging::Error),
    #[error(transparent)]
    Regularization(#[from] regularization::Error),
    #[error(transparent)]
    RegularFamily(#[from] regularization::secant::Error),
    #[error("regular-family discovery incomplete ({reason}): {progress:?}")]
    DiscoveryIncomplete {
        reason: &'static str,
        progress: DiscoveryProgress,
    },
    #[error(transparent)]
    Kernel(#[from] crate::kernel::KernelError),
    #[error(transparent)]
    Record(#[from] crate::generation::streaming::StreamingError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("unsupported threshold generation: {0}")]
    Unsupported(&'static str),
    #[error("threshold preparation receipt mismatch: {0}")]
    Association(&'static str),
    #[error("threshold preparation cancelled")]
    Cancelled,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Debug)]
pub struct PreparationOptions {
    pub threshold: ThresholdDecompositionOptions,
    pub generation: GenerationOptions,
    pub precision: PrecisionPolicy,
    pub compilation: CompilationSettings,
    pub regularization: regularization::Limits,
    pub unit: Symbol,
    pub regular_family: Option<RegularFamilyPreparation>,
    /// Deliberate certified specialization, never implicit integration bindings.
    pub fixed_fiber: BTreeMap<Symbol, Rational>,
    pub represented: Option<(represented::NumericalMeaning, represented::Limits)>,
    /// Explicit graph primitive interpretation. Requires represented values and
    /// an original GraphPoint; never applies to family/parametric inputs.
    pub graph_payloads: Option<represented::graph::GraphPayloadPolicy>,
}
impl PreparationOptions {
    pub fn new(unit: Symbol) -> Self {
        Self {
            threshold: Default::default(),
            generation: Default::default(),
            precision: Default::default(),
            compilation: Default::default(),
            regularization: Default::default(),
            unit,
            regular_family: None,
            fixed_fiber: BTreeMap::new(),
            represented: None,
            graph_payloads: None,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    Input,
    Solve,
    Verify,
    Regularize,
    Continue,
    Bind,
    StageVectors,
    Complete,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timing {
    pub stage: Stage,
    pub seconds: f64,
}
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceReceipts {
    pub request: Option<gcad::staging::RequestRecord>,
    pub raw: Option<gcad::staging::EvidenceRecord>,
    pub verification: Option<gcad::staging::ObservationRecord>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub stage: Stage,
    /// Input conversion/parameterization detail; absent on legacy input paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_input: Option<represented::graph::Progress>,
    /// Original native family point conversion; absent on all older paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family_input: Option<represented::family::Progress>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regular_family: Option<DiscoveryProgress>,
    pub completed: usize,
    pub total: Option<usize>,
    pub elapsed_seconds: f64,
    pub stage_seconds: f64,
    pub receipts: EvidenceReceipts,
    pub configuration: Option<crate::generation::streaming::RecordRef>,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationReceipt {
    pub configuration: crate::generation::streaming::RecordRef,
    pub work_directory: String,
    pub evidence: EvidenceReceipts,
    pub publication: ThresholdPublicationDescriptor,
    pub strategy: ThresholdStrategy,
    pub original_dimensions: usize,
    pub integration_dimensions: usize,
    pub cells: usize,
    pub endpoint_charts: usize,
    pub jobs: usize,
    pub timings: Vec<Timing>,
}
/// Durable solve output from an interrupted preparer. Untrusted transport;
/// recovery always reconstructs the request and reruns native verification.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparationCheckpoint {
    pub configuration: crate::generation::streaming::RecordRef,
    pub evidence: gcad::staging::EvidenceRecord,
}
impl Progress {
    pub fn checkpoint(&self) -> Option<PreparationCheckpoint> {
        Some(PreparationCheckpoint {
            configuration: self.configuration.clone()?,
            evidence: self.receipts.raw.clone()?,
        })
    }
}
impl PreparationReceipt {
    pub fn checkpoint(&self) -> Result<PreparationCheckpoint> {
        let evidence = self
            .evidence
            .raw
            .clone()
            .ok_or(Error::Association("missing raw evidence"))?;
        if self.evidence.request.as_ref() != Some(&evidence.request) {
            return Err(Error::Association("raw/request receipt"));
        }
        Ok(PreparationCheckpoint {
            configuration: self.configuration.clone(),
            evidence,
        })
    }
}
/// Compact output for a caller-authenticated recyclable preparer child.
/// The receipt can be serialized; it never reconstructs VerifiedDecomposition.
pub struct PreparedThreshold {
    pub receipt: PreparationReceipt,
}
struct Reporter<'a, F> {
    observer: &'a mut F,
    start: Instant,
    stage_start: Instant,
    stage: Stage,
    graph_input: Option<represented::graph::Progress>,
    family_input: Option<represented::family::Progress>,
    regular_family: Option<DiscoveryProgress>,
    configuration: Option<crate::generation::streaming::RecordRef>,
    receipts: EvidenceReceipts,
    timings: Vec<Timing>,
}
impl<F: FnMut(&Progress) -> ControlFlow<()>> Reporter<'_, F> {
    fn stage(&mut self, stage: Stage, completed: usize, total: Option<usize>) -> Result<()> {
        if stage != self.stage {
            self.timings.push(Timing {
                stage: self.stage,
                seconds: self.stage_start.elapsed().as_secs_f64(),
            });
            self.stage = stage;
            if stage != Stage::Input {
                self.graph_input = None;
                self.family_input = None;
            }
            self.stage_start = Instant::now();
        }
        self.poll(completed, total)
    }
    fn poll(&mut self, completed: usize, total: Option<usize>) -> Result<()> {
        if (self.observer)(&Progress {
            stage: self.stage,
            graph_input: self.graph_input,
            family_input: self.family_input,
            regular_family: self.regular_family,
            completed,
            total,
            elapsed_seconds: self.start.elapsed().as_secs_f64(),
            stage_seconds: self.stage_start.elapsed().as_secs_f64(),
            receipts: self.receipts.clone(),
            configuration: self.configuration.clone(),
        })
        .is_break()
        {
            Err(Error::Cancelled)
        } else {
            Ok(())
        }
    }
    fn bridge(&mut self, progress: regularization::Progress) -> ControlFlow<()> {
        let completed = match progress {
            regularization::Progress::Cell(i)
            | regularization::Progress::Chart(i)
            | regularization::Progress::Continue(i)
            | regularization::Progress::PrefactorWitness(i) => i,
            regularization::Progress::Factor { chart, .. } => chart,
        };
        if self.poll(completed, None).is_ok() {
            ControlFlow::Continue(())
        } else {
            ControlFlow::Break(())
        }
    }
}
fn admission(options: &PreparationOptions) -> Result<()> {
    if options.threshold.effective_strategy(&options.generation)? != ThresholdStrategy::GcadFirst {
        return Err(Error::Unsupported(
            "sector-first/source-subset geometry is not yet connected to this factory",
        ));
    }
    if options.threshold.threshold_cells.is_some() {
        return Err(Error::Unsupported(
            "partial cell publication is not admitted by the complete rational-fiber factory",
        ));
    }
    if options.generation.contour_jacobian != crate::contour::ContourJacobian::Symbolic {
        return Err(Error::Unsupported(
            "contour Jacobian choices do not apply to threshold generation",
        ));
    }
    options.precision.validate()?;
    options.compilation.validate()?;
    Ok(())
}
/// Prepare in a fresh directory. Raw evidence is published before verification.
/// Hard process/RSS limits and any invocation in a child belong to the caller.
pub fn prepare(
    input: Arc<ParametricIntegrand>,
    options: PreparationOptions,
    root: &Path,
    mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
) -> Result<PreparedThreshold> {
    if options.graph_payloads.is_some() {
        return Err(Error::Unsupported(
            "graph payload policy requires an original graph input",
        ));
    }
    prepare_with(options, root, &mut observer, |options, reporter| {
        Ok(Arc::new(
            if let Some((meaning, limits)) = options.represented {
                options.threshold.gcad_first_represented_request(
                    input,
                    &options.generation,
                    meaning,
                    limits,
                    |n| {
                        if reporter.poll(n, None).is_ok() {
                            ControlFlow::Continue(())
                        } else {
                            ControlFlow::Break(())
                        }
                    },
                )?
            } else {
                options
                    .threshold
                    .gcad_first_request(&input, &options.generation)?
            },
        ))
    })
}
fn prepare_with<F: FnMut(&Progress) -> ControlFlow<()>>(
    mut options: PreparationOptions,
    root: &Path,
    observer: &mut F,
    request: impl FnOnce(&PreparationOptions, &mut Reporter<'_, F>) -> Result<Arc<gcad::GcadRequest>>,
) -> Result<PreparedThreshold> {
    admission(&options)?;
    if root.read_dir()?.next().is_some() {
        return Err(Error::Association("preparation directory must be empty"));
    }
    let start = Instant::now();
    let mut reporter = Reporter {
        observer,
        start,
        stage_start: start,
        stage: Stage::Input,
        graph_input: None,
        family_input: None,
        regular_family: None,
        configuration: None,
        receipts: Default::default(),
        timings: Vec::new(),
    };
    reporter.poll(0, None)?;
    let request = request(&options, &mut reporter)?;
    regular::resolve(&mut options, &request)?;
    let configuration = configuration::write(root, &options)?;
    reporter.configuration = Some(configuration.clone());
    let staged = gcad::staging::StagedRequest::write(root, request)?;
    reporter.receipts.request = Some(staged.receipt().clone());
    reporter.stage(Stage::Solve, 0, None)?;
    let raw = staged.request().solve()?;
    let evidence = staged.write_evidence(root, &raw)?;
    drop(raw);
    reporter.receipts.raw = Some(evidence.clone());
    reporter.poll(evidence.cells, Some(evidence.cells))?;
    reporter.stage(Stage::Verify, 0, Some(evidence.cells))?;
    let verified = staged.verify_evidence(root, &evidence, u64::MAX)?;
    reporter.receipts.verification = Some(verified.observation);
    reporter.poll(evidence.cells, Some(evidence.cells))?;
    let owner = Arc::new(verified.result?);
    finish(
        root,
        "vectors",
        configuration,
        owner,
        options,
        reporter,
        None,
    )
}
fn finish<F: FnMut(&Progress) -> ControlFlow<()>>(
    root: &Path,
    work_directory: &str,
    configuration: crate::generation::streaming::RecordRef,
    owner: Arc<gcad::VerifiedDecomposition>,
    options: PreparationOptions,
    mut reporter: Reporter<'_, F>,
    expected: Option<(&str, &str)>,
) -> Result<PreparedThreshold> {
    let dimensions = owner.request().input().parameters().len();
    let cells = owner.cells().len();
    reporter.stage(Stage::Regularize, 0, Some(cells))?;
    let integration_dimensions = owner.request().domain().coordinates().len();
    let vectors = root.join(work_directory);
    std::fs::create_dir(&vectors)?;
    let (plan, charts) = if let Some(settings) = &options.regular_family {
        let family = regular::discover(owner, settings, |p| {
            reporter.regular_family = Some(p);
            if reporter.poll(p.family_attempts, None).is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        })?;
        let charts = family.charts().len();
        reporter.stage(Stage::Continue, 0, Some(charts))?;
        let continued = family.continue_symbolically(&options.generation)?;
        reporter.stage(Stage::Bind, charts, Some(charts))?;
        reporter.stage(Stage::StageVectors, 0, Some(charts))?;
        let plan = ThresholdCompilationPlan::prepare_family(
            &continued,
            &vectors,
            options.generation.max_order,
            options.precision,
            options.compilation,
        )?;
        drop(continued);
        drop(family);
        (plan, charts)
    } else {
        let fiber = regularization::RegularizedFiber::admit(
            owner,
            options.fixed_fiber,
            options.unit,
            options.regularization,
            |p| reporter.bridge(p),
        )?;
        let charts = fiber.charts().len();
        reporter.stage(Stage::Continue, 0, Some(charts))?;
        let continued = fiber.continue_symbolically(&options.generation, |p| reporter.bridge(p))?;
        reporter.stage(Stage::Bind, 0, Some(charts))?;
        let bound = continued.bind_fiber(|p| reporter.bridge(p))?;
        reporter.stage(Stage::StageVectors, 0, Some(charts))?;
        let plan = ThresholdCompilationPlan::prepare(
            &bound,
            &vectors,
            options.generation.max_order,
            options.precision,
            options.compilation,
        )?;
        drop(bound);
        drop(continued);
        drop(fiber);
        (plan, charts)
    };
    let publication = plan.publication_descriptor()?;
    let jobs = plan.job_count();
    if let Some((source, prepared)) = expected
        && (publication.source_identity != source || publication.prepared_identity != prepared)
    {
        return Err(Error::Association(
            "reverified preparation changed mathematical identity",
        ));
    }
    drop(plan);
    reporter.stage(Stage::Complete, jobs, Some(jobs))?;
    Ok(PreparedThreshold {
        receipt: PreparationReceipt {
            configuration,
            work_directory: work_directory.into(),
            evidence: reporter.receipts,
            publication,
            strategy: ThresholdStrategy::GcadFirst,
            original_dimensions: dimensions,
            integration_dimensions,
            cells,
            endpoint_charts: charts,
            jobs,
            timings: reporter.timings,
        },
    })
}

/// Reverify existing raw evidence in this caller's process; never call solve.
/// Run in a recyclable preparer child when the coordinator must stay compact.
/// The caller must pin the original source/card/build and expected semantic ID.
pub fn resume(
    root: &Path,
    prior: &PreparationReceipt,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: &str,
    maximum_bytes: u64,
    mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
) -> Result<PreparedThreshold> {
    if prior.publication.source_identity != expected_source
        || prior.publication.prepared_identity != expected_prepared
        || prior.strategy != ThresholdStrategy::GcadFirst
    {
        return Err(Error::Association(
            "resume source, recipe or prepared identity",
        ));
    }
    recover(
        root,
        &prior.checkpoint()?,
        work_directory,
        expected_source,
        Some(expected_prepared),
        maximum_bytes,
        None,
        &mut observer,
    )
}
/// Recover a durable raw solve after interrupted preparation, without solving.
/// No previous completed prepared identity exists here. Once a complete receipt
/// exists, use `resume` to require its semantic identity. The caller authenticates
/// the checkpoint/source/card/build association before invoking this function.
pub fn resume_evidence(
    root: &Path,
    checkpoint: &PreparationCheckpoint,
    work_directory: &str,
    expected_source: &str,
    maximum_bytes: u64,
    mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
) -> Result<PreparedThreshold> {
    recover(
        root,
        checkpoint,
        work_directory,
        expected_source,
        None,
        maximum_bytes,
        None,
        &mut observer,
    )
}
enum OriginalPoint {
    Graph(Arc<represented::graph::GraphPoint>),
    Family(Arc<represented::family::FamilyPoint>),
}
#[allow(clippy::too_many_arguments)]
fn recover<F: FnMut(&Progress) -> ControlFlow<()>>(
    root: &Path,
    checkpoint: &PreparationCheckpoint,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: Option<&str>,
    maximum_bytes: u64,
    original: Option<(OriginalPoint, represented::Limits)>,
    observer: &mut F,
) -> Result<PreparedThreshold> {
    let start = Instant::now();
    let mut reporter = Reporter {
        observer,
        start,
        stage_start: start,
        stage: Stage::Input,
        graph_input: None,
        family_input: None,
        regular_family: None,
        configuration: Some(checkpoint.configuration.clone()),
        receipts: EvidenceReceipts {
            request: Some(checkpoint.evidence.request.clone()),
            raw: Some(checkpoint.evidence.clone()),
            verification: None,
        },
        timings: Vec::new(),
    };
    reporter.poll(0, None)?;
    if !matches!(
        std::path::Path::new(work_directory)
            .components()
            .collect::<Vec<_>>()
            .as_slice(),
        [std::path::Component::Normal(_)]
    ) {
        return Err(Error::Association(
            "resume work directory must be one relative component",
        ));
    }
    let remaining = maximum_bytes
        .checked_sub(checkpoint.configuration.bytes)
        .ok_or(Error::Association("configuration transport limit"))?;
    let mut options = configuration::read(root, &checkpoint.configuration, maximum_bytes)?;
    admission(&options)?;
    let raw = &checkpoint.evidence;
    let request = &raw.request;
    if request.source_identity != expected_source {
        return Err(Error::Association("resume original source"));
    }
    let staged = match original {
        Some((OriginalPoint::Graph(original), conversion_cap)) => {
            graph::admission(&options)?;
            gcad::staging::StagedRequest::read_with_original_graph(
                root,
                request,
                remaining,
                original,
                conversion_cap,
                |p| {
                    reporter.graph_input = Some(p);
                    if reporter.poll(p.converted_literals, None).is_ok() {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
                },
            )
        }
        Some((OriginalPoint::Family(original), conversion_cap)) => {
            family::admission(&options)?;
            gcad::staging::StagedRequest::read_with_original_family(
                root,
                request,
                remaining,
                original,
                conversion_cap,
                |p| {
                    reporter.family_input = Some(p);
                    if reporter.poll(p.converted_literals, None).is_ok() {
                        ControlFlow::Continue(())
                    } else {
                        ControlFlow::Break(())
                    }
                },
            )
        }
        None => gcad::staging::StagedRequest::read(root, request, remaining),
    }
    .map_err(|error| match error {
        gcad::staging::Error::Gcad(gcad::GcadError::Represented(represented::Error::Cancelled)) => {
            Error::Cancelled
        }
        error => Error::Evidence(error),
    })?;
    if options.graph_payloads.is_some() && staged.request().preparametric_graph_input().is_none() {
        return Err(Error::Association(
            "graph payload configuration on a non-graph request",
        ));
    }
    if staged.request().preparametric_graph_input().is_some() {
        graph::request_options(&options, staged.request())?;
    } else if staged.request().preparametric_family_input().is_some() {
        family::request_options(&options, staged.request())?;
    } else if let Some(owner) = staged.request().represented_input() {
        // Legacy v1 configurations did not repeat the parametric represented
        // policy. Recover it only from the reconstructed native request owner;
        // never from a new caller or by treating that request as exact-only.
        let policy = (owner.meaning(), owner.limits());
        if options.represented.is_some_and(|stored| stored != policy) {
            return Err(Error::Association("represented policy/configuration"));
        }
        options.represented = Some(policy);
    }
    if staged.request().domain().coordinates().len() == 1 && options.regular_family.is_some() {
        return Err(Error::Association(
            "regular-family configuration on legacy rational request",
        ));
    }
    regular::resolve(&mut options, staged.request())?;
    reporter.stage(Stage::Verify, 0, Some(raw.cells))?;
    let verified = staged.verify_evidence(root, raw, remaining)?;
    reporter.receipts.verification = Some(verified.observation);
    reporter.poll(raw.cells, Some(raw.cells))?;
    finish(
        root,
        work_directory,
        checkpoint.configuration.clone(),
        Arc::new(verified.result?),
        options,
        reporter,
        expected_prepared.map(|p| (expected_source, p)),
    )
}
