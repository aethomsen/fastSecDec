//! Exact finite numerator faces only. Bulk numerator functions stay compact.
//! Ambient mixed derivatives are already present in `Definition::body` before
//! this native polynomial remainder is formed; these operations do not commute.
use super::*;
use crate::threshold::continued::Definition;
use std::collections::BTreeMap;
use symbolica::{
    atom::AtomView,
    domains::atom::AtomField,
    id::{Pattern, Replacement},
};

pub(crate) struct FaceReduction {
    pub chart: usize,
    pub definition: usize,
    pub role: usize,
    pub body: Atom,
    pub equation: Atom,
    pub quotient: Atom,
    pub remainder: Atom,
}

pub(super) fn reduce(
    family: &NormalizedFamily,
    charts: &mut [Atom],
    definitions: &[Definition],
) -> Result<Vec<FaceReduction>> {
    if family.roots.len() != 2 {
        return Ok(Vec::new());
    }
    let mut budget = regular::Budget::new(family.limits.geometry.clone());
    if let Some(cancel) = &family.limits.cancellation {
        budget = budget.with_cancellation(cancel.clone());
    }
    let n = family.formals.len();
    let variable = Atom::var(family.formals[n - 1]);
    let field = AtomField {
        statistical_zero_test: false,
        ..Default::default()
    };
    let mut receipts = Vec::new();
    let mut cache = BTreeMap::<(usize, usize), (Atom, Atom, Atom)>::new();
    for (chart, expression) in charts.iter_mut().enumerate() {
        let mut failure = None;
        *expression = expression.replace_map_bottom_up(|node, _, out| {
            if failure.is_some() {
                return;
            }
            let AtomView::Fun(call) = node else {
                return;
            };
            let args = call.iter().map(|a| a.to_owned()).collect::<Vec<_>>();
            let owned = call.get_symbol() == family.numerator
                || (call.get_symbol() == Symbol::DERIVATIVE
                    && args.get(n + 1) == Some(&Atom::var(family.numerator)));
            if !owned {
                return;
            }
            let Some((index, definition)) = definitions.iter().enumerate().find(|(_, d)| {
                d.head == call.get_symbol()
                    && d.tags.len() + n == args.len()
                    && args.starts_with(&d.tags)
            }) else {
                failure = Some(invalid("unregistered owned finite-face numerator call"));
                return;
            };
            let actual = &args[definition.tags.len()..];
            let Some(role) = family
                .roots
                .iter()
                .position(|root| Some(root) == actual.last())
            else {
                return;
            };
            // Exact issued-root equality binds its branch and current prefix.
            // Prefix face substitutions need a separate transported-root contract.
            if definition.formals != family.formals || actual[..n - 1] != family.arguments[..n - 1]
            {
                failure = Some(unsupported(
                    "finite root face requires its issued prefix arguments",
                ));
                return;
            }
            let calculation = (|| -> Result<Atom> {
                budget.check()?;
                let section = &family.charts[chart].geometry.sections()[role];
                if section.axis() != n - 1
                    || section.source().axes()[n - 1].symbol() != family.formals[n - 1]
                {
                    return Err(invalid("finite-face root coordinate association"));
                }
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    cache.entry((index, role))
                {
                    if receipts.len() >= family.limits.density.terms {
                        return Err(Error::ResourceIncomplete(
                            "finite-face cache allowance".into(),
                        ));
                    }
                    let equation = section.equation();
                    for a in [&definition.body, equation] {
                        meromorphic::preflight_expression(a, family.limits.density)
                            .map_err(meromorphic_error)?;
                    }
                    let body = definition.body.to_polynomial_in_vars_with_field::<u32>(
                        std::slice::from_ref(&variable),
                        &field,
                    );
                    let divisor = equation.to_polynomial_in_vars_with_field::<u32>(
                        std::slice::from_ref(&variable),
                        &field,
                    );
                    // Strictly lower degree has native quotient zero without
                    // dividing by a leading coefficient. Other reductions need
                    // an exact constant unit or a separate localization receipt.
                    if divisor.is_zero()
                        || divisor.degree(0) == 0
                        || (body.degree(0) >= divisor.degree(0)
                            && Rational::try_from(divisor.lcoeff().as_view()).is_err())
                    {
                        return Err(unsupported(
                            "finite-face divisor requires a nonzero rational leading coefficient",
                        ));
                    }
                    if body
                        .nterms()
                        .checked_mul(divisor.nterms())
                        .is_none_or(|v| v > family.limits.density.terms)
                        || body.degree(0) > u32::from(family.limits.density.degree)
                    {
                        return Err(Error::ResourceIncomplete(
                            "finite-face native remainder support bound".into(),
                        ));
                    }
                    let (quotient, remainder) = body.quot_rem(&divisor, false);
                    if quotient
                        .nterms()
                        .checked_add(remainder.nterms())
                        .is_none_or(|count| count > family.limits.density.terms)
                    {
                        return Err(Error::ResourceIncomplete(
                            "finite-face remainder output support".into(),
                        ));
                    }
                    if &(&quotient * &divisor) + &remainder != body {
                        return Err(invalid("native finite-face polynomial identity"));
                    }
                    let atom = |p: &symbolica::poly::polynomial::MultivariatePolynomial<
                        AtomField,
                        u32,
                    >| {
                        p.to_expression_with_coeff_map(|_, a, out| *out = a.clone())
                    };
                    let quotient = atom(&quotient);
                    let remainder = atom(&remainder);
                    for value in [&quotient, &remainder] {
                        meromorphic::preflight_expression(value, family.limits.density)
                            .map_err(meromorphic_error)?;
                    }
                    entry.insert((equation.clone(), quotient, remainder));
                }
                let (equation, quotient, remainder) = &cache[&(index, role)];
                if receipts.len() >= family.limits.density.terms {
                    return Err(Error::ResourceIncomplete(
                        "finite-face receipt inventory".into(),
                    ));
                }
                receipts.push(FaceReduction {
                    chart,
                    definition: index,
                    role,
                    body: definition.body.clone(),
                    equation: equation.clone(),
                    quotient: quotient.clone(),
                    remainder: remainder.clone(),
                });
                let value = remainder.replace_multiple(definition.formals.iter().zip(actual).map(
                    |(s, a)| {
                        Replacement::new(
                            Pattern::Literal(Atom::var(*s)),
                            Pattern::Literal(a.clone()),
                        )
                    },
                ));
                budget.check()?;
                Ok(value)
            })();
            match calculation {
                Ok(value) => **out = value,
                Err(error) => failure = Some(error),
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
    }
    Ok(receipts)
}
