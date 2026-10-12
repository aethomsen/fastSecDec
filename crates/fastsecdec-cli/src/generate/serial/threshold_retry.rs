//! Explicit new CLI attempt; the original journal remains read-only and locked.
use super::*;
use jobs::threshold::retry::RetryAttempt;
use std::path::PathBuf;

pub(super) struct RetrySeed {
    _source: Journal,
    pub attempt: RetryAttempt,
}
pub(super) fn journal_directory(output: &Path) -> CliResult<PathBuf> {
    Ok(output.with_file_name(format!(
        "{}.generation",
        output
            .file_name()
            .ok_or("missing output basename")?
            .to_string_lossy()
    )))
}
fn canonical_destination(path: &Path) -> CliResult<PathBuf> {
    let name = path.file_name().ok_or("missing retry output basename")?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(parent.canonicalize()?.join(name))
}
pub(super) fn open(
    input: &Path,
    output: &Path,
    old_output: &Path,
    overlay: &Path,
    overrides: crate::config::GenerationOverrides,
    resume: bool,
) -> CliResult<RetrySeed> {
    if resume {
        return Err(
            "endpoint resource retry is a new attempt; it cannot be combined with --resume".into(),
        );
    }
    let output = canonical_destination(output)?;
    let old_output = canonical_destination(old_output)?;
    let new_journal = journal_directory(&output)?;
    let old_journal = journal_directory(&old_output)?;
    if new_journal == old_journal
        || new_journal.starts_with(&old_journal)
        || old_journal.starts_with(&new_journal)
    {
        return Err("endpoint retry needs a distinct, disjoint output/journal".into());
    }
    let (metadata, data) = crate::artifact::paths(&output)?;
    for path in [&new_journal, &metadata, &data] {
        match fs::symlink_metadata(path) {
            Ok(_) => {
                return Err(
                    "endpoint retry output/journal already exists; choose a new basename".into(),
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    let resources: fastsecdec::threshold::generation::EndpointPreparationResources =
        serde_json::from_reader(File::open(overlay)?)?;
    resources.validate()?;
    let source = Journal::retained_endpoint(input, &old_output, overrides)?;
    let attempt = RetryAttempt::read(&source.root, &old_output, resources)?;
    Ok(RetrySeed {
        _source: source,
        attempt,
    })
}
#[cfg(test)]
mod tests;
