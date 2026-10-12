//! Synchronous native threshold calls, exclusively inside recyclable CLI children.
pub(in super::super) mod retry;
use super::*;
use fastsecdec::{
    kernel::{ThresholdCompilationWork, ThresholdPublicationPlan, ThresholdWorkReceipt},
    threshold::generation as threshold,
};
use std::{io::Read, sync::Arc};

pub(in super::super) const TRANSPORT_BYTES: u64 = 1024 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PreparedThreshold {
    pub attempt: String,
    pub directory: String,
    pub native: threshold::PreparationReceipt,
    pub provenance: Provenance,
    pub generation: GenerationRecord,
    pub timings: GenerationTimings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<threshold::EndpointRetryCheckpoint>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CompiledThreshold {
    pub directory: String,
    pub data: PathBuf,
    pub receipt: ThresholdWorkReceipt,
    pub compilation_seconds: f64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[derive(Clone, Debug)]
struct Checkpoint {
    directory: String,
    native: threshold::PreparationCheckpoint,
    provenance: Provenance,
    build: String,
    // Bind raw evidence to the issued scientific card plus CLI overrides.
    // Source identity alone deliberately does not identify compiler policy.
    issued_settings: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    retry: Option<threshold::EndpointRetryCheckpoint>,
}
fn issued_settings(
    input: &Path,
    overrides: crate::config::GenerationOverrides,
) -> CliResult<String> {
    let card = artifact::SourceFingerprint::RunCardScientificInput.hash(&fs::read(input)?)?;
    Ok(blake3::hash(&serde_json::to_vec(&(
        "threshold-cli-issued-settings-v1",
        card,
        overrides,
    ))?)
    .to_hex()
    .to_string())
}
pub(in super::super) fn rooted(root: &Path, directory: &str) -> CliResult<PathBuf> {
    let path = Path::new(directory);
    if directory.is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err("invalid threshold staging directory".into());
    }
    Ok(root.join(path))
}
pub(in super::super) fn publication(
    root: &Path,
    prepared: &PreparedThreshold,
) -> CliResult<ThresholdPublicationPlan> {
    let directory = rooted(root, &prepared.directory)?;
    Ok(ThresholdPublicationPlan::from_trusted_preparer(
        &rooted(&directory, &prepared.native.work_directory)?,
        prepared.native.publication.clone(),
        &prepared.native.publication.source_identity,
        &prepared.native.publication.prepared_identity,
        TRANSPORT_BYTES,
    )?)
}
pub(in super::super) fn validate_prepared(
    root: &Path,
    prepared: &PreparedThreshold,
) -> CliResult<()> {
    let directory = rooted(root, &prepared.directory)?;
    let checkpoint = prepared.native.checkpoint()?;
    if let Some(retry) = &prepared.retry {
        if retry.preparation.configuration != checkpoint.configuration
            || retry.preparation.evidence != checkpoint.evidence
        {
            return Err("threshold retry/native response identity mismatch".into());
        }
        retry.lineage.verify(&directory)?;
    }
    if prepared.native.strategy != fastsecdec::threshold::ThresholdStrategy::GcadFirst
        || checkpoint.evidence.request.source_identity
            != prepared.native.publication.source_identity
        || checkpoint.evidence.cells != prepared.native.cells
        || prepared.native.evidence.verification.is_none()
    {
        return Err("threshold preparation scope/evidence mismatch".into());
    }
    checkpoint.configuration.verify(&directory)?;
    checkpoint.evidence.request.record.verify(&directory)?;
    checkpoint.evidence.record.verify(&directory)?;
    prepared
        .native
        .evidence
        .verification
        .as_ref()
        .unwrap()
        .record
        .verify(&directory)?;
    let plan = publication(root, prepared)?;
    if plan.original_dimensions() != prepared.native.original_dimensions
        || !plan
            .integration_dimensions()
            .eq(std::iter::once(prepared.native.integration_dimensions))
        || plan.cell_count() != prepared.native.cells
        || plan.endpoint_chart_count() != prepared.native.endpoint_charts
    {
        return Err("threshold preparation dimensions/cell/chart inventory changed".into());
    }
    if plan.job_count() != prepared.native.jobs {
        return Err("threshold preparation job inventory changed".into());
    }
    Ok(())
}
pub(in super::super) fn validate_compiled(compiled: &CompiledThreshold) -> CliResult<()> {
    let record = compiled.receipt.record();
    let mut file = File::open(&compiled.data)?;
    if file.metadata()?.len() != record.length {
        return Err("threshold program length mismatch".into());
    }
    let mut hash = blake3::Hasher::new();
    let mut bytes = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut bytes)?;
        if n == 0 {
            break;
        }
        hash.update(&bytes[..n]);
    }
    if hash.finalize().to_hex().as_str() != record.digest {
        return Err("threshold program checksum mismatch".into());
    }
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare(
    root: &Path,
    input: &Path,
    workers: usize,
    overrides: crate::config::GenerationOverrides,
    attempt: &str,
    prior: Option<&PreparedThreshold>,
    emit: &mut dyn FnMut(GenerationSnapshot) -> std::io::Result<()>,
) -> CliResult<Response> {
    prepare_with_retry(root, input, workers, overrides, attempt, prior, None, emit)
}
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_with_retry(
    root: &Path,
    input: &Path,
    workers: usize,
    overrides: crate::config::GenerationOverrides,
    attempt: &str,
    prior: Option<&PreparedThreshold>,
    retry: Option<&retry::RetryAttempt>,
    emit: &mut dyn FnMut(GenerationSnapshot) -> std::io::Result<()>,
) -> CliResult<Response> {
    if attempt.is_empty()
        || !attempt
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err("invalid threshold preparer attempt".into());
    }
    let loaded = input::load_observed_with_overrides(input, overrides, |_| Ok(()))?;
    if !loaded.card.generation.threshold_enabled() {
        return Err("threshold preparer requires explicit threshold capability".into());
    }
    if !loaded.runtime_mass_constraints.is_empty() {
        return Err(
            "threshold CLI runtime model masses are not supported; use fixed model masses".into(),
        );
    }
    let (generation, mut generation_record, provenance, mut timings) =
        preparation::context(&loaded, workers)?;
    let source_identity =
        generation::source_identity(&loaded.integrand, &loaded.runtime_parameters, &[])?;
    let issued_settings = issued_settings(input, overrides)?;
    if let Some(retry) = retry {
        retry.validate(input, &issued_settings, &source_identity)?;
    }
    let checkpoint_path = root.join("threshold-checkpoint.json");
    let saved_checkpoint: Option<Checkpoint> = if prior.is_none() && checkpoint_path.exists() {
        Some(serde_json::from_reader(File::open(&checkpoint_path)?)?)
    } else {
        None
    };
    if let Some(checkpoint) = &saved_checkpoint {
        if checkpoint.issued_settings != issued_settings {
            return Err("threshold checkpoint issued settings changed".into());
        }
        if checkpoint.build != crate::process::child::build_identity()
            || checkpoint.native.evidence.request.source_identity != source_identity
        {
            return Err("threshold checkpoint build/source changed".into());
        }
        super::super::journal::validate_provenance(input, &checkpoint.provenance)?;
    }
    if let Some(prior) = prior {
        if prior.native.publication.source_identity != source_identity {
            return Err("threshold anchor source changed".into());
        }
        super::super::journal::validate_provenance(input, &prior.provenance)?;
    }
    if saved_checkpoint.as_ref().is_some_and(|p| p.retry.is_some())
        != (retry.is_some() && saved_checkpoint.is_some())
        || prior.is_some_and(|p| p.retry.is_some()) != (retry.is_some() && prior.is_some())
    {
        return Err("endpoint retry/ordinary checkpoint capability changed".into());
    }
    let directory = prior
        .map(|p| p.directory.clone())
        .or_else(|| saved_checkpoint.as_ref().map(|p| p.directory.clone()))
        .unwrap_or_else(|| {
            if retry.is_some() {
                "threshold-retry".into()
            } else {
                format!("threshold-{attempt}")
            }
        });
    let stage_root = rooted(root, &directory)?;
    let mut options = threshold::PreparationOptions::new(input::symbol(&format!(
        "fastsecdec_threshold::unit_{}",
        attempt.replace('-', "_")
    ))?);
    options.threshold = loaded
        .card
        .threshold_decomposition
        .native(&loaded.runtime_parameters)?;
    // Source/domain classification and roles remain native, shared with HEPKit.
    options.enable_automatic_endpoint_routes();
    if let Some(resources) = &loaded.card.threshold_decomposition.endpoint_resources {
        options.apply_endpoint_resources(resources)?;
    }
    options.generation = generation;
    options.compilation = loaded.card.generation.evaluator;
    let start = Instant::now();
    let mut error = None;
    let mut gate = ProgressGate::default();
    let mut last_checkpoint = None;
    let active_retry = std::cell::RefCell::new(
        prior
            .and_then(|p| p.retry.clone())
            .or_else(|| saved_checkpoint.as_ref().and_then(|p| p.retry.clone())),
    );
    let mut observe = |p: &threshold::Progress| {
        if let Some(checkpoint) = p.checkpoint()
            && (retry.is_none() || active_retry.borrow().is_some())
        {
            let saved = (|| -> CliResult<()> {
                let bytes = serde_json::to_vec(&Checkpoint {
                    directory: directory.clone(),
                    native: checkpoint,
                    provenance: provenance.clone(),
                    build: crate::process::child::build_identity(),
                    issued_settings: issued_settings.clone(),
                    retry: active_retry.borrow().clone(),
                })?;
                if last_checkpoint.as_ref() != Some(&bytes) {
                    artifact::atomic_write(&checkpoint_path, &bytes)?;
                    last_checkpoint = Some(bytes);
                }
                Ok(())
            })();
            match saved {
                Ok(()) => {}
                Err(e) => {
                    error = Some(e.to_string());
                    return ControlFlow::Break(());
                }
            }
        }
        let stage = match p.stage {
            threshold::Stage::Input => GenerationStage::Input,
            threshold::Stage::Solve | threshold::Stage::Verify => GenerationStage::Geometry,
            threshold::Stage::Regularize => GenerationStage::Mapping,
            threshold::Stage::Continue => GenerationStage::Subtraction,
            threshold::Stage::Bind => GenerationStage::Expansion,
            threshold::Stage::StageVectors => GenerationStage::CoefficientExpansion,
            threshold::Stage::Complete => GenerationStage::Compilation,
        };
        if !gate.accept(stage, start.elapsed()) {
            return ControlFlow::Continue(());
        }
        let snapshot = GenerationSnapshot {
            stage,
            completed: p.completed,
            total: p.total,
            sectors: 0,
            kernels: 0,
            elapsed_seconds: p.elapsed_seconds,
            timings: timings.clone(),
            coefficient_expansion: None,
            formula_preparation: None,
            detail: match (p.stage, p.regular_family) {
                (threshold::Stage::Regularize, Some(d)) => format!(
                    "Threshold Regularize · dyadic level {} · {} endpoint checks · {} family attempts",
                    d.dyadic_level, d.endpoint_queries, d.family_attempts
                ),
                _ => format!("Threshold {:?} · {} completed", p.stage, p.completed),
            },
        };
        if let Err(e) = emit(snapshot) {
            error = Some(e.to_string());
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let prepared = if let Some(retry) = retry {
        let mut retry_observe = |p: &threshold::EndpointRetryProgress| {
            *active_retry.borrow_mut() = p.checkpoint.clone();
            observe(&p.preparation)
        };
        let restored = prior
            .and_then(|p| p.retry.as_ref())
            .or_else(|| saved_checkpoint.as_ref().and_then(|p| p.retry.as_ref()));
        let result = if let Some(checkpoint) = restored {
            retry.bind(&stage_root, checkpoint)?;
            threshold::resume_endpoint_retry(
                &stage_root,
                checkpoint,
                &format!("vectors-{attempt}"),
                &source_identity,
                prior.map(|p| p.native.publication.prepared_identity.as_str()),
                TRANSPORT_BYTES,
                &mut retry_observe,
            )
        } else {
            // The native sidecar is durable before the frontend status checkpoint;
            // recover it after a process loss in that narrow interval.
            let sidecar = stage_root.join("endpoint-retry-checkpoint.json");
            if sidecar.is_file() {
                let checkpoint = serde_json::from_reader(File::open(sidecar)?)?;
                retry.bind(&stage_root, &checkpoint)?;
                threshold::resume_endpoint_retry(
                    &stage_root,
                    &checkpoint,
                    &format!("vectors-{attempt}"),
                    &source_identity,
                    None,
                    TRANSPORT_BYTES,
                    &mut retry_observe,
                )
            } else {
                threshold::retry_endpoint_evidence(
                    &retry.evidence_root,
                    &retry.checkpoint.native,
                    &source_identity,
                    &retry.resources,
                    &stage_root,
                    TRANSPORT_BYTES,
                    &mut retry_observe,
                )
            }
        };
        result.map(|result| {
            *active_retry.borrow_mut() = Some(result.checkpoint);
            result.prepared
        })
    } else if let Some(prior) = prior {
        threshold::resume(
            &stage_root,
            &prior.native,
            &format!("vectors-{attempt}"),
            &source_identity,
            &prior.native.publication.prepared_identity,
            TRANSPORT_BYTES,
            &mut observe,
        )
    } else if let Some(checkpoint) = saved_checkpoint {
        threshold::resume_evidence(
            &stage_root,
            &checkpoint.native,
            &format!("vectors-{attempt}"),
            &source_identity,
            TRANSPORT_BYTES,
            &mut observe,
        )
    } else {
        fs::create_dir(&stage_root)?;
        threshold::prepare(
            Arc::new(loaded.integrand),
            options,
            &stage_root,
            &mut observe,
        )
    };
    if let Some(error) = error {
        return Err(error.into());
    }
    let prepared = prepared?;
    if let Some(retry) = retry {
        let active = active_retry.borrow();
        generation_record.endpoint_retry = Some(retry.bind(
            &stage_root,
            active.as_ref().ok_or("missing completed retry lineage")?,
        )?);
    }
    if prepared.receipt.publication.source_identity != source_identity {
        return Err("native threshold result source changed".into());
    }
    for row in &prepared.receipt.timings {
        match row.stage {
            threshold::Stage::Input | threshold::Stage::Solve | threshold::Stage::Verify => {
                timings.geometry_seconds += row.seconds
            }
            threshold::Stage::Regularize => timings.mapping_seconds += row.seconds,
            threshold::Stage::Continue => timings.subtraction_seconds += row.seconds,
            threshold::Stage::Bind | threshold::Stage::StageVectors => {
                timings.coefficient_expansion_seconds += row.seconds
            }
            threshold::Stage::Complete => {}
        }
    }
    timings.total_seconds =
        timings.input_seconds + timings.parametrization_seconds + start.elapsed().as_secs_f64();
    Ok(Response::PreparedThreshold(Box::new(PreparedThreshold {
        attempt: attempt.into(),
        directory,
        native: prepared.receipt,
        provenance,
        generation: generation_record,
        timings,
        retry: active_retry.into_inner(),
    })))
}
pub(super) fn compile(
    root: &Path,
    directory: &str,
    work: ThresholdCompilationWork,
    output: &Path,
    emit: &mut dyn FnMut(GenerationSnapshot) -> std::io::Result<()>,
) -> CliResult<Response> {
    let directory_path = rooted(root, directory)?;
    let start = Instant::now();
    emit(GenerationSnapshot {
        stage: GenerationStage::Compilation,
        completed: work.index(),
        total: None,
        sectors: 0,
        kernels: 0,
        elapsed_seconds: 0.,
        timings: Default::default(),
        coefficient_expansion: None,
        formula_preparation: None,
        detail: format!("Threshold contribution {}", work.index()),
    })?;
    let temporary = output.with_extension("writing");
    let result = (|| -> CliResult<ThresholdWorkReceipt> {
        let mut writer = BufWriter::new(File::create(&temporary)?);
        let receipt = work.compile_record(&directory_path, TRANSPORT_BYTES, &mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        fs::rename(&temporary, output)?;
        File::open(output.parent().ok_or("threshold output parent")?)?.sync_all()?;
        Ok(receipt)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    Ok(Response::CompiledThreshold(Box::new(CompiledThreshold {
        directory: directory.into(),
        data: output.into(),
        receipt: result?,
        compilation_seconds: start.elapsed().as_secs_f64(),
    })))
}
