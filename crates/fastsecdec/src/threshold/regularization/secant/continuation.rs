use super::*;
use symbolica::{
    atom::AtomView,
    evaluate::{FunctionRegistrationOptions, InliningPolicy},
};
pub struct ContinuedFamily<'a> {
    certificate: &'a NormalizedFamily,
    charts: Vec<Atom>,
    expression: Atom,
    profiles: Vec<Vec<crate::generation::EndpointProfileRow>>,
    functions: FunctionMap,
    options: GenerationOptions,
    definitions: Vec<crate::threshold::continued::Definition>,
    face_reductions: Vec<super::faces::FaceReduction>,
}
impl NormalizedFamily {
    pub fn continue_symbolically(
        &self,
        options: &GenerationOptions,
    ) -> Result<ContinuedFamily<'_>> {
        if options.mode != crate::generation::GenerationMode::Symbolic
            || options.contour_enabled()
            || options.contour_jacobian != crate::contour::ContourJacobian::Symbolic
            || options.source_sectors.is_some()
        {
            return Err(unsupported(
                "symbolic endpoints require no contour or selected-source routing",
            ));
        }
        let mut charts = Vec::new();
        let mut profiles = Vec::new();
        for chart in &self.charts {
            let out = crate::generation::threshold_subtract(
                chart
                    .terms
                    .iter()
                    .map(|t| (t.prefactor.clone(), t.regular.clone(), t.powers.clone()))
                    .collect(),
                &self.coordinates,
                &self.regulators,
                options,
            )
            .map_err(native)?;
            charts.push(out.expression);
            profiles.push(out.endpoint_profiles);
        }

        let mut functions = FunctionMap::new();
        let mut definitions = Vec::new();
        for (index, body) in self.bodies.iter().enumerate() {
            definitions.push(crate::threshold::continued::Definition {
                head: self.numerator,
                tags: vec![Atom::num(index)],
                formals: self.formals.clone(),
                body: body.clone(),
                orders: None,
                always_inline: true,
            });
            functions
                .add_tagged_function_with_options(
                    self.numerator,
                    vec![Atom::num(index)],
                    self.formals.clone(),
                    body.clone(),
                    FunctionRegistrationOptions::new().inlining(InliningPolicy::Always),
                )
                .map_err(native)?;
        }
        // Native DERIVATIVE carries one order for the tag and each source axis,
        // followed by the owned head, body slot and actual composed arguments.
        // Register only the finite mixed jets actually requested by subtraction.
        let n = self.formals.len();
        let max_source_jet = options
            .max_subtractions_per_axis
            .checked_mul(n)
            .ok_or_else(|| {
                Error::ResourceIncomplete("composed numerator derivative bound".into())
            })?;
        let mut jets = BTreeSet::new();
        let mut failure = None;
        for chart_expression in &charts {
            chart_expression.visitor(&mut |node| {
                if let AtomView::Fun(f) = node
                    && f.get_symbol() == Symbol::DERIVATIVE
                {
                    let args = f.iter().collect::<Vec<_>>();
                    if args.len() == 2 * (n + 1) + 1
                        && args[n + 1] == Atom::var(self.numerator).as_view()
                    {
                        let orders = args[..n + 1]
                            .iter()
                            .map(|a| {
                                Rational::try_from(*a)
                                    .ok()
                                    .filter(|x| x.is_integer())
                                    .and_then(|x| x.numerator_ref().to_i64())
                                    .and_then(|x| usize::try_from(x).ok())
                            })
                            .collect::<Option<Vec<_>>>();
                        let slot = Rational::try_from(args[n + 2])
                            .ok()
                            .filter(|x| x.is_integer())
                            .and_then(|x| x.numerator_ref().to_i64())
                            .and_then(|x| usize::try_from(x).ok());
                        match (orders, slot) {
                            (Some(orders), Some(slot))
                                if orders[0] == 0
                                    && slot < self.bodies.len()
                                    && orders.iter().all(|x| *x <= max_source_jet) =>
                            {
                                jets.insert((orders, slot));
                            }
                            _ => failure = Some(invalid("malformed owned numerator derivative")),
                        }
                    }
                }
                failure.is_none()
            });
        }
        if let Some(e) = failure {
            return Err(e);
        }
        for (orders, slot) in jets {
            let mut body = self.bodies[slot].clone();
            for (s, count) in self.formals.iter().zip(&orders[1..]) {
                for _ in 0..*count {
                    body = body.derivative(*s);
                }
            }
            let derivative_orders = orders.clone();
            let tags = orders
                .into_iter()
                .map(Atom::num)
                .chain([Atom::var(self.numerator), Atom::num(slot)])
                .collect::<Vec<_>>();
            definitions.push(crate::threshold::continued::Definition {
                head: Symbol::DERIVATIVE,
                tags: tags.clone(),
                formals: self.formals.clone(),
                body: body.clone(),
                orders: Some(derivative_orders),
                always_inline: true,
            });
            functions
                .add_tagged_function_with_options(
                    Symbol::DERIVATIVE,
                    tags,
                    self.formals.clone(),
                    body,
                    FunctionRegistrationOptions::new().inlining(InliningPolicy::Always),
                )
                .map_err(native)?;
        }
        let face_reductions = super::faces::reduce(self, &mut charts, &definitions)?;
        let expression: Atom = charts.iter().cloned().sum();
        Ok(ContinuedFamily {
            certificate: self,
            charts,
            expression,
            profiles,
            functions,
            options: options.clone(),
            definitions,
            face_reductions,
        })
    }
}
impl ContinuedFamily<'_> {
    pub(crate) fn face_reductions(&self) -> &[super::faces::FaceReduction] {
        &self.face_reductions
    }
    pub(crate) fn definitions(&self) -> &[crate::threshold::continued::Definition] {
        &self.definitions
    }
    pub fn certificate(&self) -> &NormalizedFamily {
        self.certificate
    }
    pub fn expression(&self) -> &Atom {
        &self.expression
    }
    pub fn charts(&self) -> &[Atom] {
        &self.charts
    }
    pub fn functions(&self) -> &FunctionMap {
        &self.functions
    }
    pub fn options(&self) -> &GenerationOptions {
        &self.options
    }
    pub fn profiles(&self) -> &[Vec<crate::generation::EndpointProfileRow>] {
        &self.profiles
    }
    /// Exact root calls remain owned. No legacy exact materialization route is
    /// exposed even if all integration coordinates disappear from a coefficient.
    pub fn expand_vector(
        &self,
        max_order: i32,
    ) -> Result<BTreeMap<i32, symbolica::atom::AliasedAtom>> {
        crate::generation::threshold_expand_vector(
            &self.expression,
            &self.certificate.coordinates,
            self.certificate.regulators[0],
            max_order,
        )
        .map_err(native)
    }
}
use std::collections::BTreeMap;
