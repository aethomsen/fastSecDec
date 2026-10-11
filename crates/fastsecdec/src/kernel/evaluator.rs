//! Compile-time availability with explicit runtime selection of native owners.
use super::{EvaluatorBackend, EvaluatorTiming, KernelError, program::ExactProgram};
use symbolica::domains::float::Complex;
mod batch;
mod checked;
mod conditioning;
mod mapping;
mod observed;
mod saved;
use crate::contour::functions::dynamic::diagnostics::{Accumulator, Configuration, Phase};
pub(super) use conditioning::Conditioning;
pub(super) use mapping::MappingRequirements;
pub(in crate::kernel) use saved::SavedPrimary;

#[cfg(feature = "threshold-decomposition")]
impl crate::kernel::algebraic::Scope {
    /// Reuse native callback-domain and masked-preparation checks for a
    /// caller-owned exact program. This confers no kernel or geometry authority.
    pub(crate) fn map_checked<T: symbolica::evaluate::EvaluationDomain>(
        &self,
        exact: &ExactProgram,
        bits: u32,
        coefficient: impl Fn(&Complex<symbolica::domains::rational::Rational>) -> T,
    ) -> Result<symbolica::evaluate::ExpressionEvaluator<T>, String> {
        self.enter(bits, || {
            MappingRequirements::new(exact)?.map(exact, coefficient, bits)
        })
    }
}

macro_rules! evaluator {
    ($name:ident, $scalar:ty, $invalid:expr, $physical:expr) => {
        #[derive(Clone)]
        pub(super) enum $name {
            #[cfg(feature = "native")]
            Symjit(symbolica::evaluate::JITCompiledEvaluator<$scalar>),
            Eager(symbolica::evaluate::ExpressionEvaluator<$scalar>),
            /// A failed callback cannot become an accepted finite value merely
            /// because later arithmetic or an external consumer masks it.
            Dynamic(
                Box<Self>,
                Option<String>,
                crate::kernel::callback_attempt::Modes,
            ),
            Checked(Box<checked::Checked<Self>>),
            Observed(Box<observed::Observed<Self>>),
        }
        impl $name {
            pub(in crate::kernel) fn saved_primary(&self) -> Result<Option<Vec<u8>>, KernelError> {
                match self {
                    #[cfg(feature = "native")]
                    Self::Symjit(evaluator) => {
                        bincode::encode_to_vec(evaluator, bincode::config::standard())
                            .map(Some)
                            .map_err(|error| {
                                KernelError::Artifact(format!(
                                    "native primary evaluator encoding: {error}"
                                ))
                            })
                    }
                    Self::Eager(_) => Ok(None),
                    Self::Dynamic(evaluator, _, _) => evaluator.saved_primary(),
                    Self::Checked(owner) => owner.evaluator.saved_primary(),
                    Self::Observed(owner) => owner.evaluator.saved_primary(),
                }
            }
            pub(super) fn evaluate(&mut self, input: &[$scalar], output: &mut [$scalar]) {
                match self {
                    Self::Observed(owner) => {
                        owner
                            .diagnostics
                            .measure(owner.configuration, Phase::Evaluation, || {
                                owner.evaluator.evaluate(input, output)
                            })
                    }
                    #[cfg(feature = "native")]
                    Self::Symjit(evaluator) => evaluator.evaluate(input, output),
                    Self::Eager(evaluator) => evaluator.evaluate(input, output),
                    Self::Checked(checked) => {
                        checked.point.clear();
                        checked.point.extend(input.iter().map($physical));
                        if !checked
                            .validation
                            .evaluate(&checked.point, || checked.evaluator.evaluate(input, output))
                        {
                            output.fill($invalid);
                        }
                    }
                    Self::Dynamic(evaluator, last_error, modes) => {
                        let (_, failure) = modes.isolated(|| evaluator.evaluate(input, output));
                        *last_error = failure;
                        if last_error.is_some() {
                            output.fill($invalid);
                        }
                    }
                }
            }
            pub(super) fn evaluate_batch(
                &mut self,
                input: &[$scalar],
                output: &mut [$scalar],
                rows: usize,
                inputs: usize,
                outputs: usize,
            ) -> Vec<EvaluatorTiming> {
                match self {
                    Self::Observed(owner) => {
                        owner
                            .diagnostics
                            .measure(owner.configuration, Phase::Evaluation, || {
                                owner
                                    .evaluator
                                    .evaluate_batch(input, output, rows, inputs, outputs)
                            })
                    }
                    #[cfg(feature = "native")]
                    Self::Symjit(evaluator) => {
                        batch::evaluate_jit(evaluator, input, output, rows, inputs, outputs)
                    }
                    Self::Eager(evaluator) => {
                        batch::evaluate_eager(evaluator, input, output, rows, inputs, outputs)
                    }
                    Self::Checked(_) => {
                        let mut timings = Vec::with_capacity(rows);
                        for row in 0..rows {
                            let started = std::time::Instant::now();
                            self.evaluate(
                                &input[row * inputs..(row + 1) * inputs],
                                &mut output[row * outputs..(row + 1) * outputs],
                            );
                            let mut timing = EvaluatorTiming::default();
                            timing.record(started);
                            timings.push(timing);
                        }
                        timings
                    }
                    Self::Dynamic(evaluator, last_error, modes) => {
                        let (timings, failure) = modes.isolated(|| {
                            evaluator.evaluate_batch(input, output, rows, inputs, outputs)
                        });
                        *last_error = failure;
                        if last_error.is_some() {
                            // The matrix owner does not expose the failing row.
                            // Discard every provisional row, then let existing
                            // scalar precision rescue retry each actual point.
                            output.fill($invalid);
                        }
                        timings
                    }
                }
            }
            pub(super) fn symjit_ir_bytes(&self) -> Option<usize> {
                match self {
                    #[cfg(feature = "native")]
                    Self::Symjit(evaluator) => Some(evaluator.as_bytes().len()),
                    Self::Eager(_) => None,
                    Self::Dynamic(evaluator, _, _) => evaluator.symjit_ir_bytes(),
                    Self::Checked(checked) => checked.evaluator.symjit_ir_bytes(),
                    Self::Observed(owner) => owner.evaluator.symjit_ir_bytes(),
                }
            }
            pub(super) fn last_dynamic_error(&self) -> Option<&str> {
                match self {
                    Self::Dynamic(inner, error, _) => {
                        error.as_deref().or_else(|| inner.last_dynamic_error())
                    }
                    Self::Checked(checked) => checked
                        .evaluator
                        .last_dynamic_error()
                        .or(checked.validation.last_error.as_deref()),
                    Self::Observed(owner) => owner.evaluator.last_dynamic_error(),
                    _ => None,
                }
            }
            pub(super) fn has_dynamic_callbacks(&self) -> bool {
                matches!(
                    self,
                    Self::Dynamic(..) | Self::Checked(..) | Self::Observed(..)
                )
            }
            pub(super) fn has_contour_callbacks(&self) -> bool {
                match self {
                    Self::Dynamic(inner, _, modes) => {
                        modes.contour || inner.has_contour_callbacks()
                    }
                    Self::Checked(_) => true,
                    Self::Observed(owner) => owner.evaluator.has_contour_callbacks(),
                    _ => false,
                }
            }
            pub(super) fn execution_backend(&self) -> EvaluatorBackend {
                match self {
                    #[cfg(feature = "native")]
                    Self::Symjit(_) => EvaluatorBackend::Symjit,
                    Self::Eager(_) => EvaluatorBackend::Eager,
                    Self::Dynamic(evaluator, _, _) => evaluator.execution_backend(),
                    Self::Checked(checked) => checked.evaluator.execution_backend(),
                    Self::Observed(owner) => owner.evaluator.execution_backend(),
                }
            }
            pub(super) fn clear_dynamic_attempt(&mut self) {
                match self {
                    Self::Dynamic(inner, error, _) => {
                        *error = None;
                        inner.clear_dynamic_attempt();
                    }
                    Self::Checked(checked) => {
                        checked.validation.clear_attempt();
                        checked.evaluator.clear_dynamic_attempt();
                    }
                    Self::Observed(owner) => owner.evaluator.clear_dynamic_attempt(),
                    _ => {}
                }
            }
            pub(super) fn validation(
                &self,
            ) -> Option<&crate::kernel::contour::dynamic::validation::Validation> {
                match self {
                    Self::Checked(checked) => Some(&checked.validation),
                    Self::Observed(owner) => owner.evaluator.validation(),
                    _ => None,
                }
            }
            pub(super) fn validation_mut(
                &mut self,
            ) -> Option<&mut crate::kernel::contour::dynamic::validation::Validation> {
                match self {
                    Self::Checked(checked) => Some(&mut checked.validation),
                    Self::Observed(owner) => owner.evaluator.validation_mut(),
                    _ => None,
                }
            }
            pub(super) fn diagnostics_configuration(&self) -> Configuration {
                match self {
                    Self::Observed(owner) => owner.configuration,
                    _ => Configuration::default(),
                }
            }
            pub(super) fn runtime_diagnostics(&self) -> Option<&Accumulator> {
                match self {
                    Self::Observed(owner) => Some(&owner.diagnostics),
                    _ => None,
                }
            }
            pub(super) fn clear_runtime_diagnostics(&mut self) {
                if let Self::Observed(owner) = self {
                    owner.diagnostics.clear();
                }
            }
            fn with_dynamic_fence(self, requirements: &MappingRequirements) -> Self {
                let modes = requirements.callback_modes();
                let result = if modes.algebraic
                    || (modes.contour && requirements.specification().is_none())
                {
                    Self::Dynamic(Box::new(self), None, modes)
                } else {
                    self
                };
                let result = if let Some(specification) = requirements.specification() {
                    Self::Checked(Box::new(checked::Checked::new(result, specification)))
                } else {
                    result
                };
                if requirements.has_dynamic_callbacks()
                    && requirements.diagnostics_configuration().enabled()
                {
                    Self::Observed(Box::new(observed::Observed::new(
                        result,
                        requirements.diagnostics_configuration(),
                    )))
                } else {
                    result
                }
            }
        }
    };
}
evaluator!(RealEvaluator, f64, f64::NAN, |value: &f64| *value);
evaluator!(
    ComplexEvaluator,
    Complex<f64>,
    Complex::new(f64::NAN, f64::NAN),
    |value: &Complex<f64>| if value.im == 0. { value.re } else { f64::NAN }
);

#[cfg(test)]
pub(super) fn real(
    exact: &ExactProgram,
    backend: EvaluatorBackend,
) -> Result<RealEvaluator, KernelError> {
    let requirements = MappingRequirements::new(exact).map_err(KernelError::Compilation)?;
    real_prepared(exact, backend, &requirements, None)
}

pub(super) fn real_prepared(
    exact: &ExactProgram,
    backend: EvaluatorBackend,
    requirements: &MappingRequirements,
    saved: Option<&[u8]>,
) -> Result<RealEvaluator, KernelError> {
    #[cfg(feature = "native")]
    if let Some(saved) = saved {
        return requirements
            .prepare(|| SavedPrimary::restore(saved))
            .map(|evaluator| RealEvaluator::Symjit(evaluator).with_dynamic_fence(requirements))
            .map_err(KernelError::Artifact);
    }
    #[cfg(feature = "portable")]
    let _ = saved;
    #[cfg(feature = "native")]
    if !backend.is_eager() {
        return requirements
            .prepare(|| exact.jit_compile::<f64>(settings()))
            .map(|evaluator| RealEvaluator::Symjit(evaluator).with_dynamic_fence(requirements))
            .map_err(KernelError::Compilation);
    }
    #[cfg(feature = "portable")]
    if backend == EvaluatorBackend::Symjit {
        return Err(KernelError::Compilation(
            "SymJIT is unavailable in a portable build".into(),
        ));
    }
    requirements
        .map(exact, |value| value.re.to_f64(), 53)
        .map(|evaluator| RealEvaluator::Eager(evaluator).with_dynamic_fence(requirements))
        .map_err(KernelError::Compilation)
}

#[cfg(test)]
pub(super) fn complex(
    exact: &ExactProgram,
    backend: EvaluatorBackend,
) -> Result<ComplexEvaluator, KernelError> {
    let requirements = MappingRequirements::new(exact).map_err(KernelError::Compilation)?;
    complex_prepared(exact, backend, &requirements, None)
}

pub(super) fn complex_prepared(
    exact: &ExactProgram,
    backend: EvaluatorBackend,
    requirements: &MappingRequirements,
    saved: Option<&[u8]>,
) -> Result<ComplexEvaluator, KernelError> {
    #[cfg(feature = "native")]
    if let Some(saved) = saved {
        return requirements
            .prepare(|| SavedPrimary::restore(saved))
            .map(|evaluator| ComplexEvaluator::Symjit(evaluator).with_dynamic_fence(requirements))
            .map_err(KernelError::Artifact);
    }
    #[cfg(feature = "portable")]
    let _ = saved;
    #[cfg(feature = "native")]
    if !backend.is_eager() {
        return requirements
            .prepare(|| exact.jit_compile::<Complex<f64>>(settings()))
            .map(|evaluator| ComplexEvaluator::Symjit(evaluator).with_dynamic_fence(requirements))
            .map_err(KernelError::Compilation);
    }
    #[cfg(feature = "portable")]
    if backend == EvaluatorBackend::Symjit {
        return Err(KernelError::Compilation(
            "SymJIT is unavailable in a portable build".into(),
        ));
    }
    requirements
        .map(
            exact,
            |value| Complex::new(value.re.to_f64(), value.im.to_f64()),
            53,
        )
        .map(|evaluator| ComplexEvaluator::Eager(evaluator).with_dynamic_fence(requirements))
        .map_err(KernelError::Compilation)
}

#[cfg(feature = "native")]
fn settings() -> symbolica::evaluate::JITCompilationSettings {
    symbolica::evaluate::JITCompilationSettings::default()
        .optimization_level(2)
        .direct_translation(true)
        .with_option("use_threads", "false")
}

#[cfg(test)]
mod dynamic_tests;
#[cfg(test)]
mod eager_tests;
#[cfg(test)]
mod tests;
