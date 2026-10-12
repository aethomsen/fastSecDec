use super::*;
use crate::threshold::maps::normalized::Limits;
use std::sync::atomic::AtomicBool;
use symbolica::symbol;

pub(super) fn fixture() -> Arc<AffinePrefixCover> {
    let (first, _) = super::super::tests::fixture();
    AffinePrefixCover::prepare_next_bound(
        first,
        SectionSide::Upper,
        &DecompositionOptions::default(),
        &mut Budget::new(Limits::default()),
    )
    .unwrap()
}

#[test]
fn genuine_prefix_complete_fan_full_cubic_content_and_selected_witnesses() {
    let cover = fixture();
    let mut budget = Budget::new(Limits::default());
    assert_eq!(cover.parent().len(), 1);
    assert_eq!(cover.half_counts(), &[1, 1]);
    assert_eq!(cover.leaves().len(), 2);
    assert!(transport::complete_inventory(
        1,
        cover.half_counts(),
        cover.leaves()
    ));
    assert_eq!(
        cover.next_bound(),
        cover.parent().source().native_axes()[1]
            .upper
            .as_ref()
            .unwrap()
    );
    assert_eq!(cover.dependent_symbols().len(), 1);
    assert_eq!(cover.relations().len(), 1);
    for (i, point) in [Rational::from((1, 64)), Rational::from((1, 16))]
        .into_iter()
        .enumerate()
    {
        let leaf = cover.leaf(i).unwrap();
        let witness = leaf
            .interior_witness(vec![point.clone()], &mut budget)
            .unwrap();
        assert_eq!(witness.source_prefix(), &[point]);
        assert!(Arc::ptr_eq(witness.leaf().cover(), &cover));
        let equation = leaf
            .prepare_equation(Rational::from((1, 1i64 << 40)), &mut budget)
            .unwrap();
        assert_eq!(
            equation.coefficients().len(),
            4,
            "native degree-generic cubic equation retained"
        );
        assert!(equation.leading_unit().margin() > &Rational::zero());
        let mut roles = cover.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>();
        roles.push(cover.root_symbol());
        let ring = algebra::Ring::new(roles).unwrap();
        assert!(
            ring.polynomial(
                &(equation.total_pullback() - equation.common_content() * equation.equation()),
                &budget
            )
            .unwrap()
            .is_zero()
        );
        assert!(!equation.divisor_units().is_empty());
    }
}

#[test]
fn genuine_prefix_missing_children_wrong_divisor_roles_cancel_and_limits() {
    let cover = fixture();
    let parent = cover.parent().clone();
    let mut owned = Arc::try_unwrap(cover).ok().unwrap();
    owned.leaves.pop();
    assert!(!transport::complete_inventory(
        parent.len(),
        &owned.half_counts,
        &owned.leaves
    ));
    let mut b = Budget::new(Limits::default());
    let bad = FactoredDivisorProposal {
        target: DivisorTarget::Leading,
        coefficient: Rational::one(),
        factors: vec![Atom::var(symbol!("prefix_cover_bad::foreign"))],
    };
    assert!(matches!(
        AffinePrefixCover::prepare_with_divisors(
            parent.clone(),
            SectionSide::Upper,
            &[bad],
            &DecompositionOptions::default(),
            &mut b
        ),
        Err(Error::Invalid(_))
    ));
    let bad = FactoredDivisorProposal {
        target: DivisorTarget::Discriminant,
        coefficient: Rational::one(),
        factors: vec![Atom::one()],
    };
    assert!(
        AffinePrefixCover::prepare_with_divisors(
            parent.clone(),
            SectionSide::Upper,
            &[bad],
            &DecompositionOptions::default(),
            &mut b
        )
        .is_err()
    );
    let mut cancelled =
        Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)));
    assert!(matches!(
        AffinePrefixCover::prepare_next_bound(
            parent.clone(),
            SectionSide::Upper,
            &DecompositionOptions::default(),
            &mut cancelled
        ),
        Err(Error::Cancelled)
    ));
    let options = DecompositionOptions {
        max_sectors: 1,
        ..Default::default()
    };
    assert!(matches!(
        AffinePrefixCover::prepare_next_bound(parent, SectionSide::Upper, &options, &mut b),
        Err(Error::ResourceIncomplete(_))
    ));
}

#[test]
fn genuine_source_witness_cannot_be_reused_in_another_leaf() {
    let cover = fixture();
    let mut b = Budget::new(Limits::default());
    assert!(
        cover
            .leaf(1)
            .unwrap()
            .interior_witness(vec![Rational::from((1, 64))], &mut b)
            .is_err()
    );
    assert!(
        cover
            .leaf(0)
            .unwrap()
            .interior_witness(vec![Rational::from((1, 16))], &mut b)
            .is_err()
    );
    assert!(
        cover
            .leaf(0)
            .unwrap()
            .interior_witness(vec![Rational::one()], &mut b)
            .is_err()
    );
    assert!(
        cover
            .leaf(0)
            .unwrap()
            .prepare_equation(Rational::zero(), &mut b)
            .is_err()
    );
    assert!(cover.leaf(2).is_err());
}

#[test]
fn original_positive_unit_is_not_lost_by_independent_box_reduction() {
    let cover = fixture();
    let roles = cover.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>();
    let a = cover.dependent_symbols()[0];
    let axis = roles.iter().position(|s| *s == a).unwrap();
    let ring = algebra::Ring::new(roles.clone()).unwrap();
    let relations = cover
        .relations()
        .iter()
        .map(|r| (r.dependent(), r.equation().clone()))
        .collect::<Vec<_>>();
    let bounds = cover
        .slots()
        .iter()
        .map(|(_, b)| b.clone())
        .collect::<Vec<_>>();
    let b = Budget::new(Limits::default());
    let (constraints, domain) = ring.box_domain(&bounds, &b).unwrap();
    // Genuine selected A=1/sqrt(128). The reduced affine representative has
    // an unrelated negative corner in the independent enclosing A interval.
    let original = (Atom::var(a) - Atom::num((1, 10))).pow(2) + Atom::num((1, 10000));
    let (reduced, _) = transport::reduce(&ring, &roles, &original, &relations, &b).unwrap();
    let reduced = ring
        .polynomial(&reduced, &b)
        .unwrap()
        .replace(axis, &Rational::from((1, 8)));
    assert!(reduced.is_constant());
    assert!(reduced.get_constant() < Rational::zero());
    let receipt = equation::unit(
        &ring,
        &roles,
        &original,
        &relations,
        &constraints,
        &domain,
        &Rational::from((1, 1000000)),
        &b,
    )
    .unwrap();
    assert_eq!(receipt.sign(), 1);
    assert_eq!(
        ring.polynomial(receipt.representative(), &b).unwrap(),
        ring.polynomial(&original, &b).unwrap()
    );
    assert!(receipt.relation_quotients().iter().all(Atom::is_zero));
    assert!(
        equation::unit(
            &ring,
            &roles,
            &original,
            &relations,
            &constraints,
            &domain,
            &Rational::one(),
            &b
        )
        .is_err()
    );
}

#[test]
fn partial_tower_representative_keeps_exact_indexed_identity_and_native_sign() {
    let (a, c) = symbolica::symbol!("partial_tower::a", "partial_tower::c");
    let roles = [a, c];
    let ring = algebra::Ring::new(roles.to_vec()).unwrap();
    let relations = [
        (a, Atom::var(a).pow(2) - Atom::num((1, 128))),
        (c, Atom::var(c).pow(2) - Atom::num((1, 128))),
    ];
    let bounds = vec![[Rational::zero(), Rational::from((1, 8))]; 2];
    let b = Budget::new(Limits::default());
    let (constraints, domain) = ring.box_domain(&bounds, &b).unwrap();
    let positive = (Atom::var(a) - Atom::num((1, 10))).pow(2) + Atom::num((1, 10000));
    let original = &positive + Atom::num(100) * &relations[1].1;
    let (full, _) = transport::reduce(&ring, &roles, &original, &relations, &b).unwrap();
    // Neither the original nor the fully reduced polynomial is positive on
    // this independent box. Reducing only c preserves a positive square in a.
    for (p, point) in [
        (&original, [Rational::zero(), Rational::zero()]),
        (&full, [Rational::from((1, 8)), Rational::zero()]),
    ] {
        let p = ring.polynomial(p, &b).unwrap();
        assert!(p.evaluate(Clone::clone, &point) < Rational::zero());
    }
    let receipt = equation::unit(
        &ring,
        &roles,
        &original,
        &relations,
        &constraints,
        &domain,
        &Rational::from((1, 1000000)),
        &b,
    )
    .unwrap();
    assert_eq!(receipt.sign(), 1);
    assert_eq!(
        ring.polynomial(receipt.representative(), &b).unwrap(),
        ring.polynomial(&positive, &b).unwrap()
    );
    assert_eq!(receipt.relation_quotients().len(), 2);
    assert!(receipt.relation_quotients()[0].is_zero());
    let rebuilt = receipt.representative()
        + receipt
            .relation_quotients()
            .iter()
            .zip(&relations)
            .map(|(q, (_, r))| q * r)
            .sum::<Atom>();
    assert!(
        ring.polynomial(&(original - rebuilt), &b)
            .unwrap()
            .is_zero()
    );
    // This internal arithmetic control does not itself issue source authority;
    // the cover supplies authenticated equations and selected-root roles.
}
