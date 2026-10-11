use super::*;
use crate::threshold::gcad::GcadRequest;
use symbolica::{
    domains::float::Complex,
    prelude::{PolyVariable, Q},
};
use symgcad::algebra::Poly;

fn polynomial(
    expression: &Atom,
    variable: Symbol,
    limits: Limits,
) -> Result<symbolica::poly::polynomial::MultivariatePolynomial<AtomField, u32>> {
    let degree = u16::try_from(limits.max_degree)
        .map_err(|_| Error::ResourceIncomplete("polynomial preflight degree limit"))?;
    meromorphic::preflight_expression(
        expression,
        meromorphic::Limits {
            degree,
            ..limits.prefactors
        },
    )
    .map_err(prefactor_error)?;
    let field = AtomField {
        statistical_zero_test: false,
        ..AtomField::new()
    };
    let polynomial =
        expression.to_polynomial_in_vars_with_field::<u32>(&[Atom::var(variable)], &field);
    if polynomial.degree(0) > limits.max_degree {
        return Err(Error::ResourceIncomplete("rational polynomial degree"));
    }
    if (&polynomial)
        .into_iter()
        .any(|t| t.coefficient.contains_symbol(variable))
    {
        return Err(unsupported(
            "expression is not an admitted bounded-degree coordinate polynomial",
        ));
    }
    Ok(polynomial)
}
/// Native finite epsilon coefficients. Tags index this complete body list;
/// original term and epsilon degree remain explicit in the component map.
pub(super) struct NumeratorPlan {
    head: Symbol,
    pub bodies: Vec<Atom>,
    pub components: Vec<Vec<(u32, usize)>>,
}
pub(super) fn numerators(
    request: &GcadRequest,
    parameters: &BTreeMap<Symbol, Rational>,
    name: Symbol,
    limits: Limits,
) -> Result<NumeratorPlan> {
    let x = request.domain().coordinates()[0];
    let eps = request.input().regulator();
    let mut plan = NumeratorPlan {
        head: name,
        bodies: Vec::new(),
        components: Vec::new(),
    };
    for term in request.prepared_terms() {
        if term.prefactor().contains_symbol(name)
            || term
                .factors()
                .iter()
                .any(|f| f.polynomial().contains_symbol(name) || f.exponent().contains_symbol(name))
        {
            return Err(invalid(
                "owned numerator function collides with density symbols",
            ));
        }
        let body = term
            .factors()
            .iter()
            .filter(|f| f.role() == FactorRole::Polynomial)
            .map(|f| {
                request
                    .kinematics()
                    .specialize_exact(f.polynomial())
                    .pow(request.kinematics().specialize_exact(f.exponent()))
            })
            .product::<Atom>();
        // Conversion is bounded before native work and only in epsilon.
        // Its AtomField coefficients retain factored coordinate bodies.
        let epsilon_polynomial = polynomial(&body, eps, limits)?;
        let mut components = Vec::new();
        for coefficient in &epsilon_polynomial {
            let witness = substitute(coefficient.coefficient, parameters);
            let p = polynomial(&witness, x, limits)?;
            if (&p)
                .into_iter()
                .any(|t| Complex::<Rational>::try_from(t.coefficient.as_view()).is_err())
            {
                return Err(unsupported(
                    "numerator needs exact complex-polynomial closure regularity",
                ));
            }
            if plan.bodies.len() >= limits.prefactors.terms {
                return Err(Error::ResourceIncomplete(
                    "numerator coefficient body count",
                ));
            }
            components.push((coefficient.exponents[0], plan.bodies.len()));
            plan.bodies.push(coefficient.coefficient.clone());
        }
        plan.components.push(components);
    }
    Ok(plan)
}

#[derive(Clone, Copy)]
struct Pullback<'a> {
    image: &'a Atom,
    source: Symbol,
    target: Symbol,
}
fn unit(
    source: &Atom,
    exponent: &Atom,
    pullback: Pullback<'_>,
    parameters: &BTreeMap<Symbol, Rational>,
    factor: Option<usize>,
    limits: Limits,
) -> Result<(ClosedUnit, Atom)> {
    let Pullback {
        image,
        source: x,
        target: t,
    } = pullback;
    let mapped = source.replace(Atom::var(x)).with(image.clone()).cancel();
    // Valuation is symbolic in retained parameter inputs. A special fiber that
    // increases it is rejected by the endpoint unit checks, not silently baked
    // into an expression with an unresolved symbolic 0/0 at the face.
    let p = polynomial(&mapped, t, limits)?;
    if p.is_zero() {
        return Err(unsupported("identically zero singular factor"));
    }
    let multiplicity = p.degree_bounds(0).0;
    let field = AtomField {
        statistical_zero_test: false,
        ..AtomField::new()
    };
    let monomial = Atom::var(t)
        .pow(multiplicity)
        .to_polynomial_in_vars_with_field::<u32>(&[Atom::var(t)], &field);
    let (quotient, remainder) = p.quot_rem(&monomial, false);
    if !remainder.is_zero() {
        return Err(invalid("native monomial quotient was not exact"));
    }
    let residual =
        quotient.to_expression_with_coeff_map(|_, coefficient, out| *out = coefficient.clone());
    let witness = substitute(&residual, parameters).cancel();
    let variables = Arc::new(vec![PolyVariable::from(t)]);
    let rational: Poly = witness
        .try_to_polynomial(&Q, variables.clone())
        .map_err(|e| {
            unsupported(&format!(
                "singular unit is not rational on the admitted fiber: {e}"
            ))
        })?;
    if rational.variables() != &variables {
        return Err(unsupported(
            "singular unit has undeclared coefficient variables",
        ));
    }
    if rational.degree(0) > u16::MAX as u32 {
        return Err(invalid(
            "specialized degree exceeds Symbolica's certified real-root isolation exponent range",
        ));
    }
    // The checked map contains only the original t symbol: there is no prefix
    // to specialize. Reuse the native conversion used by symGCAD's wrapper.
    let univariate = rational.to_univariate_from_univariate(0);
    let lower = univariate.evaluate(&Rational::zero());
    let upper = univariate.evaluate(&Rational::one());
    if lower <= Rational::zero()
        || upper <= Rational::zero()
        || symgcad::roots::roots_in_open_interval(&univariate, &Rational::zero(), &Rational::one())
            != 0
    {
        return Err(unsupported(
            "residual factor is not a positive unit on the entire closed interval",
        ));
    }
    Ok((
        ClosedUnit {
            original_factor: factor,
            multiplicity,
            residual,
            lower_value: lower,
            upper_value: upper,
        },
        Atom::num(multiplicity) * exponent,
    ))
}

pub(super) fn terms(
    half_chart: &IntervalChart,
    t: Symbol,
    numerator_plan: &NumeratorPlan,
    parameters: &BTreeMap<Symbol, Rational>,
    limits: Limits,
    chart: usize,
    observer: &mut impl FnMut(Progress) -> ControlFlow<()>,
) -> Result<Vec<NormalizedTerm>> {
    let admission = &half_chart.admission;
    let image = &half_chart.image;
    let measure = &half_chart.measure;
    let owner = admission.map().source().decomposition();
    let request = owner.request();
    let x = request.domain().coordinates()[0];
    let eps = request.input().regulator();
    let cell = owner
        .cells()
        .nth(admission.map().source().cell_index())
        .ok_or_else(|| invalid("missing verified cell"))?;
    let causal = CausalCell::new(cell, &[eps])?;
    let signs = causal
        .magnitudes()
        .map(|m| ((m.factor().term_index, m.factor().factor_index), m.sign()))
        .collect::<BTreeMap<_, _>>();
    let mut output = Vec::new();
    for (term_index, term) in request.prepared_terms().iter().enumerate() {
        let mut power = Atom::Zero;
        let mut units = Vec::new();
        let mut regular = numerator_plan.components[term_index]
            .iter()
            .map(|(order, body)| {
                Atom::var(eps).pow(*order)
                    * numerator_plan
                        .head
                        .call(&[Atom::num(*body), image.clone()][..])
            })
            .sum::<Atom>();
        let kinematics = request.kinematics();
        let monomial = kinematics.specialize_exact(&term.monomial_powers()[0]);
        endpoint_power_with_regulators(&monomial, &[eps])
            .map_err(|e| unsupported(&e.to_string()))?;
        if !monomial.is_zero() {
            let (certificate, shift) = unit(
                &Atom::var(x),
                &monomial,
                Pullback {
                    image,
                    source: x,
                    target: t,
                },
                parameters,
                None,
                limits,
            )?;
            regular *= certificate.residual.pow(&monomial);
            power += shift;
            units.push(certificate);
        }
        for (factor_index, factor) in term.factors().iter().enumerate() {
            if factor.role() == FactorRole::Polynomial {
                continue;
            }
            poll(
                Progress::Factor {
                    chart,
                    term: term_index,
                    factor: factor_index,
                },
                observer,
            )?;
            let exponent = kinematics.specialize_exact(factor.exponent());
            // An exactly absent factor needs neither a branch nor a unit.
            // The original GCAD request still owns any stricter source admission.
            if exponent.is_zero() {
                continue;
            }
            endpoint_power_with_regulators(&exponent, &[eps])
                .map_err(|e| unsupported(&e.to_string()))?;
            let sign = *signs
                .get(&(term_index, factor_index))
                .ok_or_else(|| invalid("missing signed factor association"))?;
            let source =
                Atom::num(i64::from(sign)) * kinematics.specialize_exact(factor.polynomial());
            let (certificate, shift) = unit(
                &source,
                &exponent,
                Pullback {
                    image,
                    source: x,
                    target: t,
                },
                parameters,
                Some(factor_index),
                limits,
            )?;
            regular *= certificate.residual.pow(&exponent);
            power += shift;
            units.push(certificate);
        }
        let prefactor = kinematics.specialize_exact(term.prefactor())
            * &causal.term_phases()[term_index]
            * measure;
        output.push(NormalizedTerm {
            prefactor,
            regular,
            power: power.expand(),
            units,
        });
    }
    Ok(output)
}
