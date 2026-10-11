use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{Limits as GcadLimits, PositiveProjectiveQuotient, Problem, SolverOptions},
        projective::PositiveProjectivePreparation,
    },
};
use std::sync::{OnceLock, atomic::AtomicBool};
use symbolica::{atom::AtomCore, symbol};

#[test]
fn normalized_minimum_valuation_is_not_the_leading_monomial_degree() {
    let (p, z) = symbol!("normalized_valuation::p", "normalized_valuation::z");
    let b = Budget::new(Limits::default());
    let ring = algebra::Ring::new(vec![p, z]).unwrap();
    let total = ring
        .polynomial(
            &(Atom::var(p).pow(2) * (Atom::var(z) - Atom::one()) + Atom::var(p).pow(4)),
            &b,
        )
        .unwrap();
    assert_eq!(total.degree_bounds(0), (2, 4));
    assert_eq!(total.ldegree(0), 4);
    let (removed, q) =
        transform::presentation_quotient(&ring, &total, &[p], &Rational::one(), &b).unwrap();
    assert_eq!(removed, ring.polynomial(&Atom::var(p).pow(2), &b).unwrap());
    assert_eq!(
        q,
        ring.polynomial(&(Atom::var(z) - Atom::one() + Atom::var(p).pow(2)), &b)
            .unwrap()
    );
    assert!(
        !total
            .quot_rem(&ring.polynomial(&Atom::var(p).pow(4), &b).unwrap(), false)
            .1
            .is_zero()
    );
}

fn owner() -> Arc<PositiveProjectiveQuotient> {
    static OWNER: OnceLock<Arc<PositiveProjectiveQuotient>> = OnceLock::new();
    OWNER
        .get_or_init(|| {
            let (a, x, y, z, eps) = symbol!(
                "normalized_test::a",
                "normalized_test::x",
                "normalized_test::y",
                "normalized_test::z",
                "normalized_test::eps"
            );
            let f = (Atom::var(a) - Atom::var(x))
                * (Atom::var(y).pow(2) - Atom::var(a) * Atom::var(x))
                * (Atom::var(z).pow(2) - Atom::var(a) * Atom::var(x) - Atom::var(y).pow(2));
            let input = Arc::new(
                ParametricIntegrand::new(
                    vec![a, x, y, z],
                    eps,
                    ParametricDomain::ProjectiveSimplex,
                    vec![ParametricTerm::new(
                        Atom::one(),
                        vec![Atom::zero(); 4],
                        vec![
                            PolynomialFactor::new(f, Atom::num((-4, 5)), FactorRole::Singularity)
                                .with_semantics(FactorSemantics::Causal),
                        ],
                    )],
                )
                .unwrap(),
            );
            let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 0).unwrap());
            let p = Problem {
                variables: (0..4).map(|i| format!("x{i}")).collect(),
                constraints: (0..4).map(|i| format!("x{i}>0")).collect(),
                split: vec!["(x0-x1)*(x2^2-x0*x1)*(x3^2-x0*x1-x2^2)".into()],
                solver: SolverOptions {
                    homogeneous_anchor: Some("x0".into()),
                    homogeneous_block: (0..4).map(|i| format!("x{i}")).collect(),
                    ordering: "given".into(),
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
            Arc::new(
                PositiveProjectiveQuotient::verify(
                    prep,
                    Arc::new(symgcad::solver::solve(&p).unwrap()),
                )
                .unwrap(),
            )
        })
        .clone()
}
pub(crate) fn fixture() -> (Arc<NormalizedPrefix>, SectionProposal, SectionProposal) {
    let owner = owner();
    let (u, v, t, p, h, z, w) = symbol!(
        "normalized_test::u",
        "normalized_test::v",
        "normalized_test::t",
        "normalized_test::p",
        "normalized_test::h",
        "normalized_test::Z",
        "normalized_test::W"
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
    let mut budget = Budget::new(Limits::default());
    for i in 0..owner.cell_count() {
        let source = PositiveProjectiveCellMap::new(owner.clone(), i, vec![u, v, t]).unwrap();
        let prefix = NormalizedPrefix::new(source, &budget).unwrap();
        if let Ok(power) = prefix.power(2, &budget)
            && let Ok(one) = power.section(first.clone(), &mut budget)
            && one.section(second.clone(), &mut budget).is_ok()
        {
            return (power, first, second);
        }
    }
    panic!("native complete cell inventory contains no expected genuine two-stage chain");
}
#[test]
fn normalized_source_prefix_two_selected_sections_and_exact_factors() {
    let (power, first, second) = fixture();
    let mut b = Budget::new(Limits::default());
    let one = power.section(first, &mut b).unwrap();
    let two = one.section(second, &mut b).unwrap();
    assert_eq!(two.len(), 3);
    assert!(two.complete_open_map());
    assert!(Arc::ptr_eq(two.source(), power.source()));
    assert_eq!(two.steps().len(), 3);
    for step in two.steps() {
        if let PrefixStep::Section { section, .. } = step {
            assert!(
                (section.total_pullback()
                    - section.removed_presentation_monomial() * section.equation())
                .expand()
                .is_zero()
            );
            assert_eq!(section.original_bound().index, 1);
            assert_eq!(section.exponent_determinant(), &Rational::from(2));
            assert_eq!(
                section.range_certificates().len(),
                2 * section.prefix().len()
            );
            assert_eq!(section.derivative_certificate().sign, 1);
        }
    }
    let (u, v, z, w) = symbol!(
        "normalized_test::u",
        "normalized_test::v",
        "normalized_test::Z",
        "normalized_test::W"
    );
    assert!(
        (two.images()[1].clone() - Atom::var(u) * Atom::var(v) * Atom::var(z))
            .expand()
            .is_zero()
    );
    assert!(
        (two.diagonal_product() - Atom::num(2) * Atom::var(u).pow(3) * Atom::var(z) * Atom::var(w))
            .expand()
            .is_zero()
    );
    // Both original partial owners remain independently usable and incomplete.
    assert!(!power.complete_open_map());
    assert!(!one.complete_open_map());
}
#[test]
fn normalized_proposals_reject_prefix_range_witness_and_symbol_grafts() {
    let (power, first, second) = fixture();
    let mut b = Budget::new(Limits::default());
    let one = power.section(first.clone(), &mut b).unwrap();
    let mut wrong = second.clone();
    wrong.chart.source_prefix[1] *= Atom::num(2);
    assert!(one.section(wrong, &mut b).is_err());
    let mut wrong = second.clone();
    wrong.chart.parameter_images[1] += Atom::one();
    assert!(one.section(wrong, &mut b).is_err());
    let mut wrong = second.clone();
    wrong.chart.bounds[1][1] = Rational::from((1, 8));
    assert!(one.section(wrong, &mut b).is_err());
    let mut wrong = second.clone();
    wrong.chart.witness[1] = Rational::from((5, 4));
    assert!(one.section(wrong, &mut b).is_err());
    let mut wrong = first.clone();
    wrong.bracket.lower = Rational::from(2);
    wrong.bracket.upper = Rational::from(3);
    assert!(power.section(wrong, &mut b).is_err());
    let mut wrong = first.clone();
    wrong.root_symbol = power.source().unit_coordinates()[0];
    assert!(power.section(wrong, &mut b).is_err());
    let mut wrong = first.clone();
    wrong.chart.source_prefix[0] += Atom::var(symbol!("symgcad::n0"));
    assert!(power.section(wrong, &mut b).is_err());
    let mut wrong = first.clone();
    wrong.removed_coefficient = Rational::zero();
    assert!(power.section(wrong, &mut b).is_err());
    let mut wrong = first;
    wrong.chart.source_prefix.push(Atom::one());
    assert!(power.section(wrong, &mut b).is_err());
    assert_eq!(power.len(), 1);
    assert_eq!(one.len(), 2);
}
#[test]
fn normalized_cancelled_or_limited_admission_preserves_source() {
    let (power, first, _) = fixture();
    let mut cancelled =
        Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)));
    assert!(matches!(
        power.section(first.clone(), &mut cancelled),
        Err(Error::Cancelled)
    ));
    let mut limited = Budget::new(Limits {
        max_terms: 1,
        ..Default::default()
    });
    assert!(matches!(
        power.section(first, &mut limited),
        Err(Error::ResourceIncomplete(_))
    ));
    assert_eq!(power.len(), 1);
    assert!(power.previous().unwrap().is_empty());
    assert!(matches!(
        power.power(u32::MAX, &Budget::new(Limits::default())),
        Err(Error::ResourceIncomplete(_))
    ));
}
