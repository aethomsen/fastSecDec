use serde::{Deserialize, Serialize};

/// Mathematical prescription; validation is deliberately configured separately.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum ContourMode {
    #[default]
    Off,
    Fixed {
        lambda: f64,
    },
    Dynamical {
        safety_fraction: f64,
        /// Omitted serialized limits use the approved unit caps.
        #[serde(default = "unit_cap")]
        lambda_cap: f64,
        #[serde(default = "unit_cap")]
        displacement_cap: f64,
        #[serde(default)]
        construction: DynamicConstruction,
    },
}

// Serde's internally tagged unit variant accepts trailing fields even with
// deny_unknown_fields. Use an empty struct variant at this wire boundary so
// `mode = "off"` cannot silently ignore a supplied mathematical setting.
impl<'de> Deserialize<'de> for ContourMode {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Off {},
            Fixed {
                lambda: f64,
            },
            Dynamical {
                safety_fraction: f64,
                #[serde(default = "unit_cap")]
                lambda_cap: f64,
                #[serde(default = "unit_cap")]
                displacement_cap: f64,
                #[serde(default)]
                construction: DynamicConstruction,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Off {} => Self::Off,
            Wire::Fixed { lambda } => Self::Fixed { lambda },
            Wire::Dynamical {
                safety_fraction,
                lambda_cap,
                displacement_cap,
                construction,
            } => Self::Dynamical {
                safety_fraction,
                lambda_cap,
                displacement_cap,
                construction,
            },
        })
    }
}

fn unit_cap() -> f64 {
    1.0
}

/// Mathematical radius construction, independent of artifact codec versions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DynamicConstruction {
    Polynomial,
    #[default]
    SignAware,
}

impl ContourMode {
    /// Sign-aware construction with the approved unit cap defaults.
    pub fn dynamical(safety_fraction: f64) -> Self {
        Self::Dynamical {
            safety_fraction,
            lambda_cap: 1.0,
            displacement_cap: 1.0,
            construction: DynamicConstruction::SignAware,
        }
    }

    pub fn program_recipe(self) -> crate::kernel::ProgramRecipe {
        use crate::kernel::ProgramRecipe;
        match self {
            Self::Off => ProgramRecipe::UndeformedV1,
            Self::Fixed { .. } => ProgramRecipe::FixedV1,
            Self::Dynamical {
                construction: DynamicConstruction::Polynomial,
                ..
            } => ProgramRecipe::DynamicPolynomialV1,
            Self::Dynamical {
                construction: DynamicConstruction::SignAware,
                ..
            } => ProgramRecipe::DynamicSignAwareV1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContourValidation {
    Always,
    Pilot,
    #[default]
    Off,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContourValidationOptions {
    pub policy: ContourValidation,
    pub pilot_points: usize,
}
impl Default for ContourValidationOptions {
    fn default() -> Self {
        Self {
            policy: ContourValidation::Off,
            pilot_points: 256,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ContourSettings {
    pub deformation: ContourMode,
    pub validation: ContourValidationOptions,
}
impl ContourSettings {
    /// Admission shared by native and CLI callers; no implicit strength tuning.
    pub fn validate(&self) -> Result<(), String> {
        match self.deformation {
            ContourMode::Off => {}
            ContourMode::Fixed { lambda } => {
                if !lambda.is_finite() || lambda <= 0.0 {
                    return Err("fixed contour strength must be finite and positive".into());
                }
            }
            ContourMode::Dynamical {
                safety_fraction,
                lambda_cap,
                displacement_cap,
                ..
            } => {
                if !safety_fraction.is_finite()
                    || !(0.0..1.0).contains(&safety_fraction)
                    || safety_fraction == 0.0
                {
                    return Err("dynamic contour safety fraction must be finite and strictly between zero and one".into());
                }
                if !lambda_cap.is_finite()
                    || lambda_cap <= 0.0
                    || !displacement_cap.is_finite()
                    || displacement_cap <= 0.0
                {
                    return Err("dynamic contour caps must be finite and positive".into());
                }
            }
        }
        if self.validation.policy != ContourValidation::Off && self.validation.pilot_points == 0 {
            return Err("enabled contour validation requires at least one pilot point".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn causal_validation_is_opt_in_and_explicit_policies_survive_restore() {
        assert_eq!(ContourValidation::default(), ContourValidation::Off);
        for value in [
            serde_json::json!({}),
            serde_json::json!({"deformation":{"mode":"fixed", "lambda":0.1}}),
            serde_json::json!({"deformation":{"mode":"dynamical", "safety_fraction":0.8}, "validation":{}}),
        ] {
            let settings: ContourSettings = serde_json::from_value(value).unwrap();
            assert_eq!(settings.validation.policy, ContourValidation::Off);
            settings.validate().unwrap();
        }
        for policy in [ContourValidation::Always, ContourValidation::Pilot] {
            let settings = ContourSettings {
                deformation: ContourMode::Fixed { lambda: 0.1 },
                validation: ContourValidationOptions {
                    policy,
                    pilot_points: 2,
                },
            };
            let restored: ContourSettings =
                serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
            assert_eq!(restored, settings);
            let mut invalid = restored;
            invalid.validation.pilot_points = 0;
            assert!(invalid.validate().is_err());
        }
        let mut off = ContourSettings::default();
        off.validation.pilot_points = 0;
        off.validate().unwrap();
    }
    #[test]
    fn dynamic_defaults_preserve_math_recipe_and_reject_invalid_limits() {
        let mode: ContourMode = serde_json::from_value(serde_json::json!({
            "mode":"dynamical", "safety_fraction":0.8
        }))
        .unwrap();
        assert_eq!(mode, ContourMode::dynamical(0.8));
        assert_eq!(
            mode.program_recipe(),
            crate::kernel::ProgramRecipe::DynamicSignAwareV1
        );
        for malformed in [
            serde_json::json!({"mode":"dynamical", "safety_fraction":0.8, "lamba_cap":2}),
            serde_json::json!({"mode":"off", "lambda":0.1}),
            serde_json::json!({"mode":"fixed", "lambda":0.1,"safety_fraction":0.8}),
        ] {
            assert!(
                serde_json::from_value::<ContourMode>(malformed.clone()).is_err(),
                "{malformed}"
            );
        }
        for (safety, cap, displacement) in [
            (0.0, 1.0, 1.0),
            (1.0, 1.0, 1.0),
            (f64::NAN, 1.0, 1.0),
            (0.8, 0.0, 1.0),
            (0.8, 1.0, f64::INFINITY),
        ] {
            let settings = ContourSettings {
                deformation: ContourMode::Dynamical {
                    safety_fraction: safety,
                    lambda_cap: cap,
                    displacement_cap: displacement,
                    construction: DynamicConstruction::Polynomial,
                },
                ..Default::default()
            };
            assert!(settings.validate().is_err());
        }
    }
}
