use super::super::*;
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore},
    domains::atom::AtomField,
    poly::groebner::GroebnerBasis,
    symbol,
    tensors::matrix::Matrix,
};

fn budget() -> Budget {
    Budget::new(Limits {
        max_operations: 2_000_000,
        max_total_ideal_slots: 2_000_000,
        ..Limits::default()
    })
}
fn ring() -> Arc<Ring> {
    Arc::new(
        Ring::new(
            vec![
                symbol!("next_api::t"),
                symbol!("next_api::p"),
                symbol!("next_api::x"),
                symbol!("next_api::y"),
                symbol!("next_api::z"),
                symbol!("next_api::w"),
                symbol!("next_api::r"),
                symbol!("next_api::iz"),
                symbol!("next_api::id"),
                symbol!("next_api::unused"),
            ],
            vec![1],
        )
        .unwrap(),
    )
}
fn p(r: &Ring, s: &str) -> Poly {
    r.atom(&Atom::parse(s, "next_api", Default::default()).unwrap())
        .unwrap()
}
fn ideal(r: &Arc<Ring>, ss: &[&str], b: &mut Budget) -> Ideal {
    Ideal::new(r.clone(), ss.iter().map(|s| p(r, s)).collect(), b).unwrap()
}
fn equal(a: &Ideal, c: &Ideal, relations: &[Poly], b: &mut Budget) -> bool {
    a.generators()
        .iter()
        .all(|f| c.contains(f, relations, b).unwrap())
        && c.generators()
            .iter()
            .all(|f| a.contains(f, relations, b).unwrap())
}
fn frame(
    r: &Arc<Ring>,
    eq: &[&str],
    axes: Vec<usize>,
    dependent: Vec<usize>,
    free: Vec<usize>,
    b: &mut Budget,
) -> Arc<EtaleFrame> {
    let i = ideal(r, eq, b);
    let n = i.generators().len();
    let local = LocalizedAlgebra::new(
        i,
        axes,
        vec![Guard {
            factor: p(r, "z"),
            inverse_axis: 7,
        }],
        b,
    )
    .unwrap();
    Arc::new(
        EtaleCertificate {
            source: local,
            equations: (0..n).collect(),
            dependent_axes: dependent,
            free_axes: free,
            determinant_inverse_axis: Some(8),
        }
        .verify(b)
        .unwrap(),
    )
}
// Focused direct native capability probe, not a production receipt constructor.
fn saturation(local: &LocalizedAlgebra, input: &Ideal, e: &Poly, b: &mut Budget) -> Ideal {
    let r = local.ring();
    assert!(
        local
            .ideal()
            .generators()
            .iter()
            .chain(local.unit_relations())
            .chain(input.generators())
            .all(|f| f.degree(0) == 0)
    );
    let mut equations = local
        .ideal()
        .generators()
        .iter()
        .chain(local.unit_relations())
        .chain(input.generators())
        .cloned()
        .collect::<Vec<_>>();
    equations.push(r.coordinate(0).unwrap() * e - r.one());
    equations.retain(|f| !f.is_zero());
    let gb = GroebnerBasis::new(&equations, false).reduce_basis();
    assert!(GroebnerBasis::is_groebner_basis(&gb.system));
    assert!(equations.iter().all(|f| f.reduce(&gb.system).is_zero()));
    Ideal::new(
        r.clone(),
        gb.system.into_iter().filter(|f| f.degree(0) == 0).collect(),
        b,
    )
    .unwrap()
}
fn snc(f: Arc<EtaleFrame>, e: Poly, b: &mut Budget) -> Arc<VerifiedRelativeSnc> {
    match verify_initial_relative_snc(
        f,
        vec![InitialDivisor {
            id: BoundaryId(0),
            equation: e,
        }],
        b,
    )
    .unwrap()
    {
        SncProduction::Verified(v) => v,
        other => panic!("SNC {other:?}"),
    }
}

mod continuation;
mod geometry;

mod cycle;

pub(crate) mod nonmonomial_probe;

mod map_probe;

mod problem_probe;

mod nested_probe;
pub(crate) mod original_probe;
