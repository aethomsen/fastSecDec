use super::algebra::pull;
use super::*;
use crate::{parametric::FactorRole, threshold::phase::CausalCell};
use std::collections::BTreeSet;
use symbolica::{
    domains::{atom::AtomField, float::Complex},
    poly::PolyVariable,
};

fn polynomial_numerator(a: &Atom, coordinates: &[Symbol], eps: Symbol, w: &Work<'_>) -> Result<()> {
    meromorphic::preflight_expression(a, w.limits.prefactors).map_err(prefactor_error)?;
    let field = AtomField {
        statistical_zero_test: false,
        ..AtomField::new()
    };
    let variables = coordinates
        .iter()
        .copied()
        .chain([eps])
        .map(PolyVariable::from)
        .collect::<Vec<_>>();
    let p = a.to_polynomial_in_vars_with_field::<u32>(Arc::new(variables), &field);
    for monomial in &p {
        let coefficient = Complex::<Rational>::try_from(monomial.coefficient.as_view())
            .map_err(|_| unsupported("finite exact complex polynomial numerator required"))?;
        w.budget.rational(&coefficient.re)?;
        w.budget.rational(&coefficient.im)?;
    }
    Ok(())
}
fn powers_add(target: &mut [Atom], power: &Atom, orders: &[i64]) {
    for (t, o) in target.iter_mut().zip(orders) {
        *t += power * Atom::num(*o);
    }
}
fn fresh(owner: &VerifiedDecomposition, coordinates: &[Symbol]) -> Result<Symbol> {
    let mut occupied = crate::threshold::maps::coordinate_symbols(owner.request());
    occupied.extend(coordinates.iter().copied());
    for i in 0..10_000 {
        let symbol = Symbol::parse(
            format!("fastsecdec::resolved_projective::primitive_{i}"),
            "fastsecdec",
        )
        .map_err(native)?;
        if !occupied.contains(&symbol) && symbol.is_exportable() {
            return Ok(symbol);
        }
    }
    Err(Error::ResourceIncomplete(
        "fresh algebraic primitive role".into(),
    ))
}
pub(super) fn admit(
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    limits: Limits,
    observer: &mut dyn FnMut(Progress) -> ControlFlow<()>,
) -> Result<ResolvedProjectiveFamily> {
    let request = owner.request();
    let prepared = request
        .projective_preparation()
        .ok_or_else(|| unsupported("original affine-projective request required"))?;
    let dimensions = prepared.coordinates().len();
    if !request.kinematics().runtime_parameters.is_empty()
        || dimensions < 2
        || coordinates.len() != dimensions
        || coordinates.iter().collect::<BTreeSet<_>>().len() != dimensions
        || owner.cells().len() == 0
    {
        return Err(unsupported(
            "nonempty fixed projective cell family with distinct chart coordinates required",
        ));
    }
    let minimum_charts = u32::try_from(dimensions)
        .ok()
        .and_then(|n| 1usize.checked_shl(n))
        .and_then(|halves| halves.checked_mul(owner.cells().len()))
        .ok_or_else(|| Error::ResourceIncomplete("projective half-cube count overflow".into()))?;
    if minimum_charts > limits.max_charts {
        return Err(Error::ResourceIncomplete(
            "complete projective half-cube allowance".into(),
        ));
    }
    let mut w = Work {
        limits: &limits,
        budget: regular::Budget::new(limits.algebra.clone()),
        observer,
    };
    let primitive = fresh(&owner, &coordinates)?;
    let eps = request.input().regulator();
    let regulators = vec![eps];
    let mut inputs = Vec::new();
    let mut prefactors = Vec::new();
    let mut original_prefactors = Vec::new();
    let mut numerators = Vec::new();
    for (i, s) in prepared.coordinates().iter().enumerate() {
        if request
            .prepared_terms()
            .iter()
            .any(|t| !t.monomial_powers()[i].is_zero())
        {
            inputs.push((FactorOrigin::Coordinate(i), Atom::var(*s)));
        }
    }
    for (ti, term) in request.prepared_terms().iter().enumerate() {
        let prefactor = request.kinematics().specialize_exact(term.prefactor());
        prefactors.push(
            meromorphic::MeromorphicPrefactor::admit(prefactor.clone(), eps, limits.prefactors)
                .map_err(prefactor_error)?,
        );
        original_prefactors.push(prefactor);
        let mut numerator = Atom::one();
        for (fi, factor) in term.factors().iter().enumerate() {
            let exponent = request.kinematics().specialize_exact(factor.exponent());
            if exponent.is_zero() {
                continue;
            }
            let polynomial = request.kinematics().specialize_exact(factor.polynomial());
            w.atom(&polynomial)?;
            if factor.role() == FactorRole::Polynomial {
                let exponent = Rational::try_from(exponent.as_view()).map_err(|_| {
                    unsupported("polynomial numerator exponent must be a nonnegative integer")
                })?;
                if !exponent.is_integer() || exponent < Rational::zero() {
                    return Err(unsupported(
                        "polynomial numerator exponent must be a nonnegative integer",
                    ));
                }
                w.budget.rational(&exponent)?;
                if exponent > w.limits.prefactors.integer_power {
                    return Err(Error::ResourceIncomplete(
                        "polynomial numerator power allowance".into(),
                    ));
                }
                meromorphic::preflight_expression(&polynomial, w.limits.prefactors)
                    .map_err(prefactor_error)?;
                numerator *= polynomial.pow(Atom::num(exponent));
            } else {
                crate::generation::endpoint_power_with_regulators(&exponent, &regulators)
                    .map_err(native)?;
                geometry::rational_source(&polynomial, prepared.coordinates())?;
                inputs.push((
                    FactorOrigin::Factor {
                        term: ti,
                        factor: fi,
                    },
                    polynomial,
                ));
            }
        }
        polynomial_numerator(&numerator, prepared.coordinates(), eps, &w)?;
        numerators.push(numerator);
    }
    let mut charts = Vec::new();
    let mut strip = ConvergenceStrip::default();
    for ci in 0..owner.cells().len() {
        let mut cell_charts =
            geometry::build_cell(&owner, ci, &coordinates, primitive, &inputs, &mut w)?;
        if charts.len().saturating_add(cell_charts.len()) > limits.max_charts {
            return Err(Error::ResourceIncomplete(
                "complete projective chart allowance".into(),
            ));
        }
        let causal = CausalCell::new(
            owner
                .cells()
                .nth(ci)
                .ok_or_else(|| invalid("source cell ordinal"))?,
            &regulators,
        )
        .map_err(native)?;
        for chart in &mut cell_charts {
            w.emit(Progress::Chart(charts.len()))?;
            let measure = chart
                .receipts
                .iter()
                .find(|r| r.origin == FactorOrigin::Measure)
                .ok_or_else(|| invalid("map measure receipt"))?;
            for (ti, term) in request.prepared_terms().iter().enumerate() {
                let mut powers = measure
                    .powers
                    .iter()
                    .map(|p| Atom::num(*p))
                    .collect::<Vec<_>>();
                let mut regular = measure.magnitude()
                    * pull(&numerators[ti], prepared.coordinates(), &chart.images);
                for (i, power) in term.monomial_powers().iter().enumerate() {
                    let power = request.kinematics().specialize_exact(power);
                    crate::generation::endpoint_power_with_regulators(&power, &regulators)
                        .map_err(native)?;
                    if power.is_zero() {
                        continue;
                    }
                    let receipt = chart
                        .receipts
                        .iter()
                        .find(|r| r.origin == FactorOrigin::Coordinate(i))
                        .ok_or_else(|| invalid("active source coordinate receipt"))?;
                    if receipt.sign() != 1 {
                        return Err(invalid("source positive coordinate sign"));
                    }
                    powers_add(&mut powers, &power, &receipt.powers);
                    regular *= receipt.magnitude().pow(power);
                }
                for (fi, factor) in term.factors().iter().enumerate() {
                    let exponent = request.kinematics().specialize_exact(factor.exponent());
                    if exponent.is_zero() || factor.role() == FactorRole::Polynomial {
                        continue;
                    }
                    let receipt = chart
                        .receipts
                        .iter()
                        .find(|r| {
                            r.origin
                                == (FactorOrigin::Factor {
                                    term: ti,
                                    factor: fi,
                                })
                        })
                        .ok_or_else(|| invalid("active source factor receipt"))?;
                    let expected = causal
                        .cell()
                        .signed_factors()
                        .find(|(f, _)| f.term_index == ti && f.factor_index == fi)
                        .map(|(_, sign)| sign)
                        .ok_or_else(|| invalid("native source-factor sign association"))?;
                    if receipt.sign() != expected {
                        return Err(invalid("closed residual sign differs from original cell"));
                    }
                    powers_add(&mut powers, &exponent, &receipt.powers);
                    regular *= receipt.magnitude().pow(exponent);
                }
                for power in &powers {
                    strip.admit(power, eps).map_err(native)?;
                }
                chart.terms.push(NormalizedTerm {
                    term: ti,
                    prefactor: &original_prefactors[ti] * &causal.term_phases()[ti],
                    regular,
                    powers,
                });
            }
        }
        charts.extend(cell_charts);
    }
    let witness = meromorphic::MeromorphicWitness::construct(
        prefactors,
        strip.lower(),
        strip.upper(),
        limits.prefactors,
        |_| {
            if w.check().is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        },
    )
    .map_err(prefactor_error)?;
    w.check()?;
    Ok(ResolvedProjectiveFamily {
        owner,
        coordinates,
        regulators,
        charts,
        strip,
        witness,
    })
}
