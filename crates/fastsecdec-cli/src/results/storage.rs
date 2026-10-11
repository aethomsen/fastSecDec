use fastsecdec::{
    integration::Tolerance,
    reference::{ReferenceProvenance, ReferenceValidation},
    results::{KernelResultManifest, ResultTimings, SavedIntegrationResult},
};

use crate::{
    CliResult, artifact::Artifact, config::IntegrationInput, driver::IntegrationReport,
    reference::PreparedReference,
};

/// Result assembly from compact metadata; serial callers need no resident evaluators.
pub fn assemble_manifest(
    artifact: &Artifact,
    manifest: KernelResultManifest,
    settings: &IntegrationInput,
    report: &IntegrationReport,
    reference: Option<&PreparedReference>,
) -> CliResult<SavedIntegrationResult> {
    let mut provenance = ReferenceProvenance::new(
        "FastSecDec native integration",
        format!(
            "{}; measure multiplier {}; dimension {}",
            artifact.provenance.measure,
            artifact.provenance.measure_multiplier,
            artifact.provenance.dimension,
        ),
    );
    provenance.engine = Some(format!("FastSecDec {}", env!("CARGO_PKG_VERSION")));
    provenance.attributes.insert(
        "artifact_content_id".into(),
        artifact.content_id.clone().into(),
    );
    provenance.attributes.insert(
        "artifact_provenance".into(),
        serde_json::to_value(&artifact.provenance)?,
    );
    if artifact.programs.is_some() {
        provenance.attributes.insert(
            "selected_recipe".into(),
            serde_json::to_value(artifact.selected_recipe())?,
        );
        provenance.attributes.insert(
            "program_archive_identity".into(),
            artifact.kernel_content_id.clone().into(),
        );
    }
    provenance
        .attributes
        .insert("caller_settings".into(), serde_json::to_value(settings)?);
    provenance.attributes.insert(
        "operational_timings".into(),
        serde_json::to_value(&report.operational)?,
    );
    if let Some(total) = &report.contributions.total {
        let norms: std::collections::BTreeMap<_, _> = total
            .orders
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .map(|order| (order, total.complex_norm_uncertainty(order)))
            .collect();
        provenance.attributes.insert(
            "complex_norm_uncertainty".into(),
            serde_json::to_value(norms)?,
        );
    }
    provenance.attributes.insert(
        "process_cpu_seconds".into(),
        serde_json::to_value(report.process_cpu_seconds)?,
    );
    provenance.attributes.insert(
        "resume_status".into(),
        serde_json::to_value(&report.resume_status)?,
    );
    if let Some(contour) = &report.contour {
        provenance
            .attributes
            .insert("contour".into(), serde_json::to_value(contour)?);
    }
    if let Some(timings) = &report.generation_timings {
        provenance
            .attributes
            .insert("generation_timings".into(), serde_json::to_value(timings)?);
    }
    if let Some(timed_stop) = &report.timed_stop {
        provenance
            .attributes
            .insert("timed_stop".into(), serde_json::to_value(timed_stop)?);
    }
    if let Some(reference) = reference {
        provenance.attributes.insert(
            "reference_file".into(),
            serde_json::to_value(reference.source())?,
        );
    }
    let stored_reference = reference
        .map(|reference| reference.stored(&manifest.kernel_content_id))
        .transpose()?;
    let result = SavedIntegrationResult {
        manifest,
        scope: settings.scope.clone(),
        contributions: report.contributions.clone(),
        weight_diagnostics: report.weight_diagnostics.clone(),
        stopping_reason: report
            .snapshot
            .stop_reason
            .clone()
            .ok_or("integration result has no typed stopping reason")?,
        requested_tolerance: Some(Tolerance::new(
            settings.absolute_tolerance,
            settings.relative_tolerance,
        )?),
        requested_accuracy_target: settings.accuracy_target,
        evaluation_diagnostics: report.snapshot.evaluation_diagnostics.clone(),
        qmc_design: report.qmc_design.clone(),
        provenance,
        validation: ReferenceValidation::Unverified,
        stored_reference,
        timings: ResultTimings {
            elapsed_seconds: Some(report.elapsed_seconds),
            artifact_load_seconds: Some(report.loading_seconds),
        },
    };
    result.validate()?;
    Ok(result)
}

fn destination(path: &Path, create_parent: bool) -> CliResult<PathBuf> {
    if let Ok(path) = fs::canonicalize(path) {
        return Ok(path);
    }
    let name = path
        .file_name()
        .ok_or("result output requires a file name")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if create_parent {
        fs::create_dir_all(parent)?;
    }
    Ok(fs::canonicalize(parent)
        .or_else(|_| std::path::absolute(parent))?
        .join(name))
}

pub(super) fn protect_output<'a>(
    path: &Path,
    protected: impl IntoIterator<Item = &'a Path>,
) -> CliResult<()> {
    let target = destination(path, true)?;
    for source in protected {
        if target == destination(source, false)? {
            return Err(
                "result output must differ from its inputs, artifact and checkpoint".into(),
            );
        }
    }
    Ok(())
}
use std::{
    fs,
    path::{Path, PathBuf},
};
