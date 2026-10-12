//! Native execution of an admitted affine prefix, never a complete-cell issuer.
use super::{
    AffineNormalizedPrefix, AffineNormalizedSection, AffinePrefixStep, Budget, ConstantEndpoint,
    Error, Result, native,
};
use crate::{
    generation::identity::{CanonicalAtom, CanonicalSymbol},
    kernel::algebraic::{Number, RootProgram, Scope, attempt},
};
use serde::Serialize;
use std::{collections::BTreeSet, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluatorComposer, ExpressionEvaluator, OptimizationSettings, Slot},
};

fn digest_part<T: Serialize>(digest: &mut blake3::Hasher, value: &T) -> Result<()> {
    serde_json::to_writer(digest, value).map_err(native)
}
fn source_digest(owner: &AffineNormalizedPrefix) -> Result<blake3::Hasher> {
    let source = owner.source();
    let raw = source.decomposition().native_result();
    let mut digest = blake3::Hasher::new();
    digest_part(
        &mut digest,
        &(
            "fastsecdec-affine-prefix-source-v1",
            source
                .decomposition()
                .request()
                .source_identity()
                .map_err(native)?,
            &raw.problem,
            &raw.order,
            source.cell_index(),
            source.native_axes(),
            source
                .axes()
                .iter()
                .map(|a| CanonicalSymbol(a.symbol()))
                .collect::<Vec<_>>(),
            source
                .unit_coordinates()
                .iter()
                .copied()
                .map(CanonicalSymbol)
                .collect::<Vec<_>>(),
        ),
    )?;
    Ok(digest)
}
fn constant_digest(digest: &mut blake3::Hasher, endpoint: &ConstantEndpoint) -> Result<()> {
    digest_part(
        digest,
        &(
            endpoint.original_bound(),
            CanonicalAtom(&endpoint.expression()),
            endpoint.enclosure().map(|q| q.to_string()),
        ),
    )?;
    if let Some(section) = endpoint.section() {
        digest_part(
            digest,
            &(
                CanonicalAtom(section.equation()),
                section
                    .coefficients()
                    .iter()
                    .map(CanonicalAtom)
                    .collect::<Vec<_>>(),
                section.bracket().derivative_margin.to_string(),
                section.orientation(),
                section.derivative_certificate(),
                section.endpoint_certificates(),
                section.cylinder(),
            ),
        )?;
    }
    Ok(())
}
fn section_digest(digest: &mut blake3::Hasher, section: &AffineNormalizedSection) -> Result<()> {
    digest_part(
        digest,
        &(
            "affine-normalized-upper-v1",
            section.prefix().len(),
            section.original_bound(),
            CanonicalAtom(&section.original_polynomial().to_expression()),
            CanonicalAtom(section.total_pullback()),
            CanonicalAtom(section.removed_presentation_monomial()),
            CanonicalAtom(section.equation()),
            section
                .coefficients()
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            CanonicalAtom(section.scale()),
            section.lower().to_string(),
            CanonicalSymbol(section.root_symbol()),
            (
                section.bracket().lower.to_string(),
                section.bracket().upper.to_string(),
                section.bracket().derivative_margin.to_string(),
                section.orientation(),
            ),
        ),
    )?;
    let chart = section.chart();
    digest_part(
        digest,
        &(
            chart
                .parameters
                .iter()
                .copied()
                .map(CanonicalSymbol)
                .collect::<Vec<_>>(),
            chart
                .bounds
                .iter()
                .map(|b| [b[0].to_string(), b[1].to_string()])
                .collect::<Vec<_>>(),
            chart
                .source_prefix
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            chart
                .parameter_images
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            chart
                .witness
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            section.exponent_determinant().to_string(),
            section.derivative_certificate(),
            section.endpoint_certificates(),
            section.range_certificates(),
            section.domain(),
            (
                section.anchor().polynomial,
                section.anchor().index,
                section.anchor().interval.0.to_string(),
                section.anchor().interval.1.to_string(),
            ),
        ),
    )
}
fn step_digest(digest: &mut blake3::Hasher, step: &AffinePrefixStep) -> Result<()> {
    digest_part(
        digest,
        &(CanonicalAtom(step.image()), CanonicalAtom(step.diagonal())),
    )?;
    match step {
        AffinePrefixStep::ConstantInterval {
            lower,
            upper,
            power,
            ..
        } => {
            digest_part(digest, &("constant-interval", power))?;
            constant_digest(digest, lower)?;
            constant_digest(digest, upper)
        }
        AffinePrefixStep::NormalizedUpper { section, .. } => section_digest(digest, section),
    }
}
impl AffineNormalizedSection {
    /// Detached native arithmetic retains a digest of the actual selected branch.
    /// Its bytes alone do not restore this geometric section owner.
    pub fn root_program(&self, settings: OptimizationSettings) -> Result<Arc<RootProgram>> {
        let mut digest = source_digest(self.prefix())?;
        for step in self.prefix().steps() {
            step_digest(&mut digest, step)?;
        }
        self.root_program_in(&digest, settings)
    }
    fn root_program_in(
        &self,
        prefix: &blake3::Hasher,
        settings: OptimizationSettings,
    ) -> Result<Arc<RootProgram>> {
        let mut digest = prefix.clone();
        section_digest(&mut digest, self)?;
        let contract = serde_json::to_vec(&(
            "fastsecdec-affine-normalized-section-v1",
            digest.finalize().to_hex().to_string(),
        ))
        .map_err(native)?;
        RootProgram::prepare_shifted_from_certificate(
            self.coefficients().len() - 1,
            self.bracket().lower.clone(),
            self.bracket().upper.clone(),
            contract,
            settings,
        )
        .map_err(Into::into)
    }
}
fn fresh_tag(used: &mut BTreeSet<Symbol>, index: usize) -> Result<Symbol> {
    for attempt in 0..1024 {
        let tag = Symbol::parse(
            format!("root_{index}_{attempt}"),
            "fastsecdec_affine_program",
        )
        .map_err(native)?;
        if tag.is_exportable() && used.insert(tag) {
            return Ok(tag);
        }
    }
    Err(Error::ResourceIncomplete(
        "fresh affine root callback tag allowance".into(),
    ))
}

/// Only admitted CAD-order coordinates, followed by their partial Jacobian.
/// Constant endpoint values are owned callback outputs, never caller inputs.
pub struct CompiledAffinePrefix {
    owner: Arc<AffineNormalizedPrefix>,
    scope: Scope,
    exact: ExpressionEvaluator<Complex<Rational>>,
    inputs: Vec<Symbol>,
    coordinates: Vec<Symbol>,
}
impl CompiledAffinePrefix {
    pub fn prepare(
        owner: Arc<AffineNormalizedPrefix>,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        if owner.is_empty() {
            return Err(Error::Unsupported(
                "nonempty admitted affine prefix required for execution".into(),
            ));
        }
        let source = owner.source();
        let inputs = source.unit_coordinates()[..owner.len()].to_vec();
        let coordinates = source.axes()[..owner.len()]
            .iter()
            .map(|a| a.symbol())
            .collect();
        let mut compose = EvaluatorComposer::new(inputs.len());
        let mut scope = Scope::default();
        let (mut atoms, mut slots) = (Vec::new(), Vec::new());
        let (mut outputs, mut diagonals) = (Vec::new(), Vec::new());
        let mut used = owner.occupied();
        let mut tags = 0;
        let intermediate = settings.clone().cpe_iterations(Some(0));
        let build = |outputs: &[Atom], inputs: &[Atom]| {
            Atom::evaluator_multiple(outputs, inputs)
                .optimization_settings(intermediate.clone())
                .build()
                .map_err(native)
        };
        // The source is streamed once, then only small accepted steps are added.
        let mut digest = source_digest(&owner)?;
        for (axis, step) in owner.steps().into_iter().enumerate() {
            budget.check()?;
            match step {
                AffinePrefixStep::ConstantInterval { lower, upper, .. } => {
                    for endpoint in [lower, upper] {
                        if let Some(section) = endpoint.section() {
                            if !Arc::ptr_eq(section.source(), source) || section.axis() != axis {
                                return Err(Error::Invalid(
                                    "affine constant source/axis association".into(),
                                ));
                            }
                            let program =
                                RootProgram::prepare_affine_shifted(section, intermediate.clone())?;
                            if used.contains(&program.transport_symbol()) {
                                return Err(Error::Invalid(
                                    "affine root transport role collision".into(),
                                ));
                            }
                            let tag = fresh_tag(&mut used, tags)?;
                            tags += 1;
                            let call = program.call(tag, section.coefficients())?;
                            scope.insert(tag, program)?;
                            let slot =
                                compose.append(&build(&[call], &[])?, &[]).map_err(native)?[0];
                            atoms.push(Atom::var(endpoint.symbol().ok_or_else(|| {
                                Error::Invalid("owned constant root slot".into())
                            })?));
                            slots.push(slot);
                        }
                    }
                }
                AffinePrefixStep::NormalizedUpper { section, .. } => {
                    if !Arc::ptr_eq(section.prefix().source(), source)
                        || section.prefix().len() != axis
                    {
                        return Err(Error::Invalid(
                            "affine normalized source/prefix association".into(),
                        ));
                    }
                    let parameters = section
                        .chart()
                        .parameters
                        .iter()
                        .copied()
                        .map(Atom::var)
                        .collect::<Vec<_>>();
                    let chart = build(&section.chart().parameter_images, &atoms)?;
                    let parameter_slots = compose.append(&chart, &slots).map_err(native)?;
                    let program = section.root_program_in(&digest, intermediate.clone())?;
                    if used.contains(&program.transport_symbol()) {
                        return Err(Error::Invalid(
                            "affine root transport role collision".into(),
                        ));
                    }
                    let tag = fresh_tag(&mut used, tags)?;
                    tags += 1;
                    let call = program.call(tag, section.coefficients())?;
                    scope.insert(tag, program)?;
                    let root = compose
                        .append(&build(&[call], &parameters)?, &parameter_slots)
                        .map_err(native)?[0];
                    atoms.push(Atom::var(section.root_symbol()));
                    slots.push(root);
                }
            }
            atoms.push(Atom::var(inputs[axis]));
            slots.push(Slot::Param(axis));
            let body = build(&[step.image().clone(), step.diagonal().clone()], &atoms)?;
            let values = compose.append(&body, &slots).map_err(native)?;
            outputs.push(values[0]);
            diagonals.push(values[1]);
            step_digest(&mut digest, step)?;
        }
        let factors = inputs.iter().copied().map(Atom::var).collect::<Vec<_>>();
        let product: Atom = factors.iter().cloned().product();
        let jacobian = compose
            .append(&build(&[product], &factors)?, &diagonals)
            .map_err(native)?[0];
        outputs.push(jacobian);
        outputs.extend(diagonals);
        let exact = compose.finish(&outputs, settings).map_err(native)?;
        budget.check()?;
        Ok(Self {
            owner,
            scope,
            exact,
            inputs,
            coordinates,
        })
    }
    pub fn owner(&self) -> &Arc<AffineNormalizedPrefix> {
        &self.owner
    }
    pub fn inputs(&self) -> &[Symbol] {
        &self.inputs
    }
    pub fn coordinates(&self) -> &[Symbol] {
        &self.coordinates
    }
    pub fn scope(&self) -> &Scope {
        &self.scope
    }
    /// Outputs: CAD prefix coordinates, partial J, then individual diagonals.
    /// This native IR is execution data; composition must retain this owner.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    pub fn map<T: Number>(&self, bits: u32) -> NumericAffinePrefix<T> {
        let one = T::one_at(bits);
        let program = self.scope.enter(bits, || {
            self.exact
                .clone()
                .map_coeff_with_prec(&|q| one.from_rational(&q.re), bits)
        });
        NumericAffinePrefix {
            program,
            owner: self.owner.clone(),
        }
    }
    #[cfg(test)]
    fn native_roundtrip(&self) -> Result<Self> {
        let bytes = bincode::serde::encode_to_vec(&self.exact, bincode::config::standard())
            .map_err(native)?;
        let (exact, used) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(native)?;
        if used != bytes.len() {
            return Err(Error::Invalid(
                "affine prefix native codec trailing bytes".into(),
            ));
        }
        Ok(Self {
            owner: self.owner.clone(),
            scope: self.scope.native_roundtrip()?,
            exact,
            inputs: self.inputs.clone(),
            coordinates: self.coordinates.clone(),
        })
    }
}

/// Caller-owned native scratch, retaining all exact source/section owners.
pub struct NumericAffinePrefix<T> {
    program: ExpressionEvaluator<T>,
    owner: Arc<AffineNormalizedPrefix>,
}
impl<T: Number> NumericAffinePrefix<T> {
    pub fn owner(&self) -> &Arc<AffineNormalizedPrefix> {
        &self.owner
    }
    pub fn evaluate(&mut self, unit: &[T]) -> Result<Vec<T>> {
        let n = self.owner.len();
        if unit.len() != n
            || unit
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Invalid(
                "affine prefix inputs must be valid real values strictly inside the unit cube"
                    .into(),
            ));
        }
        let mut output = vec![unit[0].zero(); 2 * n + 1];
        attempt(|| self.program.try_evaluate(unit, &mut output))
            .map_err(Error::Native)?
            .map_err(native)?;
        if output.iter().any(|x| !x.valid()) || output[n..].iter().any(|x| *x <= x.zero()) {
            return Err(Error::Native(
                "affine prefix has invalid coordinates or nonpositive Jacobian/diagonal".into(),
            ));
        }
        output.truncate(n + 1);
        Ok(output)
    }
}

#[cfg(test)]
mod tests;
