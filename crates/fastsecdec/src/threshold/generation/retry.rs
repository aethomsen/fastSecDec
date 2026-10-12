//! Explicit new endpoint preparation from retained native raw evidence.
//! Completed checkpoints require their records; compiled artifacts alone do not
//! contain the proof. Old roots are read-only and ordinary recovery is unchanged.
use super::*;
use crate::generation::streaming::{
    RecordRef,
    codec::{self, Atoms},
};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;

mod records;
#[cfg(test)]
mod tests;
const KIND: &str = "threshold-endpoint-resource-retry-v1";

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointRetryCheckpoint {
    pub lineage: RecordRef,
    pub preparation: PreparationCheckpoint,
}
pub struct EndpointRetryPrepared {
    pub prepared: PreparedThreshold,
    pub checkpoint: EndpointRetryCheckpoint,
}
#[derive(Clone, Debug, Serialize)]
pub struct EndpointRetryProgress {
    pub preparation: Progress,
    pub checkpoint: Option<EndpointRetryCheckpoint>,
}
/// Recorded attempt history. Record references identify retained bytes; this
/// metadata alone does not certify a proof or promise that old paths still exist.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EndpointRetryLineage {
    pub original: PreparationCheckpoint,
    pub effective_configuration: RecordRef,
    pub resources: EndpointPreparationResources,
    pub original_directory: String,
    pub attempt_directory: String,
}

#[allow(clippy::too_many_arguments)]
pub fn retry_endpoint_evidence(
    evidence_root: &Path,
    checkpoint: &PreparationCheckpoint,
    expected_source: &str,
    resources: &EndpointPreparationResources,
    new_root: &Path,
    maximum_bytes: u64,
    observer: impl FnMut(&EndpointRetryProgress) -> ControlFlow<()>,
) -> Result<EndpointRetryPrepared> {
    retry(
        evidence_root,
        checkpoint,
        expected_source,
        resources,
        new_root,
        maximum_bytes,
        None,
        observer,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn retry_graph_endpoint_evidence(
    original: Arc<represented::graph::GraphPoint>,
    conversion_cap: represented::Limits,
    evidence_root: &Path,
    checkpoint: &PreparationCheckpoint,
    expected_source: &str,
    resources: &EndpointPreparationResources,
    new_root: &Path,
    maximum_bytes: u64,
    observer: impl FnMut(&EndpointRetryProgress) -> ControlFlow<()>,
) -> Result<EndpointRetryPrepared> {
    retry(
        evidence_root,
        checkpoint,
        expected_source,
        resources,
        new_root,
        maximum_bytes,
        Some((OriginalPoint::Graph(original), conversion_cap)),
        observer,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn retry_family_endpoint_evidence(
    original: Arc<represented::family::FamilyPoint>,
    conversion_cap: represented::Limits,
    evidence_root: &Path,
    checkpoint: &PreparationCheckpoint,
    expected_source: &str,
    resources: &EndpointPreparationResources,
    new_root: &Path,
    maximum_bytes: u64,
    observer: impl FnMut(&EndpointRetryProgress) -> ControlFlow<()>,
) -> Result<EndpointRetryPrepared> {
    retry(
        evidence_root,
        checkpoint,
        expected_source,
        resources,
        new_root,
        maximum_bytes,
        Some((OriginalPoint::Family(original), conversion_cap)),
        observer,
    )
}

#[allow(clippy::too_many_arguments)]
fn retry<F: FnMut(&EndpointRetryProgress) -> ControlFlow<()>>(
    evidence_root: &Path,
    checkpoint: &PreparationCheckpoint,
    expected_source: &str,
    resources: &EndpointPreparationResources,
    new_root: &Path,
    maximum_bytes: u64,
    original: Option<(OriginalPoint, represented::Limits)>,
    mut observer: F,
) -> Result<EndpointRetryPrepared> {
    let source = evidence_root.canonicalize()?;
    let target = records::destination(&source, new_root)?;
    let source_name = source
        .to_str()
        .ok_or(Error::Association("retry root must be UTF-8"))?
        .to_owned();
    let target_name = target
        .to_str()
        .ok_or(Error::Association("retry root must be UTF-8"))?
        .to_owned();
    let active = RefCell::new(None);
    let mut observe = |progress: &Progress| {
        observer(&EndpointRetryProgress {
            preparation: progress.clone(),
            checkpoint: active.borrow().clone(),
        })
    };
    let start = Instant::now();
    let mut reporter = Reporter {
        observer: &mut observe,
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
    let closure = records::references(checkpoint)?;
    // Authenticate before importing native state; restore uses the same source,
    // graph/family and typed-route admission as ordinary recovery.
    let mut total = 0u64;
    for reference in &closure {
        total = total
            .checked_add(reference.bytes)
            .ok_or(Error::Association("retry transport overflow"))?;
        if total > maximum_bytes {
            return Err(Error::Association("retry transport allowance"));
        }
        records::checked_source(&source, reference)?;
        reference.verify(&source)?;
    }
    let (mut options, route, staged) = restore_evidence_request(
        &source,
        checkpoint,
        expected_source,
        maximum_bytes,
        original,
        &mut reporter,
    )?;
    resources.apply_restored(&mut options, route)?;
    records::import(&source, &target, &closure, maximum_bytes, || {
        reporter.poll(0, None)
    })?;
    // Preserve even legacy omitted configuration fields. Source restoration may
    // derive a historical represented-input policy from the authenticated request;
    // that admission must not silently become a non-resource configuration edit.
    let (mut persisted, stored_route) =
        configuration::read(&source, &checkpoint.configuration, maximum_bytes)?;
    if stored_route != route {
        return Err(Error::Association("retry restored route changed"));
    }
    resources.apply_restored(&mut persisted, route)?;
    let configuration = configuration::write(&target, &persisted, route)?;
    let lineage = codec::write(
        &target,
        "endpoint-retry",
        KIND,
        &EndpointRetryLineage {
            original: checkpoint.clone(),
            effective_configuration: configuration.clone(),
            resources: resources.clone(),
            original_directory: source_name,
            attempt_directory: target_name,
        },
        Atoms::default(),
        Vec::new(),
    )?;
    let imported_total = closure
        .iter()
        .try_fold(0u64, |n, r| n.checked_add(r.bytes))
        .ok_or(Error::Association("retry transport overflow"))?;
    let extra_configuration = if closure.contains(&configuration) {
        0
    } else {
        configuration.bytes
    };
    if imported_total
        .checked_add(extra_configuration)
        .and_then(|n| n.checked_add(lineage.bytes))
        .is_none_or(|n| n > maximum_bytes)
    {
        return Err(Error::Association(
            "retry configuration/lineage transport allowance",
        ));
    }
    let retry_checkpoint = EndpointRetryCheckpoint {
        lineage,
        preparation: PreparationCheckpoint {
            configuration: configuration.clone(),
            evidence: checkpoint.evidence.clone(),
        },
    };
    records::publish_checkpoint(&target, &retry_checkpoint)?;
    *active.borrow_mut() = Some(retry_checkpoint.clone());
    reporter.configuration = Some(configuration.clone());
    reporter.stage(Stage::Verify, 0, Some(checkpoint.evidence.cells))?;
    let remaining = maximum_bytes
        .checked_sub(configuration.bytes)
        .ok_or(Error::Association("configuration transport limit"))?;
    let verified = staged.verify_evidence(&target, &checkpoint.evidence, remaining)?;
    reporter.receipts.verification = Some(verified.observation);
    reporter.poll(checkpoint.evidence.cells, Some(checkpoint.evidence.cells))?;
    let prepared = finish(
        &target,
        "vectors",
        configuration,
        Arc::new(verified.result?),
        options,
        reporter,
        None,
    )?;
    Ok(EndpointRetryPrepared {
        prepared,
        checkpoint: retry_checkpoint,
    })
}

fn validate_lineage(
    root: &Path,
    checkpoint: &EndpointRetryCheckpoint,
    maximum_bytes: u64,
) -> Result<EndpointRetryLineage> {
    if checkpoint.lineage.bytes > maximum_bytes {
        return Err(Error::Association("retry lineage transport allowance"));
    }
    records::checked_source(root, &checkpoint.lineage)?;
    checkpoint.lineage.verify(root)?;
    let (lineage, atoms, symbols): (EndpointRetryLineage, _, _) =
        codec::read(root, &checkpoint.lineage, KIND)?;
    if !atoms.atoms.is_empty()
        || !symbols.is_empty()
        || lineage.effective_configuration != checkpoint.preparation.configuration
        || lineage.original.evidence != checkpoint.preparation.evidence
    {
        return Err(Error::Association("retry checkpoint/lineage association"));
    }
    let mut closure = records::references(&lineage.original)?;
    if !closure.contains(&checkpoint.preparation.configuration) {
        closure.push(checkpoint.preparation.configuration.clone());
    }
    let mut total = checkpoint.lineage.bytes;
    for reference in closure {
        total = total
            .checked_add(reference.bytes)
            .ok_or(Error::Association("retry transport overflow"))?;
        if total > maximum_bytes {
            return Err(Error::Association("retry transport allowance"));
        }
        records::checked_source(root, &reference)?;
        reference.verify(root)?;
    }
    checkpoint.preparation.configuration.verify(root)?;
    let (mut options, route) =
        configuration::read(root, &lineage.original.configuration, maximum_bytes)?;
    lineage.resources.apply_restored(&mut options, route)?;
    let (effective, effective_route) =
        configuration::read(root, &checkpoint.preparation.configuration, maximum_bytes)?;
    if route != effective_route || !configuration::equivalent(&options, &effective, route)? {
        return Err(Error::Association(
            "retry changed non-resource configuration",
        ));
    }
    Ok(lineage)
}
impl EndpointRetryCheckpoint {
    /// Authenticate the record closure and resource-only configuration relation.
    /// This structural check does not replace native fresh proof verification.
    pub fn lineage(&self, root: &Path, maximum_bytes: u64) -> Result<EndpointRetryLineage> {
        validate_lineage(&root.canonicalize()?, self, maximum_bytes)
    }
}

/// Recover retained retry evidence. Supplying an expected prepared identity
/// additionally checks completed-publication reproduction, as ordinary resume
/// does; a checkpoint itself does not assert completion or incompleteness.
#[allow(clippy::too_many_arguments)]
pub fn resume_endpoint_retry(
    root: &Path,
    checkpoint: &EndpointRetryCheckpoint,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: Option<&str>,
    maximum_bytes: u64,
    observer: impl FnMut(&EndpointRetryProgress) -> ControlFlow<()>,
) -> Result<EndpointRetryPrepared> {
    resume_retry(
        root,
        checkpoint,
        work_directory,
        expected_source,
        expected_prepared,
        maximum_bytes,
        None,
        observer,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn resume_graph_endpoint_retry(
    original: Arc<represented::graph::GraphPoint>,
    conversion_cap: represented::Limits,
    root: &Path,
    checkpoint: &EndpointRetryCheckpoint,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: Option<&str>,
    maximum_bytes: u64,
    observer: impl FnMut(&EndpointRetryProgress) -> ControlFlow<()>,
) -> Result<EndpointRetryPrepared> {
    resume_retry(
        root,
        checkpoint,
        work_directory,
        expected_source,
        expected_prepared,
        maximum_bytes,
        Some((OriginalPoint::Graph(original), conversion_cap)),
        observer,
    )
}
#[allow(clippy::too_many_arguments)]
pub fn resume_family_endpoint_retry(
    original: Arc<represented::family::FamilyPoint>,
    conversion_cap: represented::Limits,
    root: &Path,
    checkpoint: &EndpointRetryCheckpoint,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: Option<&str>,
    maximum_bytes: u64,
    observer: impl FnMut(&EndpointRetryProgress) -> ControlFlow<()>,
) -> Result<EndpointRetryPrepared> {
    resume_retry(
        root,
        checkpoint,
        work_directory,
        expected_source,
        expected_prepared,
        maximum_bytes,
        Some((OriginalPoint::Family(original), conversion_cap)),
        observer,
    )
}
#[allow(clippy::too_many_arguments)]
fn resume_retry<F: FnMut(&EndpointRetryProgress) -> ControlFlow<()>>(
    root: &Path,
    checkpoint: &EndpointRetryCheckpoint,
    work_directory: &str,
    expected_source: &str,
    expected_prepared: Option<&str>,
    maximum_bytes: u64,
    original: Option<(OriginalPoint, represented::Limits)>,
    mut observer: F,
) -> Result<EndpointRetryPrepared> {
    let root = root.canonicalize()?;
    validate_lineage(&root, checkpoint, maximum_bytes)?;
    let prepared = recover(
        &root,
        &checkpoint.preparation,
        work_directory,
        expected_source,
        expected_prepared,
        maximum_bytes,
        original,
        &mut |progress| {
            observer(&EndpointRetryProgress {
                preparation: progress.clone(),
                checkpoint: Some(checkpoint.clone()),
            })
        },
    )?;
    Ok(EndpointRetryPrepared {
        prepared,
        checkpoint: checkpoint.clone(),
    })
}
