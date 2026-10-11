//! Geometry-neutral input for a privately admitted, record-local native vector.
//! This input alone carries no physical lineage or artifact publication right.
use super::*;
use std::{collections::BTreeSet, sync::Arc};
use symbolica::evaluate::FunctionMap;

pub(crate) struct PreparedCoefficientVector {
    pub coordinates: Vec<Symbol>,
    pub runtime_parameters: Vec<Symbol>,
    pub coefficients: Vec<AliasedAtom>,
    pub functions: Arc<FunctionMap>,
    pub endpoint_profiles: Vec<crate::generation::EndpointProfileRow>,
}

impl PreparedCoefficientVector {
    pub(in crate::kernel) fn build(
        &self,
        settings: CompilationSettings,
    ) -> Result<SectorProgram, KernelError> {
        settings.validate()?;
        if self.coordinates.is_empty() || self.coefficients.is_empty() {
            return Err(KernelError::Compilation(
                "a stochastic prepared vector needs coordinates and outputs".into(),
            ));
        }
        if self
            .coordinates
            .iter()
            .chain(&self.runtime_parameters)
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != self.coordinates.len() + self.runtime_parameters.len()
        {
            return Err(KernelError::Compilation(
                "duplicate prepared coordinate or runtime role".into(),
            ));
        }
        let cancellation = Cancellation::from_endpoint_profiles(
            self.endpoint_profiles.clone(),
            self.coordinates.len(),
        )?;
        let aliases = shared_aliases(&self.coefficients)?;
        let roots = self
            .coefficients
            .iter()
            .map(AliasedAtom::get_root)
            .collect::<Vec<_>>();
        let variables = self
            .coordinates
            .iter()
            .chain(&self.runtime_parameters)
            .map(|s| Atom::var(*s))
            .collect::<Vec<_>>();
        let exact = build_expression_program(
            &roots,
            &variables,
            self.functions.as_ref().clone(),
            aliases,
            settings,
        )?;
        Ok(SectorProgram {
            symbolic_endpoint_contour_partials: None,
            parameters: self.coordinates.clone(),
            runtime_parameters: self.runtime_parameters.clone(),
            exact,
            cancellation,
            exact_zero: roots.iter().map(|root| root.is_zero()).collect(),
            // Keep native complex intermediates for causal phases and registered
            // function bodies. No new realness inference is made by this seam.
            real_coefficients: vec![false; roots.len()],
        })
    }
}
