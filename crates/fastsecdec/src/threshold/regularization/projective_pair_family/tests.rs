use super::*;
pub(crate) mod fixtures;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::projective::PositiveProjectivePreparation,
};
use symbolica::{atom::AtomCore, symbol};

pub(crate) fn fixture(
    extra: Option<(Atom, u32)>,
    prefactor: Atom,
) -> Arc<PositiveProjectiveQuotient> {
    let (x, y, z, eps) = symbol!(
        "pair_test::x",
        "pair_test::y",
        "pair_test::z",
        "pair_test::eps"
    );
    let [xx, yy, zz] = [x, y, z].map(Atom::var);
    let e = Atom::var(eps);
    let u = &xx * &yy + &xx * &zz + &yy * &zz;
    let f = (&xx + &yy + &zz) * &u - Atom::num(16) * &xx * &yy * &zz;
    let degree = extra.as_ref().map(|(_, d)| *d).unwrap_or(0);
    let mut factors = vec![
        PolynomialFactor::new(
            u,
            Atom::num(-3) - Atom::num(Rational::from((i64::from(degree), 2))) + Atom::num(3) * &e,
            FactorRole::Singularity,
        )
        .with_semantics(FactorSemantics::Positive),
        PolynomialFactor::new(f, Atom::one() - Atom::num(2) * e, FactorRole::Singularity)
            .with_semantics(FactorSemantics::Causal),
    ];
    if let Some((n, _)) = extra {
        factors.push(PolynomialFactor::new(
            n,
            Atom::one(),
            FactorRole::Polynomial,
        ));
    }
    let input = Arc::new(
        ParametricIntegrand::new(
            vec![x, y, z],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                prefactor,
                vec![Atom::zero(); 3],
                factors,
            )],
        )
        .unwrap(),
    );
    let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 2).unwrap());
    let problem = symgcad::Problem {
        variables: vec!["x0".into(), "x1".into(), "x2".into()],
        constraints: vec!["x0>0".into(), "x1>0".into(), "x2>0".into()],
        split: vec!["(x0+x1+x2)*(x0*x1+x0*x2+x1*x2)-16*x0*x1*x2".into()],
        solver: symgcad::SolverOptions {
            ordering: "given".into(),
            order: vec!["x2".into(), "x0".into(), "x1".into()],
            homogeneous_anchor: Some("x2".into()),
            homogeneous_block: vec!["x0".into(), "x1".into(), "x2".into()],
            ..Default::default()
        },
        limits: symgcad::Limits {
            workers: 1,
            wall_time_secs: 30.,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    let raw = Arc::new(symgcad::solver::solve(&problem).unwrap());
    Arc::new(PositiveProjectiveQuotient::verify(prep, raw).unwrap())
}
pub(crate) fn coordinates() -> Vec<Symbol> {
    vec![symbol!("pair_test::u"), symbol!("pair_test::v")]
}
pub(crate) fn limits() -> Limits {
    let mut l = Limits::default();
    l.prefactors.source_nodes = 32768;
    l.algebra.max_terms = 100_000;
    l.algebra.certificates.bernstein_coefficients = 262144;
    l.algebra.certificates.work_limit = 1_000_000_000;
    l
}

#[test]
fn complete_algebraic_family_retains_all_source_cells_and_full_regulator_density() {
    let eps = Atom::var(symbol!("pair_test::eps"));
    let prefactor = -symbolica::transcendental::gamma().call(Atom::num(-1) + Atom::num(2) * &eps);
    let owner = fixture(None, prefactor);
    let coordinates = coordinates();
    let limits = limits();
    let proposals = fixtures::proposals(&owner, &coordinates, &limits).unwrap();
    let family = ResolvedProjectivePairFamily::admit(
        owner.clone(),
        coordinates,
        proposals,
        limits,
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(
        family
            .charts
            .iter()
            .map(|c| c.cell)
            .collect::<std::collections::BTreeSet<_>>(),
        (0..7).collect()
    );
    assert!(Arc::ptr_eq(family.owner(), &owner));
    assert_eq!(family.convergence_strip().lower(), Some(&Rational::zero()));
    assert_eq!(family.convergence_strip().upper(), Some(&Rational::one()));
    assert!(family.charts.iter().all(|c| c.terms.len() == 1));
    full_density(&family);
    for chart in &family.charts {
        let f = chart
            .factors()
            .iter()
            .find(|r| r.origin() == FactorOrigin::Factor { term: 0, factor: 1 })
            .unwrap();
        assert_eq!(f.sign(), if chart.cell == 3 { -1 } else { 1 });
    }
}

fn full_density(family: &ResolvedProjectivePairFamily) {
    use super::projective_family::algebra::pull;
    use symbolica::domains::float::Complex;
    let eps = family.regulators()[0];
    let prepared = family.owner().preparation();
    let tolerance = Rational::from((1, 2)).pow(85);
    for chart in family.charts() {
        let mut originals = Vec::new();
        let mut resolved = Vec::new();
        for (ti, source) in prepared.terms().iter().enumerate() {
            let mut original = source.prefactor().clone();
            for (s, e) in prepared.coordinates().iter().zip(source.monomial_powers()) {
                original *= Atom::var(*s).pow(e);
            }
            for (fi, f) in source.factors().iter().enumerate() {
                if f.role() == FactorRole::Polynomial {
                    original *= f.polynomial().pow(f.exponent());
                    continue;
                }
                let sign = chart
                    .factors()
                    .iter()
                    .find(|r| {
                        r.origin()
                            == FactorOrigin::Factor {
                                term: ti,
                                factor: fi,
                            }
                    })
                    .unwrap()
                    .sign();
                original *= (Atom::num(sign) * f.polynomial()).pow(f.exponent());
                if sign < 0 {
                    original *= (-Atom::i() * Atom::var(Symbol::PI) * f.exponent()).exp();
                }
            }
            originals.push(
                pull(&original, prepared.coordinates(), chart.images()) * chart.positive_measure(),
            );
            let t = &chart.terms()[ti];
            let mut value = t.prefactor() * t.regular();
            for (s, p) in family.coordinates().iter().zip(t.powers()) {
                value *= Atom::var(*s).pow(p);
            }
            resolved.push(value);
        }
        let expressions = originals
            .into_iter()
            .chain(resolved)
            .map(|a| pull(&a, &[eps], &[Atom::num(Rational::from((3, 4)))]))
            .collect::<Vec<_>>();
        let program = Atom::evaluator_multiple(
            &expressions,
            &family
                .coordinates()
                .iter()
                .copied()
                .map(Atom::var)
                .collect::<Vec<_>>(),
        )
        .optimization_settings(
            OptimizationSettings::default()
                .cores(1)
                .cpe_iterations(Some(0)),
        )
        .build()
        .unwrap();
        let cf = |q: &Rational| {
            Complex::new(
                q.to_multi_prec_float(128),
                Rational::zero().to_multi_prec_float(128),
            )
        };
        let mut numeric = chart.roots().enter(128, || {
            program.map_coeff_with_prec(
                &|q| Complex::new(q.re.to_multi_prec_float(128), q.im.to_multi_prec_float(128)),
                128,
            )
        });
        for point in [
            [Rational::from((1, 3)), Rational::from((2, 5))],
            [Rational::from((3, 4)), Rational::from((1, 2))],
        ] {
            let mut output = vec![cf(&Rational::zero()); expressions.len()];
            numeric
                .try_evaluate(&point.map(|q| cf(&q)), &mut output)
                .unwrap();
            for (left, right) in output[..prepared.terms().len()]
                .iter()
                .zip(&output[prepared.terms().len()..])
            {
                for (a, b) in [(&left.re, &right.re), (&left.im, &right.im)] {
                    let error = (a.to_rational() - b.to_rational()).abs();
                    assert!(
                        error <= &tolerance * &b.to_rational().abs().max(Rational::one()),
                        "cell {} {:?}: {error}",
                        chart.cell_index(),
                        chart.endpoint_path()
                    );
                }
            }
        }
    }
}

#[test]
fn nonconstant_homogeneous_complex_numerator_rays_keep_full_density() {
    let (x, eps) = symbol!("pair_test::x", "pair_test::eps");
    let owner = fixture(
        Some(((Atom::one() + Atom::num(2) * Atom::i()) * Atom::var(x), 1)),
        Atom::one() + Atom::var(eps),
    );
    let coordinates = coordinates();
    let limits = limits();
    let proposals = fixtures::proposals(&owner, &coordinates, &limits).unwrap();
    let family = ResolvedProjectivePairFamily::admit(
        owner,
        coordinates,
        proposals,
        limits,
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(
        family
            .charts()
            .iter()
            .any(|c| c.numerator_transport()[0].powers().iter().any(|p| *p < 0))
    );
    assert!(
        family
            .charts()
            .iter()
            .all(|c| c.numerator_transport()[0].factor_indices() == [2])
    );
    full_density(&family);
}

#[test]
fn source_coverage_selector_denominator_and_unbound_coefficient_controls() {
    let owner = fixture(None, Atom::one());
    let coordinates = coordinates();
    let limits = limits();
    let proposals = fixtures::proposals(&owner, &coordinates, &limits).unwrap();
    let mut missing = proposals.clone();
    missing.pop();
    assert!(matches!(
        ResolvedProjectivePairFamily::admit(
            owner.clone(),
            coordinates.clone(),
            missing,
            limits.clone(),
            Default::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
    let mut missing = proposals.clone();
    missing[1].pieces.pop();
    assert!(matches!(
        ResolvedProjectivePairFamily::admit(
            owner.clone(),
            coordinates.clone(),
            missing,
            limits.clone(),
            Default::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
    let mut swapped = proposals.clone();
    let FibreProposal::Native { lower, upper, .. } = &mut swapped[3].pieces[0].fibre else {
        unreachable!()
    };
    std::mem::swap(lower, upper.as_mut().unwrap());
    assert!(
        ResolvedProjectivePairFamily::admit(
            owner.clone(),
            coordinates.clone(),
            swapped,
            limits.clone(),
            Default::default(),
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    let mut pole = proposals.clone();
    if let FibreProposal::PositiveCentre { centre } = &mut pole[1].pieces[1].fibre {
        *centre /= Atom::var(coordinates[0]) - Atom::num(Rational::from((1, 2)));
    }
    assert!(
        ResolvedProjectivePairFamily::admit(
            owner.clone(),
            coordinates.clone(),
            pole,
            limits.clone(),
            Default::default(),
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    let mut wrong = proposals.clone();
    wrong[3].pieces[0].rewrites[0].expression = -&wrong[3].pieces[0].rewrites[0].expression;
    assert!(
        ResolvedProjectivePairFamily::admit(
            owner.clone(),
            coordinates.clone(),
            wrong,
            limits.clone(),
            Default::default(),
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(matches!(
        ResolvedProjectivePairFamily::admit(
            owner,
            coordinates,
            proposals,
            limits,
            Default::default(),
            |_| ControlFlow::Break(())
        ),
        Err(Error::Cancelled)
    ));
    let unbound = fixture(
        Some((
            Atom::var(symbol!("pair_test::free")) * Atom::var(symbol!("pair_test::x")),
            1,
        )),
        Atom::one(),
    );
    let units = self::coordinates();
    let limits = self::limits();
    let proposals = fixtures::proposals(&unbound, &units, &limits).unwrap();
    assert!(
        matches!(ResolvedProjectivePairFamily::admit(unbound,units,proposals,limits,Default::default(),|_|ControlFlow::Continue(())),Err(Error::Unsupported(message)) if message.contains("unbound numerator"))
    );
}

#[test]
fn native_linear_resultant_sign_zero_and_conservative_shared_ray_orders() {
    use super::projective_family::algebra::{Field, pull};
    let (x, y, eps, u, v, primitive) = symbol!(
        "pair_clear::x",
        "pair_clear::y",
        "pair_clear::eps",
        "pair_clear::u",
        "pair_clear::v",
        "pair_clear::primitive"
    );
    let roles = vec![
        symbol!("pair_clear::nx"),
        symbol!("pair_clear::dx"),
        symbol!("pair_clear::ny"),
        symbol!("pair_clear::dy"),
    ];
    let limits = limits();
    let mut observer = |_| ControlFlow::Continue(());
    let w = Work {
        limits: &limits,
        budget: crate::threshold::maps::regular::Budget::new(limits.algebra.clone()),
        observer: &mut observer,
    };
    for source in [
        Atom::zero(),
        Atom::num(7),
        Atom::var(x).pow(3) + (Atom::one() + Atom::i() * Atom::var(eps)) * Atom::var(y).pow(2),
    ] {
        let t = numerator::prepare(0, vec![2], source.clone(), &[x, y], eps, roles.clone(), &w)
            .unwrap();
        let images = vec![
            Atom::var(roles[0]) / Atom::var(roles[1]),
            Atom::var(roles[2]) / Atom::var(roles[3]),
        ];
        let expected = pull(&source, &[x, y], &images)
            * Atom::var(roles[1]).pow(Atom::num(t.degrees[0]))
            * Atom::var(roles[3]).pow(Atom::num(t.degrees[1]));
        assert!((t.expression - expected).expand().is_zero());
    }
    let t = numerator::prepare(
        0,
        vec![2],
        Atom::var(x) + Atom::var(y),
        &[x, y],
        eps,
        roles,
        &w,
    )
    .unwrap();
    let mut field = Field::new(&[], &[u, v], primitive, &w).unwrap();
    let n = numerator::compose(
        &t,
        &[Atom::one() / Atom::var(u), Atom::one() / Atom::var(u)],
        &mut field,
        &[u, v],
        &[],
        &[],
        &w,
    )
    .unwrap();
    assert_eq!(n.powers(), [-2, 0]);
    assert!(
        (n.regular() - Atom::num(2) * Atom::var(u))
            .expand()
            .is_zero()
    );
    let mut conservative = ConvergenceStrip::default();
    conservative
        .admit(&(Atom::one() - Atom::num(2) * Atom::var(eps)), eps)
        .unwrap();
    assert!(
        conservative
            .admit(&(Atom::num(-2) + Atom::var(eps)), eps)
            .is_err()
    );
    let mut exact = ConvergenceStrip::default();
    exact
        .admit(&(Atom::one() - Atom::num(2) * Atom::var(eps)), eps)
        .unwrap();
    exact.admit(&(Atom::num(-1) + Atom::var(eps)), eps).unwrap();
    // Degree transport proves regularity but does not claim a minimal valuation.
    // A vanishing cleared numerator can require a separate native refinement.
}

#[test]
fn genuine_quartic_selected_fibre_uses_the_same_normalized_constructor() {
    let (x, y, z, eps, u, v, root, primitive) = symbol!(
        "quartic_pair::x",
        "quartic_pair::y",
        "quartic_pair::z",
        "quartic_pair::eps",
        "quartic_pair::u",
        "quartic_pair::v",
        "quartic_pair::root",
        "quartic_pair::primitive"
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
    let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 2).unwrap());
    let problem = symgcad::Problem {
        variables: vec!["x0".into(), "x1".into(), "x2".into()],
        constraints: vec!["x0>0".into(), "x1>0".into(), "x2>0".into()],
        split: vec!["x1^4+x1^2*x2^2-x0^2*x2*(2*x2+x0)".into(), "x0-x2".into()],
        solver: symgcad::SolverOptions {
            ordering: "given".into(),
            order: vec!["x2".into(), "x0".into(), "x1".into()],
            homogeneous_anchor: Some("x2".into()),
            homogeneous_block: vec!["x0".into(), "x1".into(), "x2".into()],
            ..Default::default()
        },
        limits: symgcad::Limits {
            workers: 1,
            wall_time_secs: 30.,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    let raw = Arc::new(symgcad::solver::solve(&problem).unwrap());
    let owner = Arc::new(PositiveProjectiveQuotient::verify(prep, raw).unwrap());
    let limits = limits();
    let cell = (0..owner.cell_count())
        .find(|cell| {
            ResolvedProjectivePairFamily::selected_constant(
                &owner,
                ConstantReference {
                    cell: *cell,
                    side: SectionSide::Lower,
                },
                &limits,
            )
            .is_ok_and(|a| a.is_zero())
                && ResolvedProjectivePairFamily::selected_constant(
                    &owner,
                    ConstantReference {
                        cell: *cell,
                        side: SectionSide::Upper,
                    },
                    &limits,
                )
                .is_ok_and(|a| a.is_one())
                && owner.native_result().cells[*cell].axes[1].upper.is_some()
        })
        .unwrap();
    let ur = Atom::var(u);
    let zr = Atom::var(root);
    let proposal = CellProposal {
        cell,
        constants: Vec::new(),
        pieces: vec![PrefixPiece {
            fractions: [Rational::zero(), Rational::one()],
            shape: PrefixShape::Power {
                exponent: 1,
                reversed: false,
            },
            roots: vec![RootProposal {
                role: root,
                equation: ur.pow(2) * zr.pow(4) + zr.pow(2) - Atom::num(2) - &ur,
                bracket: BracketProposal {
                    lower: Rational::one(),
                    upper: Rational::from(2),
                    derivative_margin: Rational::from((1, 1024)),
                },
                witness: Rational::from((1, 2)),
            }],
            fibre: FibreProposal::Native {
                lower: Atom::zero(),
                upper: Some(&ur * &zr),
                reversed: false,
            },
            rewrites: Vec::new(),
        }],
    };
    let mut observer = |_| ControlFlow::Continue(());
    let mut w = Work {
        limits: &limits,
        budget: crate::threshold::maps::regular::Budget::new(limits.algebra.clone()),
        observer: &mut observer,
    };
    let bases = geometry::bases(
        &owner,
        &[u, v],
        &proposal,
        primitive,
        &Default::default(),
        &mut w,
    )
    .unwrap();
    assert_eq!(bases.len(), 1);
    let section = &bases[0].sections[0];
    assert_eq!(section.coefficients().len(), 5);
    assert_eq!(section.branches().len(), 1);
    let at_zero =
        super::projective_family::algebra::pull(section.equation(), &[u], &[Atom::zero()]);
    assert!(!at_zero.contains(zr.pow(4).as_view()));
    // This degree-generic branch test does not promote its one source cell to a
    // complete family; whole-source admission rejects the missing siblings.
    assert!(matches!(
        ResolvedProjectivePairFamily::admit(
            owner,
            vec![u, v],
            vec![proposal],
            limits,
            Default::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
}
