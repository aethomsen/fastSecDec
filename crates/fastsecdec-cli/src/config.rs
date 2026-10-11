use std::{collections::BTreeMap, path::PathBuf};

use fastsecdec::AtomCore;
use serde::Deserialize;

/// Normalize one input layer before merging it with lower-priority values.
pub(crate) fn canonical_parameter_names<T>(
    values: BTreeMap<String, T>,
) -> crate::CliResult<BTreeMap<String, T>> {
    let mut canonical = BTreeMap::new();
    for (name, value) in values {
        let symbol = crate::input::symbol(&name)?;
        let key = fastsecdec::Atom::var(symbol).to_canonical_string();
        if canonical.insert(key, value).is_some() {
            return Err(format!("duplicate runtime parameter {name}").into());
        }
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests;

/// CLI generation choices travel with process jobs instead of rewriting cards.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, Deserialize)]
pub(crate) struct GenerationOverrides {
    pub contour: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub threshold_decomposition: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<fastsecdec::kernel::ProgramRecipe>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contour_jacobian: Option<fastsecdec::contour::ContourJacobian>,
}
impl GenerationOverrides {
    pub(crate) fn apply(self, card: &mut RunCard) {
        card.generation.contour |= self.contour;
        card.generation.threshold_decomposition |= self.threshold_decomposition;
        if let Some(choice) = self.contour_jacobian {
            card.generation.contour_jacobian = choice;
        }
        if let Some(recipe) = self.recipe {
            card.generation.recipe = Some(recipe);
        } else if self.contour {
            card.generation.recipe = None;
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunCard {
    pub input: Option<GraphInput>,
    pub direct: Option<DirectInput>,
    #[serde(default)]
    pub kinematics: KinematicsInput,
    #[serde(default)]
    pub parameters: BTreeMap<String, toml::Value>,
    #[serde(default)]
    pub integral: IntegralInput,
    #[serde(default)]
    pub generation: GenerationInput,
    #[serde(default)]
    pub threshold_decomposition: crate::threshold_cli::Input,
    #[serde(default)]
    pub integration: IntegrationInput,
    pub reference: Option<ReferenceInput>,
}

/// Observational comparison steering; never part of numerical integration settings.
#[derive(Clone, Debug, Default, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceInput {
    pub path: PathBuf,
    pub normalization_evidence: Option<String>,
    pub kinematics_evidence: Option<String>,
    pub independence_evidence: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphInput {
    pub graph: PathBuf,
    pub model: PathBuf,
    pub parameter_card: Option<PathBuf>,
    /// Runtime independent model inputs by default; fixed requests an explicit specialization.
    #[serde(default)]
    pub model_parameters: ModelParameters,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelParameters {
    #[default]
    Runtime,
    Fixed,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KinematicsInput {
    #[serde(default)]
    pub products: Vec<Product>,
    /// Additional formal vectors in a numerator, such as external helicities.
    #[serde(default)]
    pub auxiliary_momenta: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Product {
    pub left: MomentumName,
    pub right: MomentumName,
    pub value: Option<String>,
    /// A real scalar supplied when integrating the generated kernel.
    pub symbol: Option<String>,
}

/// Integer labels preserve the native graph's P(i) shorthand. Expressions
/// allow existing HEPKit momentum names without a second vector notation.
#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum MomentumName {
    External(usize),
    Expression(String),
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegralInput {
    pub dimension: String,
    pub regulator: String,
    pub measure_multiplier: String,
    pub powers: BTreeMap<String, u32>,
}

impl Default for IntegralInput {
    fn default() -> Self {
        Self {
            dimension: "4-2*eps".into(),
            regulator: "eps".into(),
            measure_multiplier: "1".into(),
            powers: BTreeMap::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GenerationInput {
    /// Original geometry-chart ordinals, before symmetry grouping or IBP.
    pub source_sectors: Option<Vec<usize>>,
    /// Complete and persist one sector per recyclable worker process.
    pub serial: bool,
    /// Generate every contour capability, defaulting to undeformed execution.
    pub contour: bool,
    /// Resolve real causal thresholds using verified cell and endpoint maps.
    pub threshold_decomposition: bool,
    /// Explicit singleton recipe; otherwise `contour` enables the complete family.
    pub recipe: Option<fastsecdec::kernel::ProgramRecipe>,
    pub order: i32,
    pub mode: fastsecdec::generation::GenerationMode,
    pub contour_jacobian: fastsecdec::contour::ContourJacobian,
    pub subtraction: fastsecdec::generation::SubtractionStrategy,
    pub contraction_mode: fastsecdec::input::NumeratorContraction,
    pub assume_no_threshold: bool,
    pub max_sectors: usize,
    pub max_support_pairs: usize,
    pub coefficient_expansion: fastsecdec::generation::CoefficientExpansionOptions,
    pub evaluator: fastsecdec::kernel::CompilationSettings,
    /// Native graph-family preparation; unlike the library policy's default,
    /// historical CLI cards keep their original propagator representation.
    pub family_preparation: fastsecdec::parametric::FamilyPreparationPolicy,
}

impl GenerationInput {
    /// Resolve computational settings before reading models or preparing geometry.
    pub(crate) fn resolve_jacobian(&mut self) -> crate::CliResult<()> {
        self.validate_capabilities()?;
        self.evaluator = self
            .evaluator
            .resolve_contour_jacobian(self.contour_jacobian)?;
        Ok(())
    }

    pub(crate) fn threshold_enabled(&self) -> bool {
        self.threshold_decomposition
            || self.recipe == Some(fastsecdec::kernel::ProgramRecipe::ThresholdV1)
    }

    pub(crate) fn validate_capabilities(&self) -> crate::CliResult<()> {
        use fastsecdec::{generation::GenerationMode, kernel::ProgramRecipe};
        if self.threshold_enabled() {
            if self.contour || self.recipe.is_some_and(|r| r != ProgramRecipe::ThresholdV1) {
                return Err("threshold decomposition and contour/other generation recipes are mutually exclusive".into());
            }
            if self.mode != GenerationMode::Symbolic {
                return Err("threshold decomposition requires symbolic endpoint reduction".into());
            }
            if self.contour_jacobian != fastsecdec::contour::ContourJacobian::Symbolic {
                return Err("contour_jacobian is inapplicable to threshold decomposition".into());
            }
        }
        Ok(())
    }

    pub fn recipe_family(&self) -> fastsecdec::generation::RecipeFamily {
        use fastsecdec::{generation::RecipeFamily, kernel::ProgramRecipe};
        if self.threshold_enabled() {
            return RecipeFamily::single(ProgramRecipe::ThresholdV1);
        }
        match self.recipe {
            Some(recipe) => RecipeFamily::single(recipe),
            None if self.contour => RecipeFamily::contour(),
            None => RecipeFamily::single(ProgramRecipe::UndeformedV1),
        }
    }

    /// The saved mathematical default is independent of runtime selection.
    pub fn program_recipe(&self) -> fastsecdec::kernel::indexed::ProgramRecipe {
        self.recipe_family().default_recipe()
    }

    /// Admission before parametrization; a missing recipe is never substituted.
    pub(crate) fn validate_resident_recipe(
        &self,
        requested: fastsecdec::kernel::ProgramRecipe,
    ) -> crate::CliResult<()> {
        self.validate_capabilities()?;
        self.recipe_family().validate_resident(Some(requested))?;
        Ok(())
    }
}

impl Default for GenerationInput {
    fn default() -> Self {
        Self {
            source_sectors: None,
            serial: false,
            contour: false,
            threshold_decomposition: false,
            recipe: None,
            order: 0,
            mode: Default::default(),
            contour_jacobian: Default::default(),
            subtraction: Default::default(),
            contraction_mode: Default::default(),
            assume_no_threshold: false,
            max_sectors: 1_000_000,
            max_support_pairs: 10_000_000,
            coefficient_expansion: Default::default(),
            evaluator: Default::default(),
            family_preparation: fastsecdec::parametric::FamilyPreparationPolicy::Original,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct IntegrationInput {
    /// The complete physical point, included in numerical checkpoint identity.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, f64>,
    /// Native scope; absent historical settings continue to mean the full integral.
    #[serde(skip_serializing_if = "fastsecdec::results::ResultScope::is_full_integral")]
    pub scope: fastsecdec::results::ResultScope,
    pub method: String,
    pub points: u64,
    pub shifts: u32,
    pub seed: u64,
    pub package_points: u64,
    pub workers: usize,
    /// Operational chunk size, independent of statistical batches and checkpoints.
    pub evaluation_batch_size: usize,
    pub periodization: String,
    /// Omitted in legacy/default serialization to preserve old checkpoint settings.
    #[serde(skip_serializing_if = "is_legacy_lattice")]
    pub lattice: String,
    pub absolute_tolerance: f64,
    pub relative_tolerance: f64,
    #[serde(skip_serializing_if = "fastsecdec::integration::AccuracyTarget::is_default")]
    pub accuracy_target: fastsecdec::integration::AccuracyTarget,
    pub production_seconds: f64,
    /// Minimum sampling residence, excluding loading/JIT and pauses.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub serial_seconds: Option<f64>,
    /// Grow points per replica; false appends replicas of the frozen design.
    pub double_points: bool,
    /// Omission means one ordinary allocation or unlimited serial allocations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_rounds: Option<usize>,
    pub replay: fastsecdec::kernel::ReplayPolicy,
    pub stability: fastsecdec::kernel::StabilitySettings,
    /// The mathematical contour and its independently selectable validation policy.
    #[serde(skip_serializing_if = "is_default_contour")]
    pub contour: fastsecdec::contour::ContourSettings,
    /// Optional callback work; excluded from mathematical/checkpoint identity.
    #[serde(skip_serializing_if = "is_disabled_contour_diagnostics")]
    pub contour_diagnostics: fastsecdec::contour::ContourDiagnosticsMode,
    /// Explicit steering for the global discrete-sector Havana lane only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discrete_mc: Option<DiscreteMcInput>,
}

#[derive(Clone, Debug, serde::Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DiscreteMcInput {
    pub pilot_points: usize,
    pub pilot_batches: u32,
    pub pilot_iterations: usize,
    pub bins: usize,
    pub minimum_probability_density: f64,
    pub maximum_sector_probability_ratio: f64,
    pub discrete_learning_rate: f64,
    pub continuous_learning_rate: f64,
}
impl Default for DiscreteMcInput {
    fn default() -> Self {
        Self {
            pilot_points: 4096,
            pilot_batches: 8,
            pilot_iterations: 3,
            bins: 32,
            minimum_probability_density: 0.01,
            maximum_sector_probability_ratio: 100.0,
            discrete_learning_rate: 0.5,
            continuous_learning_rate: 0.5,
        }
    }
}

impl Default for IntegrationInput {
    fn default() -> Self {
        Self {
            parameters: BTreeMap::new(),
            scope: Default::default(),
            method: "qmc".into(),
            points: 4096,
            shifts: 64,
            seed: 0,
            package_points: 1024,
            workers: 1,
            evaluation_batch_size: 256,
            periodization: "korobov3".into(),
            lattice: "kuo33002".into(),
            absolute_tolerance: 1e-8,
            relative_tolerance: 1e-3,
            accuracy_target: Default::default(),
            production_seconds: 10.0,
            serial_seconds: None,
            double_points: true,
            max_rounds: None,
            replay: Default::default(),
            stability: Default::default(),
            contour: Default::default(),
            contour_diagnostics: Default::default(),
            discrete_mc: None,
        }
    }
}

fn is_legacy_lattice(value: &str) -> bool {
    value == "kuo33002"
}

fn is_default_contour(value: &fastsecdec::contour::ContourSettings) -> bool {
    value == &fastsecdec::contour::ContourSettings::default()
}

fn is_disabled_contour_diagnostics(value: &fastsecdec::contour::ContourDiagnosticsMode) -> bool {
    *value == fastsecdec::contour::ContourDiagnosticsMode::Disabled
}

impl IntegrationInput {
    pub fn ordinary_max_rounds(&self) -> usize {
        self.max_rounds.unwrap_or(1)
    }

    pub fn validate_execution(&self) -> crate::CliResult<()> {
        self.contour.validate()?;
        if self.workers == 0 || self.max_rounds == Some(0) || self.evaluation_batch_size == 0 {
            return Err("workers, max_rounds and evaluation_batch_size must be positive".into());
        }
        if let Some(seconds) = self.serial_seconds {
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err("serial_seconds must be finite and greater than zero".into());
            }
            if self.method.replace('-', "_") == "discrete_mc" {
                return Err("discrete_mc does not support --serial; use --method mc or --method adaptive_mc for per-sector Havana sampling".into());
            }
        }
        Ok(())
    }

    /// Historical checkpoints predate distance routing and retain their policy.
    /// Explicit overlays still apply afterwards and are checked for compatibility.
    pub fn restore_historical_policy(&mut self, path: &std::path::Path) -> crate::CliResult<()> {
        let checkpoint: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
        if let Some(settings) = checkpoint
            .get("settings")
            .and_then(serde_json::Value::as_object)
            && !settings.contains_key("stability")
        {
            self.stability = fastsecdec::kernel::StabilitySettings::validated();
        }
        Ok(())
    }

    /// Merge only explicitly supplied runtime keys, preserving artifact defaults.
    /// Arrays (including stability levels) replace their entire previous value.
    pub fn apply_overlay(&mut self, path: &std::path::Path) -> crate::CliResult<()> {
        let mut overlay = read_integration_overlay(path)?;
        if let Some(parameters) = overlay.get_mut("parameters") {
            let values = serde_json::from_value::<BTreeMap<String, f64>>(parameters.clone())?;
            *parameters = serde_json::to_value(canonical_parameter_names(values)?)?;
        }
        let mut effective = serde_json::to_value(&*self)?;
        merge_runtime_values(&mut effective, overlay);
        *self = serde_json::from_value(effective)?;
        Ok(())
    }

    pub fn published_lattice(&self) -> crate::CliResult<fastsecdec::integration::PublishedLattice> {
        use fastsecdec::integration::PublishedLattice;
        match self.lattice.as_str() {
            "kuo33002" => Ok(PublishedLattice::Kuo33002),
            "kuo38005" => Ok(PublishedLattice::Kuo38005),
            "kuo39101" => Ok(PublishedLattice::Kuo39101),
            "hkkn-alpha3" => Ok(PublishedLattice::HkknAlpha3),
            _ => Err("lattice must be kuo33002, kuo38005, kuo39101, or hkkn-alpha3".into()),
        }
    }

    pub fn qmc_settings(&self) -> crate::CliResult<fastsecdec::integration::QmcSettings> {
        use fastsecdec::integration::{Periodization, PublishedLattice, QmcSettings, RuleSource};
        let catalogue = self.published_lattice()?;
        let settings = QmcSettings {
            points: self.points,
            shifts: self.shifts,
            seed: self.seed,
            package_points: self.package_points,
            periodization: match self.periodization.as_str() {
                "none" => Periodization::None,
                "korobov3" => Periodization::Korobov3,
                "korobov2" => Periodization::Korobov2,
                _ => return Err("periodization must be none, korobov2 or korobov3".into()),
            },
            rule: if catalogue == PublishedLattice::Kuo33002 {
                RuleSource::Kuo
            } else {
                RuleSource::Published(catalogue)
            },
        };
        settings.validate()?;
        Ok(settings)
    }
}

pub(crate) fn merge_runtime_values(base: &mut serde_json::Value, overlay: serde_json::Value) {
    // An explicit native enum change replaces the previous variant's fields.
    // Ordinary partial settings overlays still merge field by field. Keeping
    // a former fixed lambda inside a dynamic prescription would be ambiguous,
    // and native unknown-field admission correctly refuses it.
    if let Some(mode) = overlay
        .pointer("/contour/deformation/mode")
        .and_then(serde_json::Value::as_str)
        && base
            .pointer("/contour/deformation/mode")
            .and_then(serde_json::Value::as_str)
            != Some(mode)
        && let Some(previous) = base.pointer_mut("/contour/deformation")
    {
        *previous = serde_json::json!({});
    }
    merge_runtime_fields(base, overlay);
}

fn merge_runtime_fields(base: &mut serde_json::Value, overlay: serde_json::Value) {
    match (base, overlay) {
        (serde_json::Value::Object(base), serde_json::Value::Object(overlay)) => {
            for (key, value) in overlay {
                if let Some(existing) = base.get_mut(&key) {
                    merge_runtime_fields(existing, value);
                } else {
                    base.insert(key, value);
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

/// Parse steering without registering symbols before native artifact import.
pub(crate) fn read_integration_overlay(
    path: &std::path::Path,
) -> crate::CliResult<serde_json::Value> {
    let overlay: toml::Value = toml::from_str(&std::fs::read_to_string(path)?)?;
    let mut overlay = serde_json::to_value(overlay)?;
    if let Some(table) = overlay.as_object_mut()
        && table.contains_key("integration")
    {
        if table.len() != 1 {
            return Err("runtime settings with an [integration] table cannot contain other top-level tables".into());
        }
        overlay = table.remove("integration").unwrap();
    }
    if !overlay.is_object() {
        return Err("runtime integration settings must be a TOML table".into());
    }
    Ok(overlay)
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectInput {
    pub domain: String,
    pub parameters: Vec<String>,
    /// Explicit runtime scalar roles; free atoms are never inferred as parameters.
    #[serde(default)]
    pub runtime_parameters: Vec<String>,
    pub terms: Vec<DirectTerm>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectTerm {
    #[serde(default = "one")]
    pub prefactor: String,
    #[serde(default)]
    pub monomial_powers: Vec<String>,
    #[serde(default)]
    pub factors: Vec<DirectFactor>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DirectFactor {
    pub polynomial: Option<String>,
    pub polynomial_file: Option<PathBuf>,
    pub exponent: String,
    #[serde(default = "singularity")]
    pub role: String,
    /// Explicit causal F or positive U identity; generic factors are unclassified.
    #[serde(default)]
    pub semantics: fastsecdec::parametric::FactorSemantics,
}
fn one() -> String {
    "1".into()
}
fn singularity() -> String {
    "singularity".into()
}
