//! Native instruction composition for an admitted complete open prefix.
//! Saved instruction roundtrips never issue or replace its geometric owner.
use super::{Budget, Error, NormalizedPrefix, NormalizedSection, PrefixStep, Result, native};
use crate::{
    generation::identity::{CanonicalAtom, CanonicalSymbol},
    kernel::algebraic::{Number, RootProgram, Scope, attempt},
    threshold::{gcad::SourceProvenance, maps::PositiveProjectiveCellMap},
};
use serde::Serialize;
use std::{collections::BTreeMap, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{EvaluatorComposer, ExpressionEvaluator, OptimizationSettings, Slot},
    symbol,
};

fn digest_part<T: Serialize>(digest: &mut blake3::Hasher, value: &T) -> Result<()> {
    serde_json::to_writer(digest, value).map_err(native)
}
pub(in crate::threshold::maps) fn source_digest(
    source: &PositiveProjectiveCellMap,
) -> Result<String> {
    let owner = source.decomposition();
    let identity =
        match owner.provenance() {
            SourceProvenance::Exact => {
                crate::generation::source_identity(owner.preparation().input(), &[], &[])
                    .map_err(native)?
            }
            SourceProvenance::PreparametricGraph(input) => input.source_identity().to_owned(),
            SourceProvenance::PreparametricFamily(input) => input.source_identity().to_owned(),
            SourceProvenance::ParametricRepresented(_) => return Err(Error::Unsupported(
                "normalized map represented-source identity is not issued by this projective owner"
                    .into(),
            )),
        };
    let mut digest = blake3::Hasher::new();
    digest_part(
        &mut digest,
        &(
            "fastsecdec-normalized-map-source-v1",
            identity,
            owner.preparation().anchor_index(),
            &owner.original_result().problem,
            &owner.native_result().order,
            source.cell_index(),
            source.native_axes(),
            owner
                .preparation()
                .coordinates()
                .iter()
                .copied()
                .map(CanonicalSymbol)
                .collect::<Vec<_>>(),
            source
                .unit_coordinates()
                .iter()
                .copied()
                .map(CanonicalSymbol)
                .collect::<Vec<_>>(),
        ),
    )?;
    Ok(digest.finalize().to_hex().to_string())
}
fn section_digest_part(digest: &mut blake3::Hasher, section: &NormalizedSection) -> Result<()> {
    let original = section.original_polynomial().to_expression();
    digest_part(
        digest,
        &(
            "normalized-upper-section-v1",
            section.prefix.len(),
            &section.bound,
            CanonicalAtom(&original),
            CanonicalAtom(section.total_pullback()),
            CanonicalAtom(section.removed_presentation_monomial()),
            CanonicalAtom(&section.equation),
            section
                .coefficients
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            CanonicalAtom(&section.scale),
            section.lower.to_string(),
            CanonicalSymbol(section.root_symbol),
            (
                section.bracket.lower.to_string(),
                section.bracket.upper.to_string(),
                section.bracket.derivative_margin.to_string(),
                section.orientation,
            ),
        ),
    )?;
    digest_part(
        digest,
        &(
            section
                .chart
                .parameters
                .iter()
                .copied()
                .map(CanonicalSymbol)
                .collect::<Vec<_>>(),
            section
                .chart
                .bounds
                .iter()
                .map(|b| [b[0].to_string(), b[1].to_string()])
                .collect::<Vec<_>>(),
            section
                .chart
                .source_prefix
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            section
                .chart
                .parameter_images
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
            section
                .chart
                .witness
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            section.exponent_determinant().to_string(),
            section.derivative_certificate(),
            section.endpoint_certificates(),
            section.range_certificates(),
            section.domain(),
        ),
    )
}
impl NormalizedSection {
    pub fn root_program(&self, settings: OptimizationSettings) -> Result<Arc<RootProgram>> {
        self.root_program_in(&source_digest(self.prefix.source())?, settings)
    }
    fn root_program_in(
        &self,
        source: &str,
        settings: OptimizationSettings,
    ) -> Result<Arc<RootProgram>> {
        let mut digest = blake3::Hasher::new();
        digest_part(&mut digest, &("normalized-selected-chain-v1", source))?;
        for step in self.prefix.steps() {
            digest_part(
                &mut digest,
                &(CanonicalAtom(step.image()), CanonicalAtom(step.diagonal())),
            )?;
            match step {
                PrefixStep::ConstantInterval {
                    lower,
                    upper,
                    power,
                    ..
                } => digest_part(
                    &mut digest,
                    &(
                        "constant-interval",
                        power,
                        lower.original_bound(),
                        upper.original_bound(),
                        CanonicalAtom(&lower.expression()),
                        CanonicalAtom(&upper.expression()),
                    ),
                )?,
                PrefixStep::Power {
                    lower,
                    upper,
                    power,
                    ..
                } => digest_part(
                    &mut digest,
                    &("power", lower.to_string(), upper.to_string(), power),
                )?,
                PrefixStep::Section { section, .. } => section_digest_part(&mut digest, section)?,
                PrefixStep::Open { axis, .. } => digest_part(
                    &mut digest,
                    &(
                        "open",
                        axis.axis_index(),
                        &axis.source().native_axes()[axis.axis_index()],
                    ),
                )?,
            }
        }
        section_digest_part(&mut digest, self)?;
        let contract = serde_json::to_vec(&(
            "fastsecdec-normalized-section-v1",
            digest.finalize().to_hex().to_string(),
        ))
        .map_err(native)?;
        RootProgram::prepare_shifted_from_certificate(
            self.coefficients.len() - 1,
            self.bracket.lower.clone(),
            self.bracket.upper.clone(),
            contract,
            settings,
        )
        .map_err(Into::into)
    }
}

/// Complete open coordinate execution retaining its actual source-bound prefix.
/// Output coordinates follow `coordinates()`, followed by the positive
/// integration Jacobian. Endpoint/infinity and density authority are absent.
pub struct CompiledOpenMap {
    owner: Arc<NormalizedPrefix>,
    scope: Scope,
    exact: ExpressionEvaluator<Complex<Rational>>,
    inputs: Vec<Symbol>,
    coordinates: Vec<Symbol>,
}
impl CompiledOpenMap {
    pub fn prepare(
        owner: Arc<NormalizedPrefix>,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        if !owner.complete_open_map() {
            return Err(Error::Unsupported(
                "complete admitted open prefix required for execution".into(),
            ));
        }
        let source = owner.source();
        let inputs = source.unit_coordinates().to_vec();
        let mut compose = EvaluatorComposer::new(inputs.len());
        let mut scope = Scope::default();
        let mut atoms = Vec::new();
        let mut slots = Vec::new();
        let mut coordinates = BTreeMap::new();
        let mut diagonals = Vec::new();
        let intermediate = settings.clone().cpe_iterations(Some(0));
        let build = |outputs: &[Atom], inputs: &[Atom]| {
            Atom::evaluator_multiple(outputs, inputs)
                .optimization_settings(intermediate.clone())
                .build()
                .map_err(native)
        };
        // Hash the physical source once; branch records stream only their small
        // actual cell and transformation/certificate inventory, never the proof.
        let identity = source_digest(source)?;
        for (axis, step) in owner.steps().into_iter().enumerate() {
            budget.check()?;
            if let PrefixStep::ConstantInterval { lower, upper, .. } = step {
                if axis != 0 {
                    return Err(Error::Invalid(
                        "constant endpoint outside original first axis".into(),
                    ));
                }
                for (endpoint_index, endpoint) in [lower, upper].into_iter().enumerate() {
                    if let Some(section) = endpoint.section() {
                        if !Arc::ptr_eq(section.source(), source) || section.axis() != axis {
                            return Err(Error::Invalid(
                                "positive constant source/axis association".into(),
                            ));
                        }
                        let program =
                            RootProgram::prepare_positive_constant(section, intermediate.clone())?;
                        let mut chosen = None;
                        for trial in 0..budget.limits.max_refinements {
                            budget.check()?;
                            let tag = symbol!(&format!(
                                "fastsecdec::positive_constant_call_{endpoint_index}_{trial}"
                            ));
                            if !source.decomposition().occupied_symbols().contains(&tag)
                                && !inputs.contains(&tag)
                                && owner.slots().iter().all(|(s, _)| s != &tag)
                                && tag.is_exportable()
                            {
                                chosen = Some(tag);
                                break;
                            }
                        }
                        let tag = chosen.ok_or_else(|| {
                            Error::ResourceIncomplete("constant callback tag freshness".into())
                        })?;
                        if source
                            .decomposition()
                            .occupied_symbols()
                            .contains(&program.transport_symbol())
                            || inputs.contains(&program.transport_symbol())
                            || owner
                                .slots()
                                .iter()
                                .any(|(s, _)| *s == program.transport_symbol())
                        {
                            return Err(Error::Invalid("constant transport role collision".into()));
                        }
                        let call = program.call(tag, section.coefficients())?;
                        scope.insert(tag, program)?;
                        let slot = compose.append(&build(&[call], &[])?, &[]).map_err(native)?[0];
                        atoms.push(Atom::var(
                            endpoint
                                .symbol()
                                .ok_or_else(|| Error::Invalid("constant root role".into()))?,
                        ));
                        slots.push(slot);
                    }
                }
            }
            if let PrefixStep::Section { section, .. } = step {
                if !Arc::ptr_eq(section.prefix.source(), source) || section.prefix.len() != axis {
                    return Err(Error::Invalid(
                        "normalized section source/prefix association".into(),
                    ));
                }
                let parameters = section
                    .chart
                    .parameters
                    .iter()
                    .copied()
                    .map(Atom::var)
                    .collect::<Vec<_>>();
                let chart = build(&section.chart.parameter_images, &atoms)?;
                let parameters_slots = compose.append(&chart, &slots).map_err(native)?;
                let program = section.root_program_in(&identity, intermediate.clone())?;
                let tag = symbol!(&format!("fastsecdec::normalized_map::root_slot_{axis}"));
                let call = program.call(tag, &section.coefficients)?;
                scope.insert(tag, program)?;
                let root = compose
                    .append(&build(&[call], &parameters)?, &parameters_slots)
                    .map_err(native)?[0];
                atoms.push(Atom::var(section.root_symbol));
                slots.push(root);
            }
            let unit = source.axes()[axis]
                .unit_index()
                .ok_or_else(|| Error::Invalid("normalized map integration role".into()))?;
            atoms.push(Atom::var(inputs[unit]));
            slots.push(Slot::Param(unit));
            let body = build(&[step.image().clone(), step.diagonal().clone()], &atoms)?;
            let outputs = compose.append(&body, &slots).map_err(native)?;
            coordinates.insert(source.axes()[axis].symbol(), outputs[0]);
            diagonals.push(outputs[1]);
        }
        let coordinates_order = source.decomposition().preparation().coordinates().to_vec();
        let mut outputs = coordinates_order
            .iter()
            .map(|s| {
                coordinates.get(s).copied().ok_or_else(|| {
                    Error::Invalid("complete normalized source coordinate inventory".into())
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let factors = inputs.iter().copied().map(Atom::var).collect::<Vec<_>>();
        let product: Atom = factors.iter().cloned().product();
        let jacobian = compose
            .append(&build(&[product], &factors)?, &diagonals)
            .map_err(native)?[0];
        outputs.push(jacobian);
        // Individual positives catch errors hidden by a product of two negatives.
        outputs.extend(diagonals);
        let exact = compose.finish(&outputs, settings).map_err(native)?;
        budget.check()?;
        Ok(Self {
            owner,
            scope,
            exact,
            inputs,
            coordinates: coordinates_order,
        })
    }
    pub fn owner(&self) -> &Arc<NormalizedPrefix> {
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
    /// Native outputs: source coordinates, positive integration Jacobian, then
    /// individual CAD-order diagonals for checked execution. This IR is not a
    /// transferable geometry certificate; composition callers retain this owner.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    pub fn map<T: Number>(&self, bits: u32) -> NumericOpenMap<T> {
        let one = T::one_at(bits);
        let program = self.scope.enter(bits, || {
            self.exact
                .clone()
                .map_coeff_with_prec(&|q| one.from_rational(&q.re), bits)
        });
        NumericOpenMap {
            program,
            owner: self.owner.clone(),
            dimensions: self.coordinates.len(),
        }
    }
    #[cfg(test)]
    pub(super) fn native_roundtrip(&self) -> Result<Self> {
        let bytes = bincode::serde::encode_to_vec(&self.exact, bincode::config::standard())
            .map_err(native)?;
        let (exact, used) = bincode::serde::decode_from_slice(&bytes, bincode::config::standard())
            .map_err(native)?;
        if used != bytes.len() {
            return Err(Error::Invalid(
                "normalized native codec trailing bytes".into(),
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

/// Caller-owned native scratch and checked open-unit execution. Validity and
/// uncertainty use the native Number implementation, never a reconstructed value.
pub struct NumericOpenMap<T> {
    program: ExpressionEvaluator<T>,
    owner: Arc<NormalizedPrefix>,
    dimensions: usize,
}
impl<T: Number> NumericOpenMap<T> {
    pub fn owner(&self) -> &Arc<NormalizedPrefix> {
        &self.owner
    }
    pub fn evaluate(&mut self, unit: &[T]) -> Result<Vec<T>> {
        if unit.len() != self.dimensions
            || unit
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
        {
            return Err(Error::Invalid(
                "normalized map inputs must be valid real values strictly inside the unit cube"
                    .into(),
            ));
        }
        let one = unit.first().cloned().unwrap_or_else(|| T::one_at(53));
        let mut output = vec![one.zero(); 2 * self.dimensions + 1];
        attempt(|| self.program.try_evaluate(unit, &mut output))
            .map_err(Error::Native)?
            .map_err(native)?;
        if output.iter().any(|x| !x.valid())
            || output[self.dimensions..].iter().any(|x| *x <= x.zero())
        {
            return Err(Error::Native(
                "normalized open map has invalid coordinates or nonpositive Jacobian/diagonal"
                    .into(),
            ));
        }
        output.truncate(self.dimensions + 1);
        Ok(output)
    }
}

#[cfg(test)]
mod tests;
