//! Optional local rational-coefficient identities, never numerical zero tests.
//! This is a bounded gap-cleared identity check, not a saturation algorithm.
use super::*;
use crate::threshold::continued::Definition;
use symbolica::{
    atom::{AliasedAtom, AtomView},
    id::{Pattern, Replacement},
    poly::polynomial::MultivariatePolynomial,
    prelude::{PolyVariable, Q, Z},
};

#[path = "zeros/replay.rs"]
mod replay;
#[path = "zeros/units.rs"]
mod units;
type Poly = MultivariatePolynomial<Q, u32>;

/// A local arithmetic witness. Its unit authority comes from the live sealed
/// endpoint owner; restoring this record does not restore geometric authority.
pub(crate) struct Receipt {
    pub order: i32,
    pub variables: Vec<Symbol>,
    pub atoms: Vec<Atom>,
    pub gap_power: u32,
    pub branches: [String; 2],
    pub inverse_units: Vec<(usize, u32)>,
    pub digest: String,
}
// atoms: N,D,U,H,gap,Pa,Pb,Qa,Qb, then the admitted unit polynomials.
const FIXED: usize = 9;

pub(crate) fn prove(
    bound: &ContinuedFamily<'_>,
    chart: usize,
    order: i32,
    coefficient: &AliasedAtom,
) -> Result<Option<Receipt>> {
    // Discovery is optional. A bounded miss leaves the original coefficient;
    // cancellation and inconsistent/native failures must remain distinguishable.
    match discover(bound, chart, order, coefficient) {
        Err(Error::Unsupported(_) | Error::ResourceIncomplete(_)) => Ok(None),
        other => other,
    }
}

pub(crate) fn discover(
    bound: &ContinuedFamily<'_>,
    chart: usize,
    order: i32,
    coefficient: &AliasedAtom,
) -> Result<Option<Receipt>> {
    let family = bound.certificate();
    if family.roots.len() != 2 || coefficient.get_root().is_zero() {
        return Ok(None);
    }
    if !family.runtime_parameters().is_empty() {
        return Err(unsupported(
            "parameterized paired coefficient-zero identities require a separate role/unit receipt",
        ));
    }
    let mut budget = regular::Budget::new(family.limits.geometry.clone());
    if let Some(cancel) = &family.limits.cancellation {
        budget = budget.with_cancellation(cancel.clone());
    }
    budget.check()?;
    if coefficient.get_aliases().len() > family.limits.density.terms {
        return Err(Error::ResourceIncomplete(
            "coefficient-zero alias inventory".into(),
        ));
    }
    let mut nodes = 0usize;
    for a in std::iter::once(coefficient.get_root()).chain(coefficient.get_aliases().values()) {
        // The source still contains native owned function calls. The rational
        // expansion bound applies after materialization, not to those calls.
        a.visitor(&mut |_| {
            nodes = nodes.saturating_add(1);
            nodes <= family.limits.density.source_nodes
        });
        if nodes > family.limits.density.source_nodes {
            return Err(Error::ResourceIncomplete(
                "coefficient-zero source nodes".into(),
            ));
        }
    }
    // Native-generated aliases are acyclic. This is per-record scratch; native
    // DAG materialization can allocate before its postcheck and still needs the
    // caller's process memory cap. Bulk functions are never rewritten.
    let original = coefficient.clone().into_inner();
    let roles = symbolica::symbol!(
        "fastsecdec::threshold_zero::lower",
        "fastsecdec::threshold_zero::upper"
    );
    let roles = [roles.0, roles.1];
    if roles
        .iter()
        .any(|s| density::source_uses(family.owner.request(), *s) || family.coordinates.contains(s))
    {
        return Ok(None);
    }
    let variables = roles
        .iter()
        .chain(&family.coordinates)
        .copied()
        .collect::<Vec<_>>();
    let var_map = Arc::new(
        variables
            .iter()
            .copied()
            .map(PolyVariable::from)
            .collect::<Vec<_>>(),
    );
    let Some(scratch) = materialize(&original, family, bound.definitions(), roles)? else {
        return Ok(None);
    };
    match meromorphic::preflight_expression(&scratch, family.limits.density) {
        Ok(()) => {}
        Err(meromorphic::Error::Unsupported(_)) => return Ok(None),
        Err(e) => return Err(meromorphic_error(e)),
    }
    let Some(admitted) = units::admitted(family, chart, roles, &var_map)? else {
        return Ok(None);
    };
    let Some(inverses) = units::inverses(&scratch, &admitted, &var_map, family)? else {
        return Ok(None);
    };
    budget.check()?;
    let fraction =
        match scratch.try_to_rational_polynomial::<_, _, u32>(&Q, &Z, Some(var_map.clone())) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        };
    let Some(numerator) = polynomial(&fraction.numerator.to_expression(), &var_map) else {
        return Ok(None);
    };
    let Some(denominator) = polynomial(&fraction.denominator.to_expression(), &var_map) else {
        return Ok(None);
    };
    for p in [&numerator, &denominator] {
        check(p, family, &budget)?;
    }
    if denominator.is_zero() {
        return Ok(None);
    }
    let mut product = numerator.one();
    for (index, power) in &inverses {
        for _ in 0..*power {
            pre_product(&product, &admitted[*index], family)?;
            product = &product * &admitted[*index];
            check(&product, family, &budget)?;
        }
    }
    let (multiple, residual) = product.quot_rem(&denominator, false);
    if !residual.is_zero() || &multiple * &denominator != product {
        return Ok(None);
    }
    check(&multiple, family, &budget)?;
    let geometry = &family.charts[chart].geometry;
    let sections = geometry.sections();
    let equations = sections
        .iter()
        .zip(roles)
        .map(|(section, role)| {
            let inputs = family.formals.iter().copied();
            let outputs = family.coordinates[..family.coordinates.len() - 1]
                .iter()
                .copied()
                .map(Atom::var)
                .chain([Atom::var(role)]);
            let equation =
                section
                    .equation()
                    .replace_multiple(inputs.zip(outputs).map(|(s, a)| {
                        Replacement::new(Pattern::Literal(Atom::var(s)), Pattern::Literal(a))
                    }));
            polynomial(&equation, &var_map)
        })
        .collect::<Option<Vec<_>>>();
    let Some(equations) = equations else {
        return Ok(None);
    };
    // Root-leading variable order and a rational leading unit prevent divisions
    // by an unproved coefficient depending on another coordinate/root.
    for (role, equation) in equations.iter().enumerate() {
        check(equation, family, &budget)?;
        if equation.degree(role) == 0
            || equation
                .exponents_back(0)
                .iter()
                .enumerate()
                .any(|(i, e)| i != role && *e != 0)
        {
            return Ok(None);
        }
    }
    let gap = polynomial(&(Atom::var(roles[1]) - Atom::var(roles[0])), &var_map).unwrap();
    let mut cleared = numerator.clone();
    for gap_power in 0..=2 {
        let (qa, ra) = cleared.quot_rem(&equations[0], false);
        let (qb, remainder) = ra.quot_rem(&equations[1], false);
        for p in [&qa, &ra, &qb, &remainder] {
            check(p, family, &budget)?;
        }
        if remainder.is_zero() && &(&qa * &equations[0]) + &(&qb * &equations[1]) == cleared {
            let mut polys = vec![
                numerator,
                denominator,
                product,
                multiple,
                gap,
                equations[0].clone(),
                equations[1].clone(),
                qa,
                qb,
            ];
            polys.extend(admitted);
            // Source role, rather than scope map order, binds each branch below.
            let branches = family
                .roots
                .iter()
                .map(|root| {
                    family
                        .scope
                        .owners()
                        .find_map(|(tag, p)| {
                            root.contains_symbol(tag)
                                .then(|| p.semantic_identity().to_owned())
                        })
                        .ok_or_else(|| invalid("coefficient-zero branch owner"))
                })
                .collect::<Result<Vec<_>>>()?;
            let mut receipt = Receipt {
                order,
                variables,
                atoms: polys.iter().map(Poly::to_expression).collect(),
                gap_power,
                branches: [branches[0].clone(), branches[1].clone()],
                inverse_units: inverses,
                digest: String::new(),
            };
            // Replay has fixed protocol caps, independently of caller discovery
            // limits. An identity exceeding them is an optional proof miss.
            let Some(digest) = validate(&receipt)? else {
                return Ok(None);
            };
            receipt.digest = digest;
            return Ok(Some(receipt));
        }
        if gap_power != 2 {
            pre_product(&cleared, &gap, family)?;
            cleared = &cleared * &gap;
            check(&cleared, family, &budget)?;
        }
    }
    Ok(None)
}

fn polynomial(atom: &Atom, variables: &Arc<Vec<PolyVariable>>) -> Option<Poly> {
    let p = atom
        .try_to_polynomial::<_, u32>(&Q, variables.clone())
        .ok()?;
    (p.variables() == variables).then_some(p)
}
fn check(p: &Poly, family: &NormalizedFamily, budget: &regular::Budget) -> Result<()> {
    budget.check()?;
    if p.nterms() > family.limits.density.terms
        || (0..p.nvars()).any(|i| p.degree(i) > u32::from(family.limits.density.degree))
    {
        return Err(Error::ResourceIncomplete(
            "local coefficient-zero polynomial support".into(),
        ));
    }
    Ok(())
}
fn pre_product(a: &Poly, b: &Poly, family: &NormalizedFamily) -> Result<()> {
    if a.nterms()
        .checked_mul(b.nterms())
        .is_none_or(|n| n > family.limits.density.terms)
    {
        return Err(Error::ResourceIncomplete(
            "local coefficient-zero product allowance".into(),
        ));
    }
    Ok(())
}

fn materialize(
    a: &Atom,
    family: &NormalizedFamily,
    definitions: &[Definition],
    roles: [Symbol; 2],
) -> Result<Option<Atom>> {
    let mut supported = true;
    let result = a.replace_map_bottom_up(|node, _, out| {
        if !supported {
            return;
        }
        if let Some(role) = family.roots.iter().position(|r| r.as_view() == node) {
            **out = Atom::var(roles[role]);
            return;
        }
        let AtomView::Fun(call) = node else { return };
        let args = call.iter().map(|a| a.to_owned()).collect::<Vec<_>>();
        let Some(definition) = definitions.iter().find(|d| {
            d.head == call.get_symbol()
                && args.len() == d.tags.len() + d.formals.len()
                && args.starts_with(&d.tags)
        }) else {
            supported = false;
            return;
        };
        **out = definition.body.replace_multiple(
            definition
                .formals
                .iter()
                .zip(&args[definition.tags.len()..])
                .map(|(s, a)| {
                    Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(a.clone()))
                }),
        );
    });
    Ok(supported.then_some(result))
}

/// Recheck bounded arithmetic and canonical role-index digest after symbol remap.
/// Fixed protocol caps protect local replay; this does not independently re-prove
/// the serialized nonzero unit authority or cap native internal allocation.
pub(crate) fn validate(r: &Receipt) -> Result<Option<String>> {
    replay::validate(r)
}
