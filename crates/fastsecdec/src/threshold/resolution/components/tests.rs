use super::super::{
    BoundaryId, Budget, Error, EtaleCertificate, EtaleFrame, Guard, Ideal, InitialDivisor, Limits,
    LocalizedAlgebra, Poly, Ring, SncProduction, VerifiedRelativeSnc, verify_initial_relative_snc,
};
use super::*;
use std::sync::Arc;
use symbolica::{atom::Atom, symbol};
fn ring() -> Arc<Ring> {
    Arc::new(
        Ring::new(
            vec![
                symbol!("component_test::p"),
                symbol!("component_test::x"),
                symbol!("component_test::y"),
                symbol!("component_test::u"),
                symbol!("component_test::v"),
            ],
            vec![0],
        )
        .unwrap(),
    )
}
fn b() -> Budget {
    Budget::new(Limits::default())
}
fn p(r: &Ring, s: &str) -> Poly {
    r.atom(&Atom::parse(s, "component_test", Default::default()).unwrap())
        .unwrap()
}
fn ideal(r: &Arc<Ring>, f: Vec<Poly>, b: &mut Budget) -> Arc<Ideal> {
    Arc::new(Ideal::new(r.clone(), f, b).unwrap())
}
fn local(
    r: &Arc<Ring>,
    f: Vec<Poly>,
    axes: Vec<usize>,
    guards: Vec<Guard>,
    b: &mut Budget,
) -> Arc<LocalizedAlgebra> {
    LocalizedAlgebra::new(Ideal::new(r.clone(), f, b).unwrap(), axes, guards, b).unwrap()
}
fn flat(local: Arc<LocalizedAlgebra>, b: &mut Budget) -> Arc<EtaleFrame> {
    Arc::new(
        EtaleCertificate {
            free_axes: local.axes().to_vec(),
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            determinant_inverse_axis: None,
        }
        .verify(b)
        .unwrap(),
    )
}
fn ledger(frame: Arc<EtaleFrame>, h: Poly, b: &mut Budget) -> Arc<VerifiedRelativeSnc> {
    let SncProduction::Verified(l) = verify_initial_relative_snc(
        frame,
        vec![InitialDivisor {
            id: BoundaryId(1),
            equation: h,
        }],
        b,
    )
    .unwrap() else {
        panic!("unchecked boundary")
    };
    l
}
fn boundary(frame: Arc<EtaleFrame>, h: Poly, b: &mut Budget) -> Arc<RegularAlgebra> {
    let BoundaryAlgebra::Regular(owner) =
        RegularAlgebra::boundary(ledger(frame, h, b), 0, b).unwrap()
    else {
        panic!("missing regular boundary")
    };
    owner
}
fn split(
    owner: Arc<RegularAlgebra>,
    source: Arc<Ideal>,
    b: &mut Budget,
) -> Box<VerifiedComponentSplit> {
    let ComponentProduction::Complete(split) =
        produce_component_split(owner, source, "component_test_run", b).unwrap()
    else {
        panic!("incomplete")
    };
    split
}

#[test]
fn component_producer_disconnected_boundary_and_forgery_controls() {
    let r = ring();
    let mut b = b();
    let source = local(&r, vec![], vec![1, 2], vec![], &mut b);
    let owner = boundary(
        flat(source.clone(), &mut b),
        p(&r, "component_test::x*(component_test::x-1)"),
        &mut b,
    );
    let input = ideal(&r, vec![p(&r, "component_test::x^2")], &mut b);
    let s = split(owner.clone(), input.clone(), &mut b);
    assert_eq!(s.pattern(), ComponentPattern::Mixed);
    assert!(
        owner
            .local()
            .zero(&(s.idempotent() - &p(&r, "1-component_test::x")), &mut b)
            .unwrap()
    );
    assert!(Arc::ptr_eq(s.owner(), &owner));
    assert!(Arc::ptr_eq(s.source(), &input));
    assert_eq!(s.annihilator().evidence().auxiliary_axes(), 1);
    assert!(
        !s.annihilator()
            .evidence()
            .equations()
            .generators()
            .is_empty()
    );
    assert!(!s.annihilator().evidence().basis().generators().is_empty());
    assert!(!s.cover().algebraic_locus_empty());
    assert!(s.opens().iter().all(|o| !o.empty_in_ambient()));
    assert_eq!(s.division_quotients().len(), 1);
    for q in s.division_quotients().iter().chain(s.overlap_quotient()) {
        let h = &q.owner().divisors()[q.divisor_index()].equation;
        assert!(
            source
                .zero(&(q.dividend() - &b.mul(h, q.quotient()).unwrap()), &mut b)
                .unwrap()
        );
    }
    assert!(
        !s.verify_idempotent_candidate(&r.one().zero(), &mut b)
            .unwrap()
    );
    assert!(!s.verify_idempotent_candidate(&r.one(), &mut b).unwrap());
    assert!(
        s.verify_idempotent_candidate(s.idempotent(), &mut b)
            .unwrap()
    );
    let fake = Ideal::new(r.clone(), vec![], &mut b).unwrap();
    assert!(
        !s.annihilator()
            .verify_candidate(&fake, "component_fake_check", &mut b)
            .unwrap()
    );
    assert!(
        s.annihilator()
            .verify_candidate(
                s.annihilator().annihilator(),
                "component_valid_check",
                &mut b
            )
            .unwrap()
    );
    assert_eq!(s.progress().checked_boundary_quotients, 2);
    assert_eq!(s.progress().stage, "complete");

    // The common zero COMPONENT of the entire ideal is required: a union of
    // generator zero sets would incorrectly split this unit ideal.
    let no_common_component = split(
        owner.clone(),
        ideal(
            &r,
            vec![
                p(&r, "component_test::x^2"),
                p(&r, "(component_test::x-1)^2"),
            ],
            &mut b,
        ),
        &mut b,
    );
    assert_eq!(
        no_common_component.pattern(),
        ComponentPattern::NowhereIdenticallyZero
    );
    let common_component = split(
        owner,
        ideal(
            &r,
            vec![p(&r, "component_test::x^2"), p(&r, "component_test::x^3")],
            &mut b,
        ),
        &mut b,
    );
    assert_eq!(common_component.pattern(), ComponentPattern::Mixed);
    assert_eq!(common_component.division_quotients().len(), 2);
}

#[test]
fn component_producer_proper_zeros_parameters_and_constant_patterns() {
    let r = ring();
    let mut b = b();
    let source = local(&r, vec![], vec![1, 2], vec![], &mut b);
    let owner = boundary(flat(source, &mut b), r.coordinate(1).unwrap(), &mut b);
    for expression in ["component_test::y", "component_test::p", "1"] {
        let s = split(
            owner.clone(),
            ideal(&r, vec![p(&r, expression)], &mut b),
            &mut b,
        );
        assert_eq!(s.pattern(), ComponentPattern::NowhereIdenticallyZero);
        assert!(owner.local().zero(s.idempotent(), &mut b).unwrap());
        assert!(s.opens()[0].empty_in_ambient());
    }
    for fs in [vec![], vec![p(&r, "component_test::x^2")]] {
        let s = split(owner.clone(), ideal(&r, fs, &mut b), &mut b);
        assert_eq!(s.pattern(), ComponentPattern::EverywhereIdenticallyZero);
        assert!(
            owner
                .local()
                .zero(&(s.idempotent() - &r.one()), &mut b)
                .unwrap()
        );
        assert!(s.opens()[1].empty_in_ambient());
    }
    assert!(owner.local().ring().is_parameter(0));
}

#[test]
fn component_producer_regular_ambient_zero_component_and_empty_boundary() {
    let r = ring();
    let mut b = b();
    let source = local(
        &r,
        vec![p(&r, "component_test::y^2-component_test::y")],
        vec![1, 2],
        vec![],
        &mut b,
    );
    let frame = Arc::new(
        EtaleCertificate {
            source,
            free_axes: vec![1],
            equations: vec![0],
            dependent_axes: vec![2],
            determinant_inverse_axis: Some(4),
        }
        .verify(&mut b)
        .unwrap(),
    );
    let owner = RegularAlgebra::ambient(frame.clone());
    let s = split(
        owner.clone(),
        ideal(
            &r,
            vec![p(&r, "component_test::x^2*component_test::y")],
            &mut b,
        ),
        &mut b,
    );
    assert_eq!(s.pattern(), ComponentPattern::Mixed);
    assert!(
        owner
            .local()
            .zero(&(s.idempotent() - &p(&r, "1-component_test::y")), &mut b)
            .unwrap()
    );
    assert!(s.overlap_quotient().is_none());
    assert!(s.division_quotients().is_empty());
    let owner = ledger(frame, r.one(), &mut b);
    assert!(matches!(
        RegularAlgebra::boundary(owner.clone(), 0, &mut b).unwrap(),
        BoundaryAlgebra::Empty { .. }
    ));
    assert!(RegularAlgebra::boundary(owner, 1, &mut b).is_err());
    let source = local(&r, vec![], vec![1, 2], vec![], &mut b);
    assert!(matches!(
        verify_initial_relative_snc(
            flat(source, &mut b),
            vec![InitialDivisor {
                id: BoundaryId(1),
                equation: p(&r, "component_test::x^2")
            }],
            &mut b
        )
        .unwrap(),
        SncProduction::Unresolved { .. }
    ));
}

#[test]
fn component_producer_guarded_equation_and_original_cover() {
    let r = ring();
    let mut b = b();
    let source = local(
        &r,
        vec![],
        vec![1],
        vec![Guard {
            factor: r.coordinate(0).unwrap(),
            inverse_axis: 3,
        }],
        &mut b,
    );
    let h = p(
        &r,
        "component_test::x*(component_test::x-component_test::p)*component_test::u",
    );
    let owner = boundary(flat(source.clone(), &mut b), h, &mut b);
    let s = split(
        owner.clone(),
        ideal(&r, vec![p(&r, "component_test::x^2")], &mut b),
        &mut b,
    );
    assert_eq!(s.pattern(), ComponentPattern::Mixed);
    let RegularOrigin::Boundary { equation, .. } = owner.origin() else {
        panic!()
    };
    assert!(!equation.numerator.contains(3));
    assert!(!equation.denominator.contains(3));
    assert!(s.idempotent().contains(3));
    assert!(
        s.opens()
            .iter()
            .all(|o| !o.clearing().numerator.contains(3) && !o.clearing().denominator.contains(3))
    );
    for o in s.opens() {
        assert!(
            source
                .zero(
                    &(&b.mul(&o.clearing().original, &o.clearing().denominator)
                        .unwrap()
                        - &o.clearing().numerator),
                    &mut b
                )
                .unwrap()
        );
    }
    assert!(
        s.annihilator()
            .evidence()
            .ring()
            .is_parameter(s.annihilator().evidence().auxiliary_axes())
    );
}

#[test]
fn component_producer_resource_progress_and_namespace_collision() {
    let r = ring();
    let mut b = b();
    let source = local(&r, vec![], vec![1], vec![], &mut b);
    let owner = boundary(
        flat(source, &mut b),
        p(&r, "component_test::x*(component_test::x-1)"),
        &mut b,
    );
    let input = ideal(&r, vec![p(&r, "component_test::x^2")], &mut b);
    let mut none = Budget::new(Limits {
        max_operations: 0,
        ..Limits::default()
    });
    assert!(matches!(
        produce_component_split(owner.clone(), input.clone(), "component_cap0", &mut none).unwrap(),
        ComponentProduction::Incomplete {
            progress: ComponentProgress {
                annihilator: None,
                ..
            },
            ..
        }
    ));
    let mut retained = false;
    for max_operations in 10..100 {
        let mut cap = Budget::new(Limits {
            max_operations,
            ..Limits::default()
        });
        if let ComponentProduction::Incomplete { progress, .. } = produce_component_split(
            owner.clone(),
            input.clone(),
            "component_cap_partial",
            &mut cap,
        )
        .unwrap()
            && progress.annihilator.is_some()
        {
            retained = true;
            assert!(progress.operations <= max_operations);
            break;
        }
    }
    assert!(retained);
    // Dynamic auxiliary names must never silently alias an original coordinate.
    let collision = Arc::new(
        Ring::new(
            vec![
                symbol!("component_collision::ann::aux_0"),
                symbol!("component_collision::x"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = local(&collision, vec![], vec![0, 1], vec![], &mut b);
    let owner = RegularAlgebra::ambient(flat(local, &mut b));
    let input = ideal(&collision, vec![collision.coordinate(1).unwrap()], &mut b);
    assert!(matches!(
        produce_component_split(owner, input, "component_collision", &mut b),
        Err(Error::Invalid("ring variables/parameter roles"))
    ));
}

#[test]
fn component_presentations_preserve_original_products_and_quotient_owner() {
    let r = ring();
    let mut b = b();
    let ambient = local(&r, vec![], vec![1, 2], vec![], &mut b);
    let frame = flat(ambient.clone(), &mut b);
    let owner = boundary(
        frame.clone(),
        p(&r, "component_test::x*(component_test::x-1)"),
        &mut b,
    );
    let input = ideal(
        &r,
        vec![
            p(&r, "component_test::x^2"),
            p(&r, "component_test::x^3"),
            p(&r, "component_test::x^2*(1+component_test::x)"),
        ],
        &mut b,
    );
    let originals = input.generators().to_vec();
    let s = split(owner.clone(), input.clone(), &mut b);
    assert!(Arc::ptr_eq(s.source(), &input));
    assert_eq!(input.generators(), originals);
    assert_eq!(s.annihilator().source().generators(), originals);
    assert!(Arc::ptr_eq(
        s.annihilator().source_normalization().normalizer().local(),
        owner.local()
    ));
    assert_eq!(
        s.annihilator().selected_source().ideal().generators().len(),
        1
    );
    assert_eq!(s.annihilator().evidence().auxiliary_axes(), 1);
    assert_eq!(s.division_quotients().len(), originals.len());
    for f in &originals {
        assert!(
            owner
                .local()
                .zero(&b.mul(s.idempotent(), f).unwrap(), &mut b)
                .unwrap()
        );
    }
    assert_eq!(
        s.progress().checked_products,
        s.annihilator().annihilator().generators().len()
            * s.complement().annihilator().generators().len()
    );
    for selected in s.idempotent_source_presentations() {
        assert_eq!(selected.mark(), 1);
    }
    let other_owner = boundary(
        frame,
        p(&r, "component_test::x*(component_test::x-2)"),
        &mut b,
    );
    let other = split(other_owner, input, &mut b);
    assert!(
        !s.verify_idempotent_candidate(other.idempotent(), &mut b)
            .unwrap()
    );
    assert!(
        !other
            .verify_idempotent_candidate(s.idempotent(), &mut b)
            .unwrap()
    );
}
