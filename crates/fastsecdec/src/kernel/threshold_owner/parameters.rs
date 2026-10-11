//! Detached exact admission associated with a complete source-issued template.
//! This module needs no GCAD and creates no uniform geometry certificate.
use super::*;
use crate::kernel::parameter_guard::ExactParameterGuard;

impl ThresholdMetadata {
    pub(crate) fn runtime_parameters(&self) -> Result<Vec<Symbol>, KernelError> {
        self.manifest
            .preparation
            .runtime_chamber
            .as_ref()
            .map_or_else(
                || Ok(Vec::new()),
                |c| {
                    c.runtime_parameters
                        .iter()
                        .map(|id| self.native_symbol(*id))
                        .collect()
                },
            )
    }
    pub(crate) fn parameter_guard(&self) -> Option<&Arc<ExactParameterGuard>> {
        self.parameter_guard.as_ref()
    }
    pub(crate) fn attach_parameter_guard(
        &mut self,
        guard: Arc<ExactParameterGuard>,
    ) -> Result<(), KernelError> {
        self.check_parameter_guard(&guard)?;
        self.parameter_guard = Some(guard);
        Ok(())
    }
    fn check_parameter_guard(&self, guard: &ExactParameterGuard) -> Result<(), KernelError> {
        let chamber = self
            .manifest
            .preparation
            .runtime_chamber
            .as_ref()
            .ok_or_else(|| invalid("parameter guard without uniform chamber descriptor"))?;
        if guard.public_parameters() != self.runtime_parameters()?
            || guard.prefix_to_public() != chamber.prefix_to_public
            || guard.semantic_identity().map_err(invalid)? != chamber.guard_semantic.0
            || guard.predicate_roles().map_err(invalid)? != chamber.predicates
        {
            return Err(invalid("exact parameter guard differs from issued chamber"));
        }
        Ok(())
    }
    pub(crate) fn validate_parameter_guard(&self, runtime: &[Symbol]) -> Result<(), KernelError> {
        match (
            &self.manifest.preparation.runtime_chamber,
            &self.parameter_guard,
        ) {
            (None, None) if runtime.is_empty() => Ok(()),
            (Some(_), Some(guard))
                if !runtime.is_empty() && guard.public_parameters() == runtime =>
            {
                self.check_parameter_guard(guard)
            }
            _ => Err(invalid(
                "uniform chamber/native runtime/guard capability mismatch",
            )),
        }
    }
    pub(crate) fn admit_parameter_point(
        &self,
        runtime: &[Symbol],
        ordered: &[f64],
    ) -> Result<(), KernelError> {
        // Full association is checked during native owner admission. Binding
        // does exact native scalar conversion/evaluation only, never canonical
        // projection, polynomial construction, optimization or geometry work.
        match (
            &self.manifest.preparation.runtime_chamber,
            &self.parameter_guard,
        ) {
            (None, None) if runtime.is_empty() && ordered.is_empty() => Ok(()),
            (Some(_), Some(guard))
                if guard.public_parameters() == runtime && runtime.len() == ordered.len() =>
            {
                let point = runtime
                    .iter()
                    .copied()
                    .zip(ordered.iter().map(|v| Rational::try_from(*v)))
                    .map(|(s, v)| v.map(|v| (s, v)).map_err(invalid))
                    .collect::<Result<BTreeMap<_, _>, _>>()?;
                guard.admit(&point).map_err(invalid)?;
                Ok(())
            }
            _ => Err(invalid("parameter point has no associated exact guard")),
        }
    }
}
