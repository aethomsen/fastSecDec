//! Admission for upstream coefficient mapping, which assumes that each external
//! callback and constant supports the requested numeric domain. This inspects
//! native callback metadata only; it does not validate or rewrite native IR.
use super::super::program::{self, Callback, ExactProgram};
use crate::kernel::contour::dynamic::validation::{self, Specification, Validation};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore},
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluationDomain, ExpressionEvaluator},
};

pub(in crate::kernel) struct MappingRequirements(
    Vec<Callback>,
    crate::contour::functions::dynamic::ProgramScope,
    crate::contour::functions::dynamic::requested::Mode,
    bool,
    Option<Arc<Specification>>,
    crate::contour::functions::dynamic::diagnostics::Configuration,
    crate::kernel::algebraic::Scope,
);

impl MappingRequirements {
    pub(in crate::kernel) fn diagnostics_configuration(
        &self,
    ) -> crate::contour::functions::dynamic::diagnostics::Configuration {
        self.5
    }
    /// Recognize the owner's essential callback-failure protocol once while
    /// preparing the evaluator. Ordinary and fixed programs never enter its
    /// thread-local attempt machinery while sampling.
    pub(in crate::kernel) fn has_dynamic_callbacks(&self) -> bool {
        self.3 || !self.6.is_empty()
    }
    pub(in crate::kernel) fn callback_modes(&self) -> crate::kernel::callback_attempt::Modes {
        crate::kernel::callback_attempt::Modes {
            contour: self.3,
            algebraic: !self.6.is_empty(),
        }
    }
    pub(in crate::kernel) fn validation(&self) -> Option<Validation> {
        self.4.clone().map(Validation::new)
    }
    pub(in crate::kernel) fn specification(&self) -> Option<Arc<Specification>> {
        self.4.clone()
    }

    /// Native mapping can evaluate fixed-argument callbacks. A failed constant
    /// must fail preparation even if downstream instructions mask its NaN.
    #[cfg(feature = "native")]
    pub(in crate::kernel) fn prepare<T>(
        &self,
        prepare: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.prepare_at(53, prepare)
    }
    fn prepare_at<T>(
        &self,
        bits: u32,
        prepare: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        self.6.enter(bits, || {
            let _diagnostics = self.5.enter();
            let _preparing = self.1.enter();
            let _observing = self.2.enter();
            let _checking = validation::enter_optional(self.4.clone());
            if !self.has_dynamic_callbacks() {
                return prepare();
            }
            let (result, failure) = self.callback_modes().isolated(prepare);
            match failure {
                Some(error) => Err(format!("native callback preparation failed: {error}")),
                None => result,
            }
        })
    }

    /// Called while constructing the kernel, before caller-owned workers start.
    /// In particular, parsing canonical native tags never happens per sample.
    pub(in crate::kernel) fn new(exact: &ExactProgram) -> Result<Arc<Self>, String> {
        let mut requirements = Self(
            program::callbacks(exact)?,
            Default::default(),
            crate::contour::functions::dynamic::requested::Mode::capture(),
            false,
            validation::capture(),
            crate::contour::functions::dynamic::diagnostics::Configuration::capture(),
            Default::default(),
        );
        if let Some(specification) = &requirements.4 {
            specification.admit_callbacks(&requirements.0)?;
            requirements.2 = crate::contour::functions::dynamic::requested::Mode::checked();
        }
        requirements.3 = requirements.0.iter().any(|requirement| {
            matches!(
                requirement.symbol.get_name(),
                "fastsecdec::contour::dynamic::strength_v1"
                    | "fastsecdec::contour::dynamic::requested_strength_v1"
                    | "fastsecdec::contour::smooth_hypot_v1"
                    | "fastsecdec::contour::smooth_positive_v1"
            )
        });
        requirements.1 = crate::contour::functions::dynamic::ProgramScope::capture_for(
            requirements.0.iter().flat_map(|requirement| {
                requirement
                    .tags
                    .iter()
                    .flat_map(|tag| tag.get_all_symbols(true))
            }),
        )?;
        requirements.6 = crate::kernel::algebraic::Scope::capture_for(&requirements.0)?;
        Ok(Arc::new(requirements))
    }

    pub(in crate::kernel) fn map<T: EvaluationDomain>(
        &self,
        exact: &ExactProgram,
        coefficient: impl Fn(&Complex<Rational>) -> T,
        bits: u32,
    ) -> Result<ExpressionEvaluator<T>, String> {
        self.prepare_at(bits, || {
            crate::contour::functions::dynamic::with_precision(bits, || {
                for requirement in &self.0 {
                    let info = requirement.symbol.get_evaluation_info().ok_or_else(|| {
                        format!(
                            "External function '{}' has no evaluation info",
                            requirement.symbol
                        )
            })?;
            let tags = requirement
                .tags
                .iter()
                .map(Atom::as_view)
                .collect::<Vec<_>>();
            if let Some(args) = &requirement.fixed_args {
                if args.is_empty() {
                    // Reuse the owner's fallible constant/domain conversion.
                    // Symbolica caches untagged 53-bit constants internally.
                    T::try_from_complex_float(info.evaluate_constant(&tags, bits)?)?;
                    continue;
                }
                for value in args {
                    T::try_from_complex_float(Complex::new(
                        value.re.to_multi_prec_float(bits),
                        value.im.to_multi_prec_float(bits),
                    ))?;
                }
            }
            if T::resolve_function(&tags, info).is_none() {
                return Err(format!(
                    "External function '{}' has no implementation for the requested numeric domain",
                    requirement.symbol
                ));
            }
        }
        // No generic constant fallback: fixed functions must have their own
        // implementation in T. User callbacks retain Symbolica's own contract.
        Ok(exact.clone().map_coeff_with_prec(&coefficient, bits))
            })
        })
    }
}
