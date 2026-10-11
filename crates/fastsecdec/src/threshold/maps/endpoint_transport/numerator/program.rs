//! Finite source inputs on an issued relative face. Native dependency slicing
//! precedes endpoint binding; the complete open map is never evaluated there.
use super::{Budget, Error, NumeratorFace, Result, native};
use crate::{
    kernel::algebraic::{Number, Scope, attempt},
    threshold::maps::normalized::program::CompiledOpenMap,
};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluatorComposer, ExpressionEvaluator, Instruction, OptimizationSettings, Slot},
};

fn require_straight_line(exact: &ExpressionEvaluator<Complex<Rational>>) -> Result<()> {
    if exact.export_instructions().instructions.iter().any(|i| {
        matches!(
            i,
            Instruction::IfElse(..)
                | Instruction::Goto(..)
                | Instruction::Label(..)
                | Instruction::Join(..)
        )
    }) {
        return Err(Error::Unsupported(
            "finite face input slicing requires straight-line native map instructions".into(),
        ));
    }
    Ok(())
}

/// Native finite-coordinate inputs retaining the actual face and selected roots.
/// Outputs follow the issued finite source coordinates, then direction units.
/// No complete density, endpoint unit or subtraction authority is implied.
pub struct CompiledFaceInputs {
    face: Arc<NumeratorFace>,
    map: Arc<CompiledOpenMap>,
    exact: ExpressionEvaluator<Complex<Rational>>,
    outputs: Vec<Symbol>,
}
impl CompiledFaceInputs {
    pub fn prepare(
        face: Arc<NumeratorFace>,
        map: Arc<CompiledOpenMap>,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        if !Arc::ptr_eq(map.owner(), face.transport().parent_prefix()) {
            return Err(Error::Invalid(
                "finite face inputs require the same admitted normalized map owner".into(),
            ));
        }
        // The first class obtains roots through finite source coordinates only.
        // Raw root formals must never be mistaken for external numeric inputs.
        for coefficient in face.coefficients() {
            budget.check()?;
            for body in coefficient
                .coefficients()
                .iter()
                .chain(std::iter::once(coefficient.direction_divisor()))
            {
                if face
                    .root_symbols()
                    .iter()
                    .any(|s| body.contains(Atom::var(*s).as_view()))
                {
                    return Err(Error::Unsupported(
                        "finite face coefficient contains an unselected raw root role".into(),
                    ));
                }
            }
        }
        require_straight_line(map.exact())?;
        let intermediate = settings.clone().cpe_iterations(Some(0));
        let mut select = EvaluatorComposer::new(map.inputs().len());
        let slots = (0..map.inputs().len()).map(Slot::Param).collect::<Vec<_>>();
        let all = select.append(map.exact(), &slots).map_err(native)?;
        let selected = face
            .finite_coordinates()
            .iter()
            .map(|symbol| {
                map.coordinates()
                    .iter()
                    .position(|s| s == symbol)
                    .map(|i| all[i])
                    .ok_or_else(|| Error::Invalid("foreign finite source-coordinate role".into()))
            })
            .collect::<Result<Vec<_>>>()?;
        // Native straight-line slicing must finish under original independent
        // inputs BEFORE any endpoint binding or numeric coefficient mapping.
        let finite = select
            .finish(&selected, intermediate.clone())
            .map_err(native)?;
        budget.check()?;
        let chart = &face.transport().charts()[face.chart_index()];
        let images = chart
            .unit_images()
            .iter()
            .map(|a| super::super::clearing::pull(a, &[face.normal()], &[Atom::zero()]))
            .collect::<Vec<_>>();
        let inputs = face
            .direction_units()
            .iter()
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>();
        let binding = Atom::evaluator_multiple(&images, &inputs)
            .optimization_settings(intermediate)
            .build()
            .map_err(native)?;
        let mut compose = EvaluatorComposer::new(inputs.len());
        let directions = (0..inputs.len()).map(Slot::Param).collect::<Vec<_>>();
        let bound = compose.append(&binding, &directions).map_err(native)?;
        let mut values = compose.append(&finite, &bound).map_err(native)?;
        values.extend(directions);
        let exact = compose.finish(&values, settings).map_err(native)?;
        let outputs = face
            .finite_coordinates()
            .iter()
            .chain(face.direction_units())
            .copied()
            .collect();
        budget.check()?;
        Ok(Self {
            face,
            map,
            exact,
            outputs,
        })
    }
    pub fn face(&self) -> &Arc<NumeratorFace> {
        &self.face
    }
    pub fn inputs(&self) -> &[Symbol] {
        self.face.direction_units()
    }
    pub fn outputs(&self) -> &[Symbol] {
        &self.outputs
    }
    /// Native finite-input IR, with the live face/map ownership retained here.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    pub fn scope(&self) -> &Scope {
        self.map.scope()
    }
    pub fn map<T: Number>(self: &Arc<Self>, bits: u32) -> Result<NumericFaceInputs<T>> {
        let program = self
            .scope()
            .map_checked(&self.exact, bits, |q| T::rational_at(&q.re, bits))
            .map_err(Error::Native)?;
        Ok(NumericFaceInputs {
            owner: self.clone(),
            program,
            bits,
        })
    }
}

/// Caller-owned native scratch. Only remaining direction inputs are restricted
/// to (0,1); a genuine finite zero output is allowed. Retained singularities or
/// callback failures refuse rather than inferring a removable limit.
pub struct NumericFaceInputs<T> {
    owner: Arc<CompiledFaceInputs>,
    program: ExpressionEvaluator<T>,
    bits: u32,
}
impl<T: Number> NumericFaceInputs<T> {
    pub fn owner(&self) -> &Arc<CompiledFaceInputs> {
        &self.owner
    }
    pub fn evaluate(&mut self, directions: &[T]) -> Result<Vec<T>> {
        if directions.len() != self.owner.inputs().len()
            || directions
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Invalid(
                "finite face directions must be valid and strictly inside the relative unit face"
                    .into(),
            ));
        }
        let zero = directions
            .first()
            .map_or_else(|| T::exact_zero_at(self.bits), |x| x.zero());
        let mut outputs = vec![zero; self.owner.outputs().len()];
        attempt(|| self.program.try_evaluate(directions, &mut outputs))
            .map_err(Error::Native)?
            .map_err(native)?;
        if outputs.iter().any(|x| !x.valid()) {
            return Err(Error::Native(
                "invalid native finite face input value".into(),
            ));
        }
        Ok(outputs)
    }
}

#[cfg(test)]
mod tests;
