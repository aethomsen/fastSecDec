use super::*;
use symbolica::symbol;
fn issued(b: &mut Budget) -> Arc<RelativeRecursiveBlowup> {
    let r = Arc::new(
        Ring::new(
            vec![symbol!("post_open::x"), symbol!("post_open::y")],
            vec![],
        )
        .unwrap(),
    );
    let local = LocalizedAlgebra::new(
        Ideal::new(r.clone(), vec![], b).unwrap(),
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
    let SncProduction::Verified(ledger) = verify_initial_relative_snc(
        frame.clone(),
        vec![InitialDivisor {
            id: BoundaryId(41),
            equation: r.coordinate(1).unwrap(),
        }],
        b,
    )
    .unwrap() else {
        panic!()
    };
    let history = ResolutionHistory::initial(ledger).unwrap();
    let source = Arc::new(
        MarkedIdeal::new(
            Ideal::new(
                r.clone(),
                vec![r.coordinate(0).unwrap().pow(3) + r.coordinate(1).unwrap().pow(2)],
                b,
            )
            .unwrap(),
            2,
            b,
        )
        .unwrap(),
    );
    let mut driver =
        BoundaryFreeFirstCenter::new(frame, source, "post_open_recursion", false).unwrap();
    for _ in 0..20 {
        if matches!(driver.advance(b).unwrap(), RecursiveAdvance::Complete) {
            break;
        }
    }
    let RecursiveOutcome::Center(center) = driver.outcome() else {
        panic!()
    };
    let checked = CheckedRecursiveCenter::new(
        RecursiveCenterOrigin::BoundaryFree {
            center: center.clone(),
            history,
        },
        b,
    )
    .unwrap();
    let adapted = adapt_recursive_center(checked, "post_open_adapt", b).unwrap();
    blowup_recursive_center(adapted, "post_open_blowup", b).unwrap()
}
fn cover(chart: &Arc<RelativeRecursiveChart>, b: &mut Budget) -> Arc<VerifiedOpenCover> {
    let local = chart.geometry().frame().local();
    let e = chart.geometry().exceptional().unwrap();
    let strict = chart
        .geometry()
        .divisors()
        .iter()
        .find(|d| !d.absent)
        .unwrap();
    let factor = b.mul(e, &strict.strict_equation).unwrap();
    Arc::new(
        OpenCoverCertificate {
            algebra: local.clone(),
            support: Ideal::new(local.ring().clone(), vec![], b).unwrap(),
            opens: vec![factor.clone(), local.ring().one() - factor],
        }
        .verify(b)
        .unwrap(),
    )
}
#[test]
fn post_open_retains_original_birth_and_complete_pullback_when_exceptional_is_unit() {
    let mut b = Budget::new(Limits::default());
    let blowup = issued(&mut b);
    let chart = blowup
        .charts()
        .iter()
        .find(|c| {
            c.geometry().exceptional().is_some()
                && c.geometry().divisors().iter().any(|d| !d.absent)
        })
        .unwrap()
        .clone();
    let cover = cover(&chart, &mut b);
    let old_births = chart.history().births().clone();
    let old_stage = chart.history().stage();
    for side in 0..2 {
        let CoverLocalizationProduction::Complete(open) = localize_verified_cover_open(
            chart.history().clone(),
            cover.clone(),
            side,
            &format!("post_open_side{side}"),
            &mut b,
        )
        .unwrap() else {
            panic!()
        };
        let open = Arc::new(*open);
        let refined = restrict_relative_chart(chart.clone(), open.clone(), &mut b).unwrap();
        assert!(refined.history().same_root(chart.history()));
        assert_eq!(refined.history().births(), &old_births);
        assert_eq!(refined.history().stage(), old_stage);
        assert_eq!(
            refined.history().chart_path().len(),
            chart.history().chart_path().len() + 1
        );
        let local = refined.geometry().frame().local();
        let extension = open.open().extension();
        assert!(
            local
                .zero(
                    &(refined.geometry().jacobian()
                        - &extension.pull(chart.geometry().jacobian(), &mut b).unwrap()),
                    &mut b
                )
                .unwrap()
        );
        assert!(
            local
                .zero(
                    &(refined.geometry().original_coordinate_jacobian()
                        - &extension
                            .pull(chart.geometry().original_coordinate_jacobian(), &mut b)
                            .unwrap()),
                    &mut b
                )
                .unwrap()
        );
        assert_eq!(
            refined.parent().source().mark(),
            refined.parent().target().mark()
        );
        for g in refined.parent().generators() {
            assert!(
                local
                    .zero(
                        &(&refined.geometry().pull(&g.source, &mut b).unwrap() - &g.total),
                        &mut b
                    )
                    .unwrap()
            );
        }
        if side == 0 {
            assert!(
                super::super::super::helpers::unit(
                    local,
                    refined.geometry().exceptional().unwrap(),
                    &mut b
                )
                .unwrap()
            );
            assert_eq!(
                refined.history().ledger().divisors().len(),
                chart.history().ledger().divisors().len()
            );
            for d in refined.history().ledger().divisors() {
                assert!(super::super::super::helpers::unit(local, &d.equation, &mut b).unwrap());
            }
            assert!(!refined.history().births().is_empty());
        }
        // A restriction must not issue the physical transition a second time.
        assert!(matches!(
            refined
                .history()
                .advanced_relative(refined.geometry(), &mut b),
            Err(Error::Invalid(_))
        ));
    }
}
#[test]
fn post_open_refuses_foreign_target_history_and_incomplete_cover() {
    let mut b = Budget::new(Limits::default());
    let blowup = issued(&mut b);
    let chart = blowup
        .charts()
        .iter()
        .find(|c| {
            c.geometry().exceptional().is_some()
                && c.geometry().divisors().iter().any(|d| !d.absent)
        })
        .unwrap()
        .clone();
    let cover = cover(&chart, &mut b);
    let foreign = ResolutionHistory::initial(chart.history().ledger().clone()).unwrap();
    let CoverLocalizationProduction::Complete(open) =
        localize_verified_cover_open(foreign, cover, 0, "post_open_foreign", &mut b).unwrap()
    else {
        panic!()
    };
    assert!(matches!(
        restrict_relative_chart(chart.clone(), Arc::new(*open), &mut b),
        Err(Error::Invalid(_))
    ));
    let local = chart.geometry().frame().local();
    assert!(matches!(
        OpenCoverCertificate {
            algebra: local.clone(),
            support: Ideal::new(local.ring().clone(), vec![], &mut b).unwrap(),
            opens: vec![chart.geometry().exceptional().unwrap().clone()]
        }
        .verify(&mut b),
        Err(Error::Invalid(_))
    ));
}
