//! Shared original-coordinate bodies with actual source/sign authority.
use super::{native, validate_roles};
use crate::threshold::{
    gcad::{PositiveProjectiveQuotient, ProjectiveFactorAuthority},
    maps::normalized::{Budget, Error, Result, program::CompiledOpenMap},
    phase::causal_negative_phase,
};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluatorComposer, ExpressionEvaluator, FunctionMap, OptimizationSettings, Slot},
    symbol,
};

/// Reusable native source-coordinate IR. Its source, full split-sign vector and
/// caller function/input configuration are immutable. No map/root Scope or
/// endpoint authority is transferred by reusing this body.
pub struct CompiledDensityBody {
    source: Arc<PositiveProjectiveQuotient>,
    signs: Vec<i8>,
    coordinates: Vec<Symbol>,
    regulators: Vec<Symbol>,
    coefficients: Vec<Symbol>,
    functions: FunctionMap,
    settings: OptimizationSettings,
    phases: Vec<Atom>,
    bodies: Vec<Atom>,
    exact: ExpressionEvaluator<Complex<Rational>>,
}
impl CompiledDensityBody {
    /// Prepare once from an admitted map. The body retains the verified source
    /// owner and sign authority, not this particular map or its root callbacks.
    pub fn prepare(
        map: &CompiledOpenMap,
        regulators: &[Symbol],
        coefficients: &[Symbol],
        functions: FunctionMap,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        validate_roles(map, regulators, coefficients)?;
        // Body stages deliberately disable CPE; report these effective settings.
        let settings = settings.cpe_iterations(Some(0));
        let source = map.owner().source();
        let owner = source.decomposition();
        let preparation = owner.preparation();
        let coordinates = map.coordinates();
        let mut phases = Vec::with_capacity(preparation.terms().len());
        let mut bodies = Vec::with_capacity(preparation.terms().len());
        let mut raw_bodies = Vec::with_capacity(preparation.terms().len());
        let cell = &owner.native_result().cells[source.cell_index()];
        for (term, authorities) in preparation.terms().iter().zip(owner.factor_authorities()) {
            budget.check()?;
            let mut phase = Atom::one();
            let mut body = term.prefactor().clone();
            for (coordinate, exponent) in map.coordinates().iter().zip(term.monomial_powers()) {
                body *= Atom::var(*coordinate).pow(exponent);
            }
            for (factor, authority) in term.factors().iter().zip(authorities) {
                let polynomial = match authority {
                    ProjectiveFactorAuthority::Inactive => continue,
                    ProjectiveFactorAuthority::Polynomial
                    | ProjectiveFactorAuthority::Positive { .. } => factor.polynomial().clone(),
                    ProjectiveFactorAuthority::Split { index, .. } => {
                        let sign = cell.signs[*index];
                        if !matches!(sign, -1 | 1) {
                            return Err(Error::Unsupported(
                                "open causal factor needs its actual nonzero native cell sign"
                                    .into(),
                            ));
                        }
                        if sign < 0 {
                            phase *= causal_negative_phase(factor.exponent(), regulators)
                                .map_err(native)?;
                        }
                        Atom::num(sign) * factor.polynomial()
                    }
                };
                body *= polynomial.pow(factor.exponent());
            }
            body *= preparation.measure_jacobian();
            bodies.push(&body * &phase);
            raw_bodies.push(body);
            phases.push(phase);
        }
        let n = coordinates.len();
        let body_inputs = coordinates
            .iter()
            .chain(regulators)
            .chain(coefficients)
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>();
        let raw_body = Atom::evaluator_multiple(&raw_bodies, &body_inputs)
            .function_map(functions.clone())
            .optimization_settings(settings.clone().cpe_iterations(Some(0)))
            .build()
            .map_err(native)?;
        // Caller coefficient definitions may specialize the source body, but
        // cannot replace PI or any native constant in the issued causal phase.
        // Only real regulator inputs enter this separate native phase program.
        let phase = Atom::evaluator_multiple(
            &phases,
            &regulators
                .iter()
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
        )
        .optimization_settings(settings.clone().cpe_iterations(Some(0)))
        .build()
        .map_err(native)?;
        let mut body_compose = EvaluatorComposer::new(body_inputs.len());
        let mut body_slots = body_compose
            .append(
                &raw_body,
                &(0..body_inputs.len()).map(Slot::Param).collect::<Vec<_>>(),
            )
            .map_err(native)?;
        body_slots.extend(
            body_compose
                .append(
                    &phase,
                    &(n..n + regulators.len())
                        .map(Slot::Param)
                        .collect::<Vec<_>>(),
                )
                .map_err(native)?,
        );
        let body_formals = (0..2 * bodies.len())
            .map(|i| Atom::var(symbol!(&format!("fastsecdec::open_density::body_slot_{i}"))))
            .collect::<Vec<_>>();
        let phased = (0..bodies.len())
            .map(|i| &body_formals[i] * &body_formals[i + bodies.len()])
            .collect::<Vec<_>>();
        let multiply = Atom::evaluator_multiple(&phased, &body_formals)
            .optimization_settings(settings.clone().cpe_iterations(Some(0)))
            .build()
            .map_err(native)?;
        let body_outputs = body_compose
            .append(&multiply, &body_slots)
            .map_err(native)?;
        let body = body_compose
            .finish(&body_outputs, settings.clone().cpe_iterations(Some(0)))
            .map_err(native)?;
        budget.check()?;
        Ok(Self {
            source: owner.clone(),
            signs: cell.signs.clone(),
            coordinates: coordinates.to_vec(),
            regulators: regulators.to_vec(),
            coefficients: coefficients.to_vec(),
            functions,
            settings,
            phases,
            bodies,
            exact: body,
        })
    }
    pub(super) fn admit_map(&self, map: &CompiledOpenMap) -> Result<()> {
        let source = map.owner().source();
        let owner = source.decomposition();
        if !Arc::ptr_eq(owner, &self.source)
            || !Arc::ptr_eq(owner.preparation(), self.source.preparation())
            || map.coordinates() != self.coordinates
            || map.inputs().len() != self.coordinates.len()
            || owner.native_result().cells[source.cell_index()].signs != self.signs
        {
            return Err(Error::Invalid(
                "density body source, coordinate or split-sign authority differs".into(),
            ));
        }
        // A second map may introduce unit/root/chart names absent in the first.
        validate_roles(map, &self.regulators, &self.coefficients)
    }
    pub fn source_owner(&self) -> &Arc<PositiveProjectiveQuotient> {
        &self.source
    }
    pub fn split_signs(&self) -> &[i8] {
        &self.signs
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    pub fn regulators(&self) -> &[Symbol] {
        &self.regulators
    }
    pub fn coefficient_inputs(&self) -> &[Symbol] {
        &self.coefficients
    }
    pub fn function_map(&self) -> &FunctionMap {
        &self.functions
    }
    /// Effective settings used by every protected body stage (CPE is disabled),
    /// before the independently configured final map composition.
    pub fn optimization_settings(&self) -> &OptimizationSettings {
        &self.settings
    }
    pub fn term_phases(&self) -> &[Atom] {
        &self.phases
    }
    pub fn term_bodies(&self) -> &[Atom] {
        &self.bodies
    }
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
}
