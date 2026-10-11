use super::*;
use crate::threshold::maps::normalized::Limits;
use std::sync::atomic::AtomicBool;
use symbolica::domains::float::{
    DoubleFloat, ErrorPropagatingFloat as Epf, Float, FloatLike, RealLike,
};

fn complete() -> Arc<NormalizedPrefix> {
    let (power, first, second) = super::super::tests::fixture();
    let mut budget = Budget::new(Limits::default());
    power
        .section(first, &mut budget)
        .unwrap()
        .section(second, &mut budget)
        .unwrap()
}
fn compiled() -> CompiledOpenMap {
    CompiledOpenMap::prepare(
        complete(),
        OptimizationSettings::default().cores(1),
        &Budget::new(Limits::default()),
    )
    .unwrap()
}

#[test]
fn normalized_composer_complete_map_open_guard_and_owner_lifetime() {
    let (power, _, _) = super::super::tests::fixture();
    assert!(matches!(
        CompiledOpenMap::prepare(
            power,
            OptimizationSettings::default().cores(1),
            &Budget::new(Limits::default())
        ),
        Err(Error::Unsupported(_))
    ));
    let owner = complete();
    assert!(matches!(
        CompiledOpenMap::prepare(
            owner,
            OptimizationSettings::default(),
            &Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)))
        ),
        Err(Error::Cancelled)
    ));
    let exact = compiled();
    assert_eq!(
        exact.coordinates(),
        exact
            .owner()
            .source()
            .decomposition()
            .preparation()
            .coordinates()
    );
    let weak = Arc::downgrade(exact.owner());
    let mut eval = exact.map::<f64>(53);
    let mut double = exact.map::<DoubleFloat>(106);
    drop(exact);
    assert!(weak.upgrade().is_some());
    for [u, v, t] in [
        [0.5, 0.75, 0.25],
        [0.25, 0.125, 0.75],
        [0.875, 0.625, 0.125],
    ] {
        let w = (1.0_f64 + v * v).sqrt();
        let expected = [u * u, u * v, u * t * w, 2. * u.powi(3) * w];
        let out = eval.evaluate(&[u, v, t]).unwrap();
        let dd = double.evaluate(&[u.into(), v.into(), t.into()]).unwrap();
        for (i, reference) in expected.iter().enumerate() {
            assert!((out[i] - reference).abs() < 1e-12);
            assert!((dd[i].to_f64() - reference).abs() < 1e-12);
        }
    }
    for input in [
        vec![0., 0.5, 0.5],
        vec![0.5, 1., 0.5],
        vec![0.5, f64::NAN, 0.5],
        vec![0.5],
    ] {
        assert!(eval.evaluate(&input).is_err());
    }
    assert!(eval.evaluate(&[0.5, 0.75, 0.25]).is_ok());
    drop(eval);
    drop(double);
    assert!(weak.upgrade().is_none());
}

fn isolated_w(v: &Rational, bits: u32) -> (Rational, Rational) {
    let algebra = symgcad::algebra::Algebra::new(&["v".into(), "w".into()]).unwrap();
    let polynomial = algebra.parse("w^2-1-v^2").unwrap();
    let fiber = algebra
        .specialize_univariate(&polynomial, 1, std::slice::from_ref(v))
        .unwrap();
    let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
    for root in &mut roots {
        for _ in 0..16 {
            symgcad::roots::refine_once(root);
        }
    }
    let root = roots
        .iter_mut()
        .find(|r| r.interval.0 > Rational::zero())
        .unwrap();
    for _ in 0..bits + 64 {
        symgcad::roots::refine_once(root);
    }
    root.interval.clone()
}

#[test]
fn normalized_composer_saved_ir_high_precision_and_tracked_input_refresh() {
    let exact = compiled();
    let restored = exact.native_roundtrip().unwrap();
    assert!(Arc::ptr_eq(exact.owner(), restored.owner()));
    let u = Rational::from((1, 2));
    let t = Rational::from((1, 4));
    for bits in [128, 256] {
        let mut eval = restored.map::<Float>(bits);
        let mut tracked = restored.map::<Epf<Float>>(bits);
        // Includes the exact rational W=5/4 and a genuinely algebraic W.
        for v in [
            Rational::from((3, 4)),
            Rational::from((1, 3)),
            Rational::from((3, 4)),
        ] {
            let input = [&u, &v, &t].map(|q| Float::rational_at(q, bits));
            let out = eval.evaluate(&input).unwrap();
            let (lo, hi) = isolated_w(&v, bits);
            let expected = [
                (&u * &u, &u * &u),
                (&u * &v, &u * &v),
                (&u * &t * &lo, &u * &t * &hi),
                (
                    Rational::from(2) * u.pow(3) * &lo,
                    Rational::from(2) * u.pow(3) * &hi,
                ),
            ];
            let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 12));
            for (value, (lo, hi)) in out.iter().zip(expected) {
                assert!(value.get_precision() >= bits);
                let value = value.to_rational();
                assert!(value >= lo - &tolerance && value <= hi + &tolerance);
            }
            let values = input.map(|v| Epf::new(v, 20.));
            let out = tracked.evaluate(&values).unwrap();
            assert!(out.iter().all(|v| v.valid() && v.get_absolute_error() > 0.));
        }
    }
}

#[test]
fn normalized_root_program_derivatives_use_normalized_coefficients() {
    let owner = complete();
    let PrefixStep::Section { section, .. } = owner.step().unwrap() else {
        panic!("second selected section");
    };
    let program = section
        .root_program(OptimizationSettings::default().cores(1))
        .unwrap();
    let restored = RootProgram::from_bytes(program.bytes()).unwrap();
    assert_eq!(program.bytes(), restored.bytes());
    let tag = symbol!("normalized_test::derivative_root");
    let h = section.chart().parameters[1];
    let root = program.call(tag, section.coefficients()).unwrap();
    let exact = Atom::evaluator_multiple(
        &[
            root.clone(),
            root.derivative(h),
            root.derivative(h).derivative(h),
        ],
        &section
            .chart()
            .parameters
            .iter()
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>(),
    )
    .build()
    .unwrap();
    let mut scope = Scope::default();
    scope.insert(tag, restored).unwrap();
    for bits in [128, 256] {
        let mut eval = scope.enter(bits, || {
            exact
                .clone()
                .map_coeff_with_prec(&|q| Float::rational_at(&q.re, bits), bits)
        });
        let input =
            [Rational::from((1, 2)), Rational::from((3, 4))].map(|q| Float::rational_at(&q, bits));
        let mut out = vec![Float::exact_zero_at(bits); 3];
        attempt(|| eval.try_evaluate(&input, &mut out))
            .unwrap()
            .unwrap();
        // W=sqrt(1+h²): W=5/4, W_h=3/5, W_hh=64/125.
        let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 12));
        for (value, expected) in out.iter().zip([
            Rational::from((5, 4)),
            Rational::from((3, 5)),
            Rational::from((64, 125)),
        ]) {
            let error = value.to_rational() - expected;
            assert!(error >= -tolerance.clone() && error <= tolerance);
        }
    }
}
