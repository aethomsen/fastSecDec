use super::{Error, Result, Work, native};
use symbolica::atom::{Atom, AtomCore, AtomView, Symbol};

/// Resource-only bounds for one selected source coordinate. These never supply
/// a retained degree or coefficient: the original Atom is collected natively.
/// Independent coefficient subtrees remain factored. General heap-power cases
/// are deliberately outside this sufficient class; see `power_bound`.
#[derive(Clone, Copy, Debug)]
struct CollectionBound {
    lower: usize,
    upper: usize,
    total_nodes: usize,
    coefficient_nodes: usize,
}
impl CollectionBound {
    fn support(self) -> Result<usize> {
        checked_add(
            self.upper
                .checked_sub(self.lower)
                .ok_or_else(|| Error::Invalid("univariate syntax interval".into()))?,
            1,
        )
    }
}
fn checked_add(a: usize, b: usize) -> Result<usize> {
    a.checked_add(b)
        .ok_or_else(|| Error::ResourceIncomplete("univariate bound overflow".into()))
}
fn checked_mul(a: usize, b: usize) -> Result<usize> {
    a.checked_mul(b)
        .ok_or_else(|| Error::ResourceIncomplete("univariate bound overflow".into()))
}
fn check_bound(bound: CollectionBound, work: &mut Work<'_>) -> Result<CollectionBound> {
    work.check()?;
    if bound.upper > work.limits.algebra.max_degree
        || bound.support()? > work.limits.algebra.max_terms
        || bound.coefficient_nodes > work.limits.max_coefficient_nodes
        || bound.total_nodes > checked_mul(bound.support()?, work.limits.max_coefficient_nodes)?
    {
        return Err(Error::ResourceIncomplete(
            "selected-coordinate collection bound".into(),
        ));
    }
    Ok(bound)
}
fn add_bound(
    a: CollectionBound,
    b: CollectionBound,
    work: &mut Work<'_>,
) -> Result<CollectionBound> {
    let lower = a.lower.min(b.lower);
    let upper = a.upper.max(b.upper);
    check_bound(
        CollectionBound {
            lower,
            upper,
            total_nodes: checked_add(
                checked_add(a.total_nodes, b.total_nodes)?,
                checked_add(upper - lower, 1)?,
            )?,
            coefficient_nodes: checked_add(
                checked_add(a.coefficient_nodes, b.coefficient_nodes)?,
                1,
            )?,
        },
        work,
    )
}
fn multiply_bound(
    a: CollectionBound,
    b: CollectionBound,
    pairs: &mut usize,
    work: &mut Work<'_>,
) -> Result<CollectionBound> {
    let sa = a.support()?;
    let sb = b.support()?;
    let count = checked_mul(sa, sb)?;
    *pairs = checked_add(*pairs, count)?;
    work.entries(*pairs)?;
    check_bound(
        CollectionBound {
            lower: checked_add(a.lower, b.lower)?,
            upper: checked_add(a.upper, b.upper)?,
            total_nodes: checked_add(
                checked_add(
                    checked_mul(sb, a.total_nodes)?,
                    checked_mul(sa, b.total_nodes)?,
                )?,
                checked_mul(2, count)?,
            )?,
            coefficient_nodes: checked_mul(
                sa.min(sb),
                checked_add(checked_add(a.coefficient_nodes, b.coefficient_nodes)?, 2)?,
            )?,
        },
        work,
    )
}
fn power_bound(
    b: CollectionBound,
    n: usize,
    pairs: &mut usize,
    work: &mut Work<'_>,
) -> Result<CollectionBound> {
    if n == 0 {
        return check_bound(
            CollectionBound {
                lower: 0,
                upper: 0,
                total_nodes: 1,
                coefficient_nodes: 1,
            },
            work,
        );
    }
    if n == 1 {
        return Ok(b);
    }
    let lower = checked_mul(b.lower, n)?;
    let upper = checked_mul(b.upper, n)?;
    // Native Atom integer powers can distribute through Mul. At most two
    // constructor/exponent nodes per source node are added; this is a node
    // estimate, not a bound on integer bit storage or native allocation.
    let leading = checked_add(checked_mul(3, b.coefficient_nodes)?, 2)?;
    if b.support()? == 1 {
        return check_bound(
            CollectionBound {
                lower,
                upper,
                total_nodes: leading,
                coefficient_nodes: leading,
            },
            work,
        );
    }
    // ae0 polynomial::pow uses heap_pow for n>10 or absolute degree<=2.
    // The actual degree may be smaller than the syntactic upper bound.
    let heap_possible = n > 10 || b.lower <= 2;
    if heap_possible && b.support()? != 2 {
        return Err(Error::Unsupported(
            "resource admission for a multi-term native heap power".into(),
        ));
    }
    let mut repeated = b;
    for _ in 1..n {
        repeated = multiply_bound(repeated, b, pairs, work)?;
    }
    if !heap_possible {
        return Ok(repeated);
    }
    // A consecutive binomial has one contribution per generated heap
    // coefficient and exactly n+1 possible coefficients. The next candidate
    // has zero integer multiplier; termination requires no symbolic zero
    // cancellation. General multi-term heap powers are refused above.
    let mut coefficient = leading;
    let mut total = leading;
    for _ in 0..n {
        *pairs = checked_add(*pairs, 1)?;
        work.entries(*pairs)?;
        // Include integer-scaled numerator, inverse of integer-scaled leading
        // coefficient (including native inverse-of-product normalization),
        // and the final quotient constructor.
        coefficient = checked_add(
            checked_add(coefficient, checked_mul(4, b.coefficient_nodes)?)?,
            12,
        )?;
        total = checked_add(total, coefficient)?;
        check_bound(
            CollectionBound {
                lower,
                upper,
                total_nodes: total,
                coefficient_nodes: coefficient,
            },
            work,
        )?;
    }
    *pairs = checked_add(*pairs, 1)?;
    work.entries(*pairs)?;
    check_bound(
        CollectionBound {
            lower,
            upper,
            total_nodes: total.max(repeated.total_nodes),
            coefficient_nodes: coefficient.max(repeated.coefficient_nodes),
        },
        work,
    )
}
fn expression_bound(
    node: AtomView<'_>,
    x: &Atom,
    depth: usize,
    pairs: &mut usize,
    work: &mut Work<'_>,
) -> Result<CollectionBound> {
    work.check()?;
    if depth > 64 {
        return Err(Error::ResourceIncomplete("univariate syntax depth".into()));
    }
    if !node.contains(x.as_view()) {
        let mut nodes = 0usize;
        let mut failure = None;
        node.visitor(&mut |_| {
            if let Err(e) = work.check() {
                failure = Some(e);
                return false;
            }
            match checked_add(nodes, 1) {
                Ok(n) if n <= work.limits.max_source_nodes => nodes = n,
                _ => {
                    failure = Some(Error::ResourceIncomplete(
                        "univariate coefficient source nodes".into(),
                    ));
                    return false;
                }
            }
            true
        });
        if let Some(e) = failure {
            return Err(e);
        }
        return check_bound(
            CollectionBound {
                lower: 0,
                upper: 0,
                total_nodes: nodes,
                coefficient_nodes: nodes,
            },
            work,
        );
    }
    match node {
        AtomView::Var(_) if node == x.as_view() => check_bound(
            CollectionBound {
                lower: 1,
                upper: 1,
                total_nodes: 1,
                coefficient_nodes: 1,
            },
            work,
        ),
        AtomView::Add(sum) => {
            let mut result = None;
            for term in sum {
                let bound = expression_bound(term, x, depth + 1, pairs, work)?;
                result = Some(match result {
                    None => bound,
                    Some(previous) => add_bound(previous, bound, work)?,
                });
            }
            result.ok_or_else(|| Error::Invalid("empty native sum".into()))
        }
        AtomView::Mul(product) => {
            let mut result = CollectionBound {
                lower: 0,
                upper: 0,
                total_nodes: 1,
                coefficient_nodes: 1,
            };
            for factor in product {
                let bound = expression_bound(factor, x, depth + 1, pairs, work)?;
                result = multiply_bound(result, bound, pairs, work)?;
            }
            Ok(result)
        }
        AtomView::Pow(power) => {
            let n = u64::try_from(power.get_exp())
                .map_err(|_| Error::Unsupported("non-polynomial source-coordinate power".into()))?;
            let n = usize::try_from(n).map_err(native)?;
            if n > work.limits.algebra.max_degree {
                return Err(Error::ResourceIncomplete("univariate source power".into()));
            }
            let base = expression_bound(power.get_base(), x, depth + 1, pairs, work)?;
            power_bound(base, n, pairs, work)
        }
        _ => Err(Error::Unsupported(
            "hidden source-coordinate dependence in numerator syntax".into(),
        )),
    }
}
pub(super) fn preflight(body: &Atom, coordinate: Symbol, work: &mut Work<'_>) -> Result<()> {
    work.nodes(body, work.limits.max_source_nodes)?;
    expression_bound(body.as_view(), &Atom::var(coordinate), 0, &mut 0, work)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::{AtomField, Budget, ControlFlow, Limits, degrees};
    use super::*;
    use symbolica::{function, symbol};

    fn collect(body: &Atom, coordinate: Symbol, limits: &Limits) -> Result<Vec<u32>> {
        let mut callback = || ControlFlow::Continue(());
        degrees(
            body,
            &[coordinate],
            &mut Work {
                limits,
                budget: Budget::new(limits.algebra.clone()),
                callback: &mut callback,
            },
        )
    }

    #[test]
    fn numerator_univariate_preflight_preserves_factored_coefficients() {
        let x = symbol!("univariate_resource_test::x");
        let coefficients = (0..4)
            .map(|i| {
                (0..8)
                    .map(|j| Atom::var(symbol!(format!("univariate_resource_test::c{i}_{j}"))))
                    .fold(Atom::zero(), |a, b| a + b)
            })
            .collect::<Vec<_>>();
        let body = (&coefficients[0] + Atom::var(x) * &coefficients[1]).pow(2)
            * (&coefficients[2] + Atom::var(x) * &coefficients[3]).pow(3);
        // The old general expanded multivariate estimate rejects this compact
        // univariate source although native collection has only six terms.
        use crate::threshold::regularization::meromorphic::{
            Limits as OldLimits, preflight_expression,
        };
        assert!(
            preflight_expression(
                &body,
                OldLimits {
                    degree: 256,
                    terms: 20_000,
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(collect(&body, x, &Limits::default()).unwrap(), [5]);
        let p = body.to_polynomial_in_vars_with_field::<u32>(
            &[Atom::var(x)],
            &AtomField {
                statistical_zero_test: false,
                ..Default::default()
            },
        );
        assert_eq!(p.nterms(), 6);
        let (a, b, c) = symbol!(
            "univariate_resource_test::a",
            "univariate_resource_test::b",
            "univariate_resource_test::c"
        );
        let repeated = (Atom::var(a) * Atom::var(x).pow(3)
            + Atom::var(b) * Atom::var(x).pow(4)
            + Atom::var(c) * Atom::var(x).pow(5))
        .pow(3);
        assert_eq!(collect(&repeated, x, &Limits::default()).unwrap(), [15]);
        let binomial = (Atom::var(a) * Atom::var(b) + Atom::var(c) * Atom::var(x)).pow(8);
        assert_eq!(
            collect(
                &binomial,
                x,
                &Limits {
                    max_coefficient_nodes: 64_000_000,
                    ..Default::default()
                }
            )
            .unwrap(),
            [8]
        );
    }

    #[test]
    fn numerator_univariate_preflight_refuses_hidden_and_unbounded_work() {
        let (x, a, b, c) = symbol!(
            "univariate_resource_test::x",
            "univariate_resource_test::a",
            "univariate_resource_test::b",
            "univariate_resource_test::c"
        );
        let heap =
            (Atom::var(a) + Atom::var(b) * Atom::var(x) + Atom::var(c) * Atom::var(x).pow(2))
                .pow(3);
        for body in [
            heap,
            function!(symbol!("univariate_resource_test::f"), Atom::var(x)),
            (Atom::one() + Atom::var(x)).pow(-1),
            Atom::var(x).pow(Atom::num((1, 2))),
        ] {
            assert!(matches!(
                collect(&body, x, &Limits::default()),
                Err(Error::Unsupported(_))
            ));
        }
        let body = (Atom::var(a) + Atom::var(x)).pow(3);
        assert!(matches!(
            collect(
                &body,
                x,
                &Limits {
                    max_entries: 0,
                    ..Default::default()
                }
            ),
            Err(Error::ResourceIncomplete(_))
        ));
        assert!(matches!(
            collect(
                &body,
                x,
                &Limits {
                    max_coefficient_nodes: 1,
                    ..Default::default()
                }
            ),
            Err(Error::ResourceIncomplete(_))
        ));
        assert!(matches!(
            checked_mul(usize::MAX, 2),
            Err(Error::ResourceIncomplete(_))
        ));
        let limits = Limits::default();
        let mut callback = || ControlFlow::Break(());
        assert!(matches!(
            degrees(
                &body,
                &[x],
                &mut Work {
                    limits: &limits,
                    budget: Budget::new(limits.algebra.clone()),
                    callback: &mut callback
                }
            ),
            Err(Error::Cancelled)
        ));
        assert_eq!(collect(&body, x, &limits).unwrap(), [3]);
    }
}
