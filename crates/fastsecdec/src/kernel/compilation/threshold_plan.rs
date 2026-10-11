//! Detached caller-driven compilation. Preparation writes native local vectors;
//! jobs retain compact provenance and file references, not the global proof.
use super::*;
use crate::{
    generation,
    kernel::{threshold_metadata as m, threshold_owner::StagedTables},
    results::ExactContributionPolicy,
    threshold::{records as record, regularization::BoundContinuation},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};
use symbolica::atom::{AliasedAtom, Atom};
mod worker;
pub use worker::{
    ThresholdCompilationWork, ThresholdPublicationDescriptor, ThresholdPublicationPlan,
    ThresholdWorkReceipt,
};

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ThresholdJobKind {
    /// Explicit zero exact-offset carrier, not a scientific contribution.
    Setup {},
    Exact {
        contributions: Vec<m::ContributionId>,
    },
    Stochastic {
        contribution: m::ContributionId,
    },
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Specification {
    kind: ThresholdJobKind,
    vector: Option<record::StagedVector>,
}
struct Prepared {
    token: Arc<()>,
    root: PathBuf,
    tables: StagedTables,
    specifications: Vec<Specification>,
    precision: PrecisionPolicy,
    settings: CompilationSettings,
    work_record: Option<crate::generation::streaming::RecordRef>,
}
/// A live private continuation authority has issued this detached plan. It is
/// deliberately not Deserialize; ordinary RecordRefs cannot create authority.
pub struct ThresholdCompilationPlan {
    prepared: Arc<Prepared>,
}
pub struct ThresholdCompilationJob {
    prepared: Arc<Prepared>,
    index: usize,
}
pub struct ThresholdCompilationCompletion {
    token: Arc<()>,
    index: usize,
    kind: ThresholdJobKind,
    kernels: KernelSet,
}
impl ThresholdCompilationCompletion {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn kind(&self) -> &ThresholdJobKind {
        &self.kind
    }
    pub fn kernels(&self) -> &KernelSet {
        &self.kernels
    }
    pub fn write_record(
        &self,
        writer: &mut impl std::io::Write,
    ) -> Result<crate::kernel::indexed::RecordReceipt, KernelError> {
        crate::kernel::artifact::indexed::threshold::write_record(
            writer,
            &self.kernels,
            !matches!(self.kind, ThresholdJobKind::Stochastic { .. }),
        )
    }
    pub fn into_kernels(self) -> KernelSet {
        self.kernels
    }
}
fn failure(message: impl std::fmt::Display) -> KernelError {
    KernelError::Artifact(format!("threshold plan: {message}"))
}
impl ThresholdCompilationPlan {
    pub fn prepare(
        bound: &BoundContinuation<'_>,
        staging: &Path,
        maximum: i32,
        precision: PrecisionPolicy,
        settings: CompilationSettings,
    ) -> Result<Self, KernelError> {
        Self::prepare_source(
            crate::threshold::continued::ContinuedSource::Rational(bound),
            staging,
            maximum,
            precision,
            settings,
        )
    }
    /// Compile/prepare a complete fixed regular-section family through the same
    /// native local records and caller-owned jobs. Requires Validated root policy.
    pub fn prepare_family(
        bound: &crate::threshold::regularization::secant::ContinuedFamily<'_>,
        staging: &Path,
        maximum: i32,
        precision: PrecisionPolicy,
        settings: CompilationSettings,
    ) -> Result<Self, KernelError> {
        Self::prepare_source(
            crate::threshold::continued::ContinuedSource::Algebraic(bound),
            staging,
            maximum,
            precision,
            settings,
        )
    }
    fn prepare_source(
        source: crate::threshold::continued::ContinuedSource<'_>,
        staging: &Path,
        maximum: i32,
        precision: PrecisionPolicy,
        settings: CompilationSettings,
    ) -> Result<Self, KernelError> {
        precision.validate()?;
        settings.validate()?;
        if source.options().mode != generation::GenerationMode::Symbolic
            || maximum > source.options().max_order
        {
            return Err(failure("requested continuation mode/range unavailable"));
        }
        if staging.read_dir().map_err(failure)?.next().is_some() {
            return Err(failure("staging directory must be empty"));
        }
        let request = source.request();
        let parent = request.source_identity().map_err(failure)?;
        let mut records = Vec::new();
        let mut exact = BTreeMap::<i32, Atom>::new();
        for chart in 0..source.charts().len() {
            let staged =
                record::write(staging, &parent, source, chart, maximum).map_err(failure)?;
            if matches!(staged.kind, record::VectorKind::Exact {}) {
                let local = record::read(staging, &staged, &parent, u64::MAX).map_err(failure)?;
                for (order, coefficient) in local.coefficients {
                    if !coefficient.get_aliases().is_empty() {
                        return Err(failure("unmaterialized exact aliases"));
                    }
                    *exact.entry(order).or_insert(Atom::Zero) += coefficient.get_root();
                }
            }
            records.push(staged);
        }
        let minimum = records
            .iter()
            .map(|r| r.minimum)
            .min()
            .unwrap_or(maximum)
            .min(maximum.min(0));
        let metadata = super::super::threshold_owner::ThresholdMetadata::from_source(
            source,
            &records,
            (minimum..=maximum).collect(),
        )?
        .into_local()?;
        let exact_ids = metadata
            .lineage()
            .contributions
            .iter()
            .filter(|c| matches!(c.kind, m::ContributionKind::Exact))
            .map(|c| c.id)
            .collect::<Vec<_>>();
        let mut specifications = Vec::new();
        if exact_ids.is_empty() {
            specifications.push(Specification {
                kind: ThresholdJobKind::Setup {},
                vector: None,
            });
        } else {
            for coefficient in exact.values_mut() {
                *coefficient = generation::normalize_exact_coefficient(coefficient);
            }
            let vector = record::write_exact(
                staging,
                &parent,
                &source.coordinates(),
                source.runtime_parameters(),
                &exact,
                maximum,
            )
            .map_err(failure)?;
            specifications.push(Specification {
                kind: ThresholdJobKind::Exact {
                    contributions: exact_ids,
                },
                vector: Some(vector),
            });
        }
        for staged in records {
            if matches!(staged.kind, record::VectorKind::Stochastic {}) {
                specifications.push(Specification {
                    kind: ThresholdJobKind::Stochastic {
                        contribution: m::ContributionId(staged.chart),
                    },
                    vector: Some(staged),
                });
            }
        }
        let tables = metadata.stage_tables(staging)?;
        let mut prepared = Prepared {
            token: Arc::new(()),
            root: staging.into(),
            tables,
            specifications,
            precision,
            settings,
            work_record: None,
        };
        prepared.work_record = Some(worker::write_plan(&prepared)?);
        Ok(Self {
            prepared: Arc::new(prepared),
        })
    }
    pub fn archive_writer<W: std::io::Write + std::io::Seek>(
        &self,
        writer: W,
    ) -> Result<crate::kernel::indexed::ProgramArchiveWriter<W>, KernelError> {
        let summary = self
            .prepared
            .tables
            .publication(self.prepared.settings, &self.prepared.precision)?;
        crate::kernel::indexed::ProgramArchiveWriter::new_threshold(writer, summary)
    }
    pub fn job_count(&self) -> usize {
        self.prepared.specifications.len()
    }
    pub fn contribution_inventory(&self) -> &m::LineageManifestV1 {
        self.prepared.tables.manifest()
    }
    pub fn job(&self, index: usize) -> Result<ThresholdCompilationJob, KernelError> {
        if index >= self.job_count() {
            return Err(failure("job index out of range"));
        }
        Ok(ThresholdCompilationJob {
            prepared: self.prepared.clone(),
            index,
        })
    }
    /// Acceptance checks provenance even if values happen to agree. A future
    /// durable-receipt path additionally checks persisted bytes/native lineage.
    pub fn validate_completion(
        &self,
        completion: &ThresholdCompilationCompletion,
    ) -> Result<(), KernelError> {
        if !Arc::ptr_eq(&self.prepared.token, &completion.token)
            || self
                .prepared
                .specifications
                .get(completion.index)
                .map(|s| &s.kind)
                != Some(&completion.kind)
        {
            return Err(failure("foreign compilation completion"));
        }
        Ok(())
    }
    /// Resident convenience only. A serial writer consumes individual completed
    /// records instead; this method deliberately retains the requested kernels.
    pub fn assemble(
        &self,
        completions: Vec<ThresholdCompilationCompletion>,
    ) -> Result<KernelSet, KernelError> {
        let mut ordered = BTreeMap::new();
        for completion in completions {
            self.validate_completion(&completion)?;
            if ordered.insert(completion.index, completion).is_some() {
                return Err(failure("duplicate compilation completion"));
            }
        }
        if ordered.keys().copied().collect::<BTreeSet<_>>()
            != (0..self.job_count()).collect::<BTreeSet<_>>()
        {
            return Err(failure("incomplete compilation inventory"));
        }
        let mut assembly = crate::kernel::artifact::indexed::assembly::ResidentAssembly::default();
        let mut layout = None;
        for completion in ordered.into_values() {
            let mut kernels = completion.kernels;
            layout = Some((kernels.orders.clone(), kernels.components.clone()));
            // Assembly transports evaluators/exact sums; complete provenance is
            // independently restored from the private plan's local tables below.
            kernels.threshold = None;
            assembly.push(kernels, &[], false)?;
        }
        let (orders, components) = layout.ok_or_else(|| failure("missing setup carrier"))?;
        let mut result = assembly.finish(String::new(), &orders, &components, false)?;
        let owner = self.prepared.tables.load(
            &self.prepared.root,
            m::ResidentSelection::Complete,
            u64::MAX,
        )?;
        result.attach_threshold(owner)?;
        result.initialize_artifact()?;
        Ok(result)
    }
}
impl ThresholdCompilationJob {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn kind(&self) -> &ThresholdJobKind {
        &self.prepared.specifications[self.index].kind
    }
    /// Executes one native compilation synchronously on the caller's thread.
    pub fn run(self) -> Result<ThresholdCompilationCompletion, KernelError> {
        self.run_bounded(u64::MAX)
    }
    fn run_bounded(
        self,
        maximum_bytes: u64,
    ) -> Result<ThresholdCompilationCompletion, KernelError> {
        let prepared = &self.prepared;
        let specification = &prepared.specifications[self.index];
        let orders = prepared.tables.orders();
        let runtime = Arc::new(prepared.tables.runtime_parameters()?);
        let local = specification
            .vector
            .as_ref()
            .map(|r| {
                record::read(
                    &prepared.root,
                    r,
                    prepared.tables.source_identity(),
                    maximum_bytes,
                )
                .map_err(failure)
            })
            .transpose()?;
        if local
            .as_ref()
            .is_some_and(|l| l.runtime_parameters != *runtime)
        {
            return Err(failure("local template runtime schema"));
        }
        let remaining = maximum_bytes
            .checked_sub(specification.vector.as_ref().map_or(0, |r| r.record.bytes))
            .ok_or_else(|| failure("native worker transport limit"))?;
        let coefficients = orders
            .iter()
            .map(|order| {
                local
                    .as_ref()
                    .and_then(|l| l.coefficients.get(order))
                    .cloned()
                    .unwrap_or_else(|| AliasedAtom::from(Atom::Zero))
            })
            .collect::<Vec<_>>();
        let mut exact = vec![Atom::Zero; orders.len()];
        let mut sectors = Vec::new();
        let selection = match &specification.kind {
            ThresholdJobKind::Setup {} | ThresholdJobKind::Exact { .. } => {
                for (sum, value) in exact.iter_mut().zip(coefficients) {
                    if !value.get_aliases().is_empty() {
                        return Err(failure("exact coefficient retains aliases"));
                    }
                    *sum = value.get_root().clone();
                }
                m::ResidentSelection::Selected {
                    stochastic_contributions: Vec::new(),
                    exact_policy: ExactContributionPolicy::IncludeAll,
                }
            }
            ThresholdJobKind::Stochastic { contribution } => {
                let local = local.ok_or_else(|| failure("missing stochastic vector"))?;
                let roots = local.roots;
                let input = program::PreparedCoefficientVector {
                    coordinates: local.coordinates,
                    runtime_parameters: local.runtime_parameters,
                    coefficients,
                    functions: Arc::new(local.functions),
                    endpoint_profiles: local.profiles,
                };
                let completion = roots.enter(53, || {
                    CompilationJob {
                        owner: prepared.token.clone(),
                        program_descriptor: None,
                        request_lookup: None,
                        index: self.index,
                        input: CompilationInput::Prepared(Arc::new(input)),
                        runtime_parameters: runtime.clone(),
                        precision: prepared.precision.clone(),
                        settings: prepared.settings,
                        use_complex: true,
                    }
                    .run()
                })?;
                if !Arc::ptr_eq(&completion.owner, &prepared.token)
                    || completion.index != self.index
                {
                    return Err(failure("native compilation association"));
                }
                sectors.push(completion.sector);
                m::ResidentSelection::Selected {
                    stochastic_contributions: vec![*contribution],
                    exact_policy: ExactContributionPolicy::ExcludeAll,
                }
            }
        };
        let mut kernels = KernelSet::finish(
            orders.to_vec(),
            sectors,
            exact,
            prepared.precision.clone(),
            None,
            true,
            runtime.as_ref().clone(),
            prepared.settings,
        )?;
        kernels.attach_threshold(prepared.tables.load(&prepared.root, selection, remaining)?)?;
        kernels.initialize_artifact()?;
        Ok(ThresholdCompilationCompletion {
            token: prepared.token.clone(),
            index: self.index,
            kind: specification.kind.clone(),
            kernels,
        })
    }
}
