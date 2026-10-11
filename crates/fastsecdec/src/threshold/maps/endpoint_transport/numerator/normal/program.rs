//! Native composition keeps caller source functions outside geometric stages.
use super::*;
use crate::kernel::algebraic::{Number, attempt};
use std::collections::{BTreeMap, BTreeSet};
use symbolica::{
    domains::{float::Complex, rational::Rational},
    evaluate::{
        EvaluationDomain, EvaluatorComposer, ExpressionEvaluator, FunctionMap,
        FunctionRegistrationOptions, InliningPolicy, Slot,
    },
    id::{Pattern, Replacement},
};

pub struct CompiledNormalNumeratorJets {
    owner: Arc<NormalNumeratorJets>,
    coefficient_inputs: Vec<Symbol>,
    exact: ExpressionEvaluator<Complex<Rational>>,
    coefficient_count: usize,
    geometry_count: usize,
    scalar_count: usize,
    coefficient_stage: ExpressionEvaluator<Complex<Rational>>,
}
impl CompiledNormalNumeratorJets {
    pub fn prepare(
        owner: Arc<NormalNumeratorJets>,
        coefficient_inputs: &[Symbol],
        source_functions: FunctionMap,
        settings: OptimizationSettings,
        budget: &Budget,
    ) -> Result<Self> {
        budget.check()?;
        let source = owner.catalogue.preparation().coordinates();
        let mut reserved = source
            .iter()
            .chain(owner.chart.target_units())
            .chain(owner.chart.parent_prefix().source().unit_coordinates())
            .copied()
            .collect::<BTreeSet<_>>();
        reserved.insert(owner.head);
        for section in &owner.sections {
            reserved.insert(section.section().root_symbol());
            reserved.extend(section.section().chart().parameters.iter().copied());
        }
        if coefficient_inputs
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len()
            != coefficient_inputs.len()
            || coefficient_inputs.iter().any(|s| reserved.contains(s))
        {
            return Err(Error::Invalid(
                "normal jet coefficient input collides with owned source/geometry role".into(),
            ));
        }
        let definitions = definitions::definition_index(&owner)?;
        let values = owner
            .coefficients
            .iter()
            .flat_map(|c| c.values.iter().cloned())
            .collect::<Vec<_>>();
        let mut calls = BTreeMap::new();
        for value in &values {
            let mut failure = None;
            value.visitor(&mut |node| {
                if failure.is_some() {
                    return false;
                }
                match definitions::source_call(node, owner.head, source.len()) {
                    Ok(Some((signature, args))) => {
                        match definitions.get(&signature) {
                            Some(&index) => {
                                calls.insert(node.to_owned(), (index, args));
                            }
                            None => {
                                failure = Some(Error::Invalid(
                                    "missing owned source derivative definition".into(),
                                ))
                            }
                        }
                        false
                    }
                    Err(e) => {
                        failure = Some(e);
                        false
                    }
                    _ => true,
                }
            });
            if let Some(e) = failure {
                return Err(e);
            }
        }
        let nd = owner.directions.len();
        let intermediate = settings.clone().cpe_iterations(Some(0));
        let mut compose = EvaluatorComposer::new(nd + coefficient_inputs.len());
        let directions = (0..nd).map(Slot::Param).collect::<Vec<_>>();
        // Only coordinate-free source coefficient trees see caller mappings.
        // The actual partial bodies consume these very slots: validating a
        // separate expansion would miss transitive A->x, x->0 alias changes.
        let mut coefficient_trees = BTreeSet::new();
        let mut per_definition = Vec::new();
        for definition in &owner.definitions {
            budget.check()?;
            let mut trees = BTreeSet::new();
            definition.body.visitor(&mut |node| {
                // Numeric literals, especially polynomial exponents, must
                // retain native fixed-integer lowering at finite zero inputs.
                if matches!(node, symbolica::atom::AtomView::Num(_)) {
                    return false;
                }
                if source
                    .iter()
                    .all(|s| !node.contains(Atom::var(*s).as_view()))
                {
                    trees.insert(node.to_owned());
                    false
                } else {
                    true
                }
            });
            coefficient_trees.extend(trees.iter().cloned());
            per_definition.push(trees);
        }
        if coefficient_trees.len() > budget.limits.max_terms {
            return Err(Error::ResourceIncomplete(
                "source coefficient stage inventory".into(),
            ));
        }
        let coefficient_trees = coefficient_trees.into_iter().collect::<Vec<_>>();
        let mut occupied = owner
            .definitions
            .iter()
            .flat_map(|d| d.body.get_all_symbols(true))
            .collect::<BTreeSet<_>>();
        occupied.extend(reserved.iter().copied());
        occupied.extend(coefficient_inputs.iter().copied());
        let scalar_symbols = (0usize..)
            .map(|namespace| {
                (0..coefficient_trees.len())
                    .map(|i| {
                        symbolica::symbol!(format!("fastsecdec::normal_scalar_{namespace}::v{i}"))
                    })
                    .collect::<Vec<_>>()
            })
            .find(|symbols| symbols.iter().all(|s| !occupied.contains(s)))
            .expect("finite source scalar role inventory");
        let scalar_index = coefficient_trees
            .iter()
            .cloned()
            .zip(scalar_symbols.iter().copied())
            .collect::<BTreeMap<_, _>>();
        let coefficient_atoms = coefficient_inputs
            .iter()
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>();
        let coefficient_stage = Atom::evaluator_multiple(&coefficient_trees, &coefficient_atoms)
            .function_map(source_functions)
            .optimization_settings(intermediate.clone())
            .build()
            .map_err(native)?;
        let scalar_slots = compose
            .append(
                &coefficient_stage,
                &(nd..nd + coefficient_inputs.len())
                    .map(Slot::Param)
                    .collect::<Vec<_>>(),
            )
            .map_err(native)?;
        let mut compiled_definitions = BTreeMap::new();
        let mut call_slots = Vec::new();
        let mut geometry_slots = Vec::new();
        let direction_atoms = owner
            .directions
            .iter()
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>();
        for (definition, args) in calls.values() {
            budget.check()?;
            // Only the small actual coordinate arguments enter this protected
            // empty-FunctionMap stage, including native selected-root calls.
            let geometry = Atom::evaluator_multiple(args, &direction_atoms)
                .optimization_settings(intermediate.clone())
                .build()
                .map_err(native)?;
            let geometry = compose.append(&geometry, &directions).map_err(native)?;
            geometry_slots.extend_from_slice(&geometry);
            if let std::collections::btree_map::Entry::Vacant(slot) =
                compiled_definitions.entry(*definition)
            {
                let d = &owner.definitions[*definition];
                let body = d
                    .body
                    .replace_multiple(per_definition[*definition].iter().map(|a| {
                        Replacement::new(
                            Pattern::Literal(a.clone()),
                            Pattern::Literal(Atom::var(scalar_index[a])),
                        )
                    }));
                let formals = d.formals.clone();
                let mut functions = FunctionMap::new();
                functions
                    .add_tagged_function_with_options(
                        d.head,
                        d.tags.clone(),
                        formals.clone(),
                        body,
                        FunctionRegistrationOptions::new().inlining(if d.always_inline {
                            InliningPolicy::Always
                        } else {
                            InliningPolicy::Auto
                        }),
                    )
                    .map_err(native)?;
                let mut arguments = d.tags.clone();
                arguments.extend(formals.iter().copied().map(Atom::var));
                let body = symbolica::atom::FunctionBuilder::new(d.head)
                    .add_args(&arguments)
                    .finish();
                let inputs = formals
                    .iter()
                    .chain(&scalar_symbols)
                    .copied()
                    .map(Atom::var)
                    .collect::<Vec<_>>();
                let exact = Atom::evaluator_multiple(&[body], &inputs)
                    .function_map(functions)
                    .optimization_settings(intermediate.clone())
                    .build()
                    .map_err(native)?;
                slot.insert(exact);
            }
            let mut inputs = geometry;
            inputs.extend_from_slice(&scalar_slots);
            call_slots.extend(
                compose
                    .append(&compiled_definitions[definition], &inputs)
                    .map_err(native)?,
            );
        }
        // Local assembly slots never pass through the source FunctionMap.
        let mut occupied = values
            .iter()
            .flat_map(|a| a.get_all_symbols(true))
            .collect::<BTreeSet<_>>();
        occupied.extend(coefficient_inputs.iter().copied());
        occupied.extend(owner.directions.iter().copied());
        let symbols = (0usize..)
            .map(|namespace| {
                (0..calls.len())
                    .map(|i| {
                        symbolica::symbol!(format!("fastsecdec::normal_assembly_{namespace}::v{i}"))
                    })
                    .collect::<Vec<_>>()
            })
            .find(|v| v.iter().all(|s| !occupied.contains(s)))
            .expect("finite local symbol inventory");
        let replacements = calls
            .keys()
            .zip(&symbols)
            .map(|(a, s)| {
                Replacement::new(Pattern::Literal(a.clone()), Pattern::Literal(Atom::var(*s)))
            })
            .collect::<Vec<_>>();
        let assembly = values
            .iter()
            .map(|a| a.replace_multiple(replacements.iter()))
            .collect::<Vec<_>>();
        let mut assembly_inputs = direction_atoms;
        assembly_inputs.extend(symbols.into_iter().map(Atom::var));
        let assembly = Atom::evaluator_multiple(&assembly, &assembly_inputs)
            .optimization_settings(intermediate)
            .build()
            .map_err(native)?;
        let mut assembly_slots = directions;
        assembly_slots.extend(call_slots);
        let mut outputs = compose.append(&assembly, &assembly_slots).map_err(native)?;
        let coefficient_count = outputs.len();
        let geometry_count = geometry_slots.len();
        outputs.extend(geometry_slots);
        let scalar_count = scalar_slots.len();
        outputs.extend(scalar_slots);
        let exact = compose.finish(&outputs, settings).map_err(native)?;
        budget.check()?;
        Ok(Self {
            owner,
            coefficient_inputs: coefficient_inputs.to_vec(),
            exact,
            coefficient_count,
            geometry_count,
            scalar_count,
            coefficient_stage,
        })
    }
    pub fn owner(&self) -> &Arc<NormalNumeratorJets> {
        &self.owner
    }
    pub fn coefficient_inputs(&self) -> &[Symbol] {
        &self.coefficient_inputs
    }
    /// Record-major absolute Taylor coefficients followed by retained finite
    /// geometric inputs. Arithmetic bytes require this live source/chart owner.
    pub fn exact(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.exact
    }
    /// Executed once per call and shared by all demanded source partials.
    pub fn coefficient_stage(&self) -> &ExpressionEvaluator<Complex<Rational>> {
        &self.coefficient_stage
    }
    pub fn scope(&self) -> &Scope {
        &self.owner.scope
    }
    pub fn map<T: Number>(self: &Arc<Self>, bits: u32) -> Result<NumericNormalNumeratorJets<T>>
    where
        Complex<T>: EvaluationDomain,
    {
        let program = self
            .scope()
            .map_checked(&self.exact, bits, |q| {
                Complex::new(T::rational_at(&q.re, bits), T::rational_at(&q.im, bits))
            })
            .map_err(Error::Native)?;
        Ok(NumericNormalNumeratorJets {
            owner: self.clone(),
            program,
            bits,
        })
    }
}
/// Caller-owned scratch; normal=0 is already bound symbolically. Remaining
/// direction inputs are strict, but a finite output may legitimately vanish.
pub struct NumericNormalNumeratorJets<T> {
    owner: Arc<CompiledNormalNumeratorJets>,
    program: ExpressionEvaluator<Complex<T>>,
    bits: u32,
}
impl<T: Number> NumericNormalNumeratorJets<T>
where
    Complex<T>: EvaluationDomain,
{
    pub fn evaluate(
        &mut self,
        directions: &[T],
        coefficients: &[Complex<T>],
    ) -> Result<Vec<Vec<Complex<T>>>> {
        if directions.len() != self.owner.owner.directions.len()
            || coefficients.len() != self.owner.coefficient_inputs.len()
            || directions
                .iter()
                .any(|x| !x.valid() || *x <= x.zero() || *x >= x.one())
            || coefficients.iter().any(|x| !x.re.valid() || !x.im.valid())
        {
            return Err(Error::Invalid(
                "normal jet requires strict relative directions and finite coefficient inputs"
                    .into(),
            ));
        }
        let mut inputs = directions
            .iter()
            .map(|x| Complex::new(x.clone(), T::exact_zero_at(self.bits)))
            .collect::<Vec<_>>();
        inputs.extend_from_slice(coefficients);
        let zero = directions
            .first()
            .map_or_else(|| T::exact_zero_at(self.bits), |x| x.zero());
        let mut outputs =
            vec![
                Complex::new(zero.clone(), zero);
                self.owner.coefficient_count + self.owner.geometry_count + self.owner.scalar_count
            ];
        attempt(|| self.program.try_evaluate(&inputs, &mut outputs))
            .map_err(Error::Native)?
            .map_err(native)?;
        if outputs.iter().any(|x| !x.re.valid() || !x.im.valid())
            || outputs[self.owner.coefficient_count
                ..self.owner.coefficient_count + self.owner.geometry_count]
                .iter()
                .any(|x| !x.im.is_zero())
        {
            return Err(Error::Native(
                "invalid normal coefficient or finite geometric input".into(),
            ));
        }
        Ok(outputs[..self.owner.coefficient_count]
            .chunks(self.owner.owner.depth as usize)
            .map(|v| v.to_vec())
            .collect())
    }
}
