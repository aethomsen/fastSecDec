//! The existing continued::Definition convention; only demanded native source
//! partials are generated. Geometry never becomes a source formal.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
use symbolica::atom::AtomView;
use symbolica::domains::rational::Rational;

fn occupied(catalogue: &SourceNumerators, chart: &IssuedEndpointChart) -> BTreeSet<Symbol> {
    catalogue
        .records()
        .iter()
        .flat_map(|r| r.body().get_all_symbols(true))
        .chain(chart.target_units().iter().copied())
        .chain(
            chart
                .parent_prefix()
                .source()
                .unit_coordinates()
                .iter()
                .copied(),
        )
        .chain(
            chart
                .transport()
                .root_relations()
                .iter()
                .map(|r| r.section().root_symbol()),
        )
        .chain(catalogue.preparation().coordinates().iter().copied())
        .chain(
            chart
                .transport()
                .root_relations()
                .iter()
                .flat_map(|r| r.section().chart().parameters.iter().copied()),
        )
        .chain(
            chart
                .parent_prefix()
                .source()
                .decomposition()
                .occupied_symbols()
                .iter()
                .copied(),
        )
        .collect()
}
fn fresh(catalogue: &SourceNumerators, chart: &IssuedEndpointChart, suffix: &str) -> Symbol {
    let used = occupied(catalogue, chart);
    (0usize..)
        .map(|i| symbolica::symbol!(format!("fastsecdec::normal_jet_{i}::{suffix}")))
        .find(|s| !used.contains(s))
        .expect("finite occupied symbol inventory")
}
pub(super) fn fresh_head(c: &SourceNumerators, v: &IssuedEndpointChart) -> Symbol {
    fresh(c, v, "numerator")
}
pub(super) fn fresh_root_tag(c: &SourceNumerators, v: &IssuedEndpointChart, i: usize) -> Symbol {
    fresh(c, v, &format!("root_{i}"))
}
fn integer(a: AtomView<'_>) -> Option<usize> {
    Rational::try_from(a)
        .ok()
        .filter(|r| r.is_integer())
        .and_then(|r| r.numerator_ref().to_i64())
        .and_then(|n| usize::try_from(n).ok())
}
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Signature {
    pub record: usize,
    pub orders: Vec<usize>,
}
/// Return the exact tag signature and coordinate arguments of an owned call.
/// Native selected-root DERIVATIVE calls are intentionally not source calls.
pub(super) fn source_call(
    node: AtomView<'_>,
    head: Symbol,
    n: usize,
) -> Result<Option<(Signature, Vec<Atom>)>> {
    let AtomView::Fun(f) = node else {
        return Ok(None);
    };
    let args = f.iter().collect::<Vec<_>>();
    if f.get_symbol() == head {
        if args.len() != n + 1 {
            return Err(Error::Invalid("owned numerator call arity".into()));
        }
        return Ok(Some((
            Signature {
                record: integer(args[0])
                    .ok_or_else(|| Error::Invalid("owned numerator record".into()))?,
                orders: vec![0; n + 1],
            },
            args[1..].iter().map(|x| x.to_owned()).collect(),
        )));
    }
    if f.get_symbol() != Symbol::DERIVATIVE
        || args.len() != 2 * (n + 1) + 1
        || args[n + 1] != Atom::var(head).as_view()
    {
        return Ok(None);
    }
    let orders = args[..n + 1]
        .iter()
        .map(|a| integer(*a))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| Error::Invalid("owned source derivative orders".into()))?;
    if orders[0] != 0 {
        return Err(Error::Invalid("source record tag derivative".into()));
    }
    Ok(Some((
        Signature {
            record: integer(args[n + 2])
                .ok_or_else(|| Error::Invalid("source derivative record".into()))?,
            orders,
        },
        args[n + 3..].iter().map(|x| x.to_owned()).collect(),
    )))
}
pub(super) fn collect(
    catalogue: &SourceNumerators,
    head: Symbol,
    coefficients: &[NormalCoefficient],
    depth: u32,
    work: &mut Work<'_>,
) -> Result<Vec<Definition>> {
    let formals = catalogue.preparation().coordinates();
    let mut demands = BTreeSet::new();
    for coefficient in coefficients {
        for value in &coefficient.values {
            let mut failure = None;
            value.visitor(&mut |node| {
                if failure.is_some() {
                    return false;
                }
                if let Err(e) = work.check() {
                    failure = Some(e);
                    return false;
                }
                match source_call(node, head, formals.len()) {
                    Ok(Some((s, _))) => {
                        demands.insert(s);
                    }
                    Err(e) => failure = Some(e),
                    _ => {}
                }
                failure.is_none()
            });
            if let Some(e) = failure {
                return Err(e);
            }
            work.entries(demands.len())?;
        }
    }
    let mut out = Vec::new();
    for signature in demands {
        work.check()?;
        let record = catalogue
            .records()
            .get(signature.record)
            .ok_or_else(|| Error::Invalid("foreign source derivative record".into()))?;
        let sum = signature
            .orders
            .iter()
            .try_fold(0usize, |a, b| a.checked_add(*b))
            .ok_or_else(|| Error::ResourceIncomplete("source derivative order overflow".into()))?;
        if sum >= depth as usize || record.degrees().is_none() {
            return Err(Error::Invalid(
                "source derivative demand exceeds issued jet".into(),
            ));
        }
        let mut body = record.body().clone();
        for (&formal, &order) in formals.iter().zip(&signature.orders[1..]) {
            for _ in 0..order {
                work.nodes(&body, work.limits.max_source_nodes)?;
                body = body.derivative(formal);
                work.nodes(&body, work.limits.max_coefficient_nodes)?;
            }
        }
        let derivative = sum > 0;
        out.push(Definition {
            head: if derivative { Symbol::DERIVATIVE } else { head },
            tags: if derivative {
                signature
                    .orders
                    .iter()
                    .copied()
                    .map(Atom::num)
                    .chain([Atom::var(head), Atom::num(signature.record)])
                    .collect()
            } else {
                vec![Atom::num(signature.record)]
            },
            formals: formals.to_vec(),
            body,
            orders: derivative.then_some(signature.orders),
            always_inline: true,
        });
    }
    Ok(out)
}
pub(super) fn definition_index(owner: &NormalNumeratorJets) -> Result<BTreeMap<Signature, usize>> {
    owner
        .definitions
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let record = if d.orders.is_some() {
                d.tags.last()
            } else {
                d.tags.first()
            }
            .and_then(|a| integer(a.as_view()))
            .ok_or_else(|| Error::Invalid("owned definition tags".into()))?;
            Ok((
                Signature {
                    record,
                    orders: d
                        .orders
                        .clone()
                        .unwrap_or_else(|| vec![0; d.formals.len() + 1]),
                },
                i,
            ))
        })
        .collect()
}
