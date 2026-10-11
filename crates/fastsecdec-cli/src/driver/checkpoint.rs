use super::{ResumeStatus, replay::AcceptedReplay};
use crate::{
    CliResult,
    artifact::{Artifact, atomic_write},
    config::IntegrationInput,
};
use fastsecdec::{
    integration::mc::HavanaSession,
    status::{ContourCheckpointProvenance, EvaluationDiagnostics, IntegrationStage},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize)]
pub(super) struct Checkpoint {
    format_version: u32,
    content_id: String,
    /// Indexed artifacts separate mathematical input and evaluator layout IDs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    kernel_content_id: Option<String>,
    settings: serde_json::Value,
    pub(super) round_index: usize,
    session: serde_json::Value,
    #[serde(default)]
    diagnostics: EvaluationDiagnostics,
    replay: AcceptedReplay,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    previous_complete: Option<super::refinement::PreviousProduction>,
    /// Observational evidence; deliberately excluded from settings identity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    contour: Option<ContourCheckpointProvenance>,
}

pub(super) struct RestoredCheckpoint {
    pub round_index: usize,
    pub diagnostics: EvaluationDiagnostics,
    pub replay: AcceptedReplay,
    pub session: Vec<u8>,
    pub previous_complete: Option<super::refinement::PreviousProduction>,
    pub contour: Option<ContourCheckpointProvenance>,
}

pub(super) fn settings_identity(settings: &IntegrationInput) -> CliResult<serde_json::Value> {
    let mut value = serde_json::to_value(settings)?;
    value.as_object_mut().unwrap().remove("workers");
    value.as_object_mut().unwrap().remove("max_seconds");
    value.as_object_mut().unwrap().remove("contour_diagnostics");
    value
        .as_object_mut()
        .unwrap()
        .remove("evaluation_batch_size");
    // Checking a contour does not change its mathematical prescription or the
    // production streams. Checked pilots may precede unchecked continuation.
    if let Some(contour) = value
        .get_mut("contour")
        .and_then(serde_json::Value::as_object_mut)
    {
        contour.remove("validation");
        if settings.contour.deformation == fastsecdec::contour::ContourMode::Off {
            value.as_object_mut().unwrap().remove("contour");
        }
    }
    // Ordinary historical checkpoints serialized the implicit one-round limit.
    // Artifact settings preserve omission so a later serial run can be unlimited.
    if settings.serial_seconds.is_none() {
        value["max_rounds"] = settings.ordinary_max_rounds().into();
    }
    Ok(value)
}

#[cfg(test)]
#[test]
fn invocation_deadline_does_not_change_checkpoint_statistical_identity() {
    let baseline = IntegrationInput::default();
    let mut timed = baseline.clone();
    timed.max_seconds = Some(600.);
    assert_eq!(
        settings_identity(&baseline).unwrap(),
        settings_identity(&timed).unwrap()
    );
}

#[cfg(test)]
pub(super) fn save_checkpoint(
    path: &Path,
    artifact: &Artifact,
    settings: &IntegrationInput,
    round: usize,
    session: Vec<u8>,
    diagnostics: &EvaluationDiagnostics,
    replay: &AcceptedReplay,
) -> CliResult<()> {
    save_checkpoint_with_previous(
        path,
        artifact,
        settings,
        round,
        session,
        diagnostics,
        replay,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn save_checkpoint_with_previous(
    path: &Path,
    artifact: &Artifact,
    settings: &IntegrationInput,
    round: usize,
    session: Vec<u8>,
    diagnostics: &EvaluationDiagnostics,
    replay: &AcceptedReplay,
    previous_complete: Option<&super::refinement::PreviousProduction>,
    contour: Option<&ContourCheckpointProvenance>,
) -> CliResult<()> {
    atomic_write(
        path,
        &serde_json::to_vec(&Checkpoint {
            format_version: 3,
            content_id: artifact.content_id.clone(),
            kernel_content_id: artifact
                .catalogue()
                .map(|_| artifact.kernel_content_id.clone()),
            settings: settings_identity(settings)?,
            round_index: round,
            session: serde_json::from_slice(&session)?,
            diagnostics: diagnostics.clone(),
            replay: replay.clone(),
            previous_complete: previous_complete.cloned(),
            contour: contour.cloned(),
        })?,
    )
}

pub(super) fn restore_checkpoint(
    path: &Path,
    artifact: &Artifact,
    settings: &IntegrationInput,
) -> CliResult<RestoredCheckpoint> {
    let value: serde_json::Value = serde_json::from_slice(&fs::read(path)?)?;
    if value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        != Some(3)
    {
        return Err("unsupported checkpoint version; restart integration to create a weighted-replay checkpoint".into());
    }
    let checkpoint: Checkpoint = serde_json::from_value(value)?;
    let mut historical: IntegrationInput = serde_json::from_value(checkpoint.settings.clone())?;
    if checkpoint.settings.get("stability").is_none() {
        historical.stability = fastsecdec::kernel::StabilitySettings::validated();
    }
    if checkpoint.content_id != artifact.content_id
        || checkpoint.kernel_content_id.as_deref()
            != artifact
                .catalogue()
                .map(|_| artifact.kernel_content_id.as_str())
        || settings_identity(&historical)? != settings_identity(settings)?
        || checkpoint.round_index >= settings.ordinary_max_rounds()
    {
        return Err("checkpoint input identity or integration settings differ; only the worker count, evaluation batch size, contour validation policy and contour diagnostics may change during resume".into());
    }
    Ok(RestoredCheckpoint {
        round_index: checkpoint.round_index,
        diagnostics: checkpoint.diagnostics,
        replay: checkpoint.replay,
        session: serde_json::to_vec(&checkpoint.session)?,
        previous_complete: checkpoint.previous_complete,
        contour: checkpoint.contour,
    })
}

#[cfg(test)]
pub(super) fn save_mc_checkpoint(
    path: &Path,
    artifact: &Artifact,
    settings: &IntegrationInput,
    round: usize,
    session: &HavanaSession,
    diagnostics: &EvaluationDiagnostics,
    replay: &AcceptedReplay,
) -> CliResult<ResumeStatus> {
    save_mc_checkpoint_with_previous(
        path,
        artifact,
        settings,
        round,
        session,
        diagnostics,
        replay,
        None,
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn save_mc_checkpoint_with_previous(
    path: &Path,
    artifact: &Artifact,
    settings: &IntegrationInput,
    round: usize,
    session: &HavanaSession,
    diagnostics: &EvaluationDiagnostics,
    replay: &AcceptedReplay,
    previous_complete: Option<&super::refinement::PreviousProduction>,
    contour: Option<&ContourCheckpointProvenance>,
) -> CliResult<ResumeStatus> {
    if session.stage() == IntegrationStage::Pilot {
        return Ok(ResumeStatus::PilotRestartRequired);
    }
    save_checkpoint_with_previous(
        path,
        artifact,
        settings,
        round,
        session.checkpoint()?,
        diagnostics,
        replay,
        previous_complete,
        contour,
    )?;
    Ok(ResumeStatus::CheckpointSaved)
}
