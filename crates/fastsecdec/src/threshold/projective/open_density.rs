//! Source-bound open-interior density composition. This is neither endpoint
//! continuation nor a prepared integration kernel. Native functions and caller
//! coefficient inputs retain their ordinary numerical, not certified, semantics.
use crate::{
    generation::endpoint_power_with_regulators,
    kernel::algebraic::{Number, attempt},
    threshold::maps::normalized::{Budget, Error, Result, program::CompiledOpenMap},
};
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{
        EvaluationDomain, EvaluatorComposer, ExpressionEvaluator, FunctionMap,
        OptimizationSettings, Slot,
    },
    symbol,
};

mod body;
pub use body::CompiledDensityBody;

/// All original prepared terms, evaluated in original source coordinates and
/// composed through one admitted map. No root expression is substituted into
/// a numerator. The retained map keeps the complete source/proof association.
pub struct CompiledOpenDensity {
    map: Arc<CompiledOpenMap>,
    body: Arc<CompiledDensityBody>,
    composition_settings: OptimizationSettings,
    exact: ExpressionEvaluator<Complex<Rational>>,
}

impl CompiledOpenDensity {
    /// `regulators` starts with the original physical epsilon. Coefficient
    /// inputs and FunctionMap are explicit caller execution choices; they do
    /// not bind a physical model or issue new geometric authority. Regulator
    /// values must be real and away from any poles of the supplied body.
    pub fn prepare(
        map: Arc<CompiledOpenMap>,
        regulators: &[Symbol],
        coefficients: &[Symbol],
        functions: FunctionMap,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        let body = Arc::new(CompiledDensityBody::prepare(
            &map,
            regulators,
            coefficients,
            functions,
            settings.clone(),
            budget,
        )?);
        Self::prepare_with_body(map, body, settings, budget)
    }
    /// Attach a shared body after checking this map's source, signs and roles.
    /// Composer copies its instructions and may optimize the full resulting IR;
    /// body preparation reuse does not promise shared final evaluator storage.
    pub fn prepare_with_body(
        map: Arc<CompiledOpenMap>,
        body: Arc<CompiledDensityBody>,
        composition_settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        body.admit_map(&map)?;
        let n = map.coordinates().len();
        let regulators = body.regulators();
        let coefficients = body.coefficient_inputs();
        let settings = &composition_settings;
        let bodies = body.term_bodies();
        let mut compose = EvaluatorComposer::new(n + regulators.len() + coefficients.len());
        let map_slots = compose
            .append(map.exact(), &(0..n).map(Slot::Param).collect::<Vec<_>>())
            .map_err(native)?;
        let mut body_slots = map_slots[..n].to_vec();
        body_slots.extend((n..n + regulators.len() + coefficients.len()).map(Slot::Param));
        let term_slots = compose.append(body.exact(), &body_slots).map_err(native)?;
        // Small native arithmetic stage: multiply every term by the same
        // positive map Jacobian exactly once, and retain their complete sum.
        let j = Atom::var(symbol!("fastsecdec::open_density::jacobian"));
        let terms = (0..bodies.len())
            .map(|i| Atom::var(symbol!(&format!("fastsecdec::open_density::term_{i}"))))
            .collect::<Vec<_>>();
        let weighted = terms.iter().map(|t| t * &j).collect::<Vec<_>>();
        let mut final_bodies = vec![weighted.iter().sum::<Atom>()];
        final_bodies.extend(weighted);
        let mut arguments = terms;
        arguments.push(j);
        let finish = Atom::evaluator_multiple(&final_bodies, &arguments)
            .optimization_settings(settings.clone().cpe_iterations(Some(0)))
            .build()
            .map_err(native)?;
        let mut values = term_slots;
        values.push(map_slots[n]);
        let mut outputs = compose.append(&finish, &values).map_err(native)?;
        // Retain coordinates, Jacobian, and all individual positive diagonals
        // even when a numerator/term cancels the corresponding arithmetic.
        outputs.extend(map_slots);
        let exact = compose
            .finish(&outputs, composition_settings.clone())
            .map_err(native)?;
        budget.check()?;
        Ok(Self {
            map,
            body,
            composition_settings,
            exact,
        })
    }
    pub fn map_owner(&self) -> &Arc<CompiledOpenMap> {
        &self.map
    }
    pub fn body_owner(&self) -> &Arc<CompiledDensityBody> {
        &self.body
    }
    /// Final composition settings, independent of the shared body's settings.
    pub fn composition_settings(&self) -> &OptimizationSettings {
        &self.composition_settings
    }
    pub fn regulators(&self) -> &[Symbol] {
        self.body.regulators()
    }
    pub fn coefficient_inputs(&self) -> &[Symbol] {
        self.body.coefficient_inputs()
    }
    pub fn term_phases(&self) -> &[Atom] {
        self.body.term_phases()
    }
    /// Original source-coordinate expressions, in original term order. They
    /// contain causal phases but no coordinate-map substitution or map Jacobian.
    pub fn term_bodies(&self) -> &[Atom] {
        self.body.term_bodies()
    }
    /// Native source-body IR inputs: source coordinates, regulators, coefficients.
    pub fn body_exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        self.body.exact()
    }
    /// Native composed outputs: total, all terms, source coordinates, J, diagonals.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    pub fn map<T: Number>(self: &Arc<Self>, bits: u32) -> Result<NumericOpenDensity<T>>
    where
        Complex<T>: EvaluationDomain,
    {
        let program = self
            .map
            .scope()
            .map_checked(&self.exact, bits, |q| {
                Complex::new(T::rational_at(&q.re, bits), T::rational_at(&q.im, bits))
            })
            .map_err(Error::Native)?;
        Ok(NumericOpenDensity {
            owner: self.clone(),
            program,
            bits,
        })
    }
}

/// Caller-owned execution and scratch. Every returned term includes J; the
/// complete sum has no Laurent, integration-error or endpoint interpretation.
pub struct OpenDensityValue<T> {
    pub total: Complex<T>,
    pub terms: Vec<Complex<T>>,
    pub coordinates: Vec<T>,
    pub jacobian: T,
}
pub struct NumericOpenDensity<T> {
    owner: Arc<CompiledOpenDensity>,
    program: ExpressionEvaluator<Complex<T>>,
    bits: u32,
}
impl<T: Number> NumericOpenDensity<T>
where
    Complex<T>: EvaluationDomain,
{
    pub fn owner(&self) -> &Arc<CompiledOpenDensity> {
        &self.owner
    }
    pub fn evaluate(
        &mut self,
        unit: &[T],
        regulators: &[T],
        coefficients: &[Complex<T>],
    ) -> Result<OpenDensityValue<T>> {
        let n = self.owner.map.inputs().len();
        if unit.len() != n
            || regulators.len() != self.owner.regulators().len()
            || coefficients.len() != self.owner.coefficient_inputs().len()
            || unit
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
            || regulators.iter().any(|x| !x.valid())
            || coefficients.iter().any(|x| !x.re.valid() || !x.im.valid())
        {
            return Err(Error::Invalid("open density needs strict interior real units, finite real regulators and the declared coefficient inputs".into()));
        }
        let mut inputs = unit
            .iter()
            .chain(regulators)
            .map(|x| Complex::new(x.clone(), T::exact_zero_at(self.bits)))
            .collect::<Vec<_>>();
        inputs.extend_from_slice(coefficients);
        let m = self.owner.term_bodies().len();
        let zero = unit[0].zero();
        let mut outputs = vec![Complex::new(zero.clone(), zero); m + 2 * n + 2];
        attempt(|| self.program.try_evaluate(&inputs, &mut outputs))
            .map_err(Error::Native)?
            .map_err(native)?;
        if outputs.iter().any(|x| !x.re.valid() || !x.im.valid())
            || outputs[m + 1..]
                .iter()
                .any(|x| !x.im.is_zero() || x.re <= x.re.zero())
        {
            return Err(Error::Native("open density has invalid output or nonpositive source coordinate/Jacobian/diagonal".into()));
        }
        Ok(OpenDensityValue {
            total: outputs[0].clone(),
            terms: outputs[1..m + 1].to_vec(),
            coordinates: outputs[m + 1..m + 1 + n]
                .iter()
                .map(|x| x.re.clone())
                .collect(),
            jacobian: outputs[m + 1 + n].re.clone(),
        })
    }
}
fn validate_roles(
    map: &CompiledOpenMap,
    regulators: &[Symbol],
    coefficients: &[Symbol],
) -> Result<()> {
    let source = map.owner().source();
    let preparation = source.decomposition().preparation();
    if regulators.first() != Some(&preparation.input().regulator()) {
        return Err(Error::Invalid(
            "open density requires original physical regulator first".into(),
        ));
    }
    endpoint_power_with_regulators(&Atom::Zero, regulators).map_err(native)?;
    let mut roles: BTreeSet<_> = preparation.input().parameters().iter().copied().collect();
    roles.extend(map.inputs().iter().copied());
    for step in map.owner().steps() {
        if let crate::threshold::maps::normalized::PrefixStep::Section { section, .. } = step {
            roles.insert(section.root_symbol());
            roles.extend(section.chart().parameters.iter().copied());
        }
    }
    for role in regulators.iter().chain(coefficients) {
        if !roles.insert(*role) {
            return Err(Error::Invalid(
                "open density coordinate/regulator/coefficient roles overlap".into(),
            ));
        }
    }
    Ok(())
}
fn native(error: impl std::fmt::Display) -> Error {
    Error::Native(error.to_string())
}

#[cfg(test)]
mod tests;
