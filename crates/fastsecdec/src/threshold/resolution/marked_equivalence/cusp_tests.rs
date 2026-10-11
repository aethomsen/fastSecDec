use super::super::*;
use super::*;
use symbolica::symbol;
fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn complete_factors(
    history: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    ns: &str,
    b: &mut Budget,
) -> Arc<CompletedComponentFactors> {
    let mut f = ComponentFactorFrontier::new(history, source, ns.into(), b).unwrap();
    for _ in 0..100 {
        let Some(p) = f.pending().next().cloned() else {
            break;
        };
        f.advance(&p, &ComponentFactorLimits::default(), b).unwrap();
    }
    let ComponentFactorCompletion::Complete(f) = f.try_complete().unwrap() else {
        panic!()
    };
    Arc::new(f)
}
fn initial_order(px: usize, py: usize, b: &mut Budget) -> Arc<ComponentResidualOrder> {
    let ring = Arc::new(
        Ring::new(
            vec![
                symbol!("principal_real_cusp::x"),
                symbol!("principal_real_cusp::y"),
            ],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(ring.clone(), vec![], b).unwrap(),
        vec![0, 1],
        vec![],
        b,
    )
    .unwrap();
    let frame = Arc::new(
        EtaleCertificate {
            source: local,
            equations: vec![],
            dependent_axes: vec![],
            free_axes: vec![0, 1],
            determinant_inverse_axis: None,
        }
        .verify(b)
        .unwrap(),
    );
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(frame, vec![], b).unwrap()
    else {
        panic!()
    };
    let history = ResolutionHistory::initial(ledger).unwrap();
    let f = ring.coordinate(0).unwrap().pow(px) + ring.coordinate(1).unwrap().pow(py);
    let source = Arc::new(MarkedIdeal::new(Ideal::new(ring, vec![f], b).unwrap(), 1, b).unwrap());
    let factors = complete_factors(history, source, "plane_initial", b);
    let leaf = factors.nodes().values().find_map(|n| n.leaf()).unwrap();
    let ComponentResidualProduction::Order(order) =
        produce_component_residual_order(leaf.clone(), b).unwrap()
    else {
        panic!()
    };
    Arc::new(*order)
}
#[test]
fn actual_native_cusp_coefficient_gets_a_checked_auxiliary_power_receipt() {
    let mut b = budget();
    let order = initial_order(3, 5, &mut b);
    assert_eq!(b.limits.max_mark, 64);
    for oi in 0..order.upper_order_cover().opens().len() {
        let CompanionOpenProduction::Contact(open) =
            produce_companion_open(order.clone(), oi, &format!("principal_cusp_o{oi}"), &mut b)
                .unwrap()
        else {
            continue;
        };
        let open = Arc::new(*open);
        for ci in 0..open.contact().candidates().len() {
            let CompanionCoefficientProduction::Coefficient(q) = construct_companion_coefficient(
                open.clone(),
                ci,
                [
                    symbol!(format!("principal_cusp_q{oi}_{ci}")),
                    symbol!(format!("principal_cusp_i{oi}_{ci}")),
                ],
                &mut b,
            )
            .unwrap() else {
                continue;
            };
            let frame = q.contact().contact().clone();
            let original = Arc::new(q.coefficient().coefficient().clone());
            let normalizer = QuotientNormalizer::prepare(frame.local().clone(), &mut b).unwrap();
            let NormalizationOutcome::Complete(normal) =
                normalizer.normalize(original.clone(), &mut b).unwrap()
            else {
                panic!()
            };
            let SncProduction::Verified(ledger) =
                verify_initial_relative_snc(frame.clone(), vec![], &mut b).unwrap()
            else {
                panic!()
            };
            let history = ResolutionHistory::initial(ledger).unwrap();
            let PrincipalPowerOutcome::Reduced(receipt) = produce_principal_power_presentation(
                history.clone(),
                normal.normalized().clone(),
                &mut b,
            )
            .unwrap() else {
                continue;
            };
            assert_eq!(receipt.source().mark(), 6);
            assert_eq!(receipt.power(), 2);
            assert_eq!(receipt.reduced().mark(), 3);
            let y = frame
                .local()
                .ring()
                .coordinate(frame.free_axes()[0])
                .unwrap();
            assert_eq!(receipt.source().ideal().generators(), &[y.pow(10)]);
            assert_eq!(receipt.base(), &y.pow(5));
            assert_eq!(
                receipt.source().ideal().generators()[0],
                receipt.unit() * &receipt.base().pow(receipt.power())
            );
            assert_eq!(q.coefficient().coefficient().mark(), 6);
            assert!(Arc::ptr_eq(normal.source(), &original));
            assert!(Arc::ptr_eq(receipt.history(), &history));
            assert_eq!(history.stage(), 0);
            // This is an auxiliary proof only. It does not replace q or issue
            // a reduced-source center as an original-source center.
            assert_eq!(
                q.coefficient().coefficient().ideal().generators(),
                original.ideal().generators()
            );
            return;
        }
    }
    panic!("actual cusp coefficient had no retained principal-power receipt");
}
