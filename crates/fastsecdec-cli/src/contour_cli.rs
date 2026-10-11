//! Runtime contour steering, separate from sampling and mathematical generation.
mod settings;

use clap::{Args, ValueEnum};
use fastsecdec::contour::{ContourMode, ContourSettings, ContourValidation, DynamicConstruction};

use crate::{CliResult, config::IntegrationInput};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ValidationArg {
    Always,
    Pilot,
    Off,
}

#[derive(Args, Default)]
pub(crate) struct ContourArgs {
    /// Contour prescription: fixed, off, or dynamical=<S> with 0<S<1.
    #[arg(long, value_name = "MODE")]
    contour: Option<String>,
    /// Positive fixed contour strength; never automatically reduced.
    #[arg(long, allow_hyphen_values = true)]
    lambda: Option<f64>,
    /// Positive dynamic strength cap (default 1 for a new prescription).
    #[arg(long, allow_hyphen_values = true)]
    lambda_cap: Option<f64>,
    /// Positive dynamic displacement cap (default 1 for a new prescription).
    #[arg(long, allow_hyphen_values = true)]
    displacement_cap: Option<f64>,
    /// Dynamic construction: polynomial or sign_aware (default).
    #[arg(long, value_parser = settings::parse_construction)]
    contour_construction: Option<DynamicConstruction>,
    /// Optional causal checks: always, pilot, or off (default for new settings).
    #[arg(long, value_enum)]
    contour_validation: Option<ValidationArg>,
    /// Number of independently assigned pilot points per sector.
    #[arg(long)]
    contour_pilot_points: Option<std::num::NonZeroUsize>,
    /// Optional runtime work observations: disabled (default) or aggregate.
    #[arg(long, value_parser = settings::parse_diagnostics)]
    contour_diagnostics: Option<fastsecdec::contour::ContourDiagnosticsMode>,
}

impl ContourArgs {
    pub(crate) fn load_diagnostics(
        &self,
        path: &std::path::Path,
    ) -> CliResult<(crate::artifact::Artifact, fastsecdec::kernel::KernelSet)> {
        let mut artifact = crate::artifact::Artifact::load_metadata(path)?;
        if artifact.programs.is_none() {
            return crate::artifact::Artifact::load(path);
        }
        let settings: IntegrationInput =
            serde_json::from_value(artifact.provenance.integration.clone())?;
        select_program(&mut artifact, &self.resolve(&settings, None)?)?;
        crate::artifact::Artifact::load_recipe_observed(
            path,
            fastsecdec::kernel::KernelLoadOptions::default(),
            artifact.selected_recipe(),
            |_| Ok(()),
            |_| std::ops::ControlFlow::Continue(()),
        )
    }
    /// Diagnostic commands use the artifact's recorded physical point and the
    /// same explicit contour preflight as integration. This creates no sampling
    /// session and does not contribute any observation to an integral estimate.
    pub(crate) fn prepare_diagnostics(
        &self,
        artifact: &crate::artifact::Artifact,
        kernels: &mut fastsecdec::kernel::KernelSet,
        dashboard: &mut crate::display::Dashboard,
    ) -> CliResult<Option<DiagnosticContour>> {
        let mut settings: IntegrationInput =
            serde_json::from_value(artifact.provenance.integration.clone())?;
        self.apply_integration(&mut settings)?;
        crate::bind_parameters(kernels, &settings)?;
        if kernels.contour_capable() && settings.contour.validation.policy != ContourValidation::Off
        {
            dashboard.begin_loading();
        }
        let started = std::time::Instant::now();
        let pilot =
            crate::contour_pilot::run(kernels, &settings.contour, settings.seed, None, |pilot| {
                dashboard.loading(&crate::loading::Snapshot {
                    phase: crate::loading::Phase::ContourValidation,
                    completed: Some(pilot.completed),
                    total: Some(pilot.total),
                    elapsed_seconds: started.elapsed().as_secs_f64(),
                    primary_evaluators: None,
                })?;
                if dashboard.cancelled() {
                    return Err("contour preflight cancelled".into());
                }
                Ok(())
            })?;
        Ok(
            (settings.contour.deformation != ContourMode::Off).then(|| DiagnosticContour {
                settings: settings.contour,
                pilot: pilot
                    .as_ref()
                    .map(|report| crate::contour_pilot::provenance(kernels, settings.seed, report)),
            }),
        )
    }
}

/// Choose the numerical program without binding parameters or importing any
/// Symbolica state. Legacy single-program files retain their usual admission.
pub(crate) fn select_program(
    artifact: &mut crate::artifact::Artifact,
    settings: &ContourSettings,
) -> CliResult<()> {
    let recipe = runtime_recipe(artifact.selected_recipe(), settings.deformation)?;
    if artifact.programs.is_some() {
        artifact.select_recipe(recipe)?;
    }
    Ok(())
}

fn runtime_recipe(
    saved: Option<fastsecdec::kernel::ProgramRecipe>,
    deformation: ContourMode,
) -> CliResult<fastsecdec::kernel::ProgramRecipe> {
    use fastsecdec::kernel::ProgramRecipe;
    if saved == Some(ProgramRecipe::ThresholdV1) {
        if deformation != ContourMode::Off {
            return Err("threshold-decomposition artifacts do not support contour deformation; generate a separate contour recipe for that comparison".into());
        }
        return Ok(ProgramRecipe::ThresholdV1);
    }
    Ok(deformation.program_recipe())
}

/// CLI presentation joins existing native settings and evidence without changing
/// the stable native benchmark/boundary report schemas.
pub(crate) struct DiagnosticContour {
    settings: ContourSettings,
    pilot: Option<fastsecdec::status::ContourPilotProvenance>,
}

impl DiagnosticContour {
    pub(crate) fn attach(
        self,
        kernels: &mut fastsecdec::kernel::KernelSet,
        report: &mut serde_json::Value,
        checked_work: &str,
    ) -> CliResult<()> {
        let mut checks = fastsecdec::status::ContourCheckCounters::default();
        for kernel in kernels.sectors_mut() {
            if let Some(delta) = kernel.take_contour_validation_report() {
                checks.merge(fastsecdec::status::ContourCheckCounters {
                    checked_arguments: delta.checked_arguments.try_into()?,
                    maximum_bits: delta.maximum_bits,
                })?;
            }
        }
        report["contour"] = serde_json::json!({
            "settings": self.settings,
            "pilot": self.pilot,
            "diagnostic_checks": checks,
            "checked_work": checked_work,
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct ParserProbe {
        #[command(flatten)]
        contour: ContourArgs,
    }

    #[test]
    fn contour_off_preserves_threshold_capability_and_rejects_mixed_recipes() {
        use fastsecdec::kernel::ProgramRecipe;
        assert_eq!(
            runtime_recipe(Some(ProgramRecipe::ThresholdV1), ContourMode::Off).unwrap(),
            ProgramRecipe::ThresholdV1
        );
        for mode in [
            ContourMode::Fixed { lambda: 0.1 },
            ContourMode::dynamical(0.8),
        ] {
            assert!(runtime_recipe(Some(ProgramRecipe::ThresholdV1), mode).is_err());
            assert_eq!(runtime_recipe(None, mode).unwrap(), mode.program_recipe());
        }
        for saved in [
            None,
            Some(ProgramRecipe::FixedV1),
            Some(ProgramRecipe::UndeformedV1),
        ] {
            assert_eq!(
                runtime_recipe(saved, ContourMode::Off).unwrap(),
                ProgramRecipe::UndeformedV1
            );
        }
    }

    #[test]
    fn positive_strength_has_no_upper_bound_and_validation_is_independent() {
        let args = ParserProbe::try_parse_from([
            "test",
            "--contour",
            "fixed",
            "--lambda",
            "4",
            "--contour-validation",
            "off",
        ])
        .unwrap();
        let mut settings = ContourSettings::default();
        args.contour.apply(&mut settings).unwrap();
        assert_eq!(settings.deformation, ContourMode::Fixed { lambda: 4. });
        assert_eq!(settings.validation.policy, ContourValidation::Off);
        for strength in ["0", "-1", "NaN", "inf"] {
            let args =
                ParserProbe::try_parse_from(["test", "--contour", "fixed", "--lambda", strength])
                    .unwrap();
            assert!(args.contour.apply(&mut ContourSettings::default()).is_err());
        }
    }

    #[test]
    fn pilot_override_preserves_the_configured_strength() {
        let args = ParserProbe::try_parse_from([
            "test",
            "--contour-validation",
            "pilot",
            "--contour-pilot-points",
            "31",
        ])
        .unwrap();
        let mut settings = ContourSettings {
            deformation: ContourMode::Fixed { lambda: 0.2 },
            ..Default::default()
        };
        args.contour.apply(&mut settings).unwrap();
        assert_eq!(settings.deformation, ContourMode::Fixed { lambda: 0.2 });
        assert_eq!(settings.validation.policy, ContourValidation::Pilot);
        assert_eq!(settings.validation.pilot_points, 31);
    }

    #[test]
    fn omitted_validation_keeps_new_settings_off_and_preserves_saved_policy() {
        let args =
            ParserProbe::try_parse_from(["test", "--contour", "fixed", "--lambda", "0.1"]).unwrap();
        let mut settings = ContourSettings::default();
        args.contour.apply(&mut settings).unwrap();
        assert_eq!(settings.validation.policy, ContourValidation::Off);
        settings.validation.policy = ContourValidation::Always;
        args.contour.apply(&mut settings).unwrap();
        assert_eq!(settings.validation.policy, ContourValidation::Always);
    }
}
