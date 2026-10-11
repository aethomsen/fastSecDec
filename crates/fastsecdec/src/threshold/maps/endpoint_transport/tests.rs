use super::*;
use crate::{
    parametric::{FactorSemantics, ParametricIntegrand, ParametricTerm, PolynomialFactor},
    threshold::{
        gcad::{Limits as GcadLimits, PositiveProjectiveQuotient, Problem, SolverOptions},
        maps::{
            PositiveProjectiveCellMap,
            normalized::{BracketProposal, CoefficientChart, SectionProposal},
        },
        projective::PositiveProjectivePreparation,
    },
};
use std::sync::OnceLock;
use symbolica::{
    atom::AtomCore,
    domains::{integer::Integer, rational::Rational},
    symbol,
};

/// Genuine nonlinear source, with CAD x,z,y deliberately differing from
/// preparation x,y,z and two distinct homogeneous physical term ledgers.
fn fixture() -> Arc<NormalizedPrefix> {
    static FIXTURE: OnceLock<Arc<NormalizedPrefix>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let (a, x, y, z, eps) = symbol!(
                "endpoint_test::a",
                "endpoint_test::x",
                "endpoint_test::y",
                "endpoint_test::z",
                "endpoint_test::eps"
            );
            let f = (Atom::var(a) - Atom::var(x))
                * (Atom::var(z).pow(2) - Atom::var(a) * Atom::var(x))
                * (Atom::var(y).pow(2) - Atom::var(a) * Atom::var(x) - Atom::var(z).pow(2));
            let positive = Atom::var(a) + Atom::var(x);
            let terms = vec![
                ParametricTerm::new(
                    Atom::num(2),
                    vec![Atom::zero(), Atom::one(), Atom::zero(), Atom::zero()],
                    vec![
                        PolynomialFactor::new(f.clone(), Atom::num(-1), FactorRole::Singularity)
                            .with_semantics(FactorSemantics::Causal),
                        PolynomialFactor::new(
                            positive.clone(),
                            Atom::zero(),
                            FactorRole::Singularity,
                        )
                        .with_semantics(FactorSemantics::Positive),
                        PolynomialFactor::new(Atom::one(), Atom::one(), FactorRole::Polynomial),
                        PolynomialFactor::new(
                            Atom::var(a) - Atom::one(),
                            Atom::zero(),
                            FactorRole::Singularity,
                        )
                        .with_semantics(FactorSemantics::Positive),
                    ],
                ),
                ParametricTerm::new(
                    Atom::num(3),
                    vec![Atom::zero(), Atom::zero(), Atom::num(2), Atom::zero()],
                    vec![
                        PolynomialFactor::new(f, Atom::num((-7, 5)), FactorRole::Singularity)
                            .with_semantics(FactorSemantics::Causal),
                        PolynomialFactor::new(positive, Atom::one(), FactorRole::Singularity)
                            .with_semantics(FactorSemantics::Positive),
                        PolynomialFactor::new(Atom::one(), Atom::zero(), FactorRole::Polynomial),
                    ],
                ),
            ];
            let input = Arc::new(
                ParametricIntegrand::new(
                    vec![a, x, y, z],
                    eps,
                    ParametricDomain::ProjectiveSimplex,
                    terms,
                )
                .unwrap(),
            );
            let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 0).unwrap());
            let p = Problem {
                variables: (0..4).map(|i| format!("x{i}")).collect(),
                constraints: (0..4).map(|i| format!("x{i}>0")).collect(),
                split: vec!["(x0-x1)*(x3^2-x0*x1)*(x2^2-x0*x1-x3^2)".into()],
                solver: SolverOptions {
                    homogeneous_anchor: Some("x0".into()),
                    homogeneous_block: (0..4).map(|i| format!("x{i}")).collect(),
                    ordering: "given".into(),
                    order: ["x0", "x1", "x3", "x2"].map(str::to_owned).to_vec(),
                    ..Default::default()
                },
                limits: GcadLimits {
                    workers: 1,
                    wall_time_secs: 10.,
                    memory_mib: 1024,
                    ..Default::default()
                },
                ..Default::default()
            };
            let owner = Arc::new(
                PositiveProjectiveQuotient::verify(
                    prep,
                    Arc::new(symgcad::solver::solve(&p).unwrap()),
                )
                .unwrap(),
            );
            let (u, v, t, p, h, z, w) = symbol!(
                "endpoint_test::u",
                "endpoint_test::v",
                "endpoint_test::t",
                "fastsecdec::endpoint_0::u0",
                "endpoint_test::h",
                "endpoint_test::Z",
                "endpoint_test::W"
            );
            let first = SectionProposal {
                chart: CoefficientChart {
                    parameters: vec![p],
                    bounds: vec![[Rational::zero(), Rational::one()]],
                    source_prefix: vec![Atom::var(p).pow(2)],
                    parameter_images: vec![Atom::var(u)],
                    witness: vec![Rational::from((1, 2))],
                },
                root_symbol: z,
                scale: Atom::var(p),
                removed_coefficient: Rational::one(),
                bracket: BracketProposal {
                    lower: Rational::from((1, 2)),
                    upper: Rational::from((3, 2)),
                    derivative_margin: Rational::from((1, 2)),
                },
            };
            let second = SectionProposal {
                chart: CoefficientChart {
                    parameters: vec![p, h],
                    bounds: vec![
                        [Rational::zero(), Rational::one()],
                        [Rational::zero(), Rational::from((3, 2))],
                    ],
                    source_prefix: vec![Atom::var(p).pow(2), Atom::var(p) * Atom::var(h)],
                    parameter_images: vec![Atom::var(u), Atom::var(v) * Atom::var(z)],
                    witness: vec![Rational::from((1, 2)), Rational::from((1, 4))],
                },
                root_symbol: w,
                scale: Atom::var(p),
                removed_coefficient: Rational::one(),
                bracket: BracketProposal {
                    lower: Rational::from((3, 4)),
                    upper: Rational::from(2),
                    derivative_margin: Rational::one(),
                },
            };
            let mut b = Budget::new(AlgebraLimits::default());
            for cell in 0..owner.cell_count() {
                let source =
                    PositiveProjectiveCellMap::new(owner.clone(), cell, vec![u, v, t]).unwrap();
                let initial = NormalizedPrefix::new(source, &b).unwrap();
                if let Ok(power) = initial.power(2, &b)
                    && let Ok(one) = power.section(first.clone(), &mut b)
                    && let Ok(all) = one.section(second.clone(), &mut b)
                {
                    return all;
                }
            }
            panic!("native full inventory lacks the intended issued nonlinear prefix")
        })
        .clone()
}
fn same(a: &Atom, b: &Atom) {
    assert!((a - b).together().is_zero(), "{a} != {b}");
}
fn reconstruct(f: &ExtractedFactor, units: &[Symbol]) -> Atom {
    units
        .iter()
        .zip(f.orders())
        .map(|(u, p)| Atom::var(*u).pow(Atom::num(p.clone())))
        .product::<Atom>()
        * f.numerator()
        / f.denominator()
}
fn issued(parent: Arc<NormalizedPrefix>, side: FaceSide) -> Arc<EndpointTransport> {
    let faces = parent
        .source()
        .unit_coordinates()
        .iter()
        .map(|u| FaceAxis { unit: *u, side })
        .collect::<Vec<_>>();
    EndpointTransport::prepare(parent, &faces, &Limits::default(), || {
        ControlFlow::Continue(())
    })
    .unwrap()
}
#[test]
fn endpoint_transport_nonlinear_named_source_and_complete_term_powers() {
    let parent = fixture();
    let transport = issued(parent.clone(), FaceSide::Lower);
    assert!(Arc::ptr_eq(transport.parent_prefix(), &parent));
    assert!(transport.transport_complete());
    assert!(!transport.charts().is_empty());
    let images = parent.images();
    assert_eq!(
        transport.source_images(),
        [images[0].clone(), images[2].clone(), images[1].clone()]
    );
    let preparation = parent.source().decomposition().preparation();
    assert_eq!(transport.term_links().len(), 2);
    assert_eq!(
        transport.term_links()[0].factors()[0],
        transport.term_links()[1].factors()[0]
    );
    assert_eq!(transport.term_links()[0].factors()[1], FactorLink::Inactive);
    assert_eq!(transport.term_links()[0].factors()[3], FactorLink::Inactive);
    assert!(preparation.terms()[0].factors()[3].polynomial().is_zero());
    assert!(
        !preparation.input().terms()[0].factors()[3]
            .polynomial()
            .is_zero()
    );
    assert!(
        transport
            .root_relations()
            .iter()
            .flat_map(|r| r.section().chart().parameters.iter())
            .all(|s| !transport.target_units().contains(s))
    );
    for (ordinal, chart) in transport.charts().iter().enumerate() {
        assert_eq!(chart.ordinal(), ordinal);
        assert_eq!(chart.terms().len(), 2);
        assert_eq!(chart.pending_face_generators(), 6);
        assert!(!chart.closed_faces_certified());
        for (relation, pulled) in transport
            .root_relations()
            .iter()
            .zip(chart.root_equations())
        {
            same(
                pulled,
                &pull(
                    relation.equation(),
                    parent.source().unit_coordinates(),
                    chart.unit_images(),
                ),
            );
        }
        for (i, identity) in transport.identities().iter().enumerate() {
            let original = identity.numerator() / identity.denominator();
            same(
                &reconstruct(&chart.factors()[i], transport.target_units()),
                &pull(
                    &original,
                    parent.source().unit_coordinates(),
                    chart.unit_images(),
                ),
            );
        }
        same(
            &reconstruct(chart.measure(), transport.target_units()),
            &(pull(
                &parent.diagonal_product(),
                parent.source().unit_coordinates(),
                chart.unit_images(),
            ) * chart.extra_jacobian()),
        );
        for (ti, term) in preparation.terms().iter().enumerate() {
            let links = &transport.term_links()[ti];
            let mut expected = chart
                .measure()
                .orders()
                .iter()
                .cloned()
                .map(Atom::num)
                .collect::<Vec<_>>();
            for (axis, expected_power) in expected.iter_mut().enumerate() {
                for (i, exponent) in term.monomial_powers().iter().enumerate() {
                    if let Some(link) = links.coordinates()[i] {
                        *expected_power +=
                            exponent * Atom::num(chart.factors()[link].orders()[axis].clone());
                    }
                }
                for (i, factor) in term.factors().iter().enumerate() {
                    if let Some(link) = links.factors()[i].identity() {
                        *expected_power += factor.exponent()
                            * Atom::num(chart.factors()[link].orders()[axis].clone());
                    }
                }
            }
            assert_eq!(chart.terms()[ti].source_index(), ti);
            for (a, b) in expected.iter().zip(chart.terms()[ti].powers()) {
                same(a, b);
            }
            let source = term.prefactor()
                * term
                    .monomial_powers()
                    .iter()
                    .zip(transport.source_images())
                    .map(|(e, x)| x.pow(e))
                    .product::<Atom>();
            assert!(!source.is_zero());
        }
        assert_ne!(chart.terms()[0].powers(), chart.terms()[1].powers());
    }
}
#[test]
fn endpoint_transport_upper_reflection_and_transactional_refusals() {
    let parent = fixture();
    let upper = issued(parent.clone(), FaceSide::Upper);
    let units = parent.source().unit_coordinates();
    for chart in upper.charts() {
        let native = &upper.geometry().sectors[chart.ordinal()];
        native.validate(ParametricDomain::UnitCube).unwrap();
        for (i, row) in native.exponent_matrix.iter().enumerate() {
            let distance = row
                .iter()
                .zip(upper.target_units())
                .map(|(e, u)| Atom::var(*u).pow(Atom::num(e.clone())))
                .product::<Atom>();
            same(&chart.unit_images()[i], &(Atom::one() - distance));
        }
    }
    let face = FaceAxis {
        unit: units[0],
        side: FaceSide::Lower,
    };
    assert!(matches!(
        EndpointTransport::prepare(parent.clone(), &[face], &Limits::default(), || {
            ControlFlow::Break(())
        }),
        Err(Error::Cancelled)
    ));
    assert!(
        EndpointTransport::prepare(parent.clone(), &[face, face], &Limits::default(), || {
            ControlFlow::Continue(())
        })
        .is_err()
    );
    assert!(
        EndpointTransport::prepare(
            parent.clone(),
            &[FaceAxis {
                unit: symbol!("endpoint_foreign::u"),
                ..face
            }],
            &Limits::default(),
            || ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(
        EndpointTransport::prepare(
            parent.previous().unwrap().clone(),
            &[face],
            &Limits::default(),
            || ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(matches!(
        EndpointTransport::prepare(
            parent.clone(),
            &[face],
            &Limits {
                max_entries: 1,
                ..Default::default()
            },
            || ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(parent.complete_open_map());
    assert!(issued(parent, FaceSide::Lower).transport_complete());
}
#[test]
fn endpoint_transport_native_clearing_and_minimum_control() {
    let (u, v) = symbol!("endpoint_clear::u", "endpoint_clear::v");
    let units = [u, v];
    let ring = Ring::new(units.to_vec()).unwrap();
    let limits = Limits::default();
    let mut callback = || ControlFlow::Continue(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    let expression = Atom::num((-3, 7))
        * (Atom::var(u).pow(2) + Atom::var(u).pow(4) * Atom::var(v))
        / (Atom::var(v) * (Atom::one() - Atom::var(v)).pow(2));
    let cleared = ring.clear(&expression, &units, &mut work).unwrap();
    assert_eq!(cleared.denominator_powers(), [[0, 0], [1, 2]]);
    same(&(cleared.numerator() / cleared.denominator()), &expression);
    let (extracted, minima) = ring
        .extract(
            cleared.numerator(),
            cleared.denominator(),
            &units,
            &mut work,
        )
        .unwrap();
    assert_eq!(minima, [2, 0]);
    assert_eq!(extracted.orders(), [Integer::from(2), Integer::from(-1)]);
    same(&reconstruct(&extracted, &units), &expression);
    assert!(matches!(
        ring.clear(
            &(Atom::one() / (Atom::one() + Atom::var(v))),
            &units,
            &mut work
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(
        ring.clear(&Atom::var(symbol!("symgcad::n0")), &units, &mut work)
            .is_err()
    );
    assert!(ring.clear(&Atom::zero(), &units, &mut work).is_err());
    let mut powers = vec![Atom::one()];
    assert!(crate::generation::add_scaled_powers(&mut powers, &Atom::one(), &[]).is_err());
    assert_eq!(powers, [Atom::one()]);
}

#[test]
fn endpoint_transport_rejects_incomplete_or_relabelled_native_inventory() {
    let parent = fixture();
    let limits = Limits::default();
    let replay = |owner: &EndpointTransport| {
        let mut callback = || ControlFlow::Continue(());
        let mut work = Work {
            limits: &limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut callback,
        };
        chart::prepare(owner, 0, &mut work)
    };
    let mut owner = Arc::try_unwrap(issued(parent.clone(), FaceSide::Lower)).unwrap();
    owner.geometry.sectors[0].factor_valuations[0][0] += Integer::from(1);
    assert!(replay(&owner).is_err());
    let mut owner = Arc::try_unwrap(issued(parent.clone(), FaceSide::Lower)).unwrap();
    owner.geometry.sectors[0].determinant *= Integer::from(2);
    assert!(replay(&owner).is_err());
    let mut owner = Arc::try_unwrap(issued(parent.clone(), FaceSide::Lower)).unwrap();
    owner.links[0].factors.pop();
    assert!(replay(&owner).is_err());
    let mut owner = Arc::try_unwrap(issued(parent.clone(), FaceSide::Lower)).unwrap();
    owner.links[0].factors[3] = FactorLink::LazyPolynomial;
    assert!(replay(&owner).is_err());
    let mut owner = Arc::try_unwrap(issued(parent.clone(), FaceSide::Lower)).unwrap();
    owner.links[0].coordinates[0] = None;
    assert!(replay(&owner).is_err());
    let mut owner = Arc::try_unwrap(issued(parent, FaceSide::Lower)).unwrap();
    owner.charts.pop();
    assert!(!owner.transport_complete());
}

#[test]
fn endpoint_transport_inverse_preflight_shares_bases_and_keeps_caps() {
    let (u, v) = symbol!("endpoint_inverse_control::u", "endpoint_inverse_control::v");
    let units = [u, v];
    let mut ring = Ring::new(units.to_vec()).unwrap();
    ring.reserve([symbol!("fastsecdec::endpoint_inverse_0::i0")].into_iter());
    let limits = Limits {
        algebra: AlgebraLimits {
            max_terms: 64,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut callback = || ControlFlow::Continue(());
    let mut work = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    let repeated = (1..=20u32)
        .map(|i| Atom::var(u).pow(i) / (Atom::one() - Atom::var(v)))
        .sum::<Atom>();
    let cleared = ring.clear(&repeated, &units, &mut work).unwrap();
    assert_eq!(cleared.denominator_powers(), [[0, 0], [0, 1]]);
    same(&(cleared.numerator() / cleared.denominator()), &repeated);
    let reversed = Atom::var(u) / (Atom::var(v) - Atom::one());
    let cleared = ring.clear(&reversed, &units, &mut work).unwrap();
    same(&(cleared.numerator() / cleared.denominator()), &reversed);
    assert_eq!(cleared.denominator_powers(), [[0, 0], [0, 1]]);
    let overflow = (Atom::one() - Atom::var(v)).pow(-65);
    assert!(matches!(
        ring.clear(&overflow, &units, &mut work),
        Err(Error::ResourceIncomplete(_))
    ));
    let degree = (Atom::one() - Atom::var(v)).pow(-257);
    assert!(matches!(
        ring.clear(&degree, &units, &mut work),
        Err(Error::ResourceIncomplete(_))
    ));
    let foreign = Atom::one() / (Atom::var(u) + Atom::var(v));
    assert!(matches!(
        ring.clear(&foreign, &units, &mut work),
        Err(Error::Unsupported(_))
    ));
    let mut calls = 0;
    let mut stop = || {
        calls += 1;
        if calls > 5 {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    let mut cancelled = Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut stop,
    };
    assert!(matches!(
        ring.clear(&repeated, &units, &mut cancelled),
        Err(Error::Cancelled)
    ));
}
