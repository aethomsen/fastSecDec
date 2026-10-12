use super::*;
use crate::threshold::resolution::{Limits, Ring};
use std::sync::Arc;
use symbolica::{atom::Atom, symbol};
fn ring() -> Arc<Ring> {
    Arc::new(
        Ring::new(
            vec![
                symbol!("basis_test::x"),
                symbol!("basis_test::u"),
                symbol!("basis_test::z"),
                symbol!("basis_test::v"),
            ],
            vec![],
        )
        .unwrap(),
    )
}
fn p(r: &Ring, s: &str) -> Poly {
    r.atom(&Atom::parse(s, "basis_test", Default::default()).unwrap())
        .unwrap()
}
fn b() -> Budget {
    Budget::new(Limits::default())
}
#[test]
fn native_basis_zero_inputs_and_original_inclusion() {
    let r = ring();
    let f = p(&r, "basis_test::x");
    let zero = r.one().zero();
    let mut b = b();
    assert!(checked(&[], &mut b).unwrap().is_empty());
    assert!(
        checked(std::slice::from_ref(&zero), &mut b)
            .unwrap()
            .is_empty()
    );
    let out = checked(&[zero, f.clone()], &mut b).unwrap();
    assert!(f.reduce(&out).is_zero());
    assert!(matches!(
        verify(&[f], &[], &mut b),
        Err(Error::Invalid("native basis loses an input equation"))
    ));
}
#[test]
fn native_basis_original_f4_cache_regression_and_closure_check() {
    let r = ring();
    let input = [
        "basis_test::x^2-basis_test::x-basis_test::z",
        "basis_test::u-basis_test::x*basis_test::u-1",
        "2*basis_test::x*basis_test::v-basis_test::v-1",
    ]
    .map(|s| p(&r, s));
    let mut b = b();
    let out = checked(&input, &mut b).unwrap();
    assert!(
        p(&r, "(4*basis_test::z+1)*basis_test::v^2-1")
            .reduce(&out)
            .is_zero()
    );
    assert!(matches!(
        verify(&input, &input, &mut b),
        Err(Error::Invalid("native basis fails Buchberger closure"))
    ));
    let reproduced_incomplete = [
        "basis_test::x-1/2-2*basis_test::z*basis_test::v-basis_test::v/2",
        "basis_test::u*basis_test::z+2*basis_test::z*basis_test::v+basis_test::v/2+1/2",
        "basis_test::u*basis_test::v-basis_test::u-2*basis_test::v",
    ]
    .map(|s| p(&r, s));
    assert!(matches!(
        verify(&input, &reproduced_incomplete, &mut b),
        Err(Error::Invalid("native basis fails Buchberger closure"))
    ));
}
#[test]
fn native_basis_shape_and_spair_allocation_caps() {
    let r = ring();
    let foreign = Arc::new(Ring::new(vec![symbol!("basis_test::foreign")], vec![]).unwrap());
    let mut b = b();
    assert!(matches!(
        checked(&[r.one(), foreign.one()], &mut b),
        Err(Error::Invalid("native basis input variable maps"))
    ));
    let basis = [
        p(&r, "basis_test::x^2+basis_test::u^2"),
        p(&r, "basis_test::x*basis_test::u+1"),
    ];
    let mut cap = Budget::new(Limits {
        max_degree_per_axis: 2,
        ..Limits::default()
    });
    assert!(matches!(
        verify(&[], &basis, &mut cap),
        Err(Error::ResourceIncomplete("native S-polynomial degree"))
    ));
    let mut cap = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        verify(&[], &basis, &mut cap),
        Err(Error::ResourceIncomplete("operation budget"))
    ));
}

#[path = "membership_tests.rs"]
mod membership_tests;

#[test]
fn native_order_conversion_resolves_localized_unit_ideal_with_unused_roles() {
    let r = Arc::new(
        Ring::new(
            (0..9)
                .map(|i| symbol!(format!("unit_order::x{i}")))
                .collect(),
            vec![0],
        )
        .unwrap(),
    );
    let p = |s: &str| {
        r.atom(&Atom::parse(s, "unit_order", Default::default()).unwrap())
            .unwrap()
    };
    // Captured degree-five component call; x1,x5,x7 are the only active roles.
    let input = [
        "x1*(x1-1)*(x1^3-x1+1)",
        "x5*(1-x1+x1^2-x1^4)-1",
        "x7*(x1-x1^3)-1",
        "-12*x5*x7^2-3*x7^2*x1^2-4*x5-4*x5*x1+12*x7^2+3*x7^2*x1+4*x5*x7+4*x7",
    ]
    .map(p);
    let out = checked_via_fglm(&input, &mut b()).unwrap();
    assert_eq!(out, vec![r.one()]);
    assert_eq!(out[0].variables(), r.one().variables());
    assert!(input.iter().all(|p| p.reduce(&out).is_zero()));
}

#[test]
fn native_fglm_nontrivial_basis_preserves_requested_lex_order() {
    let r = Arc::new(
        Ring::new(
            vec![symbol!("basis_order::x"), symbol!("basis_order::y")],
            vec![],
        )
        .unwrap(),
    );
    let input = ["x^2-y", "y^2-2"].map(|s| {
        r.atom(&Atom::parse(s, "basis_order", Default::default()).unwrap())
            .unwrap()
    });
    let direct = checked_ordered(&input, &mut b()).unwrap();
    let converted = checked_via_fglm(&input, &mut b()).unwrap();
    assert_eq!(converted, direct);
    assert!(GroebnerBasis::is_groebner_basis(&converted));
    assert!(
        converted
            .iter()
            .any(|p| p.degree(0) == 0 && p.degree(1) > 0)
    );
}

#[test]
fn native_fglm_positive_dimensional_fallback_and_resource_admission() {
    let r = Arc::new(
        Ring::new(
            vec![symbol!("basis_bound::x"), symbol!("basis_bound::y")],
            vec![1],
        )
        .unwrap(),
    );
    let p = |s: &str| {
        r.atom(&Atom::parse(s, "basis_bound", Default::default()).unwrap())
            .unwrap()
    };
    let input = [p("x*y")];
    let grev = checked_ordered(
        &input
            .iter()
            .map(|p| p.reorder::<GrevLexOrder>())
            .collect::<Vec<_>>(),
        &mut b(),
    )
    .unwrap();
    assert_eq!(fglm_workspace(&grev, &mut b()).unwrap(), None);
    assert_eq!(
        checked_via_fglm(&input, &mut b()).unwrap(),
        checked_ordered(&input, &mut b()).unwrap()
    );
    let finite = [p("x^9"), p("y^9")];
    let mut capped = Budget::new(Limits {
        max_terms: 64,
        ..Limits::default()
    });
    assert!(matches!(
        checked_via_fglm(&finite, &mut capped),
        Err(Error::ResourceIncomplete("native FGLM workspace bound"))
    ));
    let mut capped = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        checked_via_fglm(&input, &mut capped),
        Err(Error::ResourceIncomplete("operation budget"))
    ));
    let mut capped = Budget::new(Limits {
        max_total_ideal_slots: 0,
        ..Limits::default()
    });
    assert!(matches!(
        checked_via_fglm(&input, &mut capped),
        Err(Error::ResourceIncomplete("total ideal allocation budget"))
    ));
}

#[test]
fn default_basis_preserves_direct_lex_budget_accounting() {
    let r = Arc::new(
        Ring::new(
            vec![symbol!("basis_default::x"), symbol!("basis_default::y")],
            vec![1],
        )
        .unwrap(),
    );
    let input = [r
        .atom(&Atom::parse("x*y", "basis_default", Default::default()).unwrap())
        .unwrap()];
    let mut baseline = b();
    let expected = checked_ordered(&input, &mut baseline).unwrap();
    let mut exact = Budget::new(Limits {
        max_operations: baseline.operations(),
        max_total_ideal_slots: baseline.ideal_slots(),
        ..Limits::default()
    });
    assert_eq!(checked(&input, &mut exact).unwrap(), expected);
    assert_eq!(exact.operations(), baseline.operations());
    assert_eq!(exact.ideal_slots(), baseline.ideal_slots());
}
