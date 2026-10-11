//! Native Taylor arithmetic after finite-argument/selected-branch admission.
//! Root and source-argument jets are shared; original numerator bodies stay out
//! of this module. This adds no face, minor or denominator certificate.
use super::*;
use crate::threshold::maps::endpoint_transport::clearing::pull;
use std::collections::BTreeSet;
use symbolica::{
    atom::{AtomView, FunctionBuilder},
    domains::integer::Integer,
    id::Pattern,
    poly::series::SeriesDepth,
};

pub(super) struct Template {
    record: Symbol,
    coefficients: Vec<Atom>,
}
impl Template {
    pub(super) fn record(&self, record: usize, work: &mut Work<'_>) -> Result<Vec<Atom>> {
        self.coefficients
            .iter()
            .map(|coefficient| {
                work.check()?;
                let value = pull(coefficient, &[self.record], &[Atom::num(record)]);
                work.nodes(&value, work.limits.max_coefficient_nodes)?;
                Ok(value)
            })
            .collect()
    }
}

fn truncate(a: &Atom, normal: Symbol, depth: u32, work: &mut Work<'_>) -> Result<Atom> {
    work.nodes(a, work.limits.max_coefficient_nodes)?;
    if !a.contains_symbol(normal) {
        return Ok(a.clone());
    }
    let jet = a
        .series(normal, 0, SeriesDepth::absolute(i64::from(depth) - 1))
        .map_err(native)?
        .to_atom();
    work.nodes(&jet, work.limits.max_coefficient_nodes)?;
    Ok(jet)
}

// Replace outer calls first: later roots contain earlier native root calls.
fn replace_roots(a: &Atom, calls: &[Atom], jets: &[Atom], work: &mut Work<'_>) -> Result<Atom> {
    if calls.len() != jets.len() {
        return Err(Error::Invalid("triangular root jet inventory".into()));
    }
    let mut out = a.clone();
    for (call, jet) in calls.iter().zip(jets).rev() {
        work.check()?;
        out = out
            .replace(Pattern::Literal(call.clone()))
            .with(Pattern::Literal(jet.clone()));
        work.nodes(&out, work.limits.max_coefficient_nodes)?;
    }
    Ok(out)
}

fn arguments(
    admitted: &admission::Admission,
    normal: Symbol,
    depth: u32,
    work: &mut Work<'_>,
) -> Result<Vec<Atom>> {
    let mut root_jets = Vec::with_capacity(admitted.root_calls.len());
    for (i, call) in admitted.root_calls.iter().enumerate() {
        let input = replace_roots(call, &admitted.root_calls[..i], &root_jets, work)?;
        let AtomView::Fun(root) = input.as_view() else {
            return Err(Error::Invalid("admitted native root call".into()));
        };
        let args = root
            .iter()
            .map(|a| truncate(&a.to_owned(), normal, depth, work))
            .collect::<Result<Vec<_>>>()?;
        let mut derivative = FunctionBuilder::new(root.get_symbol())
            .add_args(&args)
            .finish();
        work.nodes(&derivative, work.limits.max_coefficient_nodes)?;
        let mut jet = Atom::zero();
        for order in 0..depth {
            work.check()?;
            if order != 0 {
                // Keep the native implicit derivative hook. In particular,
                // avoid collect_symbol(ROOT_V3) on its rational derivatives.
                derivative = derivative.derivative(normal);
                work.nodes(&derivative, work.limits.max_coefficient_nodes)?;
            }
            let coefficient = pull(&derivative, &[normal], &[Atom::zero()])
                / Atom::num(Integer::factorial(order));
            work.nodes(&coefficient, work.limits.max_coefficient_nodes)?;
            jet += coefficient * Atom::var(normal).pow(i64::from(order));
            work.nodes(&jet, work.limits.max_coefficient_nodes)?;
        }
        root_jets.push(jet);
    }
    admitted
        .images
        .iter()
        .map(|image| {
            let input = replace_roots(image, &admitted.root_calls, &root_jets, work)?;
            truncate(&input, normal, depth, work)
        })
        .collect()
}

pub(super) fn prepare(
    catalogue: &SourceNumerators,
    chart: &IssuedEndpointChart,
    admitted: &admission::Admission,
    head: Symbol,
    normal: Symbol,
    depth: u32,
    work: &mut Work<'_>,
) -> Result<Template> {
    // Reserve the entire owner inventory, including unused source roles and
    // root metadata tags. The template tag is constant with respect to normal.
    let mut occupied: BTreeSet<_> = chart
        .parent_prefix()
        .source()
        .decomposition()
        .occupied_symbols()
        .clone();
    occupied.extend(catalogue.preparation().coordinates().iter().copied());
    occupied.extend(chart.target_units().iter().copied());
    occupied.extend(
        chart
            .parent_prefix()
            .source()
            .unit_coordinates()
            .iter()
            .copied(),
    );
    occupied.extend(
        catalogue
            .records()
            .iter()
            .flat_map(|r| r.body().get_all_symbols(true)),
    );
    occupied.extend(
        admitted
            .sections
            .iter()
            .flat_map(|s| s.section().chart().parameters.iter().copied()),
    );
    occupied.extend(
        admitted
            .images
            .iter()
            .chain(&admitted.root_calls)
            .flat_map(|a| a.get_all_symbols(true)),
    );
    occupied.extend([head, normal]);
    let record = (0usize..)
        .map(|i| symbolica::symbol!(format!("fastsecdec::normal_series_{i}::record")))
        .find(|s| !occupied.contains(s) && s.is_exportable())
        .expect("finite occupied symbol inventory");
    let images = arguments(admitted, normal, depth, work)?;
    let mut args = vec![Atom::var(record)];
    args.extend(images);
    let call = FunctionBuilder::new(head).add_args(&args).finish();
    work.nodes(&call, work.limits.max_coefficient_nodes)?;
    // Native outer composition still supplies every mixed source partial and
    // factorial. A constant symbolic record tag shares this geometry across N.
    let series = call
        .series(normal, 0, SeriesDepth::absolute(i64::from(depth) - 1))
        .map_err(native)?;
    let coefficients = (0..depth)
        .map(|order| {
            let value = series.coefficient(order.into()).unwrap_or_else(Atom::zero);
            work.nodes(&value, work.limits.max_coefficient_nodes)?;
            if value.contains_symbol(normal) {
                return Err(Error::Invalid("Taylor coefficient retains normal".into()));
            }
            Ok(value)
        })
        .collect::<Result<_>>()?;
    Ok(Template {
        record,
        coefficients,
    })
}
