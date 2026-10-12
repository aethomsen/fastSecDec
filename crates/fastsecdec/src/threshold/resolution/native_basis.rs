//! Native F4 with explicit Buchberger closure and original-generator inclusion.
//! Reverse inclusion relies on F4's constructive combinations. This helper does
//! not validate an arbitrary caller-supplied basis or expose alternative CAS.
use super::{Budget, Error, Poly};
use symbolica::{
    domains::rational::RationalField,
    poly::{
        GrevLexOrder, LexOrder, MonomialOrder, groebner::GroebnerBasis,
        polynomial::MultivariatePolynomial,
    },
};
type OrderedPoly<O> = MultivariatePolynomial<RationalField, u16, O>;
type Result<T> = std::result::Result<T, Error>;

pub(super) fn checked(equations: &[Poly], budget: &mut Budget) -> Result<Vec<Poly>> {
    checked_ordered(equations, budget)
}

/// Explicit native order-conversion strategy for quotient normalization.
/// Other consumers keep direct Lex and its existing budget accounting.
pub(super) fn checked_via_fglm(equations: &[Poly], budget: &mut Budget) -> Result<Vec<Poly>> {
    admit_inputs(equations, budget)?;
    if equations.iter().all(|p| p.is_zero()) {
        return checked_ordered(equations, budget);
    }
    // Preserve every coordinate and parameter role, including unused roles.
    // This order is only for native construction, never an elimination order.
    budget.reserve_slots(equations.len())?;
    let reordered = equations
        .iter()
        .map(|p| p.reorder::<GrevLexOrder>())
        .collect::<Vec<_>>();
    let grevlex = GroebnerBasis {
        system: checked_ordered(&reordered, budget)?,
        print_stats: false,
    };
    if fglm_workspace(&grevlex.system, budget)?.is_none() {
        // FGLM cannot convert positive-dimensional ideals. Retain the exact
        // original Lex call rather than treating a reordered basis as Lex.
        return checked_ordered(equations, budget);
    }
    budget.charge(1)?;
    let lex = grevlex
        .change_order::<LexOrder>()
        .map_err(|_| Error::Invalid("native FGLM order conversion"))?
        .system;
    admit_output(equations, &lex, budget)?;
    verify(equations, &lex, budget)?;
    // Native F4 is constructive; FGLM still receives independent two-way ideal
    // checks, in addition to closure in the requested original Lex order.
    for p in &lex {
        budget.charge(1)?;
        let remainder = p.reorder::<GrevLexOrder>().reduce(&grevlex.system);
        budget.poly(&remainder)?;
        if !remainder.is_zero() {
            return Err(Error::Invalid("native FGLM adds a foreign generator"));
        }
    }
    for p in &grevlex.system {
        budget.charge(1)?;
        let remainder = p.reorder::<LexOrder>().reduce(&lex);
        budget.poly(&remainder)?;
        if !remainder.is_zero() {
            return Err(Error::Invalid("native FGLM loses a basis generator"));
        }
    }
    Ok(lex)
}

// The native FGLM admission uses these same pure leading powers. We only
// preflight its finite quotient/workspace sizes; Symbolica owns conversion.
// A positive-dimensional leading ideal returns None and uses direct Lex.
// Native F4/FGLM internals still require caller-owned hard process limits.
fn fglm_workspace(
    basis: &[OrderedPoly<GrevLexOrder>],
    budget: &mut Budget,
) -> Result<Option<usize>> {
    budget.charge(1)?;
    if basis.iter().any(|p| !p.is_zero() && p.is_constant()) {
        budget.reserve_slots(1)?;
        return Ok(Some(1));
    }
    let Some(first) = basis.first() else {
        return Ok(None);
    };
    let variables = first.nvars();
    budget.reserve_slots(variables)?;
    let bounds = (0..variables)
        .map(|axis| {
            basis
                .iter()
                .filter_map(|p| {
                    let leading = p.max_exp();
                    (leading[axis] != 0
                        && leading
                            .iter()
                            .enumerate()
                            .all(|(i, e)| i == axis || *e == 0))
                    .then_some(usize::from(leading[axis]))
                })
                .min()
        })
        .collect::<Option<Vec<_>>>();
    let Some(bounds) = bounds else {
        return Ok(None);
    };
    let dimension = bounds
        .into_iter()
        .try_fold(1usize, |n, b| n.checked_mul(b))
        .ok_or(Error::ResourceIncomplete("native FGLM quotient bound"))?;
    // Conservative dense vector/cache-entry bound, not an exact native timer
    // or allocator count. Refuse before the new finite-dimensional operation.
    let entries = dimension
        .checked_mul(dimension)
        .and_then(|n| variables.checked_add(1).and_then(|v| n.checked_mul(v)))
        .ok_or(Error::ResourceIncomplete("native FGLM workspace bound"))?;
    if entries > budget.limits.max_terms {
        return Err(Error::ResourceIncomplete("native FGLM workspace bound"));
    }
    budget.reserve_slots(dimension)?;
    Ok(Some(dimension))
}

fn admit_inputs<O: MonomialOrder>(equations: &[OrderedPoly<O>], budget: &Budget) -> Result<()> {
    budget.count(equations.len())?;
    for f in equations {
        budget.poly(f)?;
        if equations
            .first()
            .is_some_and(|p| p.variables() != f.variables())
        {
            return Err(Error::Invalid("native basis input variable maps"));
        }
    }
    Ok(())
}
fn admit_output<O: MonomialOrder>(
    input: &[OrderedPoly<O>],
    output: &[OrderedPoly<O>],
    budget: &mut Budget,
) -> Result<()> {
    budget.reserve_slots(output.len())?;
    for f in output {
        budget.poly(f)?;
        if f.is_zero()
            || input
                .first()
                .is_some_and(|p| p.variables() != f.variables())
        {
            return Err(Error::Invalid("native basis output shape"));
        }
    }
    Ok(())
}
pub(super) fn checked_ordered<O: MonomialOrder>(
    equations: &[OrderedPoly<O>],
    budget: &mut Budget,
) -> Result<Vec<OrderedPoly<O>>> {
    admit_inputs(equations, budget)?;
    budget.reserve_slots(equations.len())?;
    // The adopted native F4 does not accept literal zero generators. The zero
    // ideal is represented by an empty input, which its public API supports.
    let input = equations
        .iter()
        .filter(|f| !f.is_zero())
        .cloned()
        .collect::<Vec<_>>();
    budget.charge(1)?;
    let result = GroebnerBasis::new(&input, false).system;
    admit_output(&input, &result, budget)?;
    verify(&input, &result, budget)?;
    Ok(result)
}
fn verify<O: MonomialOrder>(
    input: &[OrderedPoly<O>],
    basis: &[OrderedPoly<O>],
    budget: &mut Budget,
) -> Result<()> {
    let n = basis.len();
    let pairs = n
        .checked_mul(n.saturating_sub(1))
        .map(|v| v / 2)
        .ok_or(Error::ResourceIncomplete("native basis S-pair count"))?;
    budget.charge(pairs)?;
    // Bounds only the explicit S-polynomial construction. Native F4/reduction
    // internals remain subject to caller-owned hard process/RSS limits.
    for (i, p) in basis.iter().enumerate() {
        for q in &basis[i + 1..] {
            if p.nterms()
                .checked_add(q.nterms())
                .is_none_or(|v| v > budget.limits.max_terms)
            {
                return Err(Error::ResourceIncomplete("native S-polynomial terms"));
            }
            for axis in 0..p.nvars() {
                let lcm = p.max_exp()[axis].max(q.max_exp()[axis]);
                for f in [p, q] {
                    if f.degree(axis)
                        .checked_add(lcm - f.max_exp()[axis])
                        .is_none_or(|v| v > budget.limits.max_degree_per_axis)
                    {
                        return Err(Error::ResourceIncomplete("native S-polynomial degree"));
                    }
                }
            }
        }
    }
    if !GroebnerBasis::is_groebner_basis(basis) {
        return Err(Error::Invalid("native basis fails Buchberger closure"));
    }
    for f in input {
        budget.charge(1)?;
        let remainder = f.reduce(basis);
        budget.poly(&remainder)?;
        if !remainder.is_zero() {
            return Err(Error::Invalid("native basis loses an input equation"));
        }
    }
    Ok(())
}
#[cfg(test)]
#[path = "native_basis/tests.rs"]
mod tests;
