use super::*;
use crate::threshold::maps::normalized::{Limits, algebra};
use std::sync::atomic::AtomicBool;
use symbolica::{
    domains::float::{ErrorPropagatingFloat as Epf, Float, RealLike},
    symbol,
};

fn complete() -> Arc<AffineNormalizedPrefix> {
    let (first, proposal) = super::super::tests::fixture();
    first
        .normalized_upper(proposal, &mut Budget::new(Limits::default()))
        .unwrap()
}
fn compile(owner: Arc<AffineNormalizedPrefix>) -> CompiledAffinePrefix {
    CompiledAffinePrefix::prepare(
        owner,
        OptimizationSettings::default().cores(1),
        &Budget::new(Limits::default()),
    )
    .unwrap()
}

#[test]
fn affine_prefix_native_partial_outputs_constant_slots_open_guards_and_lifetime() {
    let (first, _) = super::super::tests::fixture();
    assert!(matches!(
        CompiledAffinePrefix::prepare(
            first.previous().unwrap().clone(),
            OptimizationSettings::default(),
            &Budget::new(Limits::default())
        ),
        Err(Error::Unsupported(_))
    ));
    let partial = compile(first.clone());
    assert_eq!(partial.inputs(), &first.source().unit_coordinates()[..1]);
    assert_eq!(partial.coordinates(), &[first.source().axes()[0].symbol()]);
    // The native fixed-argument callback is evaluated once by map_coeff_with_prec
    // and stored at requested precision, rather than re-solved at each sample.
    let instructions = partial.exact().export_instructions();
    assert_eq!(instructions.constant_functions.len(), 1);
    assert!(!instructions.constant_functions[0].fixed_args.is_empty());
    let a = 1. / 128_f64.sqrt();
    let mut partial_numeric = partial.map::<f64>(53);
    for u in [0.25, 0.75, 0.25] {
        let values = partial_numeric.evaluate(&[u]).unwrap();
        assert_eq!(values.len(), 2);
        assert!((values[0] - a * u * u).abs() < 1e-14);
        assert!((values[1] - 2. * a * u).abs() < 1e-14);
    }
    // Owned algebraic constants cannot be supplied as extra numeric inputs.
    assert!(partial_numeric.evaluate(&[0.5, a]).is_err());
    let owner = complete();
    assert!(matches!(
        CompiledAffinePrefix::prepare(
            owner.clone(),
            OptimizationSettings::default(),
            &Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)))
        ),
        Err(Error::Cancelled)
    ));
    let exact = compile(owner);
    let weak = Arc::downgrade(exact.owner());
    let mut numeric = exact.map::<f64>(53);
    drop(exact);
    assert!(weak.upgrade().is_some());
    for unit in [vec![0., 0.5], vec![0.5, 1.], vec![f64::NAN, 0.5], vec![0.5]] {
        assert!(numeric.evaluate(&unit).is_err());
    }
    assert!(numeric.evaluate(&[0.5, 0.5]).is_ok());
    drop(numeric);
    assert!(weak.upgrade().is_none());
}

fn isolated(expression: &str, bits: u32) -> (Rational, Rational) {
    let ring = symgcad::algebra::Algebra::new(&["z".into()]).unwrap();
    let polynomial = ring.parse(expression).unwrap();
    let fiber = ring.specialize_univariate(&polynomial, 0, &[]).unwrap();
    let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
    for root in &mut roots {
        for _ in 0..bits + 64 {
            symgcad::roots::refine_once(root);
        }
    }
    roots
        .into_iter()
        .find(|root| root.interval.0 > Rational::zero())
        .unwrap()
        .interval
}
fn reference(u: &Rational, v: &Rational, bits: u32) -> [(Rational, Rational); 3] {
    let (alo, ahi) = isolated("128*z^2-1", bits);
    let (xlo, xhi) = (&alo * &u.pow(2), &ahi * &u.pow(2));
    // Z is increasing in x since Zx=1/(3Z²+1)>0. Both fibers are exact QQ.
    let (zlo, _) = isolated(&format!("z^3+z-1-({xlo})"), bits);
    let (_, zhi) = isolated(&format!("z^3+z-1-({xhi})"), bits);
    let lower = Rational::from((1, 8));
    let ylo = &lower + &(&xlo * v * &zlo);
    let yhi = &lower + &(&xhi * v * &zhi);
    // Exact original constant relation A²=1/128 simplifies only the oracle J.
    let j = Rational::from((1, 64)) * u.pow(3);
    [(xlo, xhi), (ylo, yhi), (&j * &zlo, &j * &zhi)]
}

#[test]
fn affine_prefix_native_codec_high_precision_independent_fibers_and_tracked_refresh() {
    let exact = compile(complete());
    let restored = exact.native_roundtrip().unwrap();
    assert!(Arc::ptr_eq(exact.owner(), restored.owner()));
    assert_eq!(
        restored
            .exact()
            .export_instructions()
            .constant_functions
            .len(),
        1
    );
    for bits in [128, 256] {
        let mut numeric = restored.map::<Float>(bits);
        let mut tracked = restored.map::<Epf<Float>>(bits);
        for (u, v) in [
            (Rational::from((1, 2)), Rational::from((1, 4))),
            (Rational::from((3, 4)), Rational::from((2, 3))),
            (Rational::from((1, 2)), Rational::from((1, 4))),
        ] {
            let unit = [&u, &v].map(|q| Float::rational_at(q, bits));
            let values = numeric.evaluate(&unit).unwrap();
            let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 12));
            for (value, (lo, hi)) in values.iter().zip(reference(&u, &v, bits)) {
                let center = value.to_rational();
                assert!(center >= lo - &tolerance && center <= hi + &tolerance);
            }
            let values = tracked.evaluate(&unit.map(|v| Epf::new(v, 20.))).unwrap();
            assert!(
                values
                    .iter()
                    .all(|v| v.valid() && v.get_absolute_error() > 0.)
            );
        }
    }
}

#[test]
fn affine_root_native_derivatives_include_constant_scale_and_coefficient_chain() {
    let owner = complete();
    let steps = owner.steps();
    let AffinePrefixStep::ConstantInterval { upper, .. } = steps[0] else {
        panic!("actual constant interval")
    };
    let AffinePrefixStep::NormalizedUpper { section, .. } = steps[1] else {
        panic!("actual normalized section")
    };
    let constant = upper.section().unwrap();
    let settings = OptimizationSettings::default().cores(1);
    let constant_program = RootProgram::prepare_affine_shifted(constant, settings.clone()).unwrap();
    let moving_program = section.root_program(settings.clone()).unwrap();
    let moving_program = RootProgram::from_bytes(moving_program.bytes()).unwrap();
    let (constant_tag, moving_tag) = symbol!(
        "affine_prefix_derivative::constant",
        "affine_prefix_derivative::moving"
    );
    let a = constant_program
        .call(constant_tag, constant.coefficients())
        .unwrap();
    let (u, v) = (
        owner.source().unit_coordinates()[0],
        owner.source().unit_coordinates()[1],
    );
    let x = &a * Atom::var(u).pow(2);
    let coefficients = section
        .coefficients()
        .iter()
        .map(|coefficient| {
            algebra::pull(
                coefficient,
                &section.chart().parameters,
                std::slice::from_ref(&x),
            )
        })
        .collect::<Vec<_>>();
    let z = moving_program.call(moving_tag, &coefficients).unwrap();
    let y = Atom::num(section.lower().clone()) + &x * Atom::var(v) * &z;
    // Native differentiation of the tiny actual map before evaluation, not a
    // derivative of original P at a frozen root or a separate AD implementation.
    let mixed = y.derivative(u).derivative(v);
    let twice = mixed.derivative(u);
    let body = Atom::evaluator_multiple(&[mixed, twice], &[Atom::var(u), Atom::var(v)])
        .optimization_settings(settings)
        .build()
        .unwrap();
    let mut scope = Scope::default();
    scope.insert(constant_tag, constant_program).unwrap();
    scope.insert(moving_tag, moving_program).unwrap();
    let mut numeric = scope.enter(128, || {
        body.map_coeff_with_prec(&|q| Float::rational_at(&q.re, 128), 128)
    });
    let unit =
        [Rational::from((1, 2)), Rational::from((1, 4))].map(|q| Float::rational_at(&q, 128));
    let mut output = vec![Float::exact_zero_at(128); 2];
    attempt(|| numeric.try_evaluate(&unit, &mut output))
        .unwrap()
        .unwrap();
    let a = 1. / 128_f64.sqrt();
    let u = 0.5;
    let x = a * u * u;
    let xq = Rational::try_from(x).unwrap();
    let (lo, hi) = isolated(&format!("z^3+z-1-({xq})"), 128);
    let z = ((lo + hi) / Rational::from(2)).to_f64();
    let d = 3. * z * z + 1.;
    let expected = [
        2. * a * u * (z + x / d),
        2. * a * (z + x / d) + 4. * a * a * u * u * (2. / d - 6. * x * z / (d * d * d)),
    ];
    for (value, expected) in output.iter().zip(expected) {
        assert!((value.to_f64() - expected).abs() < 1e-12);
    }
    assert!(
        (output[0].to_f64() - 2. * a * u * z).abs() > 1e-5,
        "frozen-root negative control must differ"
    );
}
