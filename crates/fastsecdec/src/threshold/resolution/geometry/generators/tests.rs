use super::*;
use symbolica::symbol;

fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn fixture() -> (Arc<Ring>, Arc<LocalizedAlgebra>) {
    let r = Arc::new(
        Ring::new(
            vec![
                symbol!("generator_presentation::x"),
                symbol!("generator_presentation::y"),
                symbol!("generator_presentation::u"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let x = r.coordinate(0).unwrap();
    let y = r.coordinate(1).unwrap();
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), vec![&y - &x], &mut budget()).unwrap(),
        vec![0, 1],
        vec![Guard {
            factor: &r.one() + &y,
            inverse_axis: 2,
        }],
        &mut budget(),
    )
    .unwrap();
    (r, local)
}
fn marked(r: &Arc<Ring>, p: Vec<Poly>, mark: usize) -> Arc<MarkedIdeal> {
    Arc::new(
        MarkedIdeal::new(
            Ideal::new(r.clone(), p, &mut budget()).unwrap(),
            mark,
            &budget(),
        )
        .unwrap(),
    )
}
#[test]
fn generator_presentation_local_unit_redundancy_mark_and_original_owner() {
    let (r, local) = fixture();
    let x = r.coordinate(0).unwrap();
    let y = r.coordinate(1).unwrap();
    let source = marked(&r, vec![&(&r.one() + &y) * &x, x.pow(2)], 3);
    let original = source.ideal().generators().to_vec();
    let receipt =
        VerifiedGeneratorPresentation::prepare(local.clone(), source.clone(), &mut budget())
            .unwrap();
    assert!(Arc::ptr_eq(receipt.normalization().source(), &source));
    assert!(Arc::ptr_eq(
        receipt.normalization().normalizer().local(),
        &local
    ));
    assert_eq!(source.ideal().generators(), original);
    assert_eq!(receipt.selected().mark(), 3);
    assert!(
        receipt
            .normalization()
            .normalizer()
            .verify_candidate(&source, &marked(&r, vec![x.clone()], 3), &mut budget())
            .unwrap()
    );
    assert!(
        receipt
            .normalization()
            .normalizer()
            .verify_candidate(&source, receipt.selected(), &mut budget())
            .unwrap()
    );
    for wrong in [marked(&r, vec![], 3), marked(&r, vec![r.one()], 3)] {
        assert!(
            !receipt
                .normalization()
                .normalizer()
                .verify_candidate(&source, &wrong, &mut budget())
                .unwrap()
        );
    }
    assert!(matches!(
        receipt.normalization().normalizer().verify_candidate(
            &source,
            &marked(&r, vec![x], 1),
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn generator_presentation_quotient_zero_unit_and_resource_refusal() {
    let (r, local) = fixture();
    let x = r.coordinate(0).unwrap();
    let y = r.coordinate(1).unwrap();
    for (polys, is_zero) in [
        (vec![&y - &x], true),
        (vec![&r.one() + &y], false),
        (vec![r.one()], false),
    ] {
        let source = marked(&r, polys, 2);
        let p =
            VerifiedGeneratorPresentation::prepare(local.clone(), source.clone(), &mut budget())
                .unwrap();
        assert_eq!(p.selected().ideal().generators().is_empty(), is_zero);
        if !is_zero {
            assert!(
                p.selected()
                    .ideal()
                    .contains(&r.one(), local.unit_relations(), &mut budget())
                    .unwrap()
            );
        }
        let mut exhausted = Budget::new(Limits {
            max_operations: 0,
            ..Limits::default()
        });
        assert!(matches!(
            VerifiedGeneratorPresentation::prepare(local.clone(), source.clone(), &mut exhausted),
            Err(Error::ResourceIncomplete(_))
        ));
        assert!(Arc::ptr_eq(p.normalization().source(), &source));
    }
    let foreign = Arc::new(Ring::new(vec![symbol!("generator_foreign::x")], vec![]).unwrap());
    assert!(matches!(
        VerifiedGeneratorPresentation::prepare(
            local,
            marked(&foreign, vec![foreign.one()], 1),
            &mut budget()
        ),
        Err(Error::Invalid(_))
    ));
}
