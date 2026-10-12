//! Compact CLI intent; exact resource admission and evidence recovery are native.
use super::*;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RetryAttempt {
    pub evidence_root: PathBuf,
    pub original_output: PathBuf,
    pub(super) checkpoint: Checkpoint,
    pub resources: threshold::EndpointPreparationResources,
}
impl RetryAttempt {
    pub(in super::super::super) fn read(
        journal_root: &Path,
        old_output: &Path,
        resources: threshold::EndpointPreparationResources,
    ) -> CliResult<Self> {
        let path = journal_root.join("threshold-checkpoint.json");
        if !path.is_file() {
            return Err("preparation evidence unavailable: retry requires retained configuration/request/raw records; a cleaned artifact alone cannot seed retry".into());
        }
        let checkpoint: Checkpoint = serde_json::from_reader(File::open(path)?)?;
        let evidence_root = rooted(journal_root, &checkpoint.directory)?.canonicalize()?;
        Ok(Self {
            evidence_root,
            original_output: old_output.to_path_buf(),
            checkpoint,
            resources,
        })
    }
    pub(super) fn validate(&self, input: &Path, settings: &str, source: &str) -> CliResult<()> {
        if self.checkpoint.issued_settings != settings
            || self.checkpoint.build != crate::process::child::build_identity()
            || self.checkpoint.native.evidence.request.source_identity != source
        {
            return Err("endpoint retry original issued settings/build/source changed".into());
        }
        super::super::super::journal::validate_provenance(input, &self.checkpoint.provenance)?;
        Ok(())
    }
    pub(in super::super::super) fn bind(
        &self,
        root: &Path,
        checkpoint: &threshold::EndpointRetryCheckpoint,
    ) -> CliResult<threshold::EndpointRetryLineage> {
        let lineage = checkpoint.lineage(root, TRANSPORT_BYTES)?;
        if lineage.original.configuration != self.checkpoint.native.configuration
            || lineage.original.evidence != self.checkpoint.native.evidence
            || serde_json::to_value(&lineage.resources)? != serde_json::to_value(&self.resources)?
        {
            return Err(
                "endpoint retry checkpoint differs from issued resource overlay/original evidence"
                    .into(),
            );
        }
        Ok(lineage)
    }
}
