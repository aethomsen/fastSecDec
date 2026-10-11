use super::*;
use crate::threshold::maps::endpoint_transport::{
    EndpointTransport, FaceAxis, FaceSide, clearing::pull, cover::EndpointChartCover,
    tests::fixture,
};
use symbolica::{
    atom::{AtomCore, Symbol},
    symbol,
};
fn original() -> Arc<EndpointTransport> {
    let prefix = fixture();
    let faces = prefix
        .source()
        .unit_coordinates()
        .iter()
        .map(|&unit| FaceAxis {
            unit,
            side: FaceSide::Lower,
        })
        .collect::<Vec<_>>();
    EndpointTransport::prepare(prefix, &faces, &super::super::Limits::default(), || {
        ControlFlow::Continue(())
    })
    .unwrap()
}
fn prepare(chart: IssuedEndpointChart) -> Arc<FactorFacePortfolio> {
    FactorFacePortfolio::prepare(chart, &Limits::default(), || ControlFlow::Continue(())).unwrap()
}
#[test]
fn physical_factor_faces_genuine_inventory_units_zeros_and_pending() {
    let source = original();
    let portfolio = prepare(source.issued_chart(0).unwrap());
    assert!(Arc::ptr_eq(portfolio.chart().transport(), &source));
    assert!(portfolio.inventory_complete());
    assert_eq!(portfolio.faces().len(), 26);
    assert_eq!(portfolio.active_factors().len(), 2);
    assert_eq!(
        portfolio
            .active_factors()
            .iter()
            .map(|a| a.source_links().len())
            .sum::<usize>(),
        3
    );
    assert_eq!(source.term_links()[0].factors()[1], FactorLink::Inactive);
    assert_eq!(
        source.term_links()[0].factors()[2],
        FactorLink::LazyPolynomial
    );
    let budget = Budget::new(AlgebraLimits::default());
    let mut counts = [0; 3];
    for face in portfolio.faces() {
        let units = face
            .face()
            .axes()
            .iter()
            .map(|a| a.unit)
            .collect::<Vec<_>>();
        let values = face
            .face()
            .axes()
            .iter()
            .map(|a| Atom::num(if a.side == FaceSide::Lower { 0 } else { 1 }))
            .collect::<Vec<_>>();
        for row in face.rows() {
            let f =
                &source.charts()[0].factors()[portfolio.active_factors()[row.factor()].identity()];
            assert!(
                (row.restriction().numerator() - pull(f.numerator(), &units, &values))
                    .expand()
                    .is_zero()
            );
            assert!(
                (row.restriction().denominator() - pull(f.denominator(), &units, &values))
                    .expand()
                    .is_zero()
            );
            match row.outcome() {
                FaceOutcome::SignedUnit {
                    numerator,
                    denominator,
                } => {
                    counts[0] += 1;
                    numerator.replay(face.proof_box(), &budget).unwrap();
                    denominator.replay(face.proof_box(), &budget).unwrap();
                }
                FaceOutcome::ZeroResidual { denominator } => {
                    counts[1] += 1;
                    assert!(row.restriction().numerator().is_zero());
                    denominator.replay(face.proof_box(), &budget).unwrap();
                }
                FaceOutcome::Unresolved(_) => counts[2] += 1,
            }
        }
    }
    assert!(
        counts[0] > 0 && counts[1] > 0 && counts[2] > 0,
        "{counts:?}"
    );
    assert!(!portfolio.factor_faces_complete());
    assert_eq!(portfolio.unresolved_rows(), counts[2]);
    let mut p = Arc::try_unwrap(portfolio).unwrap();
    p.faces.pop();
    assert!(!p.inventory_complete());
    assert!(!p.factor_faces_complete());
}
#[test]
fn physical_factor_faces_refined_owner_and_transactional_budgets() {
    let source = original();
    let limits = Limits::default();
    let view = source.issued_chart(0).unwrap();
    for l in [
        Limits {
            max_faces: 0,
            ..limits.clone()
        },
        Limits {
            max_rows: 0,
            ..limits.clone()
        },
    ] {
        assert!(matches!(
            FactorFacePortfolio::prepare(view.clone(), &l, || ControlFlow::Continue(())),
            Err(Error::ResourceIncomplete(_))
        ));
    }
    let mut count = 0;
    assert!(matches!(
        FactorFacePortfolio::prepare(view.clone(), &limits, || {
            count += 1;
            if count == 3 {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        FactorFacePortfolio::prepare(
            view,
            &Limits {
                fallback_margin: Rational::zero(),
                ..limits
            },
            || ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
    assert!(restriction::face_count(usize::BITS as usize, usize::MAX).is_err());
    let cover = EndpointChartCover::prepare(
        source.clone(),
        0,
        &source.target_units()[..1],
        &[],
        &super::super::Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(cover.leaves().len(), 2);
    for i in 0..2 {
        let p = prepare(cover.issued_chart(i).unwrap());
        assert!(p.inventory_complete());
        assert!(Arc::ptr_eq(p.chart().cover().unwrap(), &cover));
        assert_eq!(p.faces()[0].face().axes()[0].unit, cover.target_units()[0]);
        assert!(
            !source
                .target_units()
                .contains(&p.faces()[0].face().axes()[0].unit)
        );
    }
    let view = source.issued_chart(0).unwrap();
    let active = active_factors(&view).unwrap();
    let budget = Budget::new(AlgebraLimits::default());
    let s = restriction::Source::new(&view, &active, &budget).unwrap();
    let foreign = restriction::RelativeFace::new(&[symbol!("factor_faces::foreign")], 1).unwrap();
    assert!(matches!(
        s.restrict(0, &foreign, &budget),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn physical_factor_faces_constant_vertices_denominator_misses_and_replay() {
    let b = Budget::new(AlgebraLimits::default());
    let empty = real::ProofBox::from_bounds(vec![], vec![], &b).unwrap();
    let mut p = empty
        .propose(&Atom::num(2), &Rational::from((1, 2)), &b)
        .unwrap()
        .unwrap();
    p.replay(&empty, &b).unwrap();
    assert!(matches!(p.evidence(), MarginEvidence::Constant(_)));
    p.margin = Rational::from(3);
    assert!(p.replay(&empty, &b).is_err());
    let residual = |n: Atom, d: Atom| restriction::RestrictedFactor {
        numerator: n,
        denominator: d,
    };
    let limits = Limits::default();
    assert!(matches!(
        classify(
            &empty,
            &residual(Atom::num(-2), Atom::one()),
            -1,
            &limits,
            &b
        )
        .unwrap(),
        FaceOutcome::SignedUnit { .. }
    ));
    assert!(matches!(
        classify(&empty, &residual(Atom::zero(), Atom::one()), 1, &limits, &b).unwrap(),
        FaceOutcome::ZeroResidual { .. }
    ));
    assert!(matches!(
        classify(
            &empty,
            &residual(Atom::zero(), Atom::zero()),
            1,
            &limits,
            &b
        )
        .unwrap(),
        FaceOutcome::Unresolved(UnresolvedReason::ZeroDenominator)
    ));
    assert!(matches!(
        classify(
            &empty,
            &residual(Atom::zero(), Atom::num(-1)),
            1,
            &limits,
            &b
        )
        .unwrap(),
        FaceOutcome::Unresolved(UnresolvedReason::DenominatorMarginMiss)
    ));
    let x: Symbol = symbol!("factor_faces::x");
    let domain =
        real::ProofBox::from_bounds(vec![x], vec![[Rational::zero(), Rational::one()]], &b)
            .unwrap();
    assert!(matches!(
        classify(
            &domain,
            &residual(Atom::zero(), Atom::var(x)),
            1,
            &limits,
            &b
        )
        .unwrap(),
        FaceOutcome::Unresolved(UnresolvedReason::DenominatorMarginMiss)
    ));
    assert!(matches!(
        classify(
            &domain,
            &residual(Atom::var(x) - Atom::num((1, 2)), Atom::one()),
            1,
            &limits,
            &b
        )
        .unwrap(),
        FaceOutcome::Unresolved(UnresolvedReason::SignedNumeratorMarginMiss)
    ));
    let square = (Atom::num(2) * Atom::var(x) - Atom::one()).pow(2) + Atom::num((1, 10));
    let smaller = domain
        .propose(&square, &Rational::from((1, 20)), &b)
        .unwrap()
        .unwrap();
    assert_eq!(smaller.margin(), &Rational::from((1, 20)));
    smaller.replay(&domain, &b).unwrap();
    let mut native = domain
        .propose(&(Atom::one() + Atom::var(x)), &Rational::from((1, 2)), &b)
        .unwrap()
        .unwrap();
    native.replay(&domain, &b).unwrap();
    if let MarginEvidence::Native(c) = &mut native.evidence {
        c.sign = -1;
    } else {
        panic!("native box proof expected")
    };
    assert!(native.replay(&domain, &b).is_err());
}

#[test]
fn physical_factor_faces_zero_only_frontier_is_not_regular() {
    use crate::{
        parametric::{
            FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
            PolynomialFactor,
        },
        threshold::{
            gcad::{Limits as GL, PositiveProjectiveQuotient, Problem, SolverOptions},
            maps::{PositiveProjectiveCellMap, normalized::NormalizedPrefix},
            projective::PositiveProjectivePreparation,
        },
    };
    let (a, x, eps, u) = symbol!(
        "zero_face::a",
        "zero_face::x",
        "zero_face::eps",
        "zero_face::u"
    );
    let source = Arc::new(
        ParametricIntegrand::new(
            vec![a, x],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 2],
                vec![
                    PolynomialFactor::new(
                        Atom::var(x) - Atom::var(a),
                        Atom::num(-2),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                ],
            )],
        )
        .unwrap(),
    );
    let prep = Arc::new(PositiveProjectivePreparation::anchor(source, 0).unwrap());
    let p = Problem {
        variables: vec!["x0".into(), "x1".into()],
        constraints: vec!["x0>0".into(), "x1>0".into()],
        split: vec!["x1-x0".into()],
        solver: SolverOptions {
            homogeneous_anchor: Some("x0".into()),
            homogeneous_block: vec!["x0".into(), "x1".into()],
            ordering: "given".into(),
            ..Default::default()
        },
        limits: GL {
            workers: 1,
            wall_time_secs: 10.,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    let owner = Arc::new(
        PositiveProjectiveQuotient::verify(prep, Arc::new(symgcad::solver::solve(&p).unwrap()))
            .unwrap(),
    );
    let index = owner
        .native_result()
        .cells
        .iter()
        .position(|c| c.signs == [-1])
        .unwrap();
    let map = PositiveProjectiveCellMap::new(owner, index, vec![u]).unwrap();
    let b = Budget::new(AlgebraLimits::default());
    let prefix = NormalizedPrefix::new(map, &b).unwrap().open(&b).unwrap();
    let t = EndpointTransport::prepare(
        prefix,
        &[FaceAxis {
            unit: u,
            side: FaceSide::Lower,
        }],
        &super::super::Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let p = prepare(t.issued_chart(0).unwrap());
    assert!(p.inventory_complete());
    assert_eq!(p.unresolved_rows(), 0);
    assert_eq!(p.zero_residual_rows(), 1);
    assert!(!p.factor_faces_complete());
    assert_eq!(p.active_factors()[0].expected_sign(), -1);
    assert!(p.faces().iter().all(|f| f.proof_box().roles().is_empty()));
    assert!(matches!(
        p.faces()[0].rows()[0].outcome(),
        FaceOutcome::SignedUnit { .. }
    ));
    assert!(matches!(
        p.faces()[1].rows()[0].outcome(),
        FaceOutcome::ZeroResidual { .. }
    ));
}
