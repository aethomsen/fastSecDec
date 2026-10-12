use super::super::{EndpointTransport, FaceAxis, FaceSide, clearing::pull};
use super::*;
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
use symbolica::{domains::rational::Rational, symbol};

fn source(zero_boundary: bool) -> Arc<EndpointTransport> {
    let _ = symbolica::transcendental::gamma();
    let (a, x, eps, u) = symbol!(
        "closed_test::a",
        "closed_test::x",
        "closed_test::eps",
        "closed_test::u"
    );
    let f = if zero_boundary {
        Atom::var(x) - Atom::var(a)
    } else {
        -Atom::var(x) - Atom::var(a)
    };
    let input = Arc::new(
        ParametricIntegrand::new(
            vec![a, x],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::parse("gamma(1+closed_test::eps)", "symbolica", Default::default()).unwrap(),
                vec![Atom::zero(); 2],
                vec![
                    PolynomialFactor::new(
                        f,
                        -Atom::num(2) - Atom::var(eps),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                    PolynomialFactor::new(
                        Atom::num(2) * Atom::var(a) + Atom::var(x),
                        Atom::var(eps),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Positive),
                    PolynomialFactor::new(Atom::one(), Atom::one(), FactorRole::Polynomial),
                ],
            )],
        )
        .unwrap(),
    );
    let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 0).unwrap());
    let p = Problem {
        variables: vec!["x0".into(), "x1".into()],
        constraints: vec!["x0>0".into(), "x1>0".into()],
        split: vec![if zero_boundary {
            "x1-x0".into()
        } else {
            "-x1-x0".into()
        }],
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
    let b = Budget::new(Default::default());
    let prefix = NormalizedPrefix::new(map, &b).unwrap().open(&b).unwrap();
    EndpointTransport::prepare(
        prefix,
        &[FaceAxis {
            unit: u,
            side: if zero_boundary {
                FaceSide::Lower
            } else {
                FaceSide::Upper
            },
        }],
        &super::super::Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap()
}
fn same(a: &Atom, b: &Atom) {
    assert!((a - b).expand().is_zero(), "{a} != {b}");
}
#[test]
fn closed_scalar_retains_full_regulator_phase_and_source_identity() {
    let transport = source(false);
    assert_eq!(transport.charts().len(), 1);
    let chart = ClosedScalarChart::prepare(
        transport.issued_chart(0).unwrap(),
        &Limits::default(),
        OptimizationSettings::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(Arc::ptr_eq(chart.chart().transport(), &transport));
    assert_eq!(chart.terms().len(), 1);
    let eps = chart.regulator();
    let c = chart.chart().target_units()[0];
    let term = &chart.terms()[0];
    assert_eq!(term.source_index(), 0);
    assert!(term.powers().iter().all(|p| p.is_zero()));
    assert!(chart.strip().lower().is_none() && chart.strip().upper().is_none());
    assert!(term.prefactor().get_all_symbols(true).contains(&eps));
    assert_eq!(chart.prefactor_witness().gamma_witnesses().len(), 1);
    // The full regular density is (1+c)^epsilon. Its constant and derivative
    // are checked at epsilon=0 only after the complete family was issued.
    let zero = |p: &Atom| pull(p, &[eps], &[Atom::zero()]);
    same(&zero(term.regular_body()), &Atom::one());
    same(
        &zero(&term.regular_body().derivative(eps)),
        &(Atom::one() + Atom::var(c)).log(),
    );
    let b = Budget::new(Default::default());
    for unit in chart.units() {
        unit.numerator_proof()
            .replay(chart.proof_box(), &b)
            .unwrap();
        unit.denominator_proof()
            .replay(chart.proof_box(), &b)
            .unwrap();
    }
    let (mn, md) = chart.measure_proofs();
    mn.replay(chart.proof_box(), &b).unwrap();
    md.replay(chart.proof_box(), &b).unwrap();
    // Compare the issued factorization with all source coordinates/factors at
    // a nonzero regulator, retaining the causal phase once.
    let owner = transport.parent_prefix().source().decomposition();
    let original = &owner.preparation().terms()[0];
    let image = chart.chart().chart().source_images()[0].clone();
    let point = Rational::from((1, 3));
    let f = Atom::one() + &image;
    let u = Atom::num(2) + &image;
    let phase = causal_negative_phase(original.factors()[0].exponent(), &[eps]).unwrap();
    let exact = original.prefactor()
        * phase
        * f.pow(original.factors()[0].exponent())
        * u.pow(original.factors()[1].exponent())
        * chart.chart().chart().extra_jacobian()
        * pull(
            &transport.parent_prefix().diagonal_product(),
            transport.parent_prefix().source().unit_coordinates(),
            chart.chart().chart().unit_images(),
        );
    let issued = term.prefactor() * term.regular_body();
    let values = [Atom::num(point), Atom::one()];
    same(
        &pull(&issued, &[c, eps], &values),
        &pull(&exact, &[c, eps], &values),
    );
}
#[test]
fn closed_scalar_refuses_unresolved_zero_and_transactional_limits() {
    let pending = source(true);
    assert!(matches!(
        ClosedScalarChart::prepare(
            pending.issued_chart(0).unwrap(),
            &Limits::default(),
            OptimizationSettings::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::Unsupported(_))
    ));
    // The opposite endpoint orientation actually extracts that zero. It is
    // admissible as a full epsilon family requiring symbolic subtraction;
    // retaining epsilon is essential to its nonempty initial strip.
    let resolved = EndpointTransport::prepare(
        pending.parent_prefix().clone(),
        &[FaceAxis {
            unit: pending.parent_prefix().source().unit_coordinates()[0],
            side: FaceSide::Upper,
        }],
        &super::super::Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let closed = ClosedScalarChart::prepare(
        resolved.issued_chart(0).unwrap(),
        &Limits::default(),
        OptimizationSettings::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(closed.strip().upper(), Some(&Rational::from(-1)));
    assert!(closed.strip().lower().is_none());
    same(
        &closed.terms()[0].powers()[0],
        &(-Atom::num(2) - Atom::var(closed.regulator())),
    );
    let transport = source(false);
    let view = transport.issued_chart(0).unwrap();
    assert!(matches!(
        ClosedScalarChart::prepare(
            view.clone(),
            &Limits {
                max_terms: 0,
                ..Limits::default()
            },
            OptimizationSettings::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        ClosedScalarChart::prepare(
            view,
            &Limits::default(),
            OptimizationSettings::default(),
            || ControlFlow::Break(())
        ),
        Err(Error::Cancelled)
    ));
}
