use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricIntegrand, ParametricTerm, PolynomialFactor,
    },
    threshold::{
        gcad::{Limits as GcadLimits, PositiveProjectiveQuotient, Problem, SolverOptions},
        maps::{
            PositiveProjectiveCellMap,
            endpoint_transport::{
                EndpointTransport, FaceAxis, FaceSide, Limits as TransportLimits,
            },
            normalized::{BracketProposal, CoefficientChart, NormalizedPrefix, SectionProposal},
        },
        projective::PositiveProjectivePreparation,
    },
};
use fastsecdec_sectors::ParametricDomain;
use std::sync::{Arc, OnceLock};
use symbolica::{
    atom::{Atom, AtomCore},
    domains::rational::Rational,
    function, symbol,
};

/// Actual native cell: x in (0,1), y below sqrt(x+x^2), z a positive ray.
/// Preparation x,z,y and CAD x,y,z deliberately differ. No supplied selector.
pub(crate) fn fixture() -> Arc<EndpointTransport> {
    static FIXTURE: OnceLock<Arc<EndpointTransport>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let (a, x, z, y, eps) = symbol!(
                "numerator_face_test::a",
                "numerator_face_test::x",
                "numerator_face_test::z",
                "numerator_face_test::y",
                "numerator_face_test::eps"
            );
            let f = (Atom::var(a) - Atom::var(x))
                * (Atom::var(y).pow(2) - Atom::var(a) * Atom::var(x) - Atom::var(x).pow(2));
            let n = Atom::var(y) * Atom::var(z).pow(2)
                + Atom::var(a) * Atom::var(x) * Atom::var(z)
                + Atom::var(a).pow(3);
            let inactive = function!(symbol!("numerator_face_test::unused"), Atom::var(z));
            let term0 = ParametricTerm::new(
                Atom::num(2),
                vec![Atom::zero(); 4],
                vec![
                    PolynomialFactor::new(f.clone(), Atom::num((-7, 3)), FactorRole::Singularity)
                        .with_semantics(FactorSemantics::Causal),
                    PolynomialFactor::new(n.clone(), Atom::one(), FactorRole::Polynomial),
                    PolynomialFactor::new(inactive, Atom::zero(), FactorRole::Polynomial),
                    PolynomialFactor::new(Atom::zero(), Atom::zero(), FactorRole::Polynomial),
                ],
            );
            let term1 = ParametricTerm::new(
                Atom::num(3),
                vec![Atom::zero(), Atom::one(), Atom::zero(), Atom::zero()],
                vec![
                    PolynomialFactor::new(f, Atom::num(-4), FactorRole::Singularity)
                        .with_semantics(FactorSemantics::Causal),
                    PolynomialFactor::new(n, Atom::num(2), FactorRole::Polynomial),
                    PolynomialFactor::new(
                        Atom::var(eps) * Atom::var(z) + Atom::var(a),
                        Atom::one(),
                        FactorRole::Polynomial,
                    ),
                ],
            );
            let input = Arc::new(
                ParametricIntegrand::new(
                    vec![a, x, z, y],
                    eps,
                    ParametricDomain::ProjectiveSimplex,
                    vec![term0, term1],
                )
                .unwrap(),
            );
            let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 0).unwrap());
            let problem = Problem {
                variables: (0..4).map(|i| format!("x{i}")).collect(),
                constraints: (0..4).map(|i| format!("x{i}>0")).collect(),
                split: vec!["(x0-x1)*(x3^2-x0*x1-x1^2)".into()],
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
                    Arc::new(symgcad::solver::solve(&problem).unwrap()),
                )
                .unwrap(),
            );
            let (u, v, t, p, zroot) = symbol!(
                "numerator_face_test::u",
                "numerator_face_test::v",
                "numerator_face_test::t",
                "numerator_face_test::p",
                "numerator_face_test::Z"
            );
            let proposal = SectionProposal {
                chart: CoefficientChart {
                    parameters: vec![p],
                    bounds: vec![[Rational::zero(), Rational::one()]],
                    source_prefix: vec![Atom::var(p).pow(2)],
                    parameter_images: vec![Atom::var(u)],
                    witness: vec![Rational::from((1, 2))],
                },
                root_symbol: zroot,
                scale: Atom::var(p),
                removed_coefficient: Rational::one(),
                bracket: BracketProposal {
                    lower: Rational::from((1, 2)),
                    upper: Rational::from((3, 2)),
                    derivative_margin: Rational::one(),
                },
            };
            let mut budget = Budget::new(AlgebraLimits::default());
            for cell in 0..owner.cell_count() {
                let source =
                    PositiveProjectiveCellMap::new(owner.clone(), cell, vec![u, v, t]).unwrap();
                let initial = NormalizedPrefix::new(source, &budget).unwrap();
                if let Ok(one) = initial.power(2, &budget)
                    && let Ok(two) = one.section(proposal.clone(), &mut budget)
                    && let Ok(all) = two.open(&budget)
                {
                    return EndpointTransport::prepare(
                        all,
                        &[FaceAxis {
                            unit: t,
                            side: FaceSide::Upper,
                        }],
                        &TransportLimits::default(),
                        || ControlFlow::Continue(()),
                    )
                    .unwrap();
                }
            }
            panic!("actual native inventory lacks the nonlinear-prefix ray cell")
        })
        .clone()
}
pub(crate) fn issued_face() -> Arc<NumeratorFace> {
    let transport = fixture();
    let catalogue = SourceNumerators::prepare(
        transport
            .parent_prefix()
            .source()
            .decomposition()
            .preparation()
            .clone(),
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    NumeratorFace::prepare(
        catalogue,
        transport.clone(),
        0,
        transport.target_units()[2],
        2,
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap()
}
fn equal(a: &Atom, b: &Atom) {
    assert!((a - b).together().is_zero(), "{a} != {b}");
}

#[test]
fn numerator_face_native_geometry_cancels_before_compact_series() {
    use super::super::clearing::Ring;
    use symbolica::poly::series::SeriesDepth;
    let c = symbol!("numerator_face_test::canonical_c");
    let ca = Atom::var(c);
    let nested = Atom::one() - (Atom::one() - &ca);
    let original = (Atom::one() - ca.clone().pow(2)) / ((Atom::one() - &ca) * nested);
    let limits = TransportLimits::default();
    let mut callback = || ControlFlow::Continue(());
    let mut work = super::super::Work {
        limits: &limits,
        budget: Budget::new(limits.algebra.clone()),
        callback: &mut callback,
    };
    let ring = Ring::new(vec![c]).unwrap();
    let identity = super::face::canonical_image(&ring, &original, &[c], &mut work).unwrap();
    equal(identity.numerator(), &(Atom::one() + &ca));
    equal(identity.denominator(), &ca);
    assert_eq!(identity.denominator_powers(), &[[1, 0]]);
    let canonical = identity.numerator() / identity.denominator();
    equal(&canonical, &original);
    let series = (&ca * canonical)
        .series(c, Atom::zero(), SeriesDepth::relative(2))
        .unwrap();
    equal(&series.coefficient(Rational::zero()).unwrap(), &Atom::one());
    equal(&series.coefficient(Rational::one()).unwrap(), &Atom::one());
    // Actual infinity charts use products of distance roles. Native expansion
    // must normalize nested complements and product inverses before admission.
    let b = symbol!("numerator_face_test::canonical_b");
    let ba = Atom::var(b);
    let bc = &ba * &ca;
    let ray = (Atom::one() - &bc) / (Atom::one() - (Atom::one() - &bc));
    let product_ring = Ring::new(vec![b, c]).unwrap();
    let image = super::face::canonical_image(&product_ring, &ray, &[b, c], &mut work).unwrap();
    equal(image.denominator(), &bc);
    assert_eq!(image.denominator_powers(), &[[1, 0], [1, 0]]);
    equal(&(image.numerator() / image.denominator()), &ray);
    assert!(matches!(
        super::face::canonical_image(&ring, &(Atom::one() / (Atom::one() - &ca)), &[c], &mut work,),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn numerator_face_actual_source_degrees_coefficients_and_specialization() {
    let face = issued_face();
    let preparation = face.catalogue().preparation();
    let [x, z, y]: [_; 3] = preparation.coordinates().try_into().unwrap();
    assert_eq!(face.finite_coordinates(), [x, y]);
    assert_eq!(face.transport().charts().len(), 1);
    assert!(Arc::ptr_eq(
        face.catalogue().preparation(),
        face.transport()
            .parent_prefix()
            .source()
            .decomposition()
            .preparation()
    ));
    assert_eq!(face.catalogue().links().len(), 2);
    let NumeratorLink::Active { record, exponent } = face.catalogue().links()[0][1] else {
        panic!()
    };
    assert_eq!(exponent, 1);
    assert_eq!(
        face.catalogue().records()[record].degrees().unwrap(),
        [1, 2, 1]
    );
    let c = &face.coefficients()[face.record_to_coefficient()[record].unwrap()];
    assert_eq!(
        c.normal_order(),
        &symbolica::domains::integer::Integer::from(-2)
    );
    assert_eq!(
        c.cleared_order(),
        &symbolica::domains::integer::Integer::from(0)
    );
    equal(&c.coefficients()[0], &Atom::var(y));
    equal(
        &c.coefficients()[1],
        &(Atom::var(x) - Atom::num(2) * Atom::var(y)),
    );
    let NumeratorLink::Active {
        record: second,
        exponent,
    } = face.catalogue().links()[1][2]
    else {
        panic!()
    };
    assert_eq!(exponent, 1);
    let c2 = &face.coefficients()[face.record_to_coefficient()[second].unwrap()];
    let eps = preparation.input().regulator();
    assert_eq!(
        face.catalogue().records()[second].degrees().unwrap(),
        [0, 1, 0]
    );
    equal(&c2.coefficients()[0], &Atom::var(eps));
    equal(&c2.coefficients()[1], &(Atom::one() - Atom::var(eps)));
    let specialized = super::super::clearing::pull(&c2.coefficients()[0], &[eps], &[Atom::zero()]);
    assert!(specialized.is_zero());
    assert_eq!(
        c2.normal_order(),
        &symbolica::domains::integer::Integer::from(-1)
    );
    assert!(!c2.exact_zero());
    for fi in [2usize, 3] {
        let NumeratorLink::Inactive { record } = face.catalogue().links()[0][fi] else {
            panic!()
        };
        assert!(face.catalogue().records()[record].degrees().is_none());
        assert!(face.record_to_coefficient()[record].is_none());
    }
    assert_eq!(face.term_powers().len(), 2);
    equal(&face.term_powers()[0][2], &Atom::num(-4));
    equal(&face.term_powers()[1][2], &Atom::num(-7));
    assert!(!face.closed_density_certified());
    assert!(!face.transport().charts()[0].closed_faces_certified());
    assert!(matches!(
        face.source_partial(0, 1, z, &Budget::new(AlgebraLimits::default())),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn numerator_face_source_partial_is_not_composed_chain_derivative() {
    let face = issued_face();
    let coords = face.finite_coordinates();
    let units = face.transport().target_units();
    let [u, v] = [units[0], units[1]];
    let NumeratorLink::Active { record, .. } = face.catalogue().links()[0][1] else {
        panic!()
    };
    let index = face.record_to_coefficient()[record].unwrap();
    let budget = Budget::new(AlgebraLimits::default());
    equal(
        &face.source_partial(index, 1, coords[0], &budget).unwrap(),
        &Atom::one(),
    );
    equal(
        &face.source_partial(index, 1, coords[1], &budget).unwrap(),
        &Atom::num(-2),
    );
    // Independent native closed-form reference for this actual quadratic root.
    // This expression is a test oracle, not a replacement root owner.
    let z = (Atom::one() + Atom::var(u).pow(2)).pow(Atom::num((1, 2)));
    let physical = super::super::clearing::pull(
        &face.coefficients()[index].coefficients()[1],
        coords,
        &[Atom::var(u).pow(2), Atom::var(u) * Atom::var(v) * &z],
    );
    let derivative = physical.derivative(u);
    let expected =
        Atom::num(2) * Atom::var(u) - Atom::num(2) * Atom::var(v) * (&z + Atom::var(u).pow(2) / &z);
    equal(&derivative, &expected);
    assert!(!(derivative - Atom::one()).expand().is_zero());
}

#[test]
fn numerator_face_source_frame_root_and_resource_refusals_are_atomic() {
    let transport = fixture();
    let prep = transport
        .parent_prefix()
        .source()
        .decomposition()
        .preparation()
        .clone();
    let catalogue = SourceNumerators::prepare(prep.clone(), &Limits::default(), || {
        ControlFlow::Continue(())
    })
    .unwrap();
    let foreign = Arc::new(
        PositiveProjectivePreparation::anchor(prep.input().clone(), prep.anchor_index()).unwrap(),
    );
    let foreign =
        SourceNumerators::prepare(foreign, &Limits::default(), || ControlFlow::Continue(()))
            .unwrap();
    let normal = transport.target_units()[2];
    assert!(matches!(
        NumeratorFace::prepare(
            foreign,
            transport.clone(),
            0,
            normal,
            2,
            &Limits::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
    for (index, normal) in [
        (99, normal),
        (0, symbol!("numerator_face_test::foreign_normal")),
    ] {
        assert!(matches!(
            NumeratorFace::prepare(
                catalogue.clone(),
                transport.clone(),
                index,
                normal,
                2,
                &Limits::default(),
                || ControlFlow::Continue(())
            ),
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        NumeratorFace::prepare(
            catalogue.clone(),
            transport.clone(),
            0,
            transport.target_units()[0],
            2,
            &Limits::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        NumeratorFace::prepare(
            catalogue.clone(),
            transport.clone(),
            0,
            normal,
            2,
            &Limits::default(),
            || ControlFlow::Break(())
        ),
        Err(Error::Cancelled)
    ));
    let small = Limits {
        max_coefficient_nodes: 0,
        ..Default::default()
    };
    assert!(matches!(
        NumeratorFace::prepare(
            catalogue.clone(),
            transport.clone(),
            0,
            normal,
            2,
            &small,
            || ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        SourceNumerators::prepare(
            prep,
            &Limits {
                max_source_nodes: 0,
                ..Default::default()
            },
            || ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    let face = NumeratorFace::prepare(
        catalogue,
        transport.clone(),
        0,
        normal,
        2,
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(face.term_powers().len(), 2);
    assert!(transport.transport_complete());
}
