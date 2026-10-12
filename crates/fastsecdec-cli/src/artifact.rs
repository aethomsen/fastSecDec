mod indexed;
pub(crate) mod inspection;
mod programs;
pub use indexed::IndexedStorage;
pub use inspection::{InspectionIndex, KernelSummary};
pub use programs::{ProgramGeneration, ProgramStorage};

use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use fastsecdec::kernel::{KernelLoadOptions, KernelSet};
use fastsecdec::status::GenerationTimings;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::CliResult;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub revision: String,
    pub source_state: String,
}

pub fn dependencies() -> Vec<Dependency> {
    let symjit = format!(
        "native-version-code:{}",
        fastsecdec::kernel::symjit_version_code()
    );
    [
        (
            "feynkit",
            env!("FASTSECDEC_FEYNKIT_REVISION"),
            env!("FASTSECDEC_FEYNKIT_STATE"),
        ),
        (
            "symbolica",
            env!("FASTSECDEC_SYMBOLICA_REVISION"),
            env!("FASTSECDEC_SYMBOLICA_STATE"),
        ),
        (
            "numerica",
            env!("FASTSECDEC_NUMERICA_REVISION"),
            env!("FASTSECDEC_NUMERICA_STATE"),
        ),
        (
            "symjit",
            symjit.as_str(),
            "linked SymJIT Application::measure(version)",
        ),
    ]
    .into_iter()
    .map(|(name, revision, source_state)| Dependency {
        name: name.into(),
        revision: revision.into(),
        source_state: source_state.into(),
    })
    .collect()
}

fn dependency_matches(recorded: &Dependency, current: &Dependency) -> bool {
    recorded == current
        // Old artifacts stored exact Symbolica IR, never SymJIT machine code.
        // Keep their existing provenance/identity only for the same reported
        // backend; all other dependency identities remain exact comparisons.
        || (recorded.name == "symjit"
            && current.name == "symjit"
            && recorded.revision == "2.26.4"
            && recorded.source_state == "published Rust crate"
            && current.revision == "native-version-code:22604"
            && current.source_state == "linked SymJIT Application::measure(version)")
}

fn dependencies_match(recorded: &[Dependency], current: &[Dependency]) -> bool {
    recorded.len() == current.len()
        && recorded
            .iter()
            .zip(current)
            .all(|(old, new)| dependency_matches(old, new))
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceFile {
    pub path: String,
    pub blake3: String,
    #[serde(default, skip_serializing_if = "SourceFingerprint::is_bytes")]
    pub fingerprint: SourceFingerprint,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceFingerprint {
    #[default]
    Bytes,
    RunCardWithoutReference,
    /// Mathematical input only; scheduling/integration steering is observational.
    RunCardScientificInput,
}

impl SourceFingerprint {
    fn is_bytes(&self) -> bool {
        matches!(self, Self::Bytes)
    }

    pub fn hash(self, bytes: &[u8]) -> CliResult<String> {
        let hash = match self {
            Self::Bytes => blake3::hash(bytes),
            Self::RunCardWithoutReference => {
                // Preserve every parsed root field except observational reference
                // steering, including fields unknown to this application's schema.
                let mut card: toml::Table = toml::from_str(std::str::from_utf8(bytes)?)?;
                card.remove("reference");
                blake3::hash(toml::to_string(&card)?.as_bytes())
            }
            Self::RunCardScientificInput => {
                let mut card: toml::Table = toml::from_str(std::str::from_utf8(bytes)?)?;
                card.remove("reference");
                card.remove("integration");
                if let Some(generation) = card
                    .get_mut("generation")
                    .and_then(toml::Value::as_table_mut)
                {
                    generation.remove("serial");
                    generation.remove("workers");
                    if generation.is_empty() {
                        card.remove("generation");
                    }
                }
                blake3::hash(toml::to_string(&card)?.as_bytes())
            }
        };
        Ok(hash.to_hex().to_string())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub name: String,
    pub sources: Vec<SourceFile>,
    pub dependencies: Vec<Dependency>,
    pub domain: String,
    pub assume_no_threshold: bool,
    pub dimension: String,
    pub regulator: String,
    pub measure: String,
    pub measure_multiplier: String,
    pub max_order: i32,
    pub integration: serde_json::Value,
    /// Native preparation of the original graph family, when explicitly requested.
    /// Omission preserves historical/original artifact identities.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family_preparation: Option<fastsecdec::parametric::FamilyPreparationReport>,
    /// Suggested model point for humans; never automatically bound to kernels.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub model_parameter_defaults: BTreeMap<String, f64>,
}

/// Observed generation configuration, independent of mathematical identity.
/// Kernel layout, evaluator backends and timings are retained by their existing
/// artifact fields; absence here means the older producer did not record it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GenerationRecord {
    pub workers: usize,
    /// Historical retry lineage/effective resources, outside mathematical identity.
    /// These record identities do not promise retained/loadable proof files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_retry: Option<fastsecdec::threshold::generation::EndpointRetryLineage>,
    /// Requested native generation lane; absent on older artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<fastsecdec::generation::GenerationMode>,
    /// Requested endpoint subtraction strategy; absent on older artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtraction: Option<fastsecdec::generation::SubtractionStrategy>,
    /// Actual route for retained source charts, before exact-sector folding or kernel renumbering.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_chart_modes: Option<BTreeMap<usize, fastsecdec::generation::GenerationMode>>,
    /// Observed unique subtraction-formula preparation, excluded from identity.
    /// Missing on historical artifacts and runs without this phase.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula_preparation: Option<fastsecdec::status::FormulaPreparationSnapshot>,
    /// Graph-numerator policy; absent for direct inputs and historical artifacts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contraction_mode: Option<fastsecdec::input::NumeratorContraction>,
    /// The requested route can use its supported per-sector physical fallback;
    /// this field does not claim every sector followed the named-series route.
    pub requested_coefficient_expansion: fastsecdec::generation::CoefficientExpansionMethod,
    /// Actual native evaluator settings. Missing on artifacts from older producers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluator: Option<fastsecdec::kernel::CompilationSettings>,
}

/// Checks completed by this loader invocation, never claims saved by a producer.
#[derive(Default)]
pub struct ArtifactValidation {
    pub metadata_identity: bool,
    pub binary: bool,
}

#[derive(Serialize, Deserialize)]
pub struct Artifact {
    format_version: u32,
    pub content_id: String,
    pub provenance: Provenance,
    pub kernel_content_id: String,
    /// Human-readable kernel layout; executable expressions live in the sibling data file.
    kernel: Box<RawValue>,
    #[serde(skip)]
    data: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub indexed: Option<IndexedStorage>,
    /// Recipe directory; mutually exclusive with the legacy single catalogue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub programs: Option<ProgramStorage>,
    #[serde(skip)]
    selected_recipe: Option<fastsecdec::kernel::indexed::ProgramRecipe>,
    #[serde(skip)]
    staged_data: Option<PathBuf>,
    #[serde(skip)]
    source_root: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation_timings: Option<GenerationTimings>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<GenerationRecord>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<crate::config::ReferenceInput>,
    #[serde(skip)]
    pub loading_seconds: f64,
    #[serde(skip)]
    pub validation: ArtifactValidation,
    /// Optional bounded display observations; excluded from scientific identity.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "inspection::deserialize_index"
    )]
    pub inspection: Option<InspectionIndex>,
}

/// Validate the public basename and derive its two sibling paths without I/O.
/// Call before loading generation inputs or initializing workers and terminal
/// state. Persistence uses this same validation; neither sibling stores a path
/// to the other.
pub fn paths(base: &Path) -> CliResult<(PathBuf, PathBuf)> {
    let name = base
        .file_name()
        .ok_or("artifact basename requires a file name")?;
    let name = name.to_string_lossy();
    if name.ends_with(".json") || name.ends_with(".dat") {
        return Err("specify the artifact basename (for example output/integral.fsd), without .json or .dat".into());
    }
    Ok((
        base.with_file_name(format!("{name}.json")),
        base.with_file_name(format!("{name}.dat")),
    ))
}

/// Normalize a path relative to a caller-selected directory without requiring
/// the destination to exist. Absolute paths are only transient local values.
pub fn relative_path(path: &Path, base: &Path) -> CliResult<PathBuf> {
    fn normalized(path: &Path) -> CliResult<PathBuf> {
        let path = if path.as_os_str().is_empty() {
            Path::new(".")
        } else {
            path
        };
        let full = std::path::absolute(path)?;
        if let Ok(existing) = fs::canonicalize(&full) {
            return Ok(existing);
        }
        // Keep symlink resolution for the existing prefix. The suffix may name
        // a future output file or directory and is normalized only afterward.
        let (ancestor, suffix) = full
            .ancestors()
            .find_map(|ancestor| {
                fs::canonicalize(ancestor).ok().map(|resolved| {
                    (
                        resolved,
                        full.strip_prefix(ancestor).expect("path ancestor"),
                    )
                })
            })
            .ok_or("path has no existing ancestor")?;
        let mut clean = ancestor;
        for part in suffix.components() {
            match part {
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    clean.pop();
                }
                other => clean.push(other.as_os_str()),
            }
        }
        Ok(clean)
    }
    let path = normalized(path)?;
    let base = normalized(base)?;
    let a = path.components().collect::<Vec<_>>();
    let b = base.components().collect::<Vec<_>>();
    let common = a.iter().zip(&b).take_while(|(a, b)| a == b).count();
    if common == 0 {
        return Err("path and destination cannot be represented relatively".into());
    }
    let mut result = PathBuf::new();
    for _ in common..b.len() {
        result.push("..");
    }
    for part in &a[common..] {
        result.push(part.as_os_str());
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Ok(result)
}

pub fn relative_display(path: &Path) -> String {
    relative_path(path, Path::new("."))
        .unwrap_or_else(|_| PathBuf::from(path.file_name().unwrap_or_default()))
        .display()
        .to_string()
}

#[derive(Clone, Debug)]
pub enum ArtifactLoadProgress {
    Metadata,
    ReadingBinary {
        completed: usize,
        total: Option<usize>,
    },
    Native(fastsecdec::kernel::KernelLoadProgress),
}

impl Artifact {
    #[cfg(test)]
    pub fn new(kernels: &KernelSet, provenance: Provenance) -> CliResult<Self> {
        let (data, catalogue) = fastsecdec::kernel::indexed::to_bytes(kernels)?;
        let mut result = Self {
            format_version: 4,
            content_id: String::new(),
            provenance,
            kernel_content_id: catalogue.content_id.clone(),
            kernel: RawValue::from_string(serde_json::to_string_pretty(&serde_json::json!({
                "threshold_policy": "user_responsible", "threshold_certification": "not_performed",
                "orders": kernels.orders(), "components": kernels.components(),
                "sectors": kernels.sectors().len(),
                "dimensions": kernels.sectors().iter().map(|s| s.dimension()).collect::<Vec<_>>(),
                "runtime_parameters": kernels.runtime_parameters().iter().map(|p| p.get_name().to_owned()).collect::<Vec<_>>(),
                "runtime_mass_constraints": kernels.runtime_mass_constraints().iter().map(|constraint| serde_json::json!({
                    "name": constraint.name,
                    "requirement": "finite real nonzero; zero-mass specialization requires regeneration",
                })).collect::<Vec<_>>(),
                "evaluator_statistics": kernels.sectors().iter().map(|s| s.statistics()).collect::<Vec<_>>(),
            }))?)?,
            data,
            indexed: Some(IndexedStorage {
                data_file: String::new(),
                catalogue,
            }),
            programs: None,
            selected_recipe: None,
            staged_data: None,
            source_root: PathBuf::from("."),
            generation_timings: None,
            generation: None,
            reference: None,
            loading_seconds: 0.0,
            validation: ArtifactValidation::default(),
            inspection: Some(InspectionIndex::from_kernels(kernels)),
        };
        result.content_id = result.identity()?;
        Ok(result)
    }
    fn identity(&self) -> CliResult<String> {
        // Generation observations, wall timings and comparison settings do not
        // change the mathematical artifact or any evaluator's identity.
        let mut hasher = blake3::Hasher::new();
        if self.format_version >= 4 {
            hasher.update(b"fastsecdec-artifact-v4");
            let mut provenance = serde_json::to_value(&self.provenance)?;
            provenance
                .as_object_mut()
                .expect("provenance object")
                .remove("integration");
            if let Some(sources) = provenance
                .get_mut("sources")
                .and_then(serde_json::Value::as_array_mut)
            {
                for source in sources {
                    source
                        .as_object_mut()
                        .expect("source object")
                        .remove("path");
                }
            }
            serde_json::to_writer(&mut hasher, &provenance)?;
        } else {
            hasher.update(b"fastsecdec-artifact-v3");
            serde_json::to_writer(&mut hasher, &self.provenance)?;
        }
        // v4 distinguishes the scientific input from its saved representation.
        // Checkpoint/session admission must bind BOTH identities: differently
        // scheduled generation may emit different local coefficient layouts.
        if self.format_version < 4 {
            hasher.update(self.kernel_content_id.as_bytes());
            serde_json::to_writer(&mut hasher, &self.kernel)?;
        }
        Ok(hasher.finalize().to_hex().to_string())
    }
    /// Input paths enter relative to the run card and leave relative to the
    /// artifact pair; copying a directory tree preserves the provenance links.
    pub fn relocate_sources(
        &mut self,
        source_root: &Path,
        destination_root: &Path,
    ) -> CliResult<()> {
        for source in &mut self.provenance.sources {
            source.path = relative_path(&source_root.join(&source.path), destination_root)?
                .to_string_lossy()
                .into_owned();
        }
        if let Some(reference) = &mut self.reference {
            // Prepared references have been resolved against the caller's cwd.
            reference.path = relative_path(&reference.path, destination_root)?;
        }
        self.source_root = destination_root.to_path_buf();
        self.content_id = self.identity()?;
        Ok(())
    }
    pub fn source_paths(&self) -> impl Iterator<Item = PathBuf> + '_ {
        self.provenance
            .sources
            .iter()
            .map(|source| self.source_root.join(&source.path))
    }
    pub fn resolved_reference(&self) -> Option<crate::config::ReferenceInput> {
        self.reference.clone().map(|mut reference| {
            reference.path = self.source_root.join(reference.path);
            reference
        })
    }
    pub fn save(&self, base: &Path) -> CliResult<()> {
        let (metadata, data) = paths(base)?;
        if self
            .provenance
            .sources
            .iter()
            .any(|s| Path::new(&s.path).is_absolute())
            || self
                .reference
                .as_ref()
                .is_some_and(|r| r.path.is_absolute())
        {
            return Err("artifact provenance paths must be relative".into());
        }
        if self.is_indexed() {
            return self.save_indexed(base);
        }
        // The metadata commits the pair only after the complete data is durable.
        // A interrupted overwrite can leave a mismatched pair, which loading rejects.
        atomic_write(&data, &self.data)?;
        atomic_write_with(&metadata, |writer| {
            serde_json::to_writer_pretty(writer, self)?;
            Ok(())
        })
    }
    pub fn kernel_content_id(&self) -> CliResult<&str> {
        if self.kernel_content_id.is_empty() {
            return Err("artifact has no kernel content identity".into());
        }
        Ok(&self.kernel_content_id)
    }
    pub fn verify_input_sources(&self, input: &Path) -> CliResult<()> {
        let first = self
            .provenance
            .sources
            .first()
            .ok_or("artifact has no source provenance")?;
        // The caller can relocate the run card and its input directory together.
        let original_card = self.source_root.join(&first.path);
        let original_directory = original_card.parent().unwrap_or_else(|| Path::new("."));
        let supplied_directory = input.parent().unwrap_or_else(|| Path::new("."));
        for (index, source) in self.provenance.sources.iter().enumerate() {
            let path = if index == 0 {
                input.to_path_buf()
            } else {
                supplied_directory.join(relative_path(
                    &self.source_root.join(&source.path),
                    original_directory,
                )?)
            };
            if source.fingerprint.hash(&fs::read(path)?)? != source.blake3 {
                return Err(format!(
                    "input source {} changed since generation; checkpoint resume refused",
                    source.path
                )
                .into());
            }
        }
        Ok(())
    }
    /// Read the human summary only. Never opens the data sibling,
    /// initializes native evaluators, or requires a compatible native backend.
    /// The optional inspection index and generation observations are not hashed;
    /// this does not validate them against the unread binary.
    pub fn load_metadata(base: &Path) -> CliResult<Self> {
        Self::load_metadata_with_options(base, KernelLoadOptions::default())
    }
    pub fn load_metadata_with_options(base: &Path, options: KernelLoadOptions) -> CliResult<Self> {
        let started = Instant::now();
        let (metadata, _) = paths(base)?;
        let mut artifact: Self = serde_json::from_reader(BufReader::new(File::open(metadata)?))?;
        if !matches!(artifact.format_version, 3..=5) {
            return Err(
                "artifact version is unsupported; regenerate using an .fsd basename".into(),
            );
        }
        if artifact.format_version == 4 {
            if artifact.programs.is_some() {
                return Err("version-four artifact contains a recipe directory".into());
            }
            let indexed = artifact
                .indexed
                .as_ref()
                .ok_or("indexed artifact has no catalogue")?;
            indexed.validate(options.validate)?;
            if indexed.catalogue.content_id != artifact.kernel_content_id {
                return Err("indexed catalogue differs from kernel identity".into());
            }
            let summary = artifact.kernel_summary()?;
            let sectors = indexed
                .catalogue
                .records
                .iter()
                .filter(|r| r.sector.is_some())
                .collect::<Vec<_>>();
            if summary.orders != indexed.catalogue.orders
                || summary.components != indexed.catalogue.components
                || summary.sectors != sectors.len()
                || summary.dimensions.as_ref().is_some_and(|dimensions| {
                    dimensions
                        .iter()
                        .copied()
                        .ne(sectors.iter().map(|r| r.receipt.dimension.unwrap()))
                })
                || summary
                    .runtime_parameters
                    .as_ref()
                    .is_some_and(|parameters| *parameters != indexed.catalogue.runtime_parameters)
            {
                return Err("indexed summary differs from its native catalogue".into());
            }
        } else if artifact.format_version == 5 {
            if artifact.indexed.is_some() {
                return Err("recipe artifact also contains a legacy catalogue".into());
            }
            let programs = artifact
                .programs
                .as_ref()
                .ok_or("recipe artifact has no directory")?;
            programs.validate(options.validate)?;
            if programs.catalogue.content_id != artifact.kernel_content_id {
                return Err("recipe directory differs from archive identity".into());
            }
            let stored: KernelSummary = serde_json::from_str(artifact.kernel.get())?;
            let current = artifact.kernel_summary()?;
            if serde_json::to_value(stored)? != serde_json::to_value(current)? {
                return Err("default recipe summary differs from its native directory".into());
            }
        } else if artifact.is_indexed() {
            return Err("historical artifact unexpectedly contains an indexed catalogue".into());
        }
        if options.validate && artifact.content_id != artifact.identity()? {
            return Err("artifact complete content identity is invalid".into());
        }
        artifact.validation.metadata_identity = options.validate;
        artifact.source_root = base
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        artifact.loading_seconds = started.elapsed().as_secs_f64();
        Ok(artifact)
    }
    pub fn dependencies_compatible(&self) -> bool {
        dependencies_match(&self.provenance.dependencies, &dependencies())
    }
    pub fn load(base: &Path) -> CliResult<(Self, KernelSet)> {
        Self::load_with_preflight(base, |_| Ok(()))
    }
    pub fn load_with_options(
        base: &Path,
        options: KernelLoadOptions,
    ) -> CliResult<(Self, KernelSet)> {
        Self::load_observed_with_options(
            base,
            options,
            |_| Ok(()),
            |_| std::ops::ControlFlow::Continue(()),
        )
    }
    pub fn load_with_preflight(
        base: &Path,
        preflight: impl FnOnce(&Self) -> CliResult<()>,
    ) -> CliResult<(Self, KernelSet)> {
        Self::load_observed(base, preflight, |_| std::ops::ControlFlow::Continue(()))
    }
    pub fn load_observed(
        base: &Path,
        preflight: impl FnOnce(&Self) -> CliResult<()>,
        observe: impl FnMut(&ArtifactLoadProgress) -> std::ops::ControlFlow<()>,
    ) -> CliResult<(Self, KernelSet)> {
        Self::load_observed_with_options(base, KernelLoadOptions::default(), preflight, observe)
    }
    pub fn load_observed_with_options(
        base: &Path,
        options: KernelLoadOptions,
        preflight: impl FnOnce(&Self) -> CliResult<()>,
        observe: impl FnMut(&ArtifactLoadProgress) -> std::ops::ControlFlow<()>,
    ) -> CliResult<(Self, KernelSet)> {
        Self::load_recipe_observed(base, options, None, preflight, observe)
    }
    pub fn load_recipe_observed(
        base: &Path,
        options: KernelLoadOptions,
        recipe: Option<fastsecdec::kernel::indexed::ProgramRecipe>,
        preflight: impl FnOnce(&Self) -> CliResult<()>,
        mut observe: impl FnMut(&ArtifactLoadProgress) -> std::ops::ControlFlow<()>,
    ) -> CliResult<(Self, KernelSet)> {
        let started = Instant::now();
        paths(base)?;
        let poll = |event: &ArtifactLoadProgress,
                    observe: &mut dyn FnMut(&ArtifactLoadProgress) -> std::ops::ControlFlow<()>|
         -> CliResult<()> {
            if observe(event).is_break() {
                Err("artifact loading cancelled".into())
            } else {
                Ok(())
            }
        };
        poll(&ArtifactLoadProgress::Metadata, &mut observe)?;
        let mut artifact = if options.validate {
            Self::load_metadata_with_options(base, options)?
        } else {
            Self::load_metadata(base)?
        };
        if !artifact.dependencies_compatible() {
            return Err("artifact dependency identities differ from this build; regenerate with the recorded dependency revisions".into());
        }
        if let Some(recipe) = recipe {
            artifact.select_recipe(recipe)?;
        }
        preflight(&artifact)?;
        if artifact.is_indexed() {
            let data = artifact.data_path(base)?;
            poll(
                &ArtifactLoadProgress::ReadingBinary {
                    completed: 0,
                    total: None,
                },
                &mut observe,
            )?;
            let mut archive = artifact.open_program_archive(base, options)?;
            let mut reader = archive.select(
                artifact
                    .selected_recipe()
                    .ok_or("missing selected recipe")?,
            )?;
            poll(
                &ArtifactLoadProgress::Native(fastsecdec::kernel::KernelLoadProgress::Decoding),
                &mut observe,
            )?;
            let kernels = reader.load_all_with_progress(&mut |progress| {
                observe(&ArtifactLoadProgress::Native(*progress))
            })?;
            artifact.staged_data = Some(data);
            artifact.validation.binary = options.validate;
            poll(
                &ArtifactLoadProgress::Native(fastsecdec::kernel::KernelLoadProgress::Complete),
                &mut observe,
            )?;
            artifact.loading_seconds = started.elapsed().as_secs_f64();
            return Ok((artifact, kernels));
        }
        let data = artifact.data_path(base)?;
        poll(
            &ArtifactLoadProgress::ReadingBinary {
                completed: 0,
                total: None,
            },
            &mut observe,
        )?;
        let mut file = File::open(data)?;
        let total = usize::try_from(file.metadata()?.len()).ok();
        poll(
            &ArtifactLoadProgress::ReadingBinary {
                completed: 0,
                total,
            },
            &mut observe,
        )?;
        if let Some(size) = total {
            artifact.data.try_reserve_exact(size)?;
        }
        loop {
            // Read existing native bytes in bounded chunks solely to service cancellation.
            let count = Read::by_ref(&mut file)
                .take(8 * 1024 * 1024)
                .read_to_end(&mut artifact.data)?;
            poll(
                &ArtifactLoadProgress::ReadingBinary {
                    completed: artifact.data.len(),
                    total,
                },
                &mut observe,
            )?;
            if count == 0 {
                break;
            }
        }
        let kernels =
            KernelSet::from_bytes_with_options_and_progress(&artifact.data, options, |progress| {
                // Deliver completion only after matching the declared JSON/binary kernel IDs.
                if matches!(progress, fastsecdec::kernel::KernelLoadProgress::Complete) {
                    std::ops::ControlFlow::Continue(())
                } else {
                    observe(&ArtifactLoadProgress::Native(*progress))
                }
            })?;
        if kernels.content_id() != artifact.kernel_content_id()? {
            return Err("native kernel identity differs from the artifact".into());
        }
        artifact.validation.binary = options.validate;
        poll(
            &ArtifactLoadProgress::Native(fastsecdec::kernel::KernelLoadProgress::Complete),
            &mut observe,
        )?;
        artifact.loading_seconds = started.elapsed().as_secs_f64();
        Ok((artifact, kernels))
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> CliResult<()> {
    atomic_write_with(path, |writer| {
        writer.write_all(bytes)?;
        Ok(())
    })
}

fn atomic_write_with(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> CliResult<()>,
) -> CliResult<()> {
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_file_name(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let mut writer = BufWriter::new(file);
    let result = (|| -> CliResult<()> {
        write(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        fs::rename(&temporary, path)?;
        Ok(())
    })();
    drop(writer);
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
#[path = "artifact/persistence_tests.rs"]
mod persistence_tests;
#[cfg(test)]
#[path = "artifact/program_tests.rs"]
mod program_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_provenance_reports_actual_code_and_only_accepts_same_legacy_backend() {
        let current = dependencies();
        let symjit = current
            .iter()
            .find(|dependency| dependency.name == "symjit")
            .unwrap();
        assert_eq!(
            symjit.revision,
            format!(
                "native-version-code:{}",
                fastsecdec::kernel::symjit_version_code()
            )
        );
        assert!(dependencies_match(&current, &current));
        let legacy = Dependency {
            name: "symjit".into(),
            revision: "2.26.4".into(),
            source_state: "published Rust crate".into(),
        };
        let exact = Dependency {
            name: "symjit".into(),
            revision: "native-version-code:22604".into(),
            source_state: "linked SymJIT Application::measure(version)".into(),
        };
        assert!(dependency_matches(&legacy, &exact));
        let mut newer = exact.clone();
        newer.revision = "native-version-code:22700".into();
        assert!(!dependency_matches(&legacy, &newer));
        newer = exact.clone();
        newer.source_state = "unverified".into();
        assert!(!dependency_matches(&legacy, &newer));
        newer = exact;
        newer.name = "another dependency".into();
        assert!(!dependency_matches(&legacy, &newer));
        assert!(!dependencies_match(&current[..current.len() - 1], &current));
    }

    #[test]
    fn omitted_preparation_preserves_legacy_provenance_serialization() {
        let legacy = r#"{"name":"old","sources":[],"dependencies":[],"domain":"ProjectiveSimplex","assume_no_threshold":false,"dimension":"4-2*eps","regulator":"eps","measure":"native","measure_multiplier":"1","max_order":0,"integration":{}}"#;
        let provenance: Provenance = serde_json::from_str(legacy).unwrap();
        assert!(provenance.family_preparation.is_none());
        assert_eq!(serde_json::to_string(&provenance).unwrap(), legacy);
        let mode = SourceFingerprint::RunCardWithoutReference;
        assert_ne!(
            mode.hash(b"[generation]\norder=0\n").unwrap(),
            mode.hash(b"[generation]\norder=0\n[generation.family_preparation.SingleUnitTerm]\nmax_states=32\n").unwrap(),
        );
    }

    #[test]
    fn scientific_card_fingerprint_excludes_only_root_reference_and_preserves_legacy_bytes() {
        let mode = SourceFingerprint::RunCardWithoutReference;
        let original = b"unknown_root = 1\n[integration]\npoints = 1024\n";
        let comparison =
            b"unknown_root=1\n[integration]\npoints=1024\n[reference]\npath='target.json'\n";
        assert_eq!(mode.hash(original).unwrap(), mode.hash(comparison).unwrap());
        assert_ne!(
            mode.hash(original).unwrap(),
            mode.hash(b"unknown_root=2\n[integration]\npoints=1024\n")
                .unwrap()
        );
        assert_ne!(
            mode.hash(b"unknown_root=nan\n").unwrap(),
            mode.hash(b"unknown_root=inf\n").unwrap()
        );
        assert_ne!(
            mode.hash(b"unknown_root=2026-10-04\n").unwrap(),
            mode.hash(b"unknown_root='2026-10-04'\n").unwrap()
        );
        assert_ne!(
            mode.hash(original).unwrap(),
            mode.hash(b"unknown_root=1\n[integration]\npoints=2048\n")
                .unwrap()
        );
        let legacy: SourceFile =
            serde_json::from_str(r#"{"path":"input.toml","blake3":"old"}"#).unwrap();
        assert!(matches!(legacy.fingerprint, SourceFingerprint::Bytes));
        assert!(
            !serde_json::to_value(&legacy)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("fingerprint")
        );
        assert_ne!(
            legacy.fingerprint.hash(original).unwrap(),
            legacy.fingerprint.hash(comparison).unwrap()
        );
    }
}
