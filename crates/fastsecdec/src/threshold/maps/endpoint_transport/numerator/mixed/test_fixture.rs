// Genuine two-ray extension: positive z+w in source F makes a native competing-scale fan.
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
            regular::{Budget, Limits as AlgebraLimits},
        },
        projective::PositiveProjectivePreparation,
    },
};
use fastsecdec_sectors::ParametricDomain;
use std::{
    ops::ControlFlow,
    sync::{Arc, OnceLock},
};
use symbolica::{
    atom::{Atom, AtomCore},
    domains::rational::Rational,
    function, symbol,
};
pub(crate) fn fixture() -> Arc<EndpointTransport> {
    static FIXTURE: OnceLock<Arc<EndpointTransport>> = OnceLock::new();
    FIXTURE
        .get_or_init(|| {
            let (a, x, z, y, w, eps) = symbol!(
                "mixed_two_ray_test::a",
                "mixed_two_ray_test::x",
                "mixed_two_ray_test::z",
                "mixed_two_ray_test::y",
                "mixed_two_ray_test::w",
                "mixed_two_ray_test::eps"
            );
            let f = (Atom::var(a) - Atom::var(x))
                * (Atom::var(y).pow(2) - Atom::var(a) * Atom::var(x) - Atom::var(x).pow(2));
            let f = f * (Atom::var(z) + Atom::var(w));
            let n = Atom::var(y) * Atom::var(z).pow(2)
                + Atom::var(a) * Atom::var(x) * Atom::var(z)
                + Atom::var(a).pow(3);
            let inactive = function!(symbol!("mixed_two_ray_test::unused"), Atom::var(z));
            let term0 = ParametricTerm::new(
                Atom::num(2),
                vec![Atom::zero(); 5],
                vec![
                    PolynomialFactor::new(f.clone(), Atom::num((-9, 4)), FactorRole::Singularity)
                        .with_semantics(FactorSemantics::Causal),
                    PolynomialFactor::new(n.clone(), Atom::one(), FactorRole::Polynomial),
                    PolynomialFactor::new(Atom::var(y), Atom::one(), FactorRole::Polynomial),
                    PolynomialFactor::new(inactive, Atom::zero(), FactorRole::Polynomial),
                    PolynomialFactor::new(Atom::zero(), Atom::zero(), FactorRole::Polynomial),
                ],
            );
            let term1 = ParametricTerm::new(
                Atom::num(3),
                vec![
                    Atom::zero(),
                    Atom::one(),
                    Atom::zero(),
                    Atom::zero(),
                    Atom::zero(),
                ],
                vec![
                    PolynomialFactor::new(f, Atom::num((-13, 4)), FactorRole::Singularity)
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
                    vec![a, x, z, y, w],
                    eps,
                    ParametricDomain::ProjectiveSimplex,
                    vec![term0, term1],
                )
                .unwrap(),
            );
            let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 0).unwrap());
            let problem = Problem {
                variables: (0..5).map(|i| format!("x{i}")).collect(),
                constraints: (0..5).map(|i| format!("x{i}>0")).collect(),
                split: vec!["(x0-x1)*(x3^2-x0*x1-x1^2)*(x2+x4)".into()],
                solver: SolverOptions {
                    homogeneous_anchor: Some("x0".into()),
                    homogeneous_block: (0..5).map(|i| format!("x{i}")).collect(),
                    ordering: "given".into(),
                    order: ["x0", "x1", "x3", "x2", "x4"].map(str::to_owned).to_vec(),
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
            let (u, v, t, b, p, zroot) = symbol!(
                "mixed_two_ray_test::u",
                "mixed_two_ray_test::v",
                "mixed_two_ray_test::t",
                "mixed_two_ray_test::b",
                "mixed_two_ray_test::p",
                "mixed_two_ray_test::Z"
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
                    PositiveProjectiveCellMap::new(owner.clone(), cell, vec![u, v, t, b]).unwrap();
                let initial = NormalizedPrefix::new(source, &budget).unwrap();
                if let Ok(one) = initial.power(2, &budget)
                    && let Ok(two) = one.section(proposal.clone(), &mut budget)
                    && let Ok(three) = two.open(&budget)
                    && let Ok(all) = three.open(&budget)
                {
                    return EndpointTransport::prepare(
                        all,
                        &[
                            FaceAxis {
                                unit: t,
                                side: FaceSide::Upper,
                            },
                            FaceAxis {
                                unit: b,
                                side: FaceSide::Upper,
                            },
                        ],
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
