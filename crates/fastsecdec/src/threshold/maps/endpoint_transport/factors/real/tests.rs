use super::*;
use crate::threshold::maps::regular::Limits;
use std::sync::{Arc, atomic::AtomicBool};
use symbolica::symbol;

#[test]
fn native_smaller_margin_preserves_closed_unit_and_replay() {
    let x = symbol!("factor_margin_test::x");
    let budget = Budget::new(Limits::default());
    let domain =
        ProofBox::from_bounds(vec![x], vec![[Rational::one(), Rational::from(2)]], &budget)
            .unwrap();
    // There is no positive constant coefficient. The old fallback is larger
    // than the complete range, although this is a uniformly positive factor.
    let p = Atom::var(x) * Atom::num(Rational::from((1_u64, 1_u64 << 30)));
    let fallback = Rational::from((1_u64, 1_u64 << 20));
    let (ring, constraints, native) = domain.native.as_ref().unwrap();
    let polynomial = ring.polynomial(&p, &budget).unwrap();
    assert!(
        positive(
            &ring.native,
            &(&polynomial - &polynomial.constant(fallback.clone())),
            constraints,
            native,
            &budget,
        )
        .unwrap()
        .is_none()
    );
    let proof = domain.propose(&p, &fallback, &budget).unwrap().unwrap();
    assert_eq!(proof.margin(), &Rational::from((1_u64, 1_u64 << 32)));
    proof.replay(&domain, &budget).unwrap();
    let changed =
        ProofBox::from_bounds(vec![x], vec![[Rational::zero(), Rational::one()]], &budget).unwrap();
    assert!(proof.replay(&changed, &budget).is_err());
}

#[test]
fn margin_search_keeps_zero_misses_and_resource_errors_distinct() {
    let x = symbol!("factor_margin_test::x");
    let budget = Budget::new(Limits::default());
    let domain =
        ProofBox::from_bounds(vec![x], vec![[Rational::zero(), Rational::one()]], &budget).unwrap();
    let fallback = Rational::from((1, 2));
    assert!(
        domain
            .propose(&Atom::var(x), &fallback, &budget)
            .unwrap()
            .is_none()
    );
    assert!(
        domain
            .propose(&Atom::zero(), &fallback, &budget)
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        domain.propose(&Atom::one(), &Rational::zero(), &budget),
        Err(Error::Invalid(_))
    ));
    let tight = Budget::new(Limits {
        max_rational_bits: 4,
        ..Limits::default()
    });
    assert!(matches!(
        domain.propose(&Atom::var(x), &fallback, &tight),
        Err(Error::ResourceIncomplete(_))
    ));
    let cancelled =
        Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)));
    assert!(matches!(
        domain.propose(&Atom::one(), &fallback, &cancelled),
        Err(Error::Cancelled)
    ));
    // Zero-dimensional positive constants retain the native Rational path.
    let point = ProofBox::from_bounds(vec![], vec![], &budget).unwrap();
    let p = Atom::num(Rational::from((1_u64, 1_u64 << 40)));
    let proof = point.propose(&p, &fallback, &budget).unwrap().unwrap();
    assert!(matches!(proof.evidence(), MarginEvidence::Constant(_)));
    proof.replay(&point, &budget).unwrap();
}
