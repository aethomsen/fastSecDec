use super::*;
use crate::threshold::maps::normalized::Limits;
use std::sync::{OnceLock, atomic::AtomicBool};
use symbolica::symbol;

#[test]
fn strict_next_brackets_choose_native_recipe_and_restore_in_fresh_process() {
    use super::program::CompiledRestrictedNextSection;
    let directory = tempfile::tempdir().unwrap();
    for degree in [3, 4] {
        for negative in [false, true] {
            for nondyadic in [false, true] {
                let (equation, witness, mut proposal) = fixture(degree, 0, 2);
                if negative {
                    let z = proposal.root_symbol;
                    proposal.equation =
                        algebra::pull(&proposal.equation, &[z], &[Atom::var(z) + Atom::num(2)]);
                    proposal.centre += Atom::num(2) * &proposal.scale;
                    proposal.bracket.lower -= Rational::from(2);
                    proposal.bracket.upper -= Rational::from(2);
                }
                if nondyadic {
                    proposal.bracket.lower =
                        Rational::from((1, 3)) - Rational::from(if negative { 2 } else { 0 });
                }
                let owner = RestrictedNextSection::prepare(
                    equation,
                    witness,
                    proposal,
                    &mut Budget::new(Limits::default()),
                )
                .unwrap();
                let program = owner.root_program(OptimizationSettings::default()).unwrap();
                assert_eq!(program.recipe_version(), 3);
                let restored = RootProgram::from_bytes(program.bytes()).unwrap();
                assert_eq!(restored.bytes(), program.bytes());
                assert_eq!(restored.recipe_version(), program.recipe_version());
                let compiled = CompiledRestrictedNextSection::prepare(
                    owner,
                    OptimizationSettings::default().cores(1),
                    &Budget::new(Limits::default()),
                )
                .unwrap();
                let bytes = bincode::serde::encode_to_vec(
                    (compiled.exact(), compiled.scope().saved()),
                    bincode::config::standard(),
                )
                .unwrap();
                std::fs::write(
                    directory
                        .path()
                        .join(format!("{degree}-{negative}-{nondyadic}.bin")),
                    bytes,
                )
                .unwrap();
            }
        }
    }
    for degree in [3, 4] {
        for negative in [false, true] {
            let (equation, witness, proposal) = fixture_from_parent(
                control_prefix(degree, false, negative),
                degree,
                0,
                2,
                false,
                negative,
            );
            let owner = RestrictedNextSection::prepare(
                equation,
                witness,
                proposal,
                &mut Budget::new(Limits::default()),
            )
            .unwrap();
            assert!(owner.coefficients()[1..degree].iter().all(Atom::is_zero));
            assert_eq!(
                owner
                    .root_program(OptimizationSettings::default())
                    .unwrap()
                    .recipe_version(),
                1
            );
            let compiled = CompiledRestrictedNextSection::prepare(
                owner,
                OptimizationSettings::default().cores(1),
                &Budget::new(Limits::default()),
            )
            .unwrap();
            let bytes = bincode::serde::encode_to_vec(
                (compiled.exact(), compiled.scope().saved()),
                bincode::config::standard(),
            )
            .unwrap();
            std::fs::write(
                directory
                    .path()
                    .join(format!("binomial-{degree}-{negative}.bin")),
                bytes,
            )
            .unwrap();
        }
    }
    // Only the native arithmetic IR and source-bound RootProgram recipes cross
    // this process boundary. They do not reconstruct geometric owner authority.
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            &format!(
                "{}::restricted_next_cold_recipe_child",
                module_path!().strip_prefix("fastsecdec::").unwrap()
            ),
            "--ignored",
            "--nocapture",
        ])
        .env("FASTSECDEC_RESTRICTED_NEXT_COLD", directory.path())
        .status()
        .unwrap();
    assert!(status.success());
    assert!(directory.path().join("child-passed").is_file());
}

#[test]
#[ignore = "invoked by the fresh-process recipe control"]
fn restricted_next_cold_recipe_child() {
    use crate::kernel::algebraic::{Scope, attempt};
    use symbolica::{
        domains::float::{Complex, Float, FloatLike, Real, SingleFloat},
        evaluate::ExpressionEvaluator,
    };
    type Saved = (
        ExpressionEvaluator<Complex<Rational>>,
        Vec<(Symbol, Vec<u8>)>,
    );
    // Match the normal kernel loader: install native callback evaluation info
    // before decoding an arithmetic IR containing constant root calls.
    crate::kernel::algebraic::register();
    let directory = std::env::var_os("FASTSECDEC_RESTRICTED_NEXT_COLD").unwrap();
    for degree in [3u64, 4] {
        for negative in [false, true] {
            for nondyadic in [false, true] {
                let bytes = std::fs::read(
                    std::path::Path::new(&directory)
                        .join(format!("{degree}-{negative}-{nondyadic}.bin")),
                )
                .unwrap();
                let ((exact, saved), used): (Saved, usize) =
                    bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
                assert_eq!(used, bytes.len());
                let scope = Scope::restore(saved).unwrap();
                assert!(scope.owners().any(|(_, p)| p.recipe_version() == 3));
                for bits in [128, 256] {
                    let f = |q: Rational| Float::with_val(bits, 1).from_rational(&q);
                    let mut evaluator = scope.enter(bits, || {
                        exact
                            .clone()
                            .map_coeff_with_prec(&|q| f(q.re.clone()), bits)
                    });
                    for q in [Rational::from((1, 4)), Rational::from((3, 4))] {
                        eprintln!(
                            "cold degree={degree} negative={negative} nondyadic={nondyadic} bits={bits} point={q}"
                        );
                        let u = f(q);
                        let mut out = vec![f(Rational::zero()); 5];
                        attempt(|| evaluator.try_evaluate(std::slice::from_ref(&u), &mut out))
                            .unwrap()
                            .unwrap();
                        let a = f(Rational::from((1, 128))).sqrt();
                        let t = u.clone() * &u / f(Rational::from(2));
                        let x = a.clone() * &t * &t;
                        let j = f(Rational::from(2)) * &a * &t * &u;
                        let z = out[3].clone() + f(Rational::from(if negative { 2 } else { 0 }));
                        let y = f(Rational::from((1, 8))) + x.clone() * &z;
                        let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 16));
                        for (actual, expected) in
                            [(&out[0], &x), (&out[1], &j), (&out[2], &y), (&out[4], &a)]
                        {
                            assert!(
                                (actual.to_rational() - expected.to_rational()).abs() < tolerance
                            );
                        }
                        assert!(
                            (z.pow(degree) + &z - f(Rational::one()) - x)
                                .to_rational()
                                .abs()
                                < tolerance
                        );
                        assert_eq!(out[3] < f(Rational::zero()), negative);
                    }
                }
            }
        }
    }
    for degree in [3u64, 4] {
        for negative in [false, true] {
            let bytes = std::fs::read(
                std::path::Path::new(&directory).join(format!("binomial-{degree}-{negative}.bin")),
            )
            .unwrap();
            let ((exact, saved), used): (Saved, usize) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
            assert_eq!(used, bytes.len());
            let scope = Scope::restore(saved).unwrap();
            assert!(scope.owners().any(|(_, p)| p.recipe_version() == 1));
            for bits in [128, 256] {
                let f = |q: Rational| Float::with_val(bits, 1).from_rational(&q);
                let mut evaluator = scope.enter(bits, || {
                    exact
                        .clone()
                        .map_coeff_with_prec(&|q| f(q.re.clone()), bits)
                });
                for numerator in [1, 3] {
                    let u = f(Rational::from((numerator, 4)));
                    let a = f(Rational::from((1, 128))).sqrt();
                    let t = u.clone() * &u / f(Rational::from(2));
                    let x = a.clone() * &t * &t;
                    let j = f(Rational::from(2)) * &a * &t * &u;
                    let mut out = vec![f(Rational::zero()); 5];
                    attempt(|| evaluator.try_evaluate(std::slice::from_ref(&u), &mut out))
                        .unwrap()
                        .unwrap();
                    let y = f(Rational::from((1, 8))) + x.clone() * &out[3];
                    let tol = Rational::from((1, 2)).pow(u64::from(bits - 16));
                    for (actual, expected) in
                        [(&out[0], &x), (&out[1], &j), (&out[2], &y), (&out[4], &a)]
                    {
                        assert!((actual.to_rational() - expected.to_rational()).abs() < tol);
                    }
                    let sign = if negative && degree % 2 == 1 { 1 } else { -1 };
                    assert!(
                        (out[3].pow(degree) + f(Rational::from(sign)) * (f(Rational::one()) + x))
                            .to_rational()
                            .abs()
                            < tol
                    );
                    assert_eq!(out[3] < f(Rational::zero()), negative);
                    eprintln!(
                        "cold binomial PASS degree={degree} negative={negative} bits={bits} point={numerator}/4"
                    );
                }
            }
        }
    }
    std::fs::write(
        std::path::Path::new(&directory).join("child-passed"),
        b"12 packets, both precisions, native restore and source controls passed",
    )
    .unwrap();
}

#[test]
fn translated_quartic_native_recipe_conditioning_comparison() {
    use crate::kernel::algebraic::{Scope, attempt};
    use symbolica::domains::float::{Float, FloatLike, Real, SingleFloat};
    let (equation, witness, mut proposal) = fixture(4, 0, 2);
    let z = proposal.root_symbol;
    proposal.equation = algebra::pull(&proposal.equation, &[z], &[Atom::var(z) + Atom::num(2)]);
    proposal.centre += Atom::num(2) * &proposal.scale;
    proposal.bracket.lower -= Rational::from(2);
    proposal.bracket.upper -= Rational::from(2);
    let owner = RestrictedNextSection::prepare(
        equation,
        witness,
        proposal,
        &mut Budget::new(Limits::default()),
    )
    .unwrap();
    let v1 = owner
        .root_program(OptimizationSettings::default().cores(1))
        .unwrap();
    assert_eq!(v1.recipe_version(), 3);
    let v1 = RootProgram::prepare_from_certificate(
        4,
        owner.proposal().bracket.lower.clone(),
        owner.proposal().bracket.upper.clone(),
        v1.bytes().to_vec(),
        OptimizationSettings::default().cores(1),
    )
    .unwrap();
    // Test-only comparison retains the genuine issued branch recipe as its
    // complete contract; it cannot issue a new source owner or select a branch.
    let v3 = RootProgram::prepare_shifted_from_certificate(
        4,
        owner.proposal().bracket.lower.clone(),
        owner.proposal().bracket.upper.clone(),
        v1.bytes().to_vec(),
        OptimizationSettings::default().cores(1),
    )
    .unwrap();
    let roles = owner
        .parent()
        .leaf()
        .cover()
        .slots()
        .iter()
        .map(|(s, _)| Atom::var(*s))
        .collect::<Vec<_>>();
    assert_eq!(roles.len(), 2);
    let mut successes = [0, 0];
    for (index, program) in [v1, v3].into_iter().enumerate() {
        let tag = Symbol::parse(format!("recipe_{index}"), "next_cold_comparison").unwrap();
        let call = program.call(tag, owner.coefficients()).unwrap();
        let mut scope = Scope::default();
        scope.insert(tag, program).unwrap();
        let exact = Atom::evaluator_multiple(&[call], &roles)
            .optimization_settings(OptimizationSettings::default().cores(1))
            .build()
            .unwrap();
        for bits in [128, 160, 192, 256, 288, 320, 384] {
            let f = |q: Rational| Float::with_val(bits, 1).from_rational(&q);
            let mut evaluator = scope.enter(bits, || {
                exact
                    .clone()
                    .map_coeff_with_prec(&|q| f(q.re.clone()), bits)
            });
            for numerator in [1, 3] {
                let u = f(Rational::from((numerator, 4)));
                let a = f(Rational::from((1, 128))).sqrt();
                let x = a.clone() * u.pow(4) / f(Rational::from(4));
                let mut out = vec![f(Rational::zero())];
                match attempt(|| evaluator.try_evaluate(&[u, a], &mut out)) {
                    Ok(value) => {
                        value.unwrap();
                        let z = out[0].clone() + f(Rational::from(2));
                        let residual = (z.pow(4) + z - f(Rational::one()) - x).to_rational().abs();
                        assert!(residual < Rational::from((1, 2)).pow(u64::from(bits - 16)));
                        successes[index] += 1;
                        eprintln!(
                            "recipe comparison recipe={} bits={bits} point={numerator}/4 PASS residual={residual}",
                            if index == 0 { 1 } else { 3 }
                        );
                    }
                    Err(error) => {
                        assert!(error.contains("root refinement"));
                        eprintln!(
                            "recipe comparison recipe={} bits={bits} point={numerator}/4 ERROR {error}",
                            if index == 0 { 1 } else { 3 }
                        );
                    }
                }
            }
        }
    }
    assert!(successes.into_iter().all(|n| n > 0));
}

fn quartic_prefix() -> Arc<AffineNormalizedPrefix> {
    control_prefix(4, true, false)
}

fn control_prefix(degree: usize, linear: bool, negative: bool) -> Arc<AffineNormalizedPrefix> {
    use crate::{
        parametric::{
            FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
            PolynomialFactor,
        },
        threshold::{
            gcad::{GcadKinematics, GcadRequest, SolverOptions, VerifiedDecomposition},
            maps::CellMap,
            projective::AffineProjectivePreparation,
        },
    };
    type Sources = std::collections::BTreeMap<(usize, bool, bool), Arc<VerifiedDecomposition>>;
    static SOURCES: OnceLock<std::sync::Mutex<Sources>> = OnceLock::new();
    let owner = SOURCES
        .get_or_init(Default::default)
        .lock()
        .unwrap()
        .entry((degree, linear, negative))
        .or_insert_with(|| {
            let (x, y, a, eps) = symbol!(
                "next_root_quartic::x",
                "next_root_quartic::y",
                "next_root_quartic::a",
                "next_root_quartic::eps"
            );
            let xv = Atom::var(x);
            let yv = Atom::var(y);
            let sum = &xv + &yv + Atom::var(a);
            let shifted = Atom::num(8) * yv - &sum;
            let d = degree as u32;
            let odd_negative = negative && degree % 2 == 1;
            let polynomial = &sum * shifted.pow(d)
                + if linear {
                    Atom::num(8u64.pow(d - 1)) * &sum * xv.pow(d - 1) * &shifted
                } else {
                    Atom::zero()
                }
                + Atom::num(if odd_negative { 1 } else { -1 })
                    * Atom::num(8u64.pow(d))
                    * xv.pow(d)
                    * (&sum + &xv);
            let factors = [Atom::num(128) * xv.pow(2) - sum.pow(2), shifted, polynomial];
            let input = ParametricIntegrand::new(
                vec![x, y, a],
                eps,
                ParametricDomain::ProjectiveSimplex,
                vec![ParametricTerm::new(
                    Atom::one(),
                    vec![Atom::zero(); 3],
                    factors
                        .into_iter()
                        .map(|p| {
                            PolynomialFactor::new(
                                p,
                                Atom::num(Rational::from((
                                    -3i64,
                                    i64::try_from(degree + 4).unwrap(),
                                ))),
                                FactorRole::Singularity,
                            )
                            .with_semantics(FactorSemantics::Causal)
                        })
                        .collect(),
                )],
            )
            .unwrap();
            let mut limits = GcadRequest::default_limits();
            limits.wall_time_secs = 30.;
            limits.memory_mib = 1024;
            limits.workers = 1;
            let request = GcadRequest::projective(
                AffineProjectivePreparation::last_coordinate(&input).unwrap(),
                GcadKinematics::default(),
                SolverOptions {
                    order: vec!["v0".into(), "v1".into()],
                    ..Default::default()
                },
                limits,
            )
            .unwrap();
            Arc::new(request.solve_verified().unwrap())
        })
        .clone();
    let (u, v) = symbol!("next_root_quartic::u", "next_root_quartic::v");
    for i in 0..owner.cells().len() {
        let mut b = Budget::new(Limits::default());
        let prefix =
            AffineNormalizedPrefix::new(CellMap::new(owner.clone(), i, vec![u, v]).unwrap(), &b)
                .unwrap();
        let Ok(first) = prefix.constant_interval(2, &mut b) else {
            continue;
        };
        let Some(AffinePrefixStep::ConstantInterval { lower, upper, .. }) = first.step() else {
            unreachable!()
        };
        if lower.rational() != Some(&Rational::zero()) || upper.section().is_none() {
            continue;
        }
        let Ok(cover) = AffinePrefixCover::prepare_next_bound(
            first.clone(),
            SectionSide::Upper,
            &DecompositionOptions::default(),
            &mut b,
        ) else {
            continue;
        };
        let x = Rational::from((1, 64));
        let Ok(value) = witness::selected(
            &first,
            1,
            cover.next_bound(),
            std::slice::from_ref(&x),
            256,
            &b,
        ) else {
            continue;
        };
        let exact = |q: &Rational| RealBall::from_rational_bounds(q, q, 256);
        let normalized = (value - exact(&Rational::from((1, 8)))) / exact(&x);
        let lower = Rational::from(if negative { (-2, 1) } else { (1, 2) });
        let upper = Rational::from(if negative {
            (-1, 2)
        } else if linear {
            (1, 1)
        } else {
            (2, 1)
        });
        if (&normalized - exact(&lower)).is_strictly_positive()
            && (exact(&upper) - normalized).is_strictly_positive()
        {
            // This witness only locates the fixture's source cell. The real
            // constructor below still proves the full equation and global minor.
            return first;
        }
    }
    panic!("genuine quartic CAD has no expected shifted finite cell");
}

pub(super) fn fixture(
    degree: usize,
    leaf_index: usize,
    power: u32,
) -> (
    Arc<RestrictedRootEquation>,
    InteriorWitness,
    NextSectionProposal,
) {
    let parent = if degree == 3 {
        super::super::super::tests::fixture().0
    } else {
        assert_eq!(degree, 4);
        quartic_prefix()
    };
    fixture_from_parent(parent, degree, leaf_index, power, true, false)
}

fn fixture_from_parent(
    parent: Arc<AffineNormalizedPrefix>,
    degree: usize,
    leaf_index: usize,
    power: u32,
    linear: bool,
    negative: bool,
) -> (
    Arc<RestrictedRootEquation>,
    InteriorWitness,
    NextSectionProposal,
) {
    let mut b = Budget::new(Limits::default());
    let cover = AffinePrefixCover::prepare_next_bound(
        parent,
        SectionSide::Upper,
        &DecompositionOptions::default(),
        &mut b,
    )
    .unwrap();
    assert_eq!(cover.half_counts(), &[1, 1]);
    let leaf = cover.leaf(leaf_index).unwrap();
    let equation = leaf
        .prepare_equation(Rational::from((1, 1i64 << 40)), &mut b)
        .unwrap();
    let witness = leaf
        .interior_witness(
            vec![Rational::from(if leaf_index == 0 {
                (1, 64)
            } else {
                (1, 16)
            })],
            &mut b,
        )
        .unwrap();
    let u = cover.slots()[0].0;
    let z = symbol!("next_root_control::Z");
    let x = algebra::pull(
        &algebra::pull(
            &cover.parent().images()[0],
            &[u],
            leaf.restriction().unit_images(),
        ),
        &[u],
        &[Atom::var(u).pow(power)],
    );
    let q = Atom::var(z).pow(degree as u32)
        + if linear { Atom::var(z) } else { Atom::zero() }
        + Atom::num(if negative && degree % 2 == 1 { 1 } else { -1 }) * (Atom::one() + &x);
    let centre = Atom::num((1, 8));
    let scale = x;
    let transformed = algebra::pull(
        &algebra::pull(equation.total_pullback(), &[u], &[Atom::var(u).pow(power)]),
        &[cover.root_symbol()],
        &[&centre + &scale * Atom::var(z)],
    );
    let mut roles = cover.slots().iter().map(|(s, _)| *s).collect::<Vec<_>>();
    roles.push(z);
    let ring = algebra::Ring::new(roles).unwrap();
    let (presentation, remainder) = ring
        .polynomial(&transformed, &b)
        .unwrap()
        .quot_rem(&ring.polynomial(&q, &b).unwrap(), false);
    assert!(remainder.is_zero());
    assert_eq!(presentation.degree(cover.slots().len()), 0);
    (
        equation,
        witness,
        NextSectionProposal {
            unit_powers: vec![power],
            root_symbol: z,
            centre,
            scale,
            equation: q,
            presentation: ring.expression(&presentation),
            bracket: BracketProposal {
                lower: Rational::from(if negative { (-2, 1) } else { (1, 2) }),
                upper: Rational::from(if negative {
                    (-1, 2)
                } else if linear {
                    (1, 1)
                } else {
                    (2, 1)
                }),
                derivative_margin: Rational::from((1, 4)),
            },
            unit_margin: Rational::from((1, 1i64 << 40)),
        },
    )
}

#[test]
fn genuine_cubic_and_quartic_next_sections_retain_full_source_power_and_branch() {
    for degree in [3, 4] {
        for leaf in 0..2 {
            let (equation, witness, p) = fixture(degree, leaf, 2);
            let mut b = Budget::new(Limits::default());
            let section =
                RestrictedNextSection::prepare(equation.clone(), witness, p, &mut b).unwrap();
            assert!(Arc::ptr_eq(section.parent(), &equation));
            assert_eq!(section.coefficients().len(), degree + 1);
            assert_eq!(section.orientation(), 1);
            assert!(
                section
                    .endpoint_certificates()
                    .iter()
                    .all(|r| r.certificate().is_some())
            );
            assert!(
                section
                    .scale_receipt()
                    .coordinate_powers()
                    .iter()
                    .all(|p| *p > 0)
                    || leaf == 1
            );
            let u = equation.leaf().cover().slots()[0].0;
            let ring = algebra::Ring::new(vec![u]).unwrap();
            assert!(
                ring.polynomial(&(section.power_measure() - Atom::num(2) * Atom::var(u)), &b)
                    .unwrap()
                    .is_zero()
            );
            assert!(section.normalized_witness().is_strictly_positive());
            assert!(
                section
                    .root_program(OptimizationSettings::default())
                    .is_ok()
            );
        }
    }
}

#[test]
fn next_section_rejects_wrong_leaf_branch_identity_scale_and_denominator() {
    for kind in 0..8 {
        let (eq, w, mut p) = fixture(3, 0, 1);
        match kind {
            0 => p.presentation = Atom::num(2) * p.presentation,
            1 => p.scale = -p.scale,
            2 => p.scale *= Atom::var(eq.leaf().cover().slots()[0].0) - Atom::num((1, 2)),
            3 => p.centre = (Atom::var(eq.leaf().cover().slots()[0].0) - Atom::num((1, 2))).pow(-1),
            4 => {
                p.bracket = BracketProposal {
                    lower: Rational::from(-2),
                    upper: Rational::from(-1),
                    derivative_margin: Rational::from((1, 4)),
                }
            }
            5 => p.unit_powers = vec![0],
            6 => p.root_symbol = eq.leaf().cover().slots()[0].0,
            7 => p.presentation *= Atom::var(symbol!("next_root_control::foreign")),
            _ => unreachable!(),
        }
        assert!(
            RestrictedNextSection::prepare(eq, w, p, &mut Budget::new(Limits::default())).is_err(),
            "negative {kind}"
        );
    }
    let (eq, _, p) = fixture(3, 0, 1);
    let other = eq
        .leaf()
        .cover()
        .leaf(1)
        .unwrap()
        .interior_witness(
            vec![Rational::from((1, 16))],
            &mut Budget::new(Limits::default()),
        )
        .unwrap();
    assert!(matches!(
        RestrictedNextSection::prepare(eq, other, p, &mut Budget::new(Limits::default())),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn next_section_cancellation_degree_budget_and_non_dyadic_witness_refuse() {
    let (eq, w, p) = fixture(3, 0, 1);
    let mut b = Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)));
    assert!(matches!(
        RestrictedNextSection::prepare(eq, w, p, &mut b),
        Err(Error::Cancelled)
    ));
    let (eq, w, p) = fixture(3, 0, 1);
    let mut b = Budget::new(Limits {
        max_degree: 2,
        ..Default::default()
    });
    assert!(RestrictedNextSection::prepare(eq, w, p, &mut b).is_err());
    let (eq, w, p) = fixture(3, 0, 3);
    assert!(matches!(
        RestrictedNextSection::prepare(eq, w, p, &mut Budget::new(Limits::default())),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn native_tower_remainders_preserve_constant_and_eliminated_variable_frames() {
    let (u, a) = symbol!("next_remainder::u", "next_remainder::a");
    let roles = vec![u, a];
    let ring = algebra::Ring::new(roles.clone()).unwrap();
    let b = Budget::new(Limits::default());
    let relation = Atom::var(a).pow(2) - Atom::num((1, 128));
    for (body, expected) in [
        (Atom::one(), Atom::one()),
        (relation.clone() + Atom::num(3), Atom::num(3)),
        (relation.clone() * Atom::var(u), Atom::zero()),
    ] {
        let (rem, qs) =
            transport::reduce(&ring, &roles, &body, &[(a, relation.clone())], &b).unwrap();
        assert!(
            ring.polynomial(&(rem.clone() - expected), &b)
                .unwrap()
                .is_zero()
        );
        assert!(
            ring.polynomial(&(qs[0].clone() * &relation + rem - body), &b)
                .unwrap()
                .is_zero()
        );
    }
}
