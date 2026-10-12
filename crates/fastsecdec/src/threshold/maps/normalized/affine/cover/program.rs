//! Native composition of an issued prefix restriction and its owned root tower.
//! Detached instructions execute arithmetic; the retained leaf supplies authority.
use super::{AffinePrefixLeaf, Budget, Error, Result, native};
use crate::{
    kernel::algebraic::{Number, Scope, attempt},
    threshold::maps::normalized::affine::program::prepare_outputs,
};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluatorComposer, ExpressionEvaluator, OptimizationSettings, Slot},
    symbol,
};

/// One complete-cover leaf composed before the original admitted prefix.
/// Outputs are CAD prefix coordinates, its positive partial measure, then the
/// cover's owned dependent slots in their declared order. No root is a parameter.
pub struct CompiledAffineRestriction {
    leaf: AffinePrefixLeaf,
    scope: Scope,
    exact: ExpressionEvaluator<Complex<Rational>>,
    checked: ExpressionEvaluator<Complex<Rational>>,
    inputs: Vec<Symbol>,
    coordinates: Vec<Symbol>,
}
impl CompiledAffineRestriction {
    pub fn prepare(
        leaf: AffinePrefixLeaf,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        let cover = leaf.cover();
        let owner = cover.parent();
        let n = owner.len();
        let d = cover.dependent_symbols().len();
        let public_len = n
            .checked_add(1)
            .and_then(|v| v.checked_add(d))
            .ok_or_else(|| Error::ResourceIncomplete("restricted prefix output count".into()))?;
        let checked_len = public_len
            .checked_add(n)
            .and_then(|v| v.checked_add(1))
            .and_then(|v| v.checked_add(n))
            .ok_or_else(|| Error::ResourceIncomplete("restricted prefix guard count".into()))?;
        if n == 0 || leaf.restriction().unit_images().len() != n {
            return Err(Error::Invalid("restricted prefix input dimension".into()));
        }
        let inputs = owner.source().unit_coordinates()[..n].to_vec();
        let coordinates = owner.source().axes()[..n]
            .iter()
            .map(|a| a.symbol())
            .collect();
        let atoms = inputs.iter().copied().map(Atom::var).collect::<Vec<_>>();
        let extra = cover
            .dependent_symbols()
            .iter()
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>();
        let intermediate = settings.clone().cpe_iterations(Some(0));
        let (scope, parent) = prepare_outputs(owner, intermediate.clone(), budget, &extra)?;
        let mut restriction = leaf.restriction().unit_images().to_vec();
        restriction.push(leaf.restriction().measure().clone());
        let restriction = Atom::evaluator_multiple(&restriction, &atoms)
            .optimization_settings(intermediate.clone())
            .build()
            .map_err(native)?;
        let mut compose = EvaluatorComposer::new(n);
        let input_slots = (0..n).map(Slot::Param).collect::<Vec<_>>();
        let restriction = compose.append(&restriction, &input_slots).map_err(native)?;
        let parent = compose.append(&parent, &restriction[..n]).map_err(native)?;
        // Formal multiplication inputs are local to this tiny native operation;
        // neither becomes an additional external parameter or root role.
        let (j, m) = symbol!(
            "affine_restriction_program::jacobian",
            "affine_restriction_program::measure"
        );
        let product = Atom::evaluator_multiple(
            &[Atom::var(j) * Atom::var(m)],
            &[Atom::var(j), Atom::var(m)],
        )
        .optimization_settings(intermediate)
        .build()
        .map_err(native)?;
        let jacobian = compose
            .append(&product, &[parent[n], restriction[n]])
            .map_err(native)?[0];
        let mut outputs = parent[..n].to_vec();
        outputs.push(jacobian);
        outputs.extend_from_slice(&parent[2 * n + 1..]);
        // Numeric evaluation retains every original positive diagonal, the new
        // positive measure and mapped strict-open inputs. These are guards only.
        outputs.extend_from_slice(&parent[n + 1..2 * n + 1]);
        outputs.push(restriction[n]);
        outputs.extend_from_slice(&restriction[..n]);
        if outputs.len() != checked_len {
            return Err(Error::Invalid("restricted prefix output layout".into()));
        }
        let checked = compose.finish(&outputs, settings.clone()).map_err(native)?;
        // Native selection removes validation-only outputs from the exact IR
        // exposed for subsequent coefficient composition. No point is evaluated.
        let mut projection = EvaluatorComposer::new(n);
        let all = projection.append(&checked, &input_slots).map_err(native)?;
        let exact = projection
            .finish(&all[..public_len], settings)
            .map_err(native)?;
        budget.check()?;
        Ok(Self {
            leaf,
            scope,
            exact,
            checked,
            inputs,
            coordinates,
        })
    }
    pub fn leaf(&self) -> &AffinePrefixLeaf {
        &self.leaf
    }
    pub fn inputs(&self) -> &[Symbol] {
        &self.inputs
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    pub fn dependent_symbols(&self) -> &[Symbol] {
        self.leaf.cover().dependent_symbols()
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    /// CAD coordinates, positive composed partial J, then owned dependent slots.
    /// The caller must retain `leaf()` when composing this execution data.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    pub(super) fn checked(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.checked
    }
    pub fn map<T: Number>(&self, bits: u32) -> NumericAffineRestriction<T> {
        let one = T::one_at(bits);
        let program = self.scope.enter(bits, || {
            self.checked
                .clone()
                .map_coeff_with_prec(&|q| one.from_rational(&q.re), bits)
        });
        NumericAffineRestriction {
            program,
            leaf: self.leaf.clone(),
        }
    }
    #[cfg(test)]
    fn native_roundtrip(&self) -> Result<Self> {
        let bytes = bincode::serde::encode_to_vec(
            (&self.exact, &self.checked),
            bincode::config::standard(),
        )
        .map_err(native)?;
        let ((exact, checked), used) =
            bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
                .map_err(native)?;
        if used != bytes.len() {
            return Err(Error::Invalid(
                "restricted prefix native codec trailing bytes".into(),
            ));
        }
        Ok(Self {
            leaf: self.leaf.clone(),
            scope: self.scope.native_roundtrip()?,
            exact,
            checked,
            inputs: self.inputs.clone(),
            coordinates: self.coordinates.clone(),
        })
    }
}

/// Caller-owned native scratch retaining the complete cover and prefix authority.
pub struct NumericAffineRestriction<T> {
    program: ExpressionEvaluator<T>,
    leaf: AffinePrefixLeaf,
}
impl<T: Number> NumericAffineRestriction<T> {
    pub fn leaf(&self) -> &AffinePrefixLeaf {
        &self.leaf
    }
    pub fn evaluate(&mut self, unit: &[T]) -> Result<Vec<T>> {
        let n = self.leaf.cover().parent().len();
        if unit.len() != n
            || unit
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Invalid(
                "restricted prefix inputs must be valid real values strictly inside the unit cube"
                    .into(),
            ));
        }
        let visible = n + 1 + self.leaf.cover().dependent_symbols().len();
        let mut values = vec![unit[0].zero(); visible + 2 * n + 1];
        attempt(|| self.program.try_evaluate(unit, &mut values))
            .map_err(Error::Native)?
            .map_err(native)?;
        let positive = &values[visible..visible + n + 1];
        let mapped = &values[visible + n + 1..];
        if values.iter().any(|x| !x.valid())
            || values[n] <= values[n].zero()
            || positive.iter().any(|x| *x <= x.zero())
            || mapped.iter().any(|x| *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Native("restricted prefix has invalid coordinates, mapped inputs or nonpositive Jacobian/diagonal/measure".into()));
        }
        values.truncate(visible);
        Ok(values)
    }
}

#[cfg(test)]
mod tests;
