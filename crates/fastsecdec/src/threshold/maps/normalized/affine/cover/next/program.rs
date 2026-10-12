//! Native composition of an issued restricted next boundary, without a new
//! integration direction. The retained section owns its complete source proof.
use super::{Arc, Atom, Budget, Error, RestrictedNextSection, Result, Symbol, native};
use crate::{
    kernel::algebraic::{Number, Scope, attempt},
    threshold::maps::normalized::affine::cover::program::CompiledAffineRestriction,
};
use std::collections::BTreeSet;
use symbolica::{
    atom::AtomCore,
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluatorComposer, ExpressionEvaluator, OptimizationSettings, Slot},
};

/// Outputs the original CAD prefix, its positive partial measure, the selected
/// next source coordinate, normalized root, then prior owned dependent values.
/// The next coordinate is a boundary, so it contributes no additional Jacobian.
pub struct CompiledRestrictedNextSection {
    owner: Arc<RestrictedNextSection>,
    scope: Scope,
    exact: ExpressionEvaluator<Complex<Rational>>,
    checked: ExpressionEvaluator<Complex<Rational>>,
    inputs: Vec<Symbol>,
    visible: usize,
    checked_len: usize,
}
impl CompiledRestrictedNextSection {
    pub fn prepare(
        owner: Arc<RestrictedNextSection>,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        let leaf = owner.parent().leaf();
        let cover = leaf.cover();
        let n = cover.parent().len();
        let d = cover.dependent_symbols().len();
        let visible = n
            .checked_add(3)
            .and_then(|v| v.checked_add(d))
            .ok_or_else(|| Error::ResourceIncomplete("next-section output count".into()))?;
        let checked_len = n
            .checked_mul(3)
            .and_then(|v| v.checked_add(3))
            .and_then(|v| v.checked_add(visible))
            .ok_or_else(|| Error::ResourceIncomplete("next-section guard count".into()))?;
        if n == 0 || owner.power_images().len() != n || cover.slots().len() != n + d {
            return Err(Error::Invalid("next-section input/slot dimensions".into()));
        }
        let intermediate = settings.clone().cpe_iterations(Some(0));
        let parent =
            CompiledAffineRestriction::prepare(leaf.clone(), intermediate.clone(), budget)?;
        let inputs = parent.inputs().to_vec();
        if cover.slots()[..n]
            .iter()
            .map(|(s, _)| *s)
            .ne(inputs.iter().copied())
            || cover.slots()[n..]
                .iter()
                .map(|(s, _)| *s)
                .ne(parent.dependent_symbols().iter().copied())
        {
            return Err(Error::Invalid(
                "next-section named coefficient slot order".into(),
            ));
        }
        let mut scope = parent.scope().clone();
        let mut used = cover.parent().occupied();
        used.extend(cover.slots().iter().map(|(s, _)| *s));
        used.insert(cover.root_symbol());
        used.insert(owner.proposal().root_symbol);
        for (tag, program) in scope.owners() {
            used.insert(tag);
            used.insert(program.transport_symbol());
        }
        let root = owner.root_program(intermediate.clone())?;
        if !used.insert(root.transport_symbol()) {
            return Err(Error::Invalid(
                "next-section root transport collision".into(),
            ));
        }
        let tag = fresh_tag(&mut used)?;
        let call = root.call(tag, owner.coefficients())?;
        scope.insert(tag, root)?;
        let build = |outputs: &[Atom], roles: &[Atom]| {
            Atom::evaluator_multiple(outputs, roles)
                .optimization_settings(intermediate.clone())
                .build()
                .map_err(native)
        };
        let params = inputs.iter().copied().map(Atom::var).collect::<Vec<_>>();
        let slots = (0..n).map(Slot::Param).collect::<Vec<_>>();
        let mut power_atoms = owner.power_images().to_vec();
        power_atoms.push(owner.power_measure().clone());
        let mut compose = EvaluatorComposer::new(n);
        let power = compose
            .append(&build(&power_atoms, &params)?, &slots)
            .map_err(native)?;
        let prior = compose
            .append(parent.checked(), &power[..n])
            .map_err(native)?;
        let roles = cover
            .slots()
            .iter()
            .map(|(s, _)| Atom::var(*s))
            .collect::<Vec<_>>();
        // Proposal/Q already use the NEW units. Only the previous dependent
        // roots are evaluated at the powered inputs; powering these slots again
        // would apply the ramification twice.
        let mut coefficient_slots = slots.clone();
        coefficient_slots.extend_from_slice(&prior[n + 1..n + 1 + d]);
        let section = compose
            .append(
                &build(
                    &[owner.centre_expression(), owner.scale_expression(), call],
                    &roles,
                )?,
                &coefficient_slots,
            )
            .map_err(native)?;
        let operation = fresh_operation_roles(&mut used)?;
        let a = operation.iter().copied().map(Atom::var).collect::<Vec<_>>();
        let combined = compose
            .append(
                &build(&[&a[0] * &a[1], &a[2] + &a[3] * &a[4]], &a)?,
                &[prior[n], power[n], section[0], section[1], section[2]],
            )
            .map_err(native)?;
        let mut outputs = prior[..n].to_vec();
        outputs.extend([combined[0], combined[1], section[2]]);
        outputs.extend_from_slice(&prior[n + 1..n + 1 + d]);
        // Preserve all original runtime guards, then the positive power measure
        // and scale, and the strict-interior powered inputs.
        outputs.extend_from_slice(&prior[n + 1 + d..]);
        outputs.extend([power[n], section[1]]);
        outputs.extend_from_slice(&power[..n]);
        if outputs.len() != checked_len {
            return Err(Error::Invalid("next-section output layout".into()));
        }
        let checked = compose.finish(&outputs, settings.clone()).map_err(native)?;
        let mut project = EvaluatorComposer::new(n);
        let all = project.append(&checked, &slots).map_err(native)?;
        let exact = project.finish(&all[..visible], settings).map_err(native)?;
        budget.check()?;
        Ok(Self {
            owner,
            scope,
            exact,
            checked,
            inputs,
            visible,
            checked_len,
        })
    }
    pub fn owner(&self) -> &Arc<RestrictedNextSection> {
        &self.owner
    }
    pub fn inputs(&self) -> &[Symbol] {
        &self.inputs
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    pub fn map<T: Number>(&self, bits: u32) -> NumericRestrictedNextSection<T> {
        let one = T::one_at(bits);
        let program = self.scope.enter(bits, || {
            self.checked
                .clone()
                .map_coeff_with_prec(&|q| one.from_rational(&q.re), bits)
        });
        NumericRestrictedNextSection {
            program,
            owner: self.owner.clone(),
            visible: self.visible,
            checked_len: self.checked_len,
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
                "next-section native codec trailing bytes".into(),
            ));
        }
        Ok(Self {
            owner: self.owner.clone(),
            scope: self.scope.native_roundtrip()?,
            exact,
            checked,
            inputs: self.inputs.clone(),
            visible: self.visible,
            checked_len: self.checked_len,
        })
    }
}

fn fresh_tag(used: &mut BTreeSet<Symbol>) -> Result<Symbol> {
    for i in 0..1024 {
        let s = Symbol::parse(format!("root_{i}"), "fastsecdec_restricted_next_program")
            .map_err(native)?;
        if s.is_exportable() && used.insert(s) {
            return Ok(s);
        }
    }
    Err(Error::ResourceIncomplete(
        "fresh next-section callback tag allowance".into(),
    ))
}
fn fresh_operation_roles(used: &mut BTreeSet<Symbol>) -> Result<[Symbol; 5]> {
    let mut roles = Vec::new();
    for i in 0..1024 {
        let s = Symbol::parse(
            format!("operation_{i}"),
            "fastsecdec_restricted_next_program",
        )
        .map_err(native)?;
        if s.is_exportable() && used.insert(s) {
            roles.push(s);
            if roles.len() == 5 {
                return Ok(roles.try_into().unwrap());
            }
        }
    }
    Err(Error::ResourceIncomplete(
        "fresh next-section operation roles".into(),
    ))
}

/// Caller-owned numeric scratch. A next boundary neither adds a unit input nor
/// extends the admitted integration cell.
pub struct NumericRestrictedNextSection<T> {
    program: ExpressionEvaluator<T>,
    owner: Arc<RestrictedNextSection>,
    visible: usize,
    checked_len: usize,
}
impl<T: Number> NumericRestrictedNextSection<T> {
    pub fn owner(&self) -> &Arc<RestrictedNextSection> {
        &self.owner
    }
    pub fn evaluate(&mut self, unit: &[T]) -> Result<Vec<T>> {
        let n = self.owner.parent().leaf().cover().parent().len();
        if unit.len() != n
            || unit
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Invalid(
                "next-section inputs must be valid real values strictly inside unit cube".into(),
            ));
        }
        let mut values = vec![unit[0].zero(); self.checked_len];
        attempt(|| self.program.try_evaluate(unit, &mut values))
            .map_err(Error::Native)?
            .map_err(native)?;
        let g = self.visible;
        let old_positive = &values[g..g + n + 1];
        let old_mapped = &values[g + n + 1..g + 2 * n + 1];
        let power_and_scale = &values[g + 2 * n + 1..g + 2 * n + 3];
        let powered = &values[g + 2 * n + 3..];
        if values.iter().any(|x| !x.valid())
            || values[n] <= values[n].zero()
            || old_positive
                .iter()
                .chain(power_and_scale)
                .any(|x| *x <= x.zero())
            || old_mapped
                .iter()
                .chain(powered)
                .any(|x| *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Native(
                "next-section invalid value, nonpositive diagonal/measure/scale or mapped boundary"
                    .into(),
            ));
        }
        values.truncate(self.visible);
        Ok(values)
    }
}

#[cfg(test)]
mod tests;
