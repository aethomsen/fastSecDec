use super::*;
use crate::{
    parametric::FactorRole,
    threshold::{maps::ParameterChamber, phase::CausalCell},
};
use regular::{
    Budget,
    callback::{RootProgram, Scope},
};
use symbolica::{
    atom::{SymbolAttribute, SymbolBuilder},
    domains::{atom::AtomField, float::Complex},
    evaluate::OptimizationSettings,
    id::{Pattern, Replacement},
    prelude::Q,
    wrap_symbol,
};
use symgcad::{
    algebra::{Algebra, Poly},
    domain::derive_domain,
};
fn substitute(a: &Atom, from: &[Symbol], to: &[Atom]) -> Atom {
    a.replace_multiple(from.iter().zip(to).map(|(s, t)| {
        Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(t.clone()))
    }))
}
// This fence inventories complete native expressions and declared roles; a
// helper tag is not allowed to capture even an unused bound source symbol.
pub(super) fn source_uses(request: &crate::threshold::gcad::GcadRequest, symbol: Symbol) -> bool {
    request.input().regulator() == symbol
        || request.input().parameters().contains(&symbol)
        || request.domain().coordinates().contains(&symbol)
        || request.aliases().iter().any(|a| a.symbol == symbol)
        || request.kinematics().runtime_parameters.contains(&symbol)
        || request.kinematics().exact_values.contains_key(&symbol)
        || request
            .domain()
            .strict_positive()
            .iter()
            .chain(&request.kinematics().strict_positive)
            .any(|a| a.contains_symbol(symbol))
        || request
            .input()
            .terms()
            .iter()
            .chain(request.prepared_terms())
            .any(|term| {
                term.prefactor().contains_symbol(symbol)
                    || term
                        .monomial_powers()
                        .iter()
                        .any(|a| a.contains_symbol(symbol))
                    || term.factors().iter().any(|f| {
                        f.polynomial().contains_symbol(symbol)
                            || f.exponent().contains_symbol(symbol)
                    })
            })
}
fn coefficient_bodies(
    a: &Atom,
    coordinates: &[Symbol],
    eps: Symbol,
    limits: &Limits,
) -> Result<Vec<(u32, Atom)>> {
    meromorphic::preflight_expression(a, limits.density).map_err(meromorphic_error)?;
    let field = AtomField {
        statistical_zero_test: false,
        ..AtomField::new()
    };
    let ep = a.to_polynomial_in_vars_with_field::<u32>(&[Atom::var(eps)], &field);
    let mut result = Vec::new();
    for t in &ep {
        if t.coefficient.contains_symbol(eps) {
            return Err(unsupported("numerator is not a finite epsilon polynomial"));
        }
        let p = t.coefficient.to_polynomial_in_vars_with_field::<u32>(
            &coordinates
                .iter()
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
            &field,
        );
        if (&p)
            .into_iter()
            .any(|term| Complex::<Rational>::try_from(term.coefficient.as_view()).is_err())
        {
            return Err(unsupported(
                "numerator lacks exact complex polynomial all-face smoothness",
            ));
        }
        result.push((t.exponents[0], t.coefficient.clone()));
    }
    Ok(result)
}
pub(super) fn admit(
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    limits: Limits,
    root_recipe: RootRecipe,
) -> Result<NormalizedFamily> {
    attempt(owner, coordinates, None, limits, root_recipe)
        .map_err(|error| error.into_error(unsupported))
}
pub(super) fn admit_pair(
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    proposals: [regular::BracketProposal; 2],
    limits: Limits,
    root_recipe: RootRecipe,
) -> Result<NormalizedFamily> {
    attempt(owner, coordinates, Some(proposals), limits, root_recipe)
        .map_err(|error| error.into_error(unsupported))
}
pub(super) fn attempt(
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    proposals: Option<[regular::BracketProposal; 2]>,
    limits: Limits,
    root_recipe: RootRecipe,
) -> ProposalResult<NormalizedFamily> {
    let mut budget = Budget::new(limits.geometry.clone());
    if let Some(cancel) = &limits.cancellation {
        budget = budget.with_cancellation(cancel.clone());
    }
    let geometry = match proposals {
        None => geometry::admit(&owner, &coordinates, &limits, &mut budget)?
            .into_iter()
            .map(ChartGeometry::Single)
            .collect(),
        Some(proposals) => paired::admit(&owner, &coordinates, proposals, &limits, &mut budget)?
            .into_iter()
            .map(ChartGeometry::Paired)
            .collect(),
    };
    assemble(
        owner,
        coordinates,
        limits,
        root_recipe,
        budget,
        geometry,
        None,
    )
    .map_err(regular::ProposalError::Failure)
}

pub(super) fn admit_parameters(
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    limits: Limits,
) -> Result<NormalizedFamily> {
    if owner.request().kinematics().runtime_parameters.is_empty() {
        return Err(unsupported(
            "uniform parameter family requires a nonempty chamber",
        ));
    }
    let parameters = &owner.request().kinematics().runtime_parameters;
    if owner.request().prepared_terms().iter().any(|term| {
        term.monomial_powers()
            .iter()
            .chain(term.factors().iter().map(|factor| factor.exponent()))
            .any(|exponent| parameters.iter().any(|p| exponent.contains_symbol(*p)))
    }) {
        return Err(unsupported(
            "uniform parameter family requires parameter-independent exponents",
        ));
    }
    let mut budget = Budget::new(limits.geometry.clone());
    if let Some(cancel) = &limits.cancellation {
        budget = budget.with_cancellation(cancel.clone());
    }
    let geometry = geometry::admit_uniform(&owner, &coordinates, &limits, &mut budget)
        .map_err(|error| error.into_error(unsupported))?;
    let chamber = ParameterChamber::prepare(geometry.iter().map(|g| g.source().clone()).collect())
        .map_err(native)?;
    assemble(
        owner,
        coordinates,
        limits,
        RootRecipe::ShiftedAffine,
        budget,
        geometry.into_iter().map(ChartGeometry::Single).collect(),
        Some(chamber),
    )
}

fn assemble(
    owner: Arc<VerifiedDecomposition>,
    coordinates: Vec<Symbol>,
    limits: Limits,
    root_recipe: RootRecipe,
    budget: Budget,
    geometry: Vec<ChartGeometry>,
    chamber: Option<Arc<ParameterChamber>>,
) -> Result<NormalizedFamily> {
    let request = owner.request();
    let eps = request.input().regulator();
    let n = coordinates.len();
    let source_coordinates = geometry[0]
        .source()
        .axes()
        .iter()
        .map(|a| a.symbol())
        .collect::<Vec<_>>();
    let epsilon_family = [eps];
    let numerator = SymbolBuilder::new(wrap_symbol!("crate::secant_endpoint::numerator"))
        .with_attributes(&[] as &[SymbolAttribute])
        .build()
        .map_err(native)?;
    if !numerator.is_exportable() || !numerator.get_attributes().is_empty() {
        return Err(invalid("owned numerator has hooks or attributes"));
    }
    for t in request.prepared_terms() {
        if t.prefactor().contains_symbol(numerator)
            || t.factors().iter().any(|f| {
                f.polynomial().contains_symbol(numerator) || f.exponent().contains_symbol(numerator)
            })
        {
            return Err(invalid("numerator helper collides with source"));
        }
    }
    if geometry[0].sections().len() == 2 && source_uses(request, numerator) {
        return Err(invalid(
            "numerator helper role collides with complete source",
        ));
    }
    let prepare = match root_recipe {
        RootRecipe::Physical => RootProgram::prepare,
        RootRecipe::Affine => RootProgram::prepare_affine,
        RootRecipe::ShiftedAffine => RootProgram::prepare_affine_shifted,
    };
    let unit_atoms = geometry[0]
        .source()
        .axes()
        .iter()
        .map(|axis| {
            Atom::var(
                axis.unit_index()
                    .map_or(axis.symbol(), |slot| coordinates[slot]),
            )
        })
        .collect::<Vec<_>>();
    let mut roots = Vec::new();
    let mut scope = Scope::default();
    for (role, section) in geometry[0].sections().iter().enumerate() {
        let helper = prepare(section, OptimizationSettings::default().cores(1))?;
        // Preserve the historical single-branch tag exactly. A pair has explicit
        // stable lower/upper roles; matching polynomial coefficients never merge them.
        let tag = if geometry[0].sections().len() == 1 {
            symbolica::symbol!("crate::secant_endpoint::root")
        } else if role == 0 {
            symbolica::symbol!("crate::secant_endpoint::lower_root")
        } else {
            symbolica::symbol!("crate::secant_endpoint::upper_root")
        };
        if geometry[0].sections().len() == 2 && source_uses(request, tag) {
            return Err(invalid("root helper role collides with complete source"));
        }
        let coefficients = section
            .coefficients()
            .iter()
            .map(|a| substitute(a, &source_coordinates, &unit_atoms))
            .collect::<Vec<_>>();
        roots.push(
            helper
                .call(tag, &coefficients)
                .map_err(regular::Error::from)?,
        );
        scope.insert(tag, helper).map_err(regular::Error::from)?;
    }
    let algebra = Algebra::new(&owner.native_result().order).map_err(native)?;
    let constraints = owner
        .native_result()
        .normalized_constraints
        .iter()
        .map(|p| algebra.parse(p).map_err(native))
        .collect::<Result<Vec<_>>>()?;
    let domain = derive_domain(&algebra, &constraints, &[]).map_err(native)?;
    let aliases = algebra
        .names
        .iter()
        .map(|s| Atom::var(Symbol::parse(s, "symgcad").unwrap()))
        .collect::<Vec<_>>();
    let mut bodies = Vec::new();
    let mut body_plan = Vec::new();
    let mut prefactors = Vec::new();
    let mut prefactor_atoms = Vec::new();
    for term in request.prepared_terms() {
        budget.check()?;
        let mut body = term
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
        let mut prefactor = request.kinematics().specialize_exact(term.prefactor());
        if request
            .kinematics()
            .runtime_parameters
            .iter()
            .any(|p| prefactor.contains_symbol(*p))
        {
            // A finite exact polynomial prefactor belongs to the smooth numerator.
            // General parameter-dependent meromorphic/Gamma factors stay refused.
            coefficient_bodies(&prefactor, &source_coordinates, eps, &limits)?;
            body *= &prefactor;
            prefactor = Atom::one();
        }
        let mut parts = Vec::new();
        for (order, body) in coefficient_bodies(&body, &source_coordinates, eps, &limits)? {
            if bodies.len() >= limits.density.terms {
                return Err(Error::ResourceIncomplete("numerator body inventory".into()));
            }
            parts.push((order, bodies.len()));
            bodies.push(body);
        }
        body_plan.push(parts);
        prefactors.push(
            meromorphic::MeromorphicPrefactor::admit(prefactor.clone(), eps, limits.density)
                .map_err(meromorphic_error)?,
        );
        prefactor_atoms.push(prefactor);
    }
    let mut strip = ConvergenceStrip::default();
    let mut charts = Vec::new();
    for g in geometry {
        budget.check()?;
        let (image, measure, magnitude, measure_regular, width_powers) = match &g {
            ChartGeometry::Single(single) => {
                let root = &roots[0];
                let unit = Atom::var(coordinates[n - 1]);
                let image = if single.lower_cell() {
                    root * (Atom::one() - &unit)
                } else {
                    root + (Atom::one() - root) * &unit
                };
                let measure = if single.lower_cell() {
                    root.clone()
                } else {
                    Atom::one() - root
                };
                let qe = single.atom(single.endpoint_secant(), root);
                let qphi = single.pulled_secant(root, &image);
                let width_residual = single.atom(single.width().residual(), root);
                // Keep the historical symbolic product tree and powers below.
                (
                    image,
                    measure,
                    vec![width_residual.clone(), qphi, qe.clone()],
                    width_residual / qe,
                    single.width().powers().to_vec(),
                )
            }
            ChartGeometry::Paired(pair) => {
                let (image, measure, magnitude) = pair.normalized(&roots);
                (
                    image,
                    measure.clone(),
                    vec![magnitude],
                    measure,
                    vec![0; n - 1],
                )
            }
        };
        let mut mapped = unit_atoms.clone();
        *mapped.last_mut().expect("nonempty source axes") = image.clone();
        let cell = owner
            .cells()
            .nth(g.source().cell_index())
            .ok_or_else(|| invalid("source cell inventory"))?;
        let phases = CausalCell::new(cell, &epsilon_family).map_err(native)?;
        let mut terms = Vec::new();
        for (ti, term) in request.prepared_terms().iter().enumerate() {
            let mut powers = vec![Atom::zero(); n];
            for axis in g.source().axes() {
                let Some(i) = axis.unit_index() else {
                    continue;
                };
                let s = axis.symbol();
                let source_index = request
                    .domain()
                    .coordinates()
                    .iter()
                    .position(|p| *p == s)
                    .ok_or_else(|| invalid("source coordinate role"))?;
                let a = request
                    .kinematics()
                    .specialize_exact(&term.monomial_powers()[source_index]);
                crate::generation::endpoint_power_with_regulators(&a, &epsilon_family)
                    .map_err(native)?;
                if i == n - 1 && !a.is_zero() {
                    return Err(unsupported(
                        "last source-coordinate monomial needs additional face splitting",
                    ));
                }
                powers[i] = a;
            }
            let mut ledger = Vec::new();
            let mut regular = measure_regular.clone();
            for (i, power) in width_powers.as_slice().iter().enumerate() {
                powers[i] += &Atom::num(*power);
            }
            for (fi, factor) in term.factors().iter().enumerate() {
                let exponent = request.kinematics().specialize_exact(factor.exponent());
                let source = request.kinematics().specialize_exact(factor.polynomial());
                if factor.role() == FactorRole::Polynomial {
                    ledger.push(FactorLedger {
                        term: ti,
                        factor: fi,
                        source,
                        exponent,
                        kind: FactorKind::PolynomialNumerator,
                    });
                    continue;
                }
                crate::generation::endpoint_power_with_regulators(&exponent, &epsilon_family)
                    .map_err(native)?;
                if exponent.is_zero() {
                    continue;
                }
                meromorphic::preflight_expression(&source, limits.density)
                    .map_err(meromorphic_error)?;
                let alias_source = substitute(&source, &source_coordinates, &aliases);
                let p: Poly = alias_source
                    .try_to_polynomial(&Q, algebra.variables.clone())
                    .map_err(native)?;
                if p.variables() != &algebra.variables {
                    return Err(unsupported(
                        "factor retained a physical or nonrational coefficient",
                    ));
                }
                let section_poly = g.sections()[0].polynomial();
                let (ratio, remainder) = p.quot_rem(section_poly, false);
                let scale = if remainder.is_zero() {
                    Rational::try_from(ratio.to_expression().as_view()).ok()
                } else {
                    None
                };
                let kind = if let Some(scale) = scale.filter(|x| !x.is_zero()) {
                    // Original factor magnitude includes its entire nonzero
                    // scalar normalization, independent of projection sign.
                    let absolute = scale.abs();
                    regular *= match &g {
                        ChartGeometry::Single(_) => {
                            Atom::num(absolute).pow(&exponent)
                                * magnitude[0].pow(&exponent)
                                * magnitude[1].pow(&exponent)
                                / magnitude[2].pow(&exponent)
                        }
                        ChartGeometry::Paired(_) => {
                            Atom::num(absolute).pow(&exponent) * magnitude[0].pow(&exponent)
                        }
                    };
                    powers[n - 1] += &exponent;
                    for (i, power) in width_powers.as_slice().iter().enumerate() {
                        powers[i] += Atom::num(*power) * &exponent;
                    }
                    FactorKind::Section { scale }
                } else {
                    let (_, sign) = cell
                        .signed_factors()
                        .find(|(f, _)| f.term_index == ti && f.factor_index == fi)
                        .ok_or_else(|| invalid("factor sign association absent"))?;
                    let positive = &p * &p.constant(Rational::from(i64::from(sign)));
                    let margin = &positive - &positive.constant(limits.unit_margin.clone());
                    let certificate = geometry::positive(
                        &algebra,
                        &margin,
                        &constraints,
                        &domain,
                        &limits,
                        &budget,
                    )?;
                    regular *= substitute(
                        &(Atom::num(i64::from(sign)) * &source),
                        &source_coordinates,
                        &mapped,
                    )
                    .pow(&exponent);
                    FactorKind::ClosedUnit {
                        certificate: Arc::new(certificate),
                    }
                };
                ledger.push(FactorLedger {
                    term: ti,
                    factor: fi,
                    source,
                    exponent,
                    kind,
                });
            }
            for power in &powers {
                strip.admit(power, eps)?;
            }
            for (order, slot) in &body_plan[ti] {
                let call = numerator
                    .call_args(std::iter::once(Atom::num(*slot)).chain(mapped.iter().cloned()));
                terms.push(NormalizedTerm {
                    prefactor: prefactor_atoms[ti].clone()
                        * &phases.term_phases()[ti]
                        * Atom::var(eps).pow(*order),
                    regular: &regular * call,
                    powers: powers.clone(),
                    ledger: ledger.clone(),
                    term: ti,
                });
            }
        }
        charts.push(NormalizedChart {
            geometry: g,
            image,
            measure,
            terms,
        });
    }
    let witness = meromorphic::MeromorphicWitness::construct(
        prefactors,
        strip.lower(),
        strip.upper(),
        limits.density,
        |_| {
            if budget.check().is_ok() {
                ControlFlow::Continue(())
            } else {
                ControlFlow::Break(())
            }
        },
    );
    budget.check()?;
    let witness = witness.map_err(meromorphic_error)?;
    Ok(NormalizedFamily {
        owner,
        coordinates,
        chamber: chamber.map(UniformParameterChamber::issued),
        arguments: unit_atoms,
        regulators: vec![eps],
        charts,
        strip,
        witness,
        roots,
        scope,
        numerator,
        bodies,
        formals: source_coordinates,
        limits,
    })
}
