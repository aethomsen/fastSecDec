//! Thin storage owner around the native preparation/publication APIs. No Python
//! or global verified geometry is retained after native preparation returns.
use fastsecdec::{
    kernel::{
        ThresholdPublicationPlan,
        indexed::{ProgramArchiveCatalogue, ProgramArchiveWriter},
    },
    parametric::ParametricIntegrand,
    threshold::generation::{
        self as native, PreparationCheckpoint, PreparationOptions, PreparationReceipt, Progress,
    },
};
use std::{
    fs::{self, File},
    io::{Seek, SeekFrom},
    ops::ControlFlow,
    path::PathBuf,
    rc::Rc,
    sync::Arc,
};
use tempfile::TempDir;
pub(super) type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
enum Input {
    Parametric(Arc<ParametricIntegrand>),
    Graph(Arc<fastsecdec::threshold::represented::graph::GraphPoint>),
    Family(Arc<fastsecdec::threshold::represented::family::FamilyPoint>),
}
pub(super) struct Work {
    pub options: PreparationOptions,
    pub limit: u64,
    pub storage: Option<Rc<TempDir>>,
    pub receipt: Option<PreparationReceipt>,
    pub progress: Option<Progress>,
    pub next: usize,
    input: Option<Input>,
    source: Option<String>,
    root: Option<PathBuf>,
    checkpoint: Option<PreparationCheckpoint>,
    attempt: usize,
    plan: Option<ThresholdPublicationPlan>,
    archive: Option<ProgramArchiveWriter<File>>,
}
impl Work {
    pub fn new(options: PreparationOptions, limit: u64) -> Self {
        Self {
            options,
            limit,
            storage: None,
            receipt: None,
            progress: None,
            next: 0,
            input: None,
            source: None,
            root: None,
            checkpoint: None,
            attempt: 0,
            plan: None,
            archive: None,
        }
    }
    pub fn initialized(&self) -> bool {
        self.storage.is_some()
    }
    pub fn initialize(&mut self, input: ParametricIntegrand) -> Result<()> {
        self.initialize_input(Input::Parametric(Arc::new(input)))
    }
    pub fn initialize_represented(&mut self, input: crate::input::RepresentedPoint) -> Result<()> {
        self.initialize_input(match input {
            crate::input::RepresentedPoint::Graph(point) => Input::Graph(point),
            crate::input::RepresentedPoint::Family(point) => Input::Family(point),
        })
    }
    fn initialize_input(&mut self, input: Input) -> Result<()> {
        self.storage = Some(Rc::new(
            tempfile::Builder::new()
                .prefix("fastsecdec-threshold-")
                .tempdir()?,
        ));
        self.input = Some(input);
        Ok(())
    }
    pub fn prepare(
        &mut self,
        mut observer: impl FnMut(&Progress) -> ControlFlow<()>,
    ) -> Result<bool> {
        if self.receipt.is_some() {
            return Ok(true);
        }
        let storage = self
            .storage
            .as_ref()
            .ok_or("threshold input not initialized")?;
        self.attempt = self
            .attempt
            .checked_add(1)
            .ok_or("threshold retry count overflow")?;
        if self.checkpoint.is_none() {
            let root = storage.path().join(format!("preparation-{}", self.attempt));
            fs::create_dir(&root)?;
            self.root = Some(root);
        }
        let root = self
            .root
            .as_ref()
            .ok_or("missing native preparation root")?;
        let prior = self.checkpoint.clone();
        // Source authority is issued by native prepare from the immutable
        // original input AND represented-value policy. Receipt JSON cannot set it.
        let resume_source = self.source.clone();
        if prior.is_some() && resume_source.is_none() {
            return Err("missing native-issued source identity for resume".into());
        }
        let progress = &mut self.progress;
        let checkpoint = &mut self.checkpoint;
        let source = &mut self.source;
        let mut association_error = None;
        let mut observe = |p: &Progress| {
            if let Err(error) = accept_native_source(source, p) {
                association_error = Some(error);
                *checkpoint = None;
                return ControlFlow::Break(());
            }
            *progress = Some(p.clone());
            if let Some(c) = p.checkpoint() {
                *checkpoint = Some(c)
            }
            observer(p)
        };
        let input = self.input.as_ref().ok_or("missing native input")?;
        let result = match (input, prior) {
            (Input::Parametric(input), None) => {
                native::prepare(input.clone(), self.options.clone(), root, &mut observe)
            }
            (Input::Parametric(_), Some(c)) => native::resume_evidence(
                root,
                &c,
                &format!("vectors-{}", self.attempt),
                resume_source.as_deref().expect("resume checked above"),
                self.limit,
                &mut observe,
            ),
            (Input::Graph(input), None) => {
                native::prepare_graph(input.clone(), self.options.clone(), root, &mut observe)
            }
            (Input::Graph(input), Some(c)) => native::resume_graph_evidence(
                input.clone(),
                self.options
                    .represented
                    .ok_or("graph requires represented policy")?
                    .1,
                root,
                &c,
                &format!("vectors-{}", self.attempt),
                resume_source.as_deref().expect("resume checked above"),
                self.limit,
                &mut observe,
            ),
            (Input::Family(input), None) => {
                native::prepare_family(input.clone(), self.options.clone(), root, &mut observe)
            }
            (Input::Family(input), Some(c)) => native::resume_family_evidence(
                input.clone(),
                self.options
                    .represented
                    .ok_or("family requires represented policy")?
                    .1,
                root,
                &c,
                &format!("vectors-{}", self.attempt),
                resume_source.as_deref().expect("resume checked above"),
                self.limit,
                &mut observe,
            ),
        };
        if let Some(error) = association_error {
            return Err(error.into());
        }
        let prepared = match result {
            Ok(p) => p,
            Err(native::Error::Cancelled) => return Ok(false),
            Err(e) => return Err(e.into()),
        };
        let source = self
            .source
            .as_deref()
            .ok_or("missing native-issued source identity")?;
        let receipt = prepared.receipt;
        if receipt.publication.source_identity != source {
            return Err("native threshold source identity changed".into());
        }
        let plan = ThresholdPublicationPlan::from_trusted_preparer(
            &root.join(&receipt.work_directory),
            receipt.publication.clone(),
            source,
            &receipt.publication.prepared_identity,
            self.limit,
        )?;
        let archive = plan.archive_writer(File::create(storage.path().join("integral.fsd"))?)?;
        self.plan = Some(plan);
        self.archive = Some(archive);
        self.receipt = Some(receipt);
        // Original input/Atom density is no longer needed by compilation jobs.
        self.input = None;
        Ok(true)
    }
    pub fn jobs(&self) -> usize {
        self.plan
            .as_ref()
            .map_or(0, ThresholdPublicationPlan::job_count)
    }
    pub fn compile_one(&mut self) -> Result<()> {
        let plan = self
            .plan
            .as_ref()
            .ok_or("call prepare() before compilation")?;
        if self.next >= plan.job_count() {
            return Ok(());
        }
        let root = self.root.as_ref().ok_or("missing preparation root")?.join(
            &self
                .receipt
                .as_ref()
                .ok_or("missing receipt")?
                .work_directory,
        );
        // A single temporary file holds one local record; completed records are
        // copied immediately into the native archive, not collected in memory.
        let mut record =
            tempfile::tempfile_in(self.storage.as_ref().ok_or("missing storage")?.path())?;
        let receipt = plan
            .work(self.next)?
            .compile_record(&root, self.limit, &mut record)?;
        record.seek(SeekFrom::Start(0))?;
        plan.append_work_record(
            self.archive.as_mut().ok_or("missing archive writer")?,
            &mut record,
            receipt,
        )?;
        self.next += 1;
        Ok(())
    }
    pub fn finish(&mut self) -> Result<(File, ProgramArchiveCatalogue)> {
        if self.receipt.is_none() || self.next != self.jobs() {
            return Err("threshold compilation is incomplete".into());
        }
        Ok(self
            .archive
            .take()
            .ok_or("archive already published")?
            .finish()?)
    }
}

/// Called only inside a live native preparation callback. It is deliberately
/// private: imported checkpoint/receipt JSON must not establish source authority.
fn accept_native_source(
    source: &mut Option<String>,
    progress: &Progress,
) -> std::result::Result<(), &'static str> {
    if let Some(request) = &progress.receipts.request {
        if let Some(expected) = source {
            if expected != &request.source_identity {
                return Err("native threshold request source identity changed");
            }
        } else {
            *source = Some(request.source_identity.clone());
        }
    }
    if let Some(raw) = &progress.receipts.raw
        && (progress.receipts.request.as_ref() != Some(&raw.request)
            || source.as_deref() != Some(raw.request.source_identity.as_str()))
    {
        return Err("native threshold raw evidence has a different request");
    }
    Ok(())
}

#[cfg(test)]
#[path = "work/tests.rs"]
mod tests;
