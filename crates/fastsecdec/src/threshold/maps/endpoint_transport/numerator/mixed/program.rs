//! Reuses the protected native normal-jet compiler; no source/ray evaluation
//! happens before the clearing-normal coefficient and lower-face restriction.
use super::*;
use crate::kernel::algebraic::Number;
use crate::threshold::maps::endpoint_transport::numerator::normal::program::{
    JetInput, JetProgram,
};
use symbolica::{
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluationDomain, ExpressionEvaluator, FunctionMap},
};
pub struct CompiledMixedNumeratorFace {
    owner: Arc<MixedNumeratorFace>,
    data: JetProgram,
}
impl CompiledMixedNumeratorFace {
    pub fn prepare(
        owner: Arc<MixedNumeratorFace>,
        coefficient_inputs: &[Symbol],
        source_functions: FunctionMap,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        let values = owner
            .coefficients
            .iter()
            .flat_map(|c| c.values.iter().cloned())
            .collect::<Vec<_>>();
        let data = JetProgram::prepare(
            JetInput {
                catalogue: owner.face.catalogue(),
                chart: &owner.chart,
                sections: &owner.sections,
                directions: &owner.directions,
                head: owner.head,
                formals: &owner.formals,
                definitions: &owner.definitions,
                values: &values,
            },
            coefficient_inputs,
            source_functions,
            settings,
            budget,
        )?;
        Ok(Self { owner, data })
    }
    pub fn owner(&self) -> &Arc<MixedNumeratorFace> {
        &self.owner
    }
    pub fn coefficient_inputs(&self) -> &[Symbol] {
        &self.data.coefficient_inputs
    }
    /// Source-record/clearing-order/lower-order Taylor coefficients followed by retained finite
    /// geometric inputs. Arithmetic bytes require this live source/chart owner.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.data.exact
    }
    /// Executed once per call and shared by all demanded source partials.
    pub fn coefficient_stage(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.data.coefficient_stage
    }
    pub fn scope(&self) -> &Scope {
        &self.owner.scope
    }
    pub fn map<T: Number>(self: &Arc<Self>, bits: u32) -> Result<NumericMixedNumeratorFace<T>>
    where
        Complex<T>: EvaluationDomain,
    {
        let program = self
            .scope()
            .map_checked(&self.data.exact, bits, |q| {
                Complex::new(T::rational_at(&q.re, bits), T::rational_at(&q.im, bits))
            })
            .map_err(Error::Native)?;
        Ok(NumericMixedNumeratorFace {
            owner: self.clone(),
            program,
            bits,
        })
    }
}
/// Caller-owned scratch; normal=0 is already bound symbolically. Remaining
/// direction inputs are strict, but a finite output may legitimately vanish.
pub struct NumericMixedNumeratorFace<T> {
    owner: Arc<CompiledMixedNumeratorFace>,
    program: ExpressionEvaluator<Complex<T>>,
    bits: u32,
}
impl<T: Number> NumericMixedNumeratorFace<T>
where
    Complex<T>: EvaluationDomain,
{
    pub fn evaluate(
        &mut self,
        directions: &[T],
        coefficients: &[Complex<T>],
    ) -> Result<Vec<Vec<Complex<T>>>> {
        let outputs = self.owner.data.evaluate(
            &mut self.program,
            self.bits,
            self.owner.owner.directions.len(),
            directions,
            coefficients,
        )?;
        Ok(outputs
            .chunks(self.owner.owner.lower_depth as usize)
            .map(|v| v.to_vec())
            .collect())
    }
}
