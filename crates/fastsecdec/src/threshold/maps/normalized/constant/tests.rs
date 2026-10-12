use super::*;
use crate::{
    kernel::algebraic::{Number, RootProgram},
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadRequest, PositiveProjectiveQuotient, Problem, SolverOptions},
        maps::{
            endpoint_transport::{
                EndpointTransport, FaceAxis, FaceSide, Limits as TransportLimits,
            },
            normalized::{Limits, NormalizedPrefix, PrefixStep, program::CompiledOpenMap},
        },
        projective::PositiveProjectivePreparation,
    },
};
use std::{ops::ControlFlow, sync::atomic::AtomicBool};
use symbolica::{
    atom::AtomCore,
    domains::float::{ErrorPropagatingFloat as Epf, Float},
    evaluate::OptimizationSettings,
    symbol,
};
fn owner(degree: u32) -> Arc<PositiveProjectiveQuotient> {
    let (a, x, eps) = symbol!(
        "positive_constant_test::a",
        "positive_constant_test::x",
        "positive_constant_test::eps"
    );
    // Deliberately occupy the first callback-tag proposal in the genuine source.
    let occupied = symbol!("fastsecdec::positive_constant_call_0_0");
    let source = Arc::new(
        ParametricIntegrand::new(
            vec![x, a],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::var(occupied),
                vec![Atom::zero(); 2],
                [2, 3]
                    .map(|c| {
                        PolynomialFactor::new(
                            Atom::var(x).pow(degree) - Atom::num(c) * Atom::var(a).pow(degree),
                            Atom::num(Rational::from((-1i64, i64::from(degree)))),
                            FactorRole::Singularity,
                        )
                        .with_semantics(FactorSemantics::Causal)
                    })
                    .to_vec(),
            )],
        )
        .unwrap(),
    );
    let preparation = Arc::new(PositiveProjectivePreparation::anchor(source, 1).unwrap());
    let mut limits = GcadRequest::default_limits();
    limits.workers = 1;
    limits.memory_mib = 2048;
    limits.wall_time_secs = 30.;
    let p = Problem {
        variables: vec!["x0".into(), "x1".into()],
        constraints: vec!["x0>0".into(), "x1>0".into()],
        split: vec![
            format!("x0^{degree}-2*x1^{degree}"),
            format!("x0^{degree}-3*x1^{degree}"),
        ],
        solver: SolverOptions {
            homogeneous_anchor: Some("x1".into()),
            homogeneous_block: vec!["x0".into(), "x1".into()],
            order: vec!["x1".into(), "x0".into()],
            ordering: "given".into(),
            ..Default::default()
        },
        limits,
        ..Default::default()
    };
    Arc::new(
        PositiveProjectiveQuotient::verify(
            preparation,
            Arc::new(symgcad::solver::solve(&p).unwrap()),
        )
        .unwrap(),
    )
}
fn enclosure(endpoint: &ConstantEndpoint, bits: u32) -> [Rational; 2] {
    if let Some(q) = endpoint.rational() {
        return [q.clone(), q.clone()];
    }
    let mut root = endpoint.section().unwrap().anchor().clone();
    let tolerance = Rational::from((1, 2)).pow(u64::from(bits + 16));
    for _ in 0..4096 {
        if &root.interval.1 - &root.interval.0 < tolerance {
            return [root.interval.0, root.interval.1];
        }
        symgcad::roots::refine_once(&mut root);
    }
    panic!("native independent constant enclosure did not refine")
}
#[test]
fn positive_constant_degree2_and32_native_owner_codec_and_precisions() {
    for degree in [2, 32] {
        let source = owner(degree);
        let u = symbol!("positive_constant_test::u");
        let mut count = 0;
        for i in 0..source.cell_count() {
            let map = PositiveProjectiveCellMap::new(source.clone(), i, vec![u]).unwrap();
            if map.native_axes()[0]
                .lower
                .as_ref()
                .zip(map.native_axes()[0].upper.as_ref())
                .is_none()
            {
                continue;
            }
            let mut budget = Budget::new(Limits::default());
            let prefix = NormalizedPrefix::new(map.clone(), &budget).unwrap();
            let complete = prefix.constant_interval(2, &mut budget).unwrap();
            let PrefixStep::ConstantInterval {
                lower,
                upper,
                image,
                diagonal,
                ..
            } = complete.step().unwrap()
            else {
                panic!("issued interval")
            };
            assert_eq!(
                lower.original_bound(),
                map.native_axes()[0].lower.as_ref().unwrap()
            );
            assert_eq!(
                upper.original_bound(),
                map.native_axes()[0].upper.as_ref().unwrap()
            );
            assert!(lower.enclosure()[1] < upper.enclosure()[0]);
            assert!((image.derivative(u) - diagonal).expand().is_zero());
            for endpoint in [lower, upper] {
                if let Some(section) = endpoint.section() {
                    assert!(Arc::ptr_eq(section.source(), &map));
                    assert_eq!(section.coefficients().len() - 1, degree as usize);
                    assert_eq!(section.derivative_certificate().sign, 1);
                    let program = RootProgram::prepare_positive_constant(
                        section,
                        OptimizationSettings::default().cores(1),
                    )
                    .unwrap();
                    let restored = RootProgram::from_bytes(program.bytes()).unwrap();
                    assert_eq!(program.bytes(), restored.bytes());
                }
            }
            let compiled = CompiledOpenMap::prepare(
                complete.clone(),
                OptimizationSettings::default().cores(1),
                &budget,
            )
            .unwrap();
            let restored = compiled.native_roundtrip().unwrap();
            assert!(Arc::ptr_eq(restored.owner(), &complete));
            assert!(
                !restored
                    .exact()
                    .export_instructions()
                    .constant_functions
                    .is_empty()
            );
            for bits in [128, 256] {
                let lo = enclosure(lower, bits);
                let hi = enclosure(upper, bits);
                let mut eval = restored.map::<Float>(bits);
                let mut tracked = restored.map::<Epf<Float>>(bits);
                for q in [Rational::from((1, 4)), Rational::from((3, 4))] {
                    let t = &q * &q;
                    let factor = Rational::from(2) * &q;
                    let expected = [
                        [
                            (Rational::one() - &t) * &lo[0] + &t * &hi[0],
                            (Rational::one() - &t) * &lo[1] + &t * &hi[1],
                        ],
                        [&factor * &(&hi[0] - &lo[1]), &factor * &(&hi[1] - &lo[0])],
                    ];
                    let input = Float::rational_at(&q, bits);
                    let values = eval.evaluate(std::slice::from_ref(&input)).unwrap();
                    let tol = Rational::from((1, 2)).pow(u64::from(bits - 12));
                    for (v, [l, h]) in values.iter().zip(expected) {
                        let v = v.to_rational();
                        assert!(v >= l - &tol && v <= h + &tol)
                    }
                    let values = tracked.evaluate(&[Epf::new(input, 20.)]).unwrap();
                    assert!(
                        values
                            .iter()
                            .all(|v| v.valid() && v.get_absolute_error() > 0.)
                    );
                }
            }
            // Until the root-relation inventory supports this owner, transport refuses
            // explicitly instead of silently forgetting constant equations/minors.
            assert!(
                matches!(EndpointTransport::prepare(complete,&[FaceAxis{unit:u,side:FaceSide::Lower}],&TransportLimits::default(),||ControlFlow::Continue(())),Err(Error::Unsupported(reason))if reason.contains("constant endpoint root inventory"))
            );
            count += 1;
        }
        assert_eq!(count, 2);
    }
}
#[test]
fn positive_constant_refuses_wrong_branch_second_axis_and_budget() {
    let owner = owner(2);
    let u = symbol!("positive_constant_refusal::u");
    let map = PositiveProjectiveCellMap::new(owner, 0, vec![u]).unwrap();
    let mut b = Budget::new(Limits::default());
    let first = NormalizedPrefix::new(map.clone(), &b).unwrap();
    assert!(matches!(
        first.constant_interval(0, &mut b),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        first.constant_interval(
            1,
            &mut Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)))
        ),
        Err(Error::Cancelled)
    ));
    assert!(
        PositiveConstantSection::prepare(
            map,
            SectionSide::Upper,
            BracketProposal {
                lower: Rational::from(2),
                upper: Rational::from(3),
                derivative_margin: Rational::one()
            },
            &mut b
        )
        .is_err()
    );
    let complete = first.constant_interval(1, &mut b).unwrap();
    assert!(matches!(
        complete.constant_interval(1, &mut b),
        Err(Error::Unsupported(_))
    ));
}
