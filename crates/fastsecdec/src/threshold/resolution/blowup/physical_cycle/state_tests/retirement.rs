use super::*;
fn affine(r: Arc<Ring>, b: &mut Budget) -> Arc<EtaleFrame> {
    let local =
        LocalizedAlgebra::new(Ideal::new(r, vec![], b).unwrap(), vec![0, 1], vec![], b).unwrap();
    Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![0, 1],
            determinant_inverse_axis: None,
        }
        .verify(b)
        .unwrap(),
    )
}
#[test]
fn retired_companion_cannot_follow_inadmissible_future_center() {
    let mut b = Budget::new(Limits::default());
    let source = Arc::new(
        Ring::new(
            vec![symbol!("retired_g::x"), symbol!("retired_g::y")],
            vec![],
        )
        .unwrap(),
    );
    let frame = affine(source.clone(), &mut b);
    let old = Arc::new(
        MarkedIdeal::new(
            Ideal::new(source.clone(), vec![source.coordinate(0).unwrap()], &mut b).unwrap(),
            2,
            &b,
        )
        .unwrap(),
    );
    assert!(MarkedCosupport::prove(frame, old, &mut b).unwrap().empty());
    let target = Arc::new(
        Ring::new(
            vec![symbol!("retired_g::u"), symbol!("retired_g::v")],
            vec![],
        )
        .unwrap(),
    );
    let target_frame = affine(target.clone(), &mut b);
    let u = target.coordinate(0).unwrap();
    let v = target.coordinate(1).unwrap();
    // Exact standard v-chart x=u*v,y=v, recorded here as a focused
    // polynomial control only, not an issued physical-center authority.
    let total = &u * &v;
    let pulled = Arc::new(
        MarkedIdeal::new(
            Ideal::new(target.clone(), vec![total.clone()], &mut b).unwrap(),
            2,
            &b,
        )
        .unwrap(),
    );
    assert!(
        !MarkedCosupport::prove(target_frame.clone(), pulled, &mut b)
            .unwrap()
            .empty()
    );
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(
        target_frame,
        vec![InitialDivisor {
            id: BoundaryId(0),
            equation: v,
        }],
        &mut b,
    )
    .unwrap() else {
        panic!()
    };
    let CartierDivision::Quotient(first) =
        divide_cartier(ledger.clone(), 0, total, "retired_first", &mut b).unwrap()
    else {
        panic!()
    };
    assert_eq!(first.quotient(), &u);
    assert!(matches!(
        divide_cartier(
            ledger,
            0,
            first.quotient().clone(),
            "retired_second",
            &mut b
        )
        .unwrap(),
        CartierDivision::NotDivisible { .. }
    ));
    println!(
        "PASS: old (x,2) cosupport empty; total (uv,2) cosupport nonempty; v^2 controlled division refused"
    );
}
