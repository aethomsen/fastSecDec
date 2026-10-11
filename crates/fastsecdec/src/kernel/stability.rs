//! Runtime routing policy. Endpoint distances are heuristics, not error bounds.
use super::{KernelError, cancellation::Cancellation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use symbolica::{
    atom::{Atom, AtomView},
    coefficient::Coefficient,
};

pub const ARBITRARY_DECIMAL_DIGITS: u32 = 1000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecisionClass {
    #[default]
    F64,
    DoubleFloat,
    Arbitrary,
    Unstable,
}
impl PrecisionClass {
    pub fn bits(self) -> u32 {
        match self {
            Self::F64 => 53,
            Self::DoubleFloat => 106,
            Self::Arbitrary => symbolica::domains::float::Float::decimal_digits_to_bits(f64::from(
                ARBITRARY_DECIMAL_DIGITS,
            ))
            .expect("fixed supported decimal precision"),
            Self::Unstable => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StabilityMode {
    #[default]
    Distance,
    Validated,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StabilityLevel {
    pub precision: PrecisionClass,
    pub minimum_effective_distance: f64,
    #[serde(default)]
    pub power_thresholds: BTreeMap<String, f64>,
    #[serde(default)]
    pub escalate_for_large_weight_threshold: Option<f64>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StabilitySettings {
    pub mode: StabilityMode,
    pub levels: Vec<StabilityLevel>,
    pub unstable_cutoff: Option<f64>,
    pub unstable_power_thresholds: BTreeMap<String, f64>,
}
impl Default for StabilitySettings {
    fn default() -> Self {
        Self {
            mode: StabilityMode::Distance,
            levels: [
                (PrecisionClass::F64, 1e-3, Some(0.9)),
                (PrecisionClass::DoubleFloat, 1e-8, None),
                (PrecisionClass::Arbitrary, 0.0, None),
            ]
            .into_iter()
            .map(
                |(precision, minimum_effective_distance, escalate_for_large_weight_threshold)| {
                    StabilityLevel {
                        precision,
                        minimum_effective_distance,
                        power_thresholds: BTreeMap::new(),
                        escalate_for_large_weight_threshold,
                    }
                },
            )
            .collect(),
            unstable_cutoff: None,
            unstable_power_thresholds: BTreeMap::new(),
        }
    }
}
impl StabilitySettings {
    pub fn validated() -> Self {
        Self {
            mode: StabilityMode::Validated,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), KernelError> {
        let expected = [
            PrecisionClass::F64,
            PrecisionClass::DoubleFloat,
            PrecisionClass::Arbitrary,
        ];
        if self.levels.len() != 3
            || self
                .levels
                .iter()
                .zip(expected)
                .any(|(level, precision)| level.precision != precision)
        {
            return Err(invalid(
                "levels must be ordered f64, double_float, arbitrary",
            ));
        }
        let mut powers = std::collections::BTreeSet::new();
        for (index, level) in self.levels.iter().enumerate() {
            if index < 2 {
                threshold(level.minimum_effective_distance)?;
            } else if level.minimum_effective_distance != 0.0
                || !level.power_thresholds.is_empty()
                || level.escalate_for_large_weight_threshold.is_some()
            {
                return Err(invalid(
                    "the final arbitrary level must have distance zero and no escalation threshold",
                ));
            }
            if let Some(value) = level.escalate_for_large_weight_threshold
                && (!value.is_finite() || value <= 0.0)
            {
                return Err(invalid(
                    "large-weight fractions must be positive and finite, or omitted",
                ));
            }
            for (power, value) in &level.power_thresholds {
                canonical_power(power)?;
                threshold(*value)?;
                powers.insert(power.clone());
            }
        }
        if let Some(value) = self.unstable_cutoff {
            threshold(value)?;
        }
        for (power, value) in &self.unstable_power_thresholds {
            canonical_power(power)?;
            threshold(*value)?;
            powers.insert(power.clone());
        }
        for power in std::iter::once(None).chain(powers.iter().map(|p| Some(p.as_str()))) {
            let a = lookup(
                &self.levels[0].power_thresholds,
                power,
                self.levels[0].minimum_effective_distance,
            );
            let b = lookup(
                &self.levels[1].power_thresholds,
                power,
                self.levels[1].minimum_effective_distance,
            );
            let cutoff = power
                .and_then(|p| self.unstable_power_thresholds.get(p).copied())
                .or(self.unstable_cutoff);
            if b > a || cutoff.is_some_and(|c| c > b) {
                return Err(invalid(
                    "distance thresholds must be nested: cutoff <= double_float <= f64",
                ));
            }
        }
        Ok(())
    }
    pub(super) fn has_overrides(&self) -> bool {
        !self.unstable_power_thresholds.is_empty()
            || self
                .levels
                .iter()
                .any(|level| !level.power_thresholds.is_empty())
    }
}
fn invalid(message: &str) -> KernelError {
    KernelError::Stability(message.into())
}
fn threshold(value: f64) -> Result<(), KernelError> {
    if value.is_finite() && value > 0.0 && value < 1.0 {
        Ok(())
    } else {
        Err(invalid(
            "effective distances must be finite and strictly between zero and one",
        ))
    }
}
pub(super) fn canonical_power(value: &str) -> Result<String, KernelError> {
    let atom = Atom::parse(value, "fastsecdec::stability", Default::default())
        .map_err(|_| invalid("power keys must be positive exact rational numbers"))?;
    let AtomView::Num(number) = atom.as_view() else {
        return Err(invalid(
            "power keys must be positive exact rational numbers",
        ));
    };
    let Coefficient::Complex(number) = number.get_coeff_view().to_owned() else {
        return Err(invalid("power keys must be exact rational numbers"));
    };
    if !number.im.is_zero() {
        return Err(invalid("power keys must be real"));
    }
    let power = number.re;
    if power <= 0 || power.to_string() != value {
        return Err(invalid(
            "power keys must use canonical positive rational spelling, e.g. 1 or 3/2",
        ));
    }
    Ok(power.to_string())
}
fn lookup(map: &BTreeMap<String, f64>, power: Option<&str>, fallback: f64) -> f64 {
    power.and_then(|p| map.get(p).copied()).unwrap_or(fallback)
}

#[derive(Clone, Default)]
pub(super) struct Routing {
    risks: [Vec<Vec<f64>>; 3],
}
impl Routing {
    pub(super) fn new(
        cancellation: &Cancellation,
        settings: &StabilitySettings,
        powers: Option<&[Option<String>]>,
    ) -> Result<Self, KernelError> {
        settings.validate()?;
        if settings.mode == StabilityMode::Validated {
            return Ok(Self::default());
        }
        if settings.has_overrides()
            && powers.is_none()
            && cancellation.endpoint_profiles().is_none()
        {
            return Err(invalid(
                "per-power thresholds require unambiguous original singularity powers in retained metadata; regenerate this artifact",
            ));
        }
        if settings.has_overrides()
            && let Some(profiles) = cancellation.endpoint_profiles()
        {
            let risks = std::array::from_fn(|tier| {
                profiles
                    .iter()
                    .map(|row| {
                        row.axes
                            .iter()
                            .map(|axis| {
                                axis.iter()
                                    .map(|source| {
                                        let power = Some(source.original_power.as_str());
                                        let threshold = if tier < 2 {
                                            Some(lookup(
                                                &settings.levels[tier].power_thresholds,
                                                power,
                                                settings.levels[tier].minimum_effective_distance,
                                            ))
                                        } else {
                                            settings
                                                .unstable_power_thresholds
                                                .get(&source.original_power)
                                                .copied()
                                                .or(settings.unstable_cutoff)
                                        };
                                        threshold.map_or(0.0, |t| source.order as f64 / -t.log2())
                                    })
                                    .fold(0.0, f64::max)
                            })
                            .collect()
                    })
                    .collect()
            });
            return Ok(Self { risks });
        }
        let rows = cancellation.routing_rows();
        if settings.has_overrides()
            && let Some(powers) = powers
            && rows.iter().any(|row| {
                row.iter().enumerate().any(|(axis, degree)| {
                    *degree > 0 && powers.get(axis).is_none_or(Option::is_none)
                })
            })
        {
            return Err(invalid(
                "per-power thresholds lack an original power for an active cancellation coordinate; regenerate this artifact",
            ));
        }
        let risks = std::array::from_fn(|tier| {
            rows.iter()
                .map(|row| {
                    row.iter()
                        .enumerate()
                        .map(|(axis, degree)| {
                            if *degree == 0 {
                                return 0.0;
                            }
                            let power = powers.and_then(|p| p[axis].as_deref());
                            let threshold = if tier < 2 {
                                Some(lookup(
                                    &settings.levels[tier].power_thresholds,
                                    power,
                                    settings.levels[tier].minimum_effective_distance,
                                ))
                            } else {
                                power
                                    .and_then(|p| {
                                        settings.unstable_power_thresholds.get(p).copied()
                                    })
                                    .or(settings.unstable_cutoff)
                            };
                            threshold.map_or(0.0, |t| *degree as f64 / -t.log2())
                        })
                        .collect()
                })
                .collect()
        });
        Ok(Self { risks })
    }
    pub(super) fn class(&self, point: &[f64]) -> PrecisionClass {
        let crossed = |tier: usize| {
            self.risks[tier].iter().any(|row| {
                row.iter()
                    .zip(point)
                    .filter(|(budget, _)| **budget > 0.0)
                    .map(|(budget, x)| -x.log2() * budget)
                    .sum::<f64>()
                    >= 1.0
            })
        };
        if crossed(2) {
            PrecisionClass::Unstable
        } else if crossed(1) {
            PrecisionClass::Arbitrary
        } else if crossed(0) {
            PrecisionClass::DoubleFloat
        } else {
            PrecisionClass::F64
        }
    }
}

/// Native arithmetic recipe supported by the current detached root helper.
/// This reports execution requirements; it does not certify a geometric chart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlgebraicCallbackRecipe {
    /// Static rational bracket endpoints are exactly representable as finite
    /// f64 values. Roots may be irrational and of arbitrary admitted degree.
    /// Requires Validated whole-vector checks; native tracking has finite
    /// representable error range and may return precision failure.
    StaticExactF64BracketV1,
    /// Certified constant rational bracket, normalized natively to [0,1].
    /// Physical root/implicit derivatives retain tracked rational conversion.
    AffineRationalBracketV2,
    /// Loaded sectors contain both preserved V1 and affine V2 helpers.
    MixedStaticBracketsV1V2,
}

impl super::KernelSet {
    /// Inspect the arithmetic recipe without evaluating or replaying proofs.
    pub fn algebraic_callback_recipe(&self) -> Option<AlgebraicCallbackRecipe> {
        let mut v1 = false;
        let mut v2 = false;
        for (_, owner) in self.sectors.iter().flat_map(|s| s.algebraic.owners()) {
            match owner.recipe_version() {
                1 => v1 = true,
                2 => v2 = true,
                _ => unreachable!("admitted root recipe"),
            }
        }
        match (v1, v2) {
            (false, false) => None,
            (true, false) => Some(AlgebraicCallbackRecipe::StaticExactF64BracketV1),
            (false, true) => Some(AlgebraicCallbackRecipe::AffineRationalBracketV2),
            (true, true) => Some(AlgebraicCallbackRecipe::MixedStaticBracketsV1V2),
        }
    }
    /// Algebraic roots currently require native tracked whole-vector validation.
    /// Distance/cutoff policies lack a branch-cancellation acceptance gate.
    pub fn requires_validated_algebraic_callbacks(&self) -> bool {
        self.sectors
            .iter()
            .any(|sector| !sector.algebraic.is_empty())
    }
    pub fn stability_settings(&self) -> &StabilitySettings {
        &self.stability
    }
    /// Runtime policy is bound to replay/checkpoint state, not immutable template bytes.
    pub fn set_stability_settings(
        &mut self,
        settings: &StabilitySettings,
    ) -> Result<(), KernelError> {
        settings.validate()?;
        if self.requires_validated_algebraic_callbacks()
            && (settings.mode != StabilityMode::Validated
                || settings.unstable_cutoff.is_some()
                || !settings.unstable_power_thresholds.is_empty())
        {
            return Err(invalid(
                "algebraic-root kernels require validated stability without cutoff zeros",
            ));
        }
        let routings = self
            .sectors
            .iter()
            .enumerate()
            .map(|(index, sector)| {
                let powers = if settings.has_overrides()
                    && sector.cancellation.endpoint_profiles().is_none()
                {
                    self.metadata
                        .as_ref()
                        .and_then(|metadata| {
                            metadata.charts().iter().find(|chart| {
                                chart.kernel_sector() == Some(index)
                                    && chart.source_index() == chart.representative()
                            })
                        })
                        .and_then(|chart| chart.pre_subtraction())
                        .and_then(|pre| {
                            (0..sector.dimension())
                                .map(|axis| {
                                    let values = pre
                                        .terms()
                                        .iter()
                                        .filter_map(|term| term.powers().get(axis))
                                        .filter(|p| p.subtraction_count() > 0)
                                        .map(|p| (-p.constant().clone()).to_string())
                                        .collect::<std::collections::BTreeSet<_>>();
                                    if values.len() > 1 {
                                        None
                                    } else {
                                        Some(values.into_iter().next())
                                    }
                                })
                                .collect::<Option<Vec<_>>>()
                        })
                } else {
                    None
                };
                Routing::new(&sector.cancellation, settings, powers.as_deref())
            })
            .collect::<Result<Vec<_>, _>>()?;
        for (sector, routing) in self.sectors.iter_mut().zip(routings) {
            sector.stability = settings.clone();
            sector.routing = routing;
        }
        self.stability = settings.clone();
        Ok(())
    }
}
