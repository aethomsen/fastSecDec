use super::*;
use crate::threshold::maps::endpoint_transport::{Budget, Limits};
use std::ops::ControlFlow;
use symbolica::symbol;

#[test]
fn full_measure_cancels_common_polynomial_keeps_extra_j_and_net_orders() {
    let (u, v) = symbol!("measure_canonical::u", "measure_canonical::v");
    let units = [u, v];
    let ring = Ring::new(units.to_vec()).unwrap();
    let limits = Limits::default();
    let mut callback = || ControlFlow::Continue(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    let x = Atom::var(u);
    let y = Atom::var(v);
    let common = (Atom::one() - &x / Atom::num(2)).pow(2);
    let original_n = x.pow(3) * &common * (Atom::one() + &y);
    let d = x.pow(2) * &common * (Atom::num(2) - &y);
    let extra_j = Atom::num(3) * x.pow(2);
    let n = original_n * &extra_j;
    let plain = ring.extract(&n, &d, &units, &mut work).unwrap().0;
    let issued = ring.extract_measure(&n, &d, &units, &mut work).unwrap();
    assert_eq!(plain.orders(), issued.orders());
    assert_eq!(
        issued.orders(),
        &[
            symbolica::domains::integer::Integer::from(3),
            symbolica::domains::integer::Integer::from(0)
        ]
    );
    let actual = issued.numerator() / issued.denominator();
    let expected = Atom::num(3) * (Atom::one() + &y) / (Atom::num(2) - &y);
    let ratio = (actual - expected)
        .try_to_rational_polynomial::<_, _, u32>(&Q, &Z, ring.variables.clone())
        .unwrap();
    assert!(ratio.numerator.is_zero());
    assert_ne!(plain.numerator(), issued.numerator());
    // The native fraction's leading-coefficient convention must not turn the
    // positive denominator 2-v into v-2 in the issued presentation.
    assert!(
        ring.polynomial(issued.denominator(), &mut work)
            .unwrap()
            .evaluate(
                Clone::clone,
                &[Rational::from((1, 2)), Rational::from((1, 2))]
            )
            > 0
    );
}

#[test]
fn canonical_measure_refuses_zero_cancel_and_bounded_conversion() {
    let u = symbol!("measure_canonical_refusal::u");
    let ring = Ring::new(vec![u]).unwrap();
    let limits = Limits::default();
    let mut callback = || ControlFlow::Continue(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    assert!(matches!(
        ring.extract_measure(&Atom::one(), &Atom::zero(), &[u], &mut work),
        Err(Error::Invalid(_))
    ));
    let mut callback = || ControlFlow::Break(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    assert!(matches!(
        ring.extract_measure(&Atom::one(), &Atom::one(), &[u], &mut work),
        Err(Error::Cancelled)
    ));
    let mut limits = limits;
    limits.algebra.max_terms = 0;
    let mut callback = || ControlFlow::Continue(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    assert!(matches!(
        ring.extract_measure(&Atom::var(u), &Atom::one(), &[u], &mut work),
        Err(Error::ResourceIncomplete(_))
    ));
}

#[test]
fn measure_identity_products_obey_native_preconversion_term_allowance() {
    let (u, v) = symbol!("measure_product_guard::u", "measure_product_guard::v");
    let ring = Ring::new(vec![u, v]).unwrap();
    let mut limits = Limits::default();
    limits.algebra.max_terms = 3;
    let mut callback = || ControlFlow::Continue(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    let numerator = Atom::one() + Atom::var(u);
    let denominator = Atom::num(2) + Atom::var(v);
    // Each input and reduced output has two terms; their four-term exact
    // identity product must refuse before native polynomial collection.
    assert!(matches!(
        ring.extract_measure(&numerator, &denominator, &[u, v], &mut work),
        Err(Error::ResourceIncomplete(reason)) if reason.contains("transport rational preflight")
    ));
}
