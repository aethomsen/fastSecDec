//! Native O2 or portable interpreted vector kernels. Worker ownership is explicit.
pub(crate) mod algebraic;
mod artifact;
mod callback_attempt;
/// Sector-addressable native artifacts and synchronous caller-owned I/O.
pub mod indexed {
    pub use super::artifact::indexed::*;
}
pub use artifact::indexed::ProgramRecipe;
#[cfg(feature = "native")]
mod backend_version;
#[cfg(feature = "native")]
pub use backend_version::symjit_version_code;
mod cancellation;
mod compilation;
pub use compilation::{
    CompilationCompletion, CompilationDispatch, CompilationJob, CompilationSession,
};
mod compilation_settings;
pub use compilation_settings::{CompilationSettings, EvaluatorBackend};
mod complex;
mod contour;
pub use contour::{
    ContourCheckReport, ContourProductionReport, ContourValidationChart, ContourValidationReport,
};
mod diagnostics;
mod distance;
mod evaluator;
mod exact;
mod metadata;
mod model_constraints;
pub(crate) mod parameter_guard;
/// Data-only threshold provenance. Structural admission does not certify the
/// referenced mathematics or create an executable threshold artifact.
pub mod threshold_metadata;
mod threshold_owner;
pub use metadata::PortableMetadata;
pub use model_constraints::RuntimeMassConstraint;
pub use threshold_owner::{ThresholdMetadata, ThresholdResultScope};
mod precision;
mod precision_cache;
mod program;
mod recipe;
pub(crate) use program::is_real_expression;
pub(crate) use recipe::DynamicCheckSource;
pub use recipe::{DynamicChartRecipe, NativeProgramDescriptor, PositiveFactorProof};
mod projection;
mod stability;
mod statistics;
mod timing;
pub use stability::{
    ARBITRARY_DECIMAL_DIGITS, AlgebraicCallbackRecipe, PrecisionClass, StabilityLevel,
    StabilityMode, StabilitySettings,
};
pub use statistics::{EvaluatorOperations, EvaluatorStatistics};
pub use timing::{EvaluationTimings, EvaluatorTiming};
mod weighted;

#[cfg(all(test, feature = "native"))]
mod function_map_probe;
pub use precision::{PrecisionPolicy, PrecisionReport};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{
        float::{Complex, DoubleFloat, ErrorPropagatingFloat, Float, RealLike},
        rational::Rational,
    },
    evaluate::ExpressionEvaluator,
};
pub use weighted::{
    BatchEvaluationError, ReplayPolicy, ReplayReport, ReplayState, WeightedEvaluationContext,
};

#[derive(Debug, thiserror::Error)]
pub enum KernelError {
    #[error("contour deformation validation failed: {0}")]
    Contour(String),
    #[error("kernel operation cancelled")]
    Cancelled,
    #[error("invalid runtime parameters: {0}")]
    Parameters(String),
    #[error("runtime parameters must be bound before evaluation")]
    UnboundParameters,
    #[error("invalid kernel artifact: {0}")]
    Artifact(String),
    #[error("invalid precision rescue policy")]
    PrecisionPolicy,
    #[error("invalid stability settings: {0}")]
    Stability(String),
    #[error("precision rescue failed: {0}")]
    PrecisionEvaluation(String),
    #[error("precision rescue did not converge within {bits} bits")]
    PrecisionExhausted { bits: u32 },
    #[error("kernel construction failed: {0}")]
    Compilation(String),
    #[error("expected input dimension {expected}, received {actual}")]
    Dimension { expected: usize, actual: usize },
    #[error("expected {expected} output coefficients, received {actual}")]
    OutputCount { expected: usize, actual: usize },
    #[error("kernel input must lie in the finite closed unit cube")]
    InvalidPoint,
    #[error("integration weight must be finite and nonnegative")]
    InvalidWeight,
    #[error("weighted replay failed: {0}")]
    Replay(String),
    #[error("kernel produced a non-finite value at the supplied point")]
    NonFinite,
    #[error("portable evaluator serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Clone, Copy, Debug)]
pub struct CompilationProgress {
    pub completed: usize,
    pub total: usize,
    pub elapsed_seconds: f64,
}

/// Optional integrity and symbolic metadata checks for trusted artifact loading.
/// Format, numerical policy and evaluator layout checks always remain enabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KernelLoadOptions {
    /// Recompute content hashes and retained polynomial/geometry proofs.
    pub validate: bool,
}

/// Caller-owned artifact restoration, separate from generation or integration.
/// Decoding and each native evaluator construction are indivisible operations.
#[derive(Clone, Copy, Debug)]
pub enum KernelLoadProgress {
    /// Validating and decoding portable native programs and metadata.
    Decoding,
    /// Restoring sector evaluators; N/N still precedes final offset validation.
    Restoring(CompilationProgress),
    /// Evaluators, schemas and requested optional validation are admitted. Runtime
    /// constraints and exact offsets still require a valid physical point.
    Complete,
}

/// How this owner's primary evaluator was initially prepared. Later callback
/// policy changes may remap it. This is restoration evidence,
/// independent of numerical precision, causal admission and sampling work.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryEvaluatorRestoration {
    Generated,
    CacheRestored,
    CacheMissing,
    CacheIncompatible,
    Eager,
}

pub struct SectorKernel {
    algebraic: algebraic::Scope,
    contour_validation: Option<contour::SectorValidation>,
    dynamic_history: (usize, u32),
    runtime_diagnostics: crate::contour::functions::dynamic::diagnostics::Accumulator,
    dynamic_failure: Option<std::sync::Arc<contour::dynamic::failure::FailureContext>>,
    projection: Option<projection::OutputProjection>,
    cancellation: cancellation::Cancellation,
    precision: PrecisionPolicy,
    stability: StabilitySettings,
    routing: stability::Routing,
    parameters: Vec<Symbol>,
    runtime_parameters: Vec<Symbol>,
    parameters_bound: bool,
    input: Vec<f64>,
    exact_zero: Vec<bool>,
    program_bytes: std::sync::Arc<[u8]>,
    statistics: EvaluatorStatistics,
    primary_restoration: PrimaryEvaluatorRestoration,
    backend: Backend,
}

// Both variants own large evaluator workspaces with heap-backed numeric buffers.
// Keep the selected workspace inline in its worker-owned sector rather than
// adding a separate allocation and indirection solely to shrink this enum.
#[allow(clippy::large_enum_variant)]
enum Backend {
    Real(RealKernel),
    Complex(complex::ComplexKernel),
}

struct RealKernel {
    precision_cache: precision_cache::PrecisionCache<Float>,
    root_precision: Option<Box<precision_cache::PrecisionCache<ErrorPropagatingFloat<Float>>>>,
    double_cache: precision_cache::PrecisionCache<DoubleFloat>,
    f64_timing: EvaluatorTiming,
    conditioning_timing: EvaluatorTiming,
    evaluator: evaluator::RealEvaluator,
    exact_evaluator: std::sync::Arc<ExpressionEvaluator<Complex<Rational>>>,
    conditioning: evaluator::Conditioning<ErrorPropagatingFloat<f64>>,
    check_input: Vec<ErrorPropagatingFloat<f64>>,
    check_output: Vec<ErrorPropagatingFloat<f64>>,
}

impl SectorKernel {
    pub fn primary_evaluator_restoration(&self) -> PrimaryEvaluatorRestoration {
        self.primary_restoration
    }

    pub(in crate::kernel) fn saved_primary(
        &self,
    ) -> Result<Option<evaluator::SavedPrimary>, KernelError> {
        let (bytes, complex) = match &self.backend {
            Backend::Real(kernel) => (kernel.evaluator.saved_primary()?, false),
            Backend::Complex(kernel) => (kernel.saved_primary()?, true),
        };
        Ok(bytes.map(|bytes| {
            evaluator::SavedPrimary::new(
                bytes,
                &self.program_bytes,
                complex,
                self.parameters.len() + self.runtime_parameters.len(),
                self.exact_program().get_output_len(),
            )
        }))
    }
    fn clear_dynamic_attempt(&mut self) {
        match &mut self.backend {
            Backend::Real(kernel) => {
                if !kernel.evaluator.has_dynamic_callbacks() {
                    return;
                }
                kernel.evaluator.clear_dynamic_attempt();
                kernel.double_cache.clear_dynamic_attempt();
                kernel.precision_cache.clear_dynamic_attempt();
                if let Some(cache) = &mut kernel.root_precision {
                    cache.clear_dynamic_attempt();
                }
                kernel.conditioning.clear_dynamic_attempt();
            }
            Backend::Complex(kernel) => kernel.clear_dynamic_attempt(),
        }
    }

    fn explain_dynamic_error(&self, error: KernelError) -> KernelError {
        if !matches!(
            error,
            KernelError::NonFinite
                | KernelError::PrecisionExhausted { .. }
                | KernelError::PrecisionEvaluation(_)
        ) {
            return error;
        }
        let reason = match &self.backend {
            Backend::Real(kernel) => kernel
                .precision_cache
                .last_dynamic_error
                .as_deref()
                .or_else(|| {
                    kernel
                        .root_precision
                        .as_ref()
                        .and_then(|c| c.last_dynamic_error.as_deref())
                })
                .or(kernel.double_cache.last_dynamic_error.as_deref())
                .or(kernel.evaluator.last_dynamic_error())
                .or(kernel.conditioning.last_dynamic_error()),
            Backend::Complex(kernel) => kernel.last_dynamic_error(),
        };
        if let Some(context) = &self.dynamic_failure {
            let mut message = format!("unresolved deformation: {error}");
            if let Some(reason) = reason {
                message.push_str(&format!("; {reason}"));
            }
            if let Some(evidence) = context.describe(&self.input[..self.dimension()]) {
                message.push_str(&format!("; {evidence}"));
            }
            return KernelError::Contour(message);
        }
        match reason {
            Some(reason) if reason.starts_with("algebraic callback:") => {
                KernelError::PrecisionEvaluation(format!("{error}; {reason}"))
            }
            Some(reason) => KernelError::Contour(format!("{error}; {reason}")),
            None => error,
        }
    }
    pub(in crate::kernel) fn exact_program(&self) -> &program::ExactProgram {
        match &self.backend {
            Backend::Real(kernel) => &kernel.exact_evaluator,
            Backend::Complex(kernel) => kernel.exact_program(),
        }
    }
    /// Facts recorded from the actual complete-vector evaluator construction.
    /// Reading these records never constructs or executes an evaluator.
    pub fn statistics(&self) -> &EvaluatorStatistics {
        &self.statistics
    }
    pub fn dimension(&self) -> usize {
        self.parameters.len()
    }
    pub fn endpoint_profiles(&self) -> Option<&[crate::generation::EndpointProfileRow]> {
        self.cancellation.endpoint_profiles()
    }
    pub fn output_count(&self) -> usize {
        if let Some(projection) = &self.projection {
            return projection.output_count;
        }
        self.exact_zero.len()
            * if matches!(self.backend, Backend::Complex(_)) {
                2
            } else {
                1
            }
    }

    pub fn evaluate(&mut self, point: &[f64], output: &mut [f64]) -> Result<(), KernelError> {
        self.evaluate_with_diagnostics(point, output).map(|_| ())
    }

    pub fn evaluate_with_diagnostics(
        &mut self,
        point: &[f64],
        output: &mut [f64],
    ) -> Result<PrecisionReport, KernelError> {
        self.evaluate_scaled(point, output, 1.0)
    }

    fn evaluate_scaled(
        &mut self,
        point: &[f64],
        output: &mut [f64],
        weight: f64,
    ) -> Result<PrecisionReport, KernelError> {
        self.evaluate_scaled_with_primary(point, output, weight, None)
    }

    fn evaluate_scaled_with_primary(
        &mut self,
        point: &[f64],
        output: &mut [f64],
        weight: f64,
        primary: Option<&[f64]>,
    ) -> Result<PrecisionReport, KernelError> {
        if self.projection.is_some() {
            return self.project_output(output, primary, |kernel, values, primary| {
                kernel.evaluate_scaled_with_primary(point, values, weight, primary)
            });
        }
        let before = self.evaluation_metrics();
        let mut report = self
            .evaluate_scaled_inner(point, output, weight, primary)
            .map_err(|error| self.explain_dynamic_error(error))?;
        report.timings = self.evaluation_metrics().since(before);
        Ok(report)
    }

    fn evaluate_scaled_inner(
        &mut self,
        point: &[f64],
        output: &mut [f64],
        weight: f64,
        primary: Option<&[f64]>,
    ) -> Result<PrecisionReport, KernelError> {
        if point.len() != self.dimension() {
            return Err(KernelError::Dimension {
                expected: self.dimension(),
                actual: point.len(),
            });
        }
        if output.len() != self.output_count() {
            return Err(KernelError::OutputCount {
                expected: self.output_count(),
                actual: output.len(),
            });
        }
        if point
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(KernelError::InvalidPoint);
        }
        if !self.parameters_bound {
            return Err(KernelError::UnboundParameters);
        }
        if let Some(validation) = &mut self.contour_validation {
            validation.validate(point)?;
        }
        self.clear_dynamic_attempt();
        self.input[..point.len()].copy_from_slice(point);
        if self.stability.mode == StabilityMode::Distance {
            let class = self.routing.class(point);
            if let Some(primary) = primary {
                debug_assert_eq!(class, PrecisionClass::F64);
                for (out, value) in output.iter_mut().zip(primary) {
                    *out = value * weight;
                }
                if output.iter().all(|v| v.is_finite()) {
                    return Ok(PrecisionReport {
                        bits: 53,
                        ..Default::default()
                    });
                }
                return self.evaluate_distance_class(output, weight, PrecisionClass::DoubleFloat);
            }
            return self.evaluate_distance_class(output, weight, class);
        }
        let point = self.input.as_slice();
        let backend = match &mut self.backend {
            Backend::Complex(kernel) => {
                return kernel.evaluate_scaled(point, output, weight, primary);
            }
            Backend::Real(kernel) => kernel,
        };
        if let Some(primary) = primary {
            output.copy_from_slice(primary);
        } else {
            let started = std::time::Instant::now();
            backend.evaluator.evaluate(point, output);
            backend.f64_timing.record(started);
        }
        // Native replay must scale before conversion: an amplifying weight can
        // make a raw zero/subnormal result materially inaccurate.
        let range_loss = weight > 1.0
            && output
                .iter()
                .zip(&self.exact_zero)
                .any(|(value, zero)| value.abs() < f64::MIN_POSITIVE && !zero);
        for value in output.iter_mut() {
            *value *= weight;
        }
        let nonfinite = output.iter().any(|value| !value.is_finite());
        let boundary = !self.algebraic.is_empty()
            || self
                .cancellation
                .needs_check(point, self.precision.boundary_threshold);
        if boundary
            && !nonfinite
            && !range_loss
            && let Some(conditioning) = backend
                .conditioning
                .get_or_map(&backend.exact_evaluator, |value| {
                    ErrorPropagatingFloat::new(value.re.to_f64(), 15.0)
                })
        {
            for (target, value) in backend.check_input.iter_mut().zip(point) {
                *target = ErrorPropagatingFloat::new(*value, 15.0);
            }
            let started = std::time::Instant::now();
            let callback_failed =
                conditioning.evaluate_at(&backend.check_input, &mut backend.check_output, point);
            backend.conditioning_timing.record(started);
            let stable = !callback_failed
                && backend
                    .check_output
                    .iter()
                    .zip(output.iter())
                    .zip(&self.exact_zero)
                    .all(|((checked, compiled), zero)| {
                        if *zero {
                            return *compiled == 0.0;
                        }
                        let value = checked.to_f64() * weight;
                        let tolerance = self.precision.absolute_tolerance
                            + self.precision.relative_tolerance * value.abs();
                        if !self.algebraic.is_empty() {
                            return precision::accepts_weighted_primary(
                                *checked, *compiled, weight, tolerance,
                            );
                        }
                        value.is_finite()
                            && checked.get_absolute_error() * weight <= tolerance
                            && (value - compiled).abs() <= tolerance
                    });
            if stable {
                return Ok(PrecisionReport {
                    rescued: false,
                    checked: true,
                    bits: 53,
                    ..Default::default()
                });
            }
        }
        if nonfinite || boundary || range_loss {
            return precision::rescue_with_roots(
                &backend.exact_evaluator,
                (
                    &mut backend.precision_cache,
                    backend.root_precision.as_deref_mut(),
                ),
                point,
                output,
                &self.cancellation,
                &self.precision,
                weight,
            );
        }
        Ok(PrecisionReport {
            rescued: false,
            checked: false,
            bits: 53,
            ..Default::default()
        })
    }

    fn replay_scaled(
        &mut self,
        point: &[f64],
        output: &mut [f64],
        weight: f64,
        minimum_bits: u32,
    ) -> Result<PrecisionReport, KernelError> {
        if self.projection.is_some() {
            return self.project_output(output, None, |kernel, values, _| {
                kernel.replay_scaled(point, values, weight, minimum_bits)
            });
        }
        if !self.parameters_bound {
            return Err(KernelError::UnboundParameters);
        }
        if point.len() != self.dimension() {
            return Err(KernelError::Dimension {
                expected: self.dimension(),
                actual: point.len(),
            });
        }
        self.clear_dynamic_attempt();
        self.input[..point.len()].copy_from_slice(point);
        let point = self.input.as_slice();
        let mut policy = self.precision.clone();
        policy.initial_bits = policy.initial_bits.max(minimum_bits);
        let result = match &mut self.backend {
            Backend::Real(kernel) => precision::rescue_with_roots(
                &kernel.exact_evaluator,
                (
                    &mut kernel.precision_cache,
                    kernel.root_precision.as_deref_mut(),
                ),
                point,
                output,
                &self.cancellation,
                &policy,
                weight,
            ),
            Backend::Complex(kernel) => kernel.replay_scaled(point, output, weight, &policy),
        };
        result.map_err(|error| self.explain_dynamic_error(error))
    }

    /// Clone native evaluator state and buffers for an independently owned worker.
    pub fn try_clone(&self) -> Result<Self, KernelError> {
        Ok(Self {
            algebraic: self.algebraic.clone(),
            dynamic_history: (0, 0),
            runtime_diagnostics: Default::default(),
            dynamic_failure: self.dynamic_failure.clone(),
            contour_validation: self
                .contour_validation
                .as_ref()
                .map(contour::SectorValidation::fork),
            projection: self.projection.clone(),
            cancellation: self.cancellation.clone(),
            precision: self.precision.clone(),
            stability: self.stability.clone(),
            routing: self.routing.clone(),
            parameters: self.parameters.clone(),
            runtime_parameters: self.runtime_parameters.clone(),
            parameters_bound: self.parameters_bound,
            input: self.input.clone(),
            exact_zero: self.exact_zero.clone(),
            program_bytes: self.program_bytes.clone(),
            statistics: self.statistics.clone(),
            primary_restoration: self.primary_restoration,
            backend: match &self.backend {
                Backend::Complex(kernel) => Backend::Complex(kernel.try_clone()?),
                Backend::Real(kernel) => Backend::Real(RealKernel {
                    precision_cache: kernel.precision_cache.empty_clone(),
                    root_precision: kernel
                        .root_precision
                        .as_ref()
                        .map(|c| Box::new(c.empty_clone())),
                    double_cache: kernel.double_cache.empty_clone(),
                    f64_timing: Default::default(),
                    conditioning_timing: Default::default(),
                    evaluator: kernel.evaluator.clone(),
                    exact_evaluator: kernel.exact_evaluator.clone(),
                    conditioning: kernel.conditioning.clone(),
                    check_input: kernel.check_input.clone(),
                    check_output: kernel.check_output.clone(),
                }),
            },
        })
    }
}

pub struct KernelSet {
    threshold: Option<std::sync::Arc<ThresholdMetadata>>,
    program_descriptor: Option<NativeProgramDescriptor>,
    contour_checks: Vec<contour::CheckProgram>,
    contour_binding: Option<contour::ContourBinding>,
    dynamic_pilot: contour::dynamic::runtime::PilotOwners,
    contour_diagnostics: crate::contour::ContourDiagnosticsMode,
    runtime_diagnostics: crate::contour::functions::dynamic::diagnostics::Accumulator,
    compilation_settings: CompilationSettings,
    stability: StabilitySettings,
    runtime_parameters: Vec<Symbol>,
    runtime_mass_constraints: Vec<RuntimeMassConstraint>,
    template_content_id: Option<String>,
    portable_artifact: Option<std::sync::Arc<Vec<u8>>>,
    metadata: Option<crate::generation::GenerationMetadata>,
    coefficient_orders: Vec<i32>,
    components: Vec<crate::status::CoefficientComponent>,
    precision: PrecisionPolicy,
    exact_expressions: Vec<Atom>,
    exact_requests: Vec<crate::contour::functions::dynamic::requests::ExactRequest>,
    content_id: String,
    orders: Vec<i32>,
    sectors: Vec<SectorKernel>,
    exact_coefficients: Vec<f64>,
}

impl KernelSet {
    /// Copy the evaluator owners for an independently bound physical point.
    /// Immutable programs and metadata are retained; workspaces and timing
    /// counters are independent, and no symbolic recompilation is performed.
    pub fn try_clone(&self) -> Result<Self, KernelError> {
        let mut result = Self {
            threshold: self.threshold.clone(),
            program_descriptor: self.program_descriptor.clone(),
            contour_checks: self.contour_checks.clone(),
            contour_binding: self.contour_binding.clone(),
            dynamic_pilot: Default::default(),
            contour_diagnostics: self.contour_diagnostics,
            runtime_diagnostics: Default::default(),
            compilation_settings: self.compilation_settings,
            stability: self.stability.clone(),
            runtime_parameters: self.runtime_parameters.clone(),
            runtime_mass_constraints: self.runtime_mass_constraints.clone(),
            template_content_id: self.template_content_id.clone(),
            portable_artifact: self.portable_artifact.clone(),
            metadata: self.metadata.clone(),
            coefficient_orders: self.coefficient_orders.clone(),
            components: self.components.clone(),
            precision: self.precision.clone(),
            exact_expressions: self.exact_expressions.clone(),
            exact_requests: self.exact_requests.clone(),
            content_id: self.content_id.clone(),
            orders: self.orders.clone(),
            sectors: self
                .sectors
                .iter()
                .map(SectorKernel::try_clone)
                .collect::<Result<_, _>>()?,
            exact_coefficients: self.exact_coefficients.clone(),
        };
        if let Some(binding) = &result.contour_binding {
            for (index, sector) in result.sectors.iter_mut().enumerate() {
                sector.contour_validation = binding.for_sector(index);
            }
        }
        Ok(result)
    }

    /// Settings used to construct the immutable native evaluator programs.
    pub fn compilation_settings(&self) -> &CompilationSettings {
        &self.compilation_settings
    }

    /// Ordered real scalar inputs appended after integration coordinates in the native evaluator.
    pub fn runtime_parameters(&self) -> &[Symbol] {
        &self.runtime_parameters
    }

    /// Immutable compiled-template identity, independent of the bound physical point.
    pub fn template_content_id(&self) -> &str {
        self.template_content_id
            .as_deref()
            .unwrap_or(&self.content_id)
    }

    pub fn parameters_bound(&self) -> bool {
        self.runtime_parameters.is_empty() || self.template_content_id.is_some()
    }

    /// Bind the complete physical point once before creating caller-owned workers.
    /// Rebinding leaves the compiled program untouched and derives a distinct numerical identity.
    pub fn bind_parameters(
        &mut self,
        values: &std::collections::BTreeMap<Symbol, f64>,
    ) -> Result<(), KernelError> {
        if self.program_recipe().is_dynamic() {
            return Err(KernelError::Parameters(
                "dynamic contour inputs require bind_parameters_with_contour".into(),
            ));
        }
        if self.contour_capable() {
            let mut physics = values.clone();
            let lambda = physics
                .remove(&crate::contour::lambda_symbol())
                .ok_or_else(|| {
                    KernelError::Parameters(
                        "missing contour strength; use bind_parameters_with_contour".into(),
                    )
                })?;
            return self.bind_parameters_with_contour(
                &physics,
                &crate::contour::ContourSettings {
                    deformation: crate::contour::ContourMode::Fixed { lambda },
                    ..Default::default()
                },
            );
        }
        self.bind_parameters_raw(values)
    }

    fn bind_parameters_raw(
        &mut self,
        values: &std::collections::BTreeMap<Symbol, f64>,
    ) -> Result<(), KernelError> {
        if let Some(descriptor) = &self.program_descriptor {
            descriptor.admit_runtime()?;
        }
        let ordered = self.ordered_runtime_parameters(values)?;
        if let Some(threshold) = &self.threshold {
            threshold.admit_parameter_point(&self.runtime_parameters, &ordered)?;
        }
        if self.runtime_parameters.is_empty() {
            return Ok(());
        }
        let _preparing = NativeProgramDescriptor::enter_optional(self.program_descriptor.as_ref());
        let exact_coefficients = exact::evaluate(
            &self.exact_expressions,
            values,
            self.components
                .contains(&crate::status::CoefficientComponent::Imag),
        )?;
        self.commit_runtime_parameters(&ordered, exact_coefficients);
        Ok(())
    }

    fn ordered_runtime_parameters(
        &self,
        values: &std::collections::BTreeMap<Symbol, f64>,
    ) -> Result<Vec<f64>, KernelError> {
        if let Some(lambda) = values.get(&crate::contour::lambda_symbol())
            && (!lambda.is_finite() || *lambda <= 0.0)
        {
            return Err(KernelError::Parameters(
                "fixed contour strength must be finite and positive".into(),
            ));
        }
        for symbol in values.keys() {
            if !self.runtime_parameters.contains(symbol) {
                return Err(KernelError::Parameters(format!(
                    "unknown parameter {}",
                    Atom::var(*symbol).to_canonical_string()
                )));
            }
        }
        let ordered = self
            .runtime_parameters
            .iter()
            .map(|symbol| {
                let name = Atom::var(*symbol).to_canonical_string();
                match values.get(symbol) {
                    Some(value) if value.is_finite() => Ok(*value),
                    Some(_) => Err(KernelError::Parameters(format!("{name} must be finite"))),
                    None => Err(KernelError::Parameters(format!("missing value for {name}"))),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        self.validate_runtime_masses_at(values)?;
        Ok(ordered)
    }

    fn commit_runtime_parameters(&mut self, ordered: &[f64], exact_coefficients: Vec<f64>) {
        for sector in &mut self.sectors {
            let dimension = sector.dimension();
            sector.input[dimension..].copy_from_slice(ordered);
            sector.parameters_bound = true;
        }
        self.exact_coefficients = exact_coefficients;
        let original_id = self
            .template_content_id
            .get_or_insert_with(|| self.content_id.clone());
        let mut hash = blake3::Hasher::new();
        hash.update(b"fastsecdec-runtime-parameter-point-v1");
        hash.update(original_id.as_bytes());
        for value in ordered {
            hash.update(&value.to_bits().to_le_bytes());
        }
        self.content_id = hash.finalize().to_hex().to_string();
    }
    /// Legacy version-one artifacts have no retained generation metadata.
    pub fn generation_metadata(&self) -> Option<&crate::generation::GenerationMetadata> {
        self.metadata.as_ref()
    }
    pub fn components(&self) -> &[crate::status::CoefficientComponent] {
        &self.components
    }
    pub fn content_id(&self) -> &str {
        &self.content_id
    }
    pub fn orders(&self) -> &[i32] {
        &self.orders
    }
    pub fn sectors(&self) -> &[SectorKernel] {
        &self.sectors
    }
    pub fn sectors_mut(&mut self) -> &mut [SectorKernel] {
        &mut self.sectors
    }
    pub fn exact_coefficients(&self) -> &[f64] {
        &self.exact_coefficients
    }
}

#[cfg(test)]
fn has_complex_coefficients(expression: &Atom) -> bool {
    let mut found = false;
    expression.visitor(&mut |term| {
        if let symbolica::atom::AtomView::Num(number) = term {
            found |= !number.get_coeff_view().is_real();
        }
        !found
    });
    found
}

#[cfg(feature = "threshold-decomposition")]
pub use compilation::{
    ThresholdCompilationCompletion, ThresholdCompilationJob, ThresholdCompilationPlan,
    ThresholdCompilationWork, ThresholdJobKind, ThresholdPublicationDescriptor,
    ThresholdPublicationPlan, ThresholdWorkReceipt,
};

#[cfg(all(test, feature = "threshold-decomposition"))]
mod algebraic_tests;

#[cfg(all(test, feature = "threshold-decomposition"))]
mod parameter_template_tests;

#[cfg(all(test, feature = "threshold-decomposition"))]
mod projective_parameter_tests;
