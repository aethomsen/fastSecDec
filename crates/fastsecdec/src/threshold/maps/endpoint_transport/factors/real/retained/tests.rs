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
