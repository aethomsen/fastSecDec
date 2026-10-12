use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::gcad::PositiveProjectiveRequest,
};
use symbolica::symbol;
fn owner(input: Arc<ParametricIntegrand>) -> Arc<PositiveProjectiveQuotient> {
    let request = PositiveProjectiveRequest::exact(
        input,
        2,
        symgcad::SolverOptions {
            ordering: "given".into(),
            order: vec!["x2".into(), "x0".into(), "x1".into()],
            ..Default::default()
        },
        symgcad::Limits {
            workers: 1,
            wall_time_secs: 30.,
            memory_mib: 1024,
            ..Default::default()
        },
    )
    .unwrap();
    Arc::new(request.verify(Arc::new(request.solve().unwrap())).unwrap())
}
#[test]
fn automatic_algebraic_family_covers_every_source_cell_and_continues() {
    let owner = super::super::tests::fixture(None, Atom::one());
    let units = super::super::tests::coordinates();
    let limits = super::super::tests::limits();
    let issued = discover(
        owner.clone(),
        units,
        limits,
        Default::default(),
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(Arc::ptr_eq(issued.family.owner(), &owner));
    assert_eq!(
        issued
            .proposals
            .iter()
            .map(|p| p.cell)
            .collect::<BTreeSet<_>>(),
        (0..owner.cell_count()).collect()
    );
    assert_eq!(
        issued
            .family
            .charts()
            .iter()
            .map(|c| c.cell_index())
            .collect::<BTreeSet<_>>(),
        (0..owner.cell_count()).collect()
    );
    assert!(!issued.misses.is_empty());
    assert!(
        issued
            .misses
            .iter()
            .all(|m| matches!(m.error, Error::Unsupported(_) | Error::Invalid(_)))
    );
    let continued = issued
        .family
        .continue_symbolically(
            &crate::generation::GenerationOptions {
                max_order: 0,
                subtraction: crate::generation::SubtractionStrategy::Taylor,
                ..Default::default()
            },
            |_| ControlFlow::Continue(()),
        )
        .unwrap();
    assert_eq!(continued.charts().len(), issued.family.charts().len());
    assert_eq!(continued.profiles().len(), issued.family.charts().len());
}
#[test]
fn unrelated_one_cell_positive_source_has_no_quadratic_or_seven_cell_assumption() {
    let (x, y, z, eps) = symbol!(
        "discovery_positive::x",
        "discovery_positive::y",
        "discovery_positive::z",
        "discovery_positive::eps"
    );
    let input = Arc::new(
        ParametricIntegrand::new(
            vec![x, y, z],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 3],
                vec![
                    PolynomialFactor::new(
                        Atom::var(x) + Atom::var(y) + Atom::var(z),
                        Atom::num(-3),
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Positive),
                ],
            )],
        )
        .unwrap(),
    );
    let owner = owner(input);
    assert_eq!(owner.cell_count(), 1);
    let result = discover(
        owner.clone(),
        vec![
            symbol!("discovery_positive::u"),
            symbol!("discovery_positive::v"),
        ],
        super::super::tests::limits(),
        Default::default(),
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(result.misses.is_empty());
    assert_eq!(result.proposals.len(), owner.cell_count());
    assert!(
        result
            .family
            .charts()
            .iter()
            .all(|c| c.sections().is_empty())
    );
}
#[test]
fn cancellation_and_resource_caps_never_become_partial_family() {
    let owner = super::super::tests::fixture(None, Atom::one());
    let units = super::super::tests::coordinates();
    let limits = super::super::tests::limits();
    assert!(matches!(
        discover(
            owner.clone(),
            units.clone(),
            limits.clone(),
            Default::default(),
            Default::default(),
            |_| ControlFlow::Break(())
        ),
        Err(Error::Cancelled)
    ));
    let mut low = limits.clone();
    low.algebra.wall_time = Duration::ZERO;
    assert!(matches!(
        discover(
            owner.clone(),
            units.clone(),
            low,
            Default::default(),
            Default::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    let options = DiscoveryOptions {
        max_attempts: 1,
        ..Default::default()
    };
    assert!(
        matches!(discover(owner,units,limits,options,Default::default(),|_|ControlFlow::Continue(())),Err(Error::ResourceIncomplete(message))if message.contains("candidate attempt"))
    );
}
#[test]
fn higher_degree_automatic_class_refuses_without_weakening_quartic_issuer() {
    let (x, y, z, eps) = symbol!(
        "discovery_quartic::x",
        "discovery_quartic::y",
        "discovery_quartic::z",
        "discovery_quartic::eps"
    );
    let [xx, yy, zz] = [x, y, z].map(Atom::var);
    let e = Atom::var(eps);
    let p = yy.pow(4) + yy.pow(2) * zz.pow(2) - xx.pow(2) * &zz * (Atom::num(2) * &zz + &xx);
    let factors = vec![
        PolynomialFactor::new(
            &xx + &yy + &zz,
            Atom::num(-8) + Atom::num(5) * &e,
            FactorRole::Singularity,
        )
        .with_semantics(FactorSemantics::Positive),
        PolynomialFactor::new(p, Atom::one() - &e, FactorRole::Singularity)
            .with_semantics(FactorSemantics::Causal),
        PolynomialFactor::new(&xx - &zz, Atom::one() - e, FactorRole::Singularity)
            .with_semantics(FactorSemantics::Causal),
    ];
    let input = Arc::new(
        ParametricIntegrand::new(
            vec![x, y, z],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 3],
                factors,
            )],
        )
        .unwrap(),
    );
    let owner = owner(input);
    let units = vec![
        symbol!("discovery_quartic::u"),
        symbol!("discovery_quartic::v"),
    ];
    assert!(
        matches!(discover(owner,units,super::super::tests::limits(),Default::default(),Default::default(),|_|ControlFlow::Continue(())),Err(Error::Unsupported(message))if message.contains("quadratic"))
    );
    // The separate existing genuine quartic constructor control is run in the
    // same test group; no automatic refusal changes its admitted API or bytes.
}
