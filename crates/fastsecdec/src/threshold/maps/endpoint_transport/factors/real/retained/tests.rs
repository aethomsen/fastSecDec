use super::*;
use crate::threshold::maps::regular::Limits;
use symbolica::symbol;
use symgcad::domain::certify_nonzero;
fn fixture() -> (ProofBox, Atom, RetainedMargin) {
    let (x, z, unused) = symbol!(
        "retained_margin_test::x",
        "retained_margin_test::z",
        "retained_margin_test::unused"
    );
    let b = Budget::new(Limits::default());
    let roles = vec![x, z, unused];
    let domain = ProofBox::from_bounds(
        roles.clone(),
        vec![
            [Rational::zero(), Rational::one()],
            [Rational::one(), Rational::from(2)],
            [Rational::zero(), Rational::one()],
        ],
        &b,
    )
    .unwrap();
    let names = ["old_direction".into(), "selected_Z".into(), "unused".into()];
    let ring = Ring::with_names(roles.clone(), &names).unwrap();
    let constraints: Vec<String> = vec![
        "old_direction+1".into(),
        "2-old_direction".into(),
        "selected_Z-1/2".into(),
        "3-selected_Z".into(),
    ];
    let cs = constraints
        .iter()
        .map(|s| ring.native.parse(s).unwrap())
        .collect::<Vec<_>>();
    let d = derive_domain(&ring.native, &cs, &[]).unwrap();
    let polynomial = Atom::num(2) + Atom::var(x).pow(2) + Atom::var(z);
    let margin = Rational::from((1, 4));
    let target = ring
        .polynomial(&(&polynomial - Atom::num(margin.clone())), &b)
        .unwrap();
    let cert = certify_nonzero(&ring.native, &target, &d, &Default::default())
        .unwrap()
        .unwrap();
    let bytes = serde_json::to_vec_pretty(&cert).unwrap();
    let retained =
        RetainedMargin::from_json(roles, constraints, margin, bytes.into(), 1_000_000).unwrap();
    (domain, polynomial, retained)
}
#[test]
fn retained_native_frame_and_unrestricted_role_replay() {
    let (domain, p, proposal) = fixture();
    let bytes = proposal.original_bytes().to_vec();
    let b = Budget::new(Limits::default());
    let proof = domain.admit_retained(&p, Arc::new(proposal), &b).unwrap();
    proof.replay(&domain, &b).unwrap();
    let MarginEvidence::Retained(original) = proof.evidence() else {
        panic!("retained evidence")
    };
    assert_eq!(original.original_bytes(), bytes);
    assert_eq!(original.certificate().domain.variables[0], "old_direction");
    assert!(original.certificate().domain.bounds[2].lower.is_none());
}
#[test]
fn retained_target_role_domain_and_margin_mismatches_refuse() {
    let (domain, p, proposal) = fixture();
    let proposal = Arc::new(proposal);
    let b = Budget::new(Limits::default());
    assert!(
        domain
            .admit_retained(&(&p + Atom::one()), proposal.clone(), &b)
            .is_err()
    );
    assert!(domain.admit_retained(&(-&p), proposal.clone(), &b).is_err());
    let mut roles = domain.roles.clone();
    roles[0] = symbol!("retained_margin_test::foreign");
    let foreign = ProofBox::from_bounds(roles, domain.bounds.clone(), &b).unwrap();
    assert!(foreign.admit_retained(&p, proposal.clone(), &b).is_err());
    let mut bounds = domain.bounds.clone();
    bounds[0][0] = Rational::from(-2);
    let wider = ProofBox::from_bounds(domain.roles.clone(), bounds, &b).unwrap();
    assert!(wider.admit_retained(&p, proposal.clone(), &b).is_err());
    let changed = RetainedMargin::from_json(
        proposal.roles.clone(),
        proposal.constraints.clone(),
        Rational::from((1, 8)),
        proposal.bytes.clone(),
        1_000_000,
    )
    .unwrap();
    assert!(domain.admit_retained(&p, Arc::new(changed), &b).is_err());
    let mut constraints = proposal.constraints.clone();
    constraints.swap(0, 1);
    let reordered = RetainedMargin::from_json(
        proposal.roles.clone(),
        constraints,
        proposal.margin.clone(),
        proposal.bytes.clone(),
        1_000_000,
    )
    .unwrap();
    assert!(domain.admit_retained(&p, Arc::new(reordered), &b).is_err());
}
#[test]
fn retained_bytes_certificate_and_native_allowance_refuse() {
    let (domain, p, proposal) = fixture();
    assert!(matches!(
        RetainedMargin::from_json(
            proposal.roles.clone(),
            proposal.constraints.clone(),
            proposal.margin.clone(),
            proposal.bytes.clone(),
            0
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    let mut wrong = proposal.certificate.clone();
    wrong.sign = -1;
    let changed = RetainedMargin::from_json(
        proposal.roles.clone(),
        proposal.constraints.clone(),
        proposal.margin.clone(),
        serde_json::to_vec(&wrong).unwrap().into(),
        1_000_000,
    )
    .unwrap();
    assert!(
        domain
            .admit_retained(&p, Arc::new(changed), &Budget::new(Limits::default()))
            .is_err()
    );
    let limits = Limits {
        certificates: CertificateOptions {
            work_limit: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(matches!(
        domain.admit_retained(&p, Arc::new(proposal), &Budget::new(limits)),
        Err(Error::ResourceIncomplete(_))
    ));
}

#[test]
fn retained_positive_qq_normalization_replays_original_target_and_bytes() {
    let (domain, p, proposal) = fixture();
    let bytes = proposal.original_bytes().to_vec();
    let original_target = proposal.certificate().polynomial.clone();
    let scale = Rational::from((1, 256));
    let actual = Atom::num(scale.clone()) * &p;
    let proposal = proposal.with_positive_scale(scale.clone()).unwrap();
    let b = Budget::new(Limits::default());
    let proof = domain
        .admit_retained(&actual, Arc::new(proposal), &b)
        .unwrap();
    assert_eq!(proof.polynomial(), &actual);
    assert_eq!(proof.margin(), &Rational::from((1, 1024)));
    proof.replay(&domain, &b).unwrap();
    let MarginEvidence::Retained(original) = proof.evidence() else {
        panic!("retained evidence")
    };
    assert_eq!(original.scale(), &scale);
    assert_eq!(original.margin(), &Rational::from((1, 4)));
    assert_eq!(original.original_bytes(), bytes);
    assert_eq!(original.certificate().polynomial, original_target);
    // The old direct-target contract correctly refuses normalized Q with scale1.
    let (_, _, unscaled) = fixture();
    assert!(
        domain
            .admit_retained(&actual, Arc::new(unscaled), &b)
            .is_err()
    );
}

#[test]
fn retained_scale_refuses_nonpositive_wrong_normalization_and_budget() {
    for scale in [Rational::zero(), Rational::from(-1)] {
        assert!(matches!(
            fixture().2.with_positive_scale(scale),
            Err(Error::Invalid(_))
        ));
    }
    let (domain, p, proposal) = fixture();
    let proposal = Arc::new(
        proposal
            .with_positive_scale(Rational::from((1, 256)))
            .unwrap(),
    );
    let b = Budget::new(Limits::default());
    let actual = Atom::num((1, 256)) * &p;
    for wrong in [&actual + Atom::one(), -&actual, Atom::num((1, 128)) * &p] {
        assert!(domain.admit_retained(&wrong, proposal.clone(), &b).is_err());
    }
    let mut roles = domain.roles.clone();
    roles[0] = symbol!("retained_margin_test::scaled_foreign");
    let foreign = ProofBox::from_bounds(roles, domain.bounds.clone(), &b).unwrap();
    assert!(
        foreign
            .admit_retained(&actual, proposal.clone(), &b)
            .is_err()
    );
    let mut bounds = domain.bounds.clone();
    bounds[0][0] = Rational::from(-2);
    let wider = ProofBox::from_bounds(domain.roles.clone(), bounds, &b).unwrap();
    assert!(wider.admit_retained(&actual, proposal.clone(), &b).is_err());
    // No unchecked huge rational coefficient escapes the existing budget.
    let tight = Budget::new(Limits {
        max_rational_bits: 4,
        ..Limits::default()
    });
    assert!(matches!(
        domain.admit_retained(&actual, proposal, &tight),
        Err(Error::ResourceIncomplete(_))
    ));
    let (domain, p, proposal) = fixture();
    let b = Budget::new(Limits::default())
        .with_cancellation(Arc::new(std::sync::atomic::AtomicBool::new(true)));
    assert!(matches!(
        domain.admit_retained(&p, Arc::new(proposal), &b),
        Err(Error::Cancelled)
    ));
}

use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{Limits as GL, PositiveProjectiveQuotient, Problem, SolverOptions},
        maps::{
            PositiveProjectiveCellMap,
            endpoint_transport::{
                EndpointTransport, FaceAxis, FaceSide,
                closed::{ClosedScalarChart, Limits as ClosedLimits},
            },
            normalized::NormalizedPrefix,
        },
        projective::PositiveProjectivePreparation,
    },
};
use std::ops::ControlFlow;
use symbolica::evaluate::OptimizationSettings;
// Reused genuine source fixture from endpoint_transport/closed/tests.rs.
fn genuine_source(zero_boundary: bool) -> Arc<EndpointTransport> {
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
        &crate::threshold::maps::endpoint_transport::Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap()
}
#[test]
fn genuine_closed_chart_consumes_scaled_evidence_without_losing_measure_or_phase() {
    let transport = genuine_source(false);
    let chart = transport.issued_chart(0).unwrap();
    assert_eq!(transport.identities().len(), 3);
    let roles = chart.target_units().to_vec();
    let names = vec!["saved_direction".into()];
    let ring = Ring::with_names(roles.clone(), &names).unwrap();
    let constraints = vec![
        "saved_direction+1/4".to_owned(),
        "5/4-saved_direction".to_owned(),
    ];
    let cs = constraints
        .iter()
        .map(|s| ring.native.parse(s).unwrap())
        .collect::<Vec<_>>();
    let domain = derive_domain(&ring.native, &cs, &[]).unwrap();
    let b = Budget::new(Limits::default());
    let mut calls = 0;
    let closed = ClosedScalarChart::prepare_with_unit_evidence(
        chart,
        &ClosedLimits::default(),
        OptimizationSettings::default(),
        || ControlFlow::Continue(()),
        |actual| {
            // Model unchanged saved integer-numerator normalization of the same
            // source-bound residual, before its exact positive denominator removal.
            let old = Atom::num(256) * actual;
            let margin = Rational::from((1, 4));
            let target = ring.polynomial(&(&old - Atom::num(margin.clone())), &b)?;
            let certificate = certify_nonzero(&ring.native, &target, &domain, &Default::default())
                .unwrap()
                .unwrap();
            let bytes = serde_json::to_vec_pretty(&certificate).unwrap();
            let proposal = RetainedMargin::from_json(
                roles.clone(),
                constraints.clone(),
                margin,
                bytes.clone().into(),
                1_000_000,
            )?
            .with_positive_scale(Rational::from((1, 256)))?;
            assert_eq!(proposal.original_bytes(), bytes);
            calls += 1;
            Ok(Some(Arc::new(proposal)))
        },
    )
    .unwrap();
    assert!(calls > 0);
    assert_eq!(closed.units().len(), 3);
    assert_eq!(closed.terms().len(), 1);
    assert_eq!(closed.terms()[0].source_index(), 0);
    assert!(Arc::ptr_eq(closed.chart().transport(), &transport));
    for unit in closed.units() {
        unit.numerator_proof()
            .replay(closed.proof_box(), &b)
            .unwrap();
        unit.denominator_proof()
            .replay(closed.proof_box(), &b)
            .unwrap();
    }
    let (n, d) = closed.measure_proofs();
    n.replay(closed.proof_box(), &b).unwrap();
    d.replay(closed.proof_box(), &b).unwrap();
    assert_eq!(closed.prefactor_witness().gamma_witnesses().len(), 1);
}
