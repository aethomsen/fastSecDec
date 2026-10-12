//! Durable, exclusively owned receipt journal. Never deserialize native work.
mod validation;
use super::{
    family::RecipeFamily,
    jobs::{Job, Request, Response},
};
use crate::{
    CliResult,
    artifact::{self, Dependency, SourceFingerprint},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};
pub(super) use validation::{validate, validate_provenance};

// Schema three binds the full canonical capability family before resume.
const JOURNAL_VERSION: u32 = 3;

#[derive(Serialize, Deserialize)]
struct Entry {
    request_hash: String,
    response: String,
    response_hash: Option<String>,
}
#[derive(Serialize, Deserialize)]
struct State {
    version: u32,
    build_identity: String,
    input_hash: String,
    #[serde(default)]
    generation_overrides: crate::config::GenerationOverrides,
    dependencies: Vec<Dependency>,
    #[serde(default)]
    family: Option<RecipeFamily>,
    run_directory: String,
    completed: bool,
    entries: BTreeMap<String, Entry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    threshold_preparation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    endpoint_retry: Option<super::jobs::threshold::retry::RetryAttempt>,
}
impl State {
    fn validate_resume(
        &self,
        input: &Path,
        generation_overrides: crate::config::GenerationOverrides,
        family: &RecipeFamily,
    ) -> CliResult<()> {
        let state = self;
        let input_hash = SourceFingerprint::RunCardScientificInput.hash(&fs::read(input)?)?;
        let dependencies = artifact::dependencies();
        if state.version != JOURNAL_VERSION {
            return Err(format!(
                    "generation resume refused: staging format {} is incompatible with version {}; regenerate using a new output basename",
                    state.version, JOURNAL_VERSION,
                ).into());
        }
        if state.family.as_ref() != Some(family) {
            return Err(
                "generation resume refused: requested recipe family or default changed".into(),
            );
        }
        if state.build_identity != crate::process::child::build_identity()
            || state.input_hash != input_hash
            || state.generation_overrides != generation_overrides
            || state.dependencies != dependencies
        {
            return Err(
                    "generation resume refused: input settings, FastSecDec build or native dependencies changed"
                        .into(),
                );
        }
        Ok(())
    }
}
pub(super) struct Journal {
    _lock: File,
    directory: PathBuf,
    pub root: PathBuf,
    state: State,
}
impl Journal {
    #[cfg(test)]
    pub fn open(input: &Path, output: &Path, resume: bool, run_id: &str) -> CliResult<Self> {
        Self::open_with_overrides(input, output, resume, run_id, Default::default())
    }

    #[cfg(test)]
    pub fn open_with_overrides(
        input: &Path,
        output: &Path,
        resume: bool,
        run_id: &str,
        generation_overrides: crate::config::GenerationOverrides,
    ) -> CliResult<Self> {
        let mut card: crate::config::RunCard = toml::from_str(&fs::read_to_string(input)?)?;
        generation_overrides.apply(&mut card);
        Self::open_with_family(
            input,
            output,
            resume,
            run_id,
            generation_overrides,
            RecipeFamily::single(card.generation.program_recipe()),
        )
    }

    pub fn open_with_family(
        input: &Path,
        output: &Path,
        resume: bool,
        run_id: &str,
        generation_overrides: crate::config::GenerationOverrides,
        family: RecipeFamily,
    ) -> CliResult<Self> {
        let directory = output.with_file_name(format!(
            "{}.generation",
            output
                .file_name()
                .ok_or("missing output basename")?
                .to_string_lossy()
        ));
        fs::create_dir_all(&directory)?;
        if fs::symlink_metadata(&directory)?.file_type().is_symlink() {
            return Err("generation staging directory cannot be a symlink".into());
        }
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(directory.join("lock"))?;
        lock.try_lock().map_err(|error| {
            format!("another process owns the generation staging directory: {error}")
        })?;
        let input_hash = SourceFingerprint::RunCardScientificInput.hash(&fs::read(input)?)?;
        let dependencies = artifact::dependencies();
        let journal = directory.join("journal.json");
        let previous = if journal.exists() {
            Some(serde_json::from_reader::<_, State>(File::open(&journal)?)?)
        } else {
            None
        };
        let state = if resume {
            let state = previous.ok_or("no resumable generation journal exists")?;
            state.validate_resume(input, generation_overrides, &family)?;
            state
        } else {
            if previous.as_ref().is_some_and(|old| !old.completed) {
                return Err("unfinished generation staging exists; use --resume or choose a different output basename".into());
            }
            State {
                version: JOURNAL_VERSION,
                build_identity: crate::process::child::build_identity(),
                input_hash,
                generation_overrides,
                dependencies,
                family: Some(family),
                run_directory: format!("run-{run_id}"),
                completed: false,
                entries: BTreeMap::new(),
                threshold_preparation: None,
                endpoint_retry: None,
            }
        };
        if Path::new(&state.run_directory).components().count() != 1
            || !matches!(
                Path::new(&state.run_directory).components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return Err("invalid generation staging run directory".into());
        }
        let root = directory.join(&state.run_directory);
        fs::create_dir_all(&root)?;
        fs::create_dir_all(root.join("jobs"))?;
        fs::create_dir_all(root.join("receipts"))?;
        let out = Self {
            _lock: lock,
            directory,
            root,
            state,
        };
        out.save()?;
        Ok(out)
    }
    /// Read and lock existing retained evidence without recreating a cleaned run
    /// or rewriting any old journal/receipt. Validation is identical to resume.
    pub fn retained_endpoint(
        input: &Path,
        output: &Path,
        overrides: crate::config::GenerationOverrides,
    ) -> CliResult<Self> {
        let directory = super::threshold_retry::journal_directory(output)?;
        if fs::symlink_metadata(&directory)?.file_type().is_symlink() {
            return Err("retained staging directory cannot be a symlink".into());
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(directory.join("lock"))?;
        lock.try_lock()
            .map_err(|e| format!("original preparation is still owned: {e}"))?;
        let state: State = serde_json::from_reader(File::open(directory.join("journal.json"))?)?;
        state.validate_resume(
            input,
            overrides,
            &RecipeFamily::single(fastsecdec::kernel::ProgramRecipe::ThresholdV1),
        )?;
        if !matches!(
            Path::new(&state.run_directory)
                .components()
                .collect::<Vec<_>>()
                .as_slice(),
            [std::path::Component::Normal(_)]
        ) {
            return Err("invalid retained generation run directory".into());
        }
        let root = directory.join(&state.run_directory);
        if !root.is_dir() {
            return Err("preparation evidence unavailable: the completed artifact's run directory was cleaned; retained raw/configuration records are required".into());
        }
        if fs::symlink_metadata(&root)?.file_type().is_symlink() {
            return Err("retained generation run directory cannot be a symlink".into());
        }
        Ok(Self {
            _lock: lock,
            directory,
            root,
            state,
        })
    }
    pub fn endpoint_retry(
        &mut self,
        issued: Option<super::jobs::threshold::retry::RetryAttempt>,
    ) -> CliResult<Option<super::jobs::threshold::retry::RetryAttempt>> {
        if let Some(issued) = issued {
            if self.state.endpoint_retry.is_some()
                || !self.state.entries.is_empty()
                || self.state.completed
            {
                return Err("endpoint resource retry requires an unused new journal".into());
            }
            self.state.endpoint_retry = Some(issued);
            self.save()?;
        }
        Ok(self.state.endpoint_retry.clone())
    }
    pub fn completed(&self) -> bool {
        self.state.completed
    }
    /// Children inherit this locked open-file description, so coordinator death
    /// cannot release exclusive residency while native work is still alive.
    pub fn residency_lock(&self) -> &File {
        &self._lock
    }
    pub fn response_path(&self, key: &str) -> PathBuf {
        self.root.join("receipts").join(format!("{key}.json"))
    }
    pub fn job_path(&self, key: &str) -> PathBuf {
        self.root.join("jobs").join(format!("{key}.json"))
    }
    fn save(&self) -> CliResult<()> {
        artifact::atomic_write(
            &self.directory.join("journal.json"),
            &serde_json::to_vec(&self.state)?,
        )?;
        File::open(&self.directory)?.sync_all()?;
        Ok(())
    }
    fn request_hash(request: &Request) -> CliResult<String> {
        let normalized = match request {
            Request::Prepare {
                input, overrides, ..
            } => Request::Prepare {
                input: input.clone(),
                workers: 0,
                overrides: *overrides,
            },
            Request::PreparePrograms {
                input,
                overrides,
                recipes,
                ..
            } => Request::PreparePrograms {
                input: input.clone(),
                workers: 0,
                overrides: *overrides,
                recipes: recipes.clone(),
            },
            _ => request.clone(),
        };
        Ok(blake3::hash(&serde_json::to_vec(&normalized)?)
            .to_hex()
            .to_string())
    }
    /// Register immutable request intent before issuing it. An unacknowledged
    /// but durable matching receipt can be adopted after a coordinator crash.
    pub fn request(&mut self, key: &str, request: Request) -> CliResult<(Job, Option<Response>)> {
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err("invalid generation job key".into());
        }
        if let Request::PreparePrograms { recipes, .. } = &request
            && self
                .state
                .family
                .as_ref()
                .is_none_or(|family| family.recipes() != recipes)
        {
            return Err("issued preparation differs from the durable requested family".into());
        }
        let request_hash = Self::request_hash(&request)?;
        let response = self.response_path(key);
        if let Some(entry) = self.state.entries.get(key) {
            if entry.request_hash != request_hash {
                return Err(format!("generation resume refused: job {key} changed").into());
            }
        } else {
            self.state.entries.insert(
                key.into(),
                Entry {
                    request_hash,
                    response: format!("receipts/{key}.json"),
                    response_hash: None,
                },
            );
            self.save()?;
        }
        let job = Job {
            root: self.root.clone(),
            response: response.clone(),
            request,
        };
        artifact::atomic_write(&self.job_path(key), &serde_json::to_vec(&job)?)?;
        let existing = if response.exists() {
            Some(self.accept(key, &job.request)?)
        } else {
            if self.state.entries[key].response_hash.is_some() {
                return Err(format!("committed generation receipt {key} is missing").into());
            }
            None
        };
        Ok((job, existing))
    }
    pub fn accept(&mut self, key: &str, request: &Request) -> CliResult<Response> {
        let path = self.response_path(key);
        let bytes = fs::read(&path)?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let entry = self
            .state
            .entries
            .get(key)
            .ok_or("unissued generation completion")?;
        if entry.request_hash != Self::request_hash(request)?
            || entry
                .response_hash
                .as_ref()
                .is_some_and(|expected| expected != &hash)
        {
            return Err("generation receipt identity changed".into());
        }
        let response: Response = serde_json::from_slice(&bytes)?;
        validate(request, &response, &self.root)?;
        self.state.entries.get_mut(key).unwrap().response_hash = Some(hash);
        self.save()?;
        Ok(response)
    }
    pub fn threshold_preparation(
        &mut self,
    ) -> CliResult<Option<super::jobs::threshold::PreparedThreshold>> {
        let Some(key) = self.state.threshold_preparation.clone() else {
            return Ok(None);
        };
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return Err("invalid threshold anchor key".into());
        }
        let job: Job = serde_json::from_reader(File::open(self.job_path(&key))?)?;
        if job.root != self.root || job.response != self.response_path(&key) {
            return Err("threshold anchor job path changed".into());
        }
        match self.accept(&key, &job.request)? {
            Response::PreparedThreshold(value) => Ok(Some(*value)),
            _ => Err("threshold anchor has wrong response kind".into()),
        }
    }
    pub fn anchor_threshold_preparation(&mut self, key: &str) -> CliResult<()> {
        if self.state.threshold_preparation.is_some() {
            return Err("threshold preparation anchor is immutable".into());
        }
        if self
            .state
            .entries
            .get(key)
            .is_none_or(|entry| entry.response_hash.is_none())
        {
            return Err("threshold anchor was not accepted".into());
        }
        self.state.threshold_preparation = Some(key.into());
        self.save()
    }
    pub fn complete(&mut self) -> CliResult<()> {
        self.state.completed = true;
        self.save()
    }
    pub fn cleanup(&self) -> CliResult<()> {
        fs::remove_dir_all(&self.root)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn card(path: &Path) {
        fs::write(path,"[direct]\ndomain='unit_cube'\nparameters=['x']\n[[direct.terms]]\nmonomial_powers=['0']\n[[direct.terms.factors]]\npolynomial='1+x'\nexponent='1'\nrole='polynomial'\n[generation]\norder=0\n").unwrap();
    }
    #[test]
    fn durable_unacknowledged_prepare_is_adopted_with_a_changed_worker_count() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.toml");
        card(&input);
        let output = dir.path().join("integral.fsd");
        let mut journal = Journal::open(&input, &output, false, "first").unwrap();
        let (_, cached) = journal
            .request(
                "prepare",
                Request::Prepare {
                    input: input.clone(),
                    workers: 1,
                    overrides: Default::default(),
                },
            )
            .unwrap();
        assert!(cached.is_none());
        super::super::jobs::execute(&journal.job_path("prepare"), &mut |_| Ok(())).unwrap();
        let before = fs::read(journal.response_path("prepare")).unwrap();
        drop(journal);
        let mut resumed = Journal::open(&input, &output, true, "second").unwrap();
        let (_, cached) = resumed
            .request(
                "prepare",
                Request::Prepare {
                    input: input.clone(),
                    workers: 3,
                    overrides: Default::default(),
                },
            )
            .unwrap();
        assert!(matches!(cached, Some(Response::Prepared(_))));
        assert_eq!(before, fs::read(resumed.response_path("prepare")).unwrap());
        // A scheduling-only card edit leaves its scientific source fingerprint unchanged.
        fs::write(
            &input,
            fs::read_to_string(&input)
                .unwrap()
                .replace("order=0", "order=0\nserial=true"),
        )
        .unwrap();
        drop(resumed);
        let resumed = Journal::open(&input, &output, true, "third").unwrap();
        drop(resumed);
        fs::write(
            &input,
            fs::read_to_string(&input)
                .unwrap()
                .replace("order=0", "order=1"),
        )
        .unwrap();
        assert!(Journal::open(&input, &output, true, "changed").is_err());
    }
    #[test]
    fn lock_and_receipt_kind_prevent_concurrent_or_foreign_completions() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.toml");
        card(&input);
        let output = dir.path().join("integral.fsd");
        let mut journal = Journal::open(&input, &output, false, "one").unwrap();
        assert!(Journal::open(&input, &output, true, "two").is_err());
        let request = Request::Prepare {
            input,
            workers: 1,
            overrides: Default::default(),
        };
        journal.request("prepare", request.clone()).unwrap();
        let foreign = Response::Symmetry(fastsecdec::generation::streaming::SymmetryAssignment {
            program_recipe: fastsecdec::kernel::indexed::ProgramRecipe::UndeformedV1,
            source_id: "foreign".into(),
            source: 0,
            representative: 0,
            permutation: vec![],
        });
        artifact::atomic_write(
            &journal.response_path("prepare"),
            &serde_json::to_vec(&foreign).unwrap(),
        )
        .unwrap();
        assert!(journal.accept("prepare", &request).is_err());
    }

    #[test]
    fn legacy_staging_is_rejected_without_modifying_its_receipts() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.toml");
        card(&input);
        let output = dir.path().join("integral.fsd");
        let journal = Journal::open(&input, &output, false, "first").unwrap();
        let path = journal.directory.join("journal.json");
        let receipt = journal.response_path("prepare");
        fs::write(&receipt, b"prior work remains available").unwrap();
        drop(journal);
        let mut state: serde_json::Value =
            serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        state.as_object_mut().unwrap().remove("family");
        for version in [1, 2] {
            state["version"] = version.into();
            let original = serde_json::to_vec(&state).unwrap();
            fs::write(&path, &original).unwrap();
            let error = Journal::open(&input, &output, true, "second")
                .err()
                .expect("legacy native staging was admitted");
            assert!(
                error
                    .to_string()
                    .contains(&format!("staging format {version}"))
            );
            assert!(error.to_string().contains("new output basename"));
            assert_eq!(fs::read(&path).unwrap(), original);
            assert_eq!(fs::read(&receipt).unwrap(), b"prior work remains available");
        }
    }

    #[test]
    fn generation_resume_checks_cli_contour_capability_without_rewriting_input() {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("input.toml");
        card(&input);
        let original = fs::read(&input).unwrap();
        let output = dir.path().join("integral.fsd");
        let overrides = crate::config::GenerationOverrides {
            contour: true,
            ..Default::default()
        };
        let journal =
            Journal::open_with_overrides(&input, &output, false, "first", overrides).unwrap();
        drop(journal);
        assert!(Journal::open(&input, &output, true, "incompatible").is_err());
        let resumed =
            Journal::open_with_overrides(&input, &output, true, "same", overrides).unwrap();
        drop(resumed);
        assert_eq!(fs::read(&input).unwrap(), original);
    }
}
