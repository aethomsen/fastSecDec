use super::*;
use crate::threshold::maps::normalized::Limits;
use std::sync::atomic::AtomicBool;
use symbolica::domains::float::{ErrorPropagatingFloat as Epf, Float, FloatLike, Real};

fn section(degree: usize, leaf: usize, power: u32) -> Arc<RestrictedNextSection> {
    let (equation, witness, proposal) = super::super::tests::fixture(degree, leaf, power);
    RestrictedNextSection::prepare(
        equation,
        witness,
        proposal,
        &mut Budget::new(Limits::default()),
    )
    .unwrap()
}
fn compile(owner: Arc<RestrictedNextSection>) -> CompiledRestrictedNextSection {
    CompiledRestrictedNextSection::prepare(
        owner,
        OptimizationSettings::default().cores(1),
        &Budget::new(Limits::default()),
    )
    .unwrap()
}

#[test]
fn restricted_next_cubic_quartic_power_frame_codec_and_boundary_measure() {
    for degree in [3, 4] {
        for leaf in 0..2 {
            let compiled = compile(section(degree, leaf, 2));
            assert_eq!(compiled.inputs().len(), 1);
            assert_eq!(compiled.exact().get_output_len(), 5);
            assert_eq!(compiled.checked.get_output_len(), 11);
            let restored = compiled.native_roundtrip().unwrap();
            assert!(Arc::ptr_eq(compiled.owner(), restored.owner()));
            // The owned constant root remains fixed at the mapping precision.
            assert_eq!(
                restored
                    .checked
                    .export_instructions()
                    .constant_functions
                    .len(),
                1
            );
            for bits in [128, 256] {
                let mut numeric = restored.map::<Float>(bits);
                let mut tracked = restored.map::<Epf<Float>>(bits);
                let a = Float::rational_at(&Rational::from((1, 128)), bits).sqrt();
                let f = |q: Rational| Float::rational_at(&q, bits);
                let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 16));
                for q in [
                    Rational::from((1, 4)),
                    Rational::from((3, 4)),
                    Rational::from((1, 4)),
                ] {
                    let u = f(q.clone());
                    let values = numeric.evaluate(std::slice::from_ref(&u)).unwrap();
                    // Independent elementary restriction: lower u²/2, upper
                    // 1-u²/2. The native first prefix is x=A*t².
                    let t = if leaf == 0 {
                        u.clone() * &u / f(Rational::from(2))
                    } else {
                        f(Rational::one()) - u.clone() * &u / f(Rational::from(2))
                    };
                    let x = a.clone() * &t * &t;
                    let j = f(Rational::from(2)) * &a * &t * &u;
                    let z = values[3].clone();
                    assert!(z > f(Rational::from((1, 2))) && z < f(Rational::one()));
                    let residual = z.pow(degree as u64) + &z - f(Rational::one()) - &x;
                    let y = f(Rational::from((1, 8))) + x.clone() * &z;
                    for (actual, expected) in [
                        (&values[0], &x),
                        (&values[1], &j),
                        (&values[2], &y),
                        (&values[4], &a),
                    ] {
                        assert!((actual.to_rational() - expected.to_rational()).abs() < tolerance);
                    }
                    assert!(residual.to_rational().abs() < tolerance);
                    // Powering coefficient unit slots again changes Q: require
                    // an explicit nonzero residual against that wrong frame.
                    let uu = u.clone() * &u;
                    let wrong_t = if leaf == 0 {
                        uu.clone() * &uu / f(Rational::from(2))
                    } else {
                        f(Rational::one()) - uu.clone() * &uu / f(Rational::from(2))
                    };
                    let wrong_x = a.clone() * &wrong_t * &wrong_t;
                    assert!(
                        (z.pow(degree as u64) + &z - f(Rational::one()) - wrong_x)
                            .to_rational()
                            .abs()
                            > (1, 1_000_000)
                    );
                    // A boundary has no dx/dz measure: multiplying by the
                    // positive normalization scale would be a scientific error.
                    assert!(
                        (values[1].to_rational() - (j.clone() * &x).to_rational()).abs()
                            > (1, 1_000_000)
                    );
                    let tv = tracked.evaluate(&[Epf::new(u, 20.)]).unwrap();
                    assert!(tv.iter().all(|v| v.valid() && v.get_absolute_error() > 0.));
                }
            }
        }
    }
}

#[test]
fn restricted_next_runtime_retains_owner_and_every_parent_power_scale_guard() {
    let owner = section(3, 0, 2);
    assert!(matches!(
        CompiledRestrictedNextSection::prepare(
            owner.clone(),
            OptimizationSettings::default(),
            &Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)))
        ),
        Err(Error::Cancelled)
    ));
    let compiled = compile(owner.clone());
    let weak = Arc::downgrade(&owner);
    let mut numeric = compiled.map::<f64>(53);
    drop(owner);
    drop(compiled);
    assert!(weak.upgrade().is_some());
    for p in [vec![], vec![0.], vec![1.], vec![f64::NAN], vec![0.5, 0.5]] {
        assert!(numeric.evaluate(&p).is_err());
    }
    assert!(numeric.evaluate(&[0.5]).is_ok());
    // Mutate only private scratch. A positive public J cannot hide any failed
    // original diagonal, restriction measure, power measure, or scale guard.
    let mut baseline = vec![0.; numeric.checked_len];
    numeric.program.try_evaluate(&[0.5], &mut baseline).unwrap();
    for (index, bad) in [
        (5, -1.),
        (6, -1.),
        (7, 0.),
        (7, 1.),
        (8, -1.),
        (9, -1.),
        (10, 0.),
        (10, 1.),
    ] {
        let mut values = baseline.clone();
        values[index] = bad;
        let atoms = values
            .into_iter()
            .map(|v| Atom::num(Rational::try_from(v).unwrap()))
            .collect::<Vec<_>>();
        let u = Atom::var(numeric.owner().parent().leaf().cover().slots()[0].0);
        numeric.program = Atom::evaluator_multiple(&atoms, &[u])
            .optimization_settings(OptimizationSettings::default().cores(1))
            .build()
            .unwrap()
            .map_coeff(&|q| q.re.to_f64());
        assert!(numeric.evaluate(&[0.5]).is_err(), "guard {index}");
    }
    drop(numeric);
    assert!(weak.upgrade().is_none());
}

#[test]
fn restricted_next_native_derivative_keeps_root_and_power_chain() {
    use crate::{
        kernel::algebraic::RootProgram,
        threshold::maps::normalized::{affine::AffinePrefixStep, algebra},
    };
    use symbolica::symbol;
    for degree in [3, 4] {
        for leaf in 0..2 {
            let owner = section(degree, leaf, 2);
            let compiled = compile(owner.clone());
            let prefix = owner.parent().leaf().cover().parent();
            let Some(AffinePrefixStep::ConstantInterval { upper, .. }) = prefix.step() else {
                panic!("genuine first interval")
            };
            let constant = upper.section().unwrap();
            let constant_symbol = upper.symbol().unwrap();
            let settings = OptimizationSettings::default().cores(1);
            let cp = RootProgram::prepare_affine_shifted(constant, settings.clone()).unwrap();
            let rp = owner.root_program(settings.clone()).unwrap();
            let (ct, rt) = symbol!(
                "restricted_next_derivative::constant",
                "restricted_next_derivative::root"
            );
            let call = cp.call(ct, constant.coefficients()).unwrap();
            let bind = |a: &Atom| algebra::pull(a, &[constant_symbol], std::slice::from_ref(&call));
            let coefficients = owner.coefficients().iter().map(bind).collect::<Vec<_>>();
            let z = rp.call(rt, &coefficients).unwrap();
            let x = bind(&owner.scale_expression());
            let y = bind(&owner.centre_expression()) + &x * &z;
            let u = compiled.inputs()[0];
            let body = Atom::evaluator_multiple(
                &[y.clone(), y.derivative(u), x.derivative(u)],
                &[Atom::var(u)],
            )
            .optimization_settings(settings)
            .build()
            .unwrap();
            let mut scope = Scope::default();
            scope.insert(ct, cp).unwrap();
            scope.insert(rt, rp).unwrap();
            for bits in [128, 256] {
                let mut native = scope.enter(bits, || {
                    body.clone()
                        .map_coeff_with_prec(&|q| Float::rational_at(&q.re, bits), bits)
                });
                let point = [Float::rational_at(&Rational::from((3, 8)), bits)];
                let mut values = vec![Float::exact_zero_at(bits); 3];
                attempt(|| native.try_evaluate(&point, &mut values))
                    .unwrap()
                    .unwrap();
                let actual = compiled.map::<Float>(bits).evaluate(&point).unwrap();
                let z = actual[3].clone();
                let x = actual[0].clone();
                let denominator = Float::rational_at(&Rational::from(degree as i64), bits)
                    * z.pow((degree - 1) as u64)
                    + Float::rational_at(&Rational::one(), bits);
                let expected = values[2].clone() * (z.clone() + x / denominator);
                let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 14));
                assert!((values[0].to_rational() - actual[2].to_rational()).abs() < tolerance);
                assert!((values[1].to_rational() - expected.to_rational()).abs() < tolerance);
                assert!(
                    (values[1].to_rational() - (values[2].clone() * z).to_rational()).abs()
                        > (1, 100_000_000)
                );
            }
        }
    }
}

#[test]
fn restricted_next_evaluates_admitted_canonical_map_at_removable_interior_denominator() {
    let (equation, witness, mut proposal) = super::super::tests::fixture(3, 0, 2);
    let u = equation.leaf().cover().slots()[0].0;
    let x = Atom::var(u);
    let original = proposal.centre.clone();
    proposal.centre += (x.pow(2) - &x + Atom::num((1, 4))) / (x.clone() - Atom::num((1, 2))) - &x
        + Atom::num((1, 2));
    let syntax = Atom::evaluator_multiple(&[proposal.centre.clone()], std::slice::from_ref(&x))
        .optimization_settings(OptimizationSettings::default().cores(1))
        .build()
        .unwrap();
    let mut original_runtime = syntax.map_coeff(&|q| q.re.to_f64());
    let mut wrong = [0.];
    original_runtime.try_evaluate(&[0.5], &mut wrong).unwrap();
    assert!(
        !wrong[0].is_finite(),
        "original spelling must expose the removable0/0 control"
    );
    let owner = RestrictedNextSection::prepare(
        equation,
        witness,
        proposal,
        &mut Budget::new(Limits::default()),
    )
    .unwrap();
    assert_eq!(owner.centre_expression(), original);
    let compiled = compile(owner.clone());
    let values = compiled
        .map::<Float>(128)
        .evaluate(&[Float::rational_at(&Rational::from((1, 2)), 128)])
        .unwrap();
    let y = Float::rational_at(&Rational::from((1, 8)), 128) + values[0].clone() * &values[3];
    assert!((values[2].to_rational() - y.to_rational()).abs() < Rational::from((1, 2)).pow(112));
    // The original supplied spelling remains inspectable as provenance.
    assert_ne!(owner.proposal().centre, owner.centre_expression());
}
