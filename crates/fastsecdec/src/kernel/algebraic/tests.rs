//! Native callback/domain and bounded-codec regression controls.
use super::{Error, RootProgram, Scope, numeric::ROOT};
use symbolica::{atom::Atom, domains::rational::Rational, evaluate::OptimizationSettings, symbol};
#[test]
fn callback_scope_and_bounded_codec() {
    assert!(super::is_root(*ROOT));
    assert!(!super::is_root(symbol!("other::selected_root_v1")));
    assert!(!super::is_root(symbol!(
        "fastsecdec::algebraic::selected_root_v3"
    )));
    super::failed("outer root".into());
    crate::contour::functions::dynamic::failure("outer contour".into());
    // Ordinary arithmetic must neither inspect nor clear either callback TLS.
    let x = symbol!("algebraic_probe::ordinary_x");
    for backend in [
        crate::kernel::EvaluatorBackend::Eager,
        crate::kernel::EvaluatorBackend::Symjit,
    ] {
        let program = crate::kernel::program::PreparedCoefficientVector {
            coordinates: vec![x],
            coefficients: vec![(Atom::var(x) + Atom::one()).into()],
            functions: Default::default(),
            endpoint_profiles: vec![],
        }
        .build(crate::kernel::CompilationSettings {
            backend,
            ..Default::default()
        })
        .unwrap();
        let mut ordinary = crate::kernel::SectorKernel::from_program_with_backend(
            program,
            &Default::default(),
            false,
            backend,
        )
        .unwrap();
        match &ordinary.backend {
            crate::kernel::Backend::Real(k) => assert!(!k.evaluator.has_dynamic_callbacks()),
            _ => panic!("ordinary real control changed backend kind"),
        }
        let mut out = [0.];
        ordinary.evaluate(&[0.25], &mut out).unwrap();
        assert_eq!(out, [1.25]);
        assert_eq!(
            super::FAILURE.with_borrow(Clone::clone).as_deref(),
            Some("outer root")
        );
    }
    let both = crate::kernel::callback_attempt::Modes {
        contour: true,
        algebraic: true,
    };
    let (v, failure) = both.isolated(|| {
        assert!(super::FAILURE.with_borrow(Option::is_none));
        assert!(crate::contour::functions::dynamic::take_failure().is_none());
        super::failed("first root".into());
        super::failed("second root".into());
        crate::contour::functions::dynamic::failure("inner contour".into());
        let (_, nested) = both.isolated(|| {
            super::failed("nested root".into());
            crate::contour::functions::dynamic::failure("nested contour".into());
        });
        assert!(nested.unwrap().contains("nested root"));
        17
    });
    assert_eq!(v, 17);
    let failure = failure.unwrap();
    assert!(failure.contains("first root") && failure.contains("inner contour"));
    assert!(!failure.contains("second root"));
    assert_eq!(super::FAILURE.take().as_deref(), Some("outer root"));
    assert_eq!(
        crate::contour::functions::dynamic::take_failure().as_deref(),
        Some("outer contour")
    );
    let (_, failure) = both.isolated(|| 0);
    assert!(failure.is_none());
    let root = RootProgram::prepare_from_certificate(
        2,
        Rational::from(0),
        Rational::from(2),
        b"probe selected positive sqrt2 branch".to_vec(),
        OptimizationSettings::default(),
    )
    .unwrap();
    let encoded = root.bytes();
    let restored = RootProgram::from_bytes(encoded).unwrap();
    assert_eq!(restored.bytes(), encoded);
    let (wire, _): (super::program::Wire<'_>, usize) =
        bincode::borrow_decode_from_slice(encoded, bincode::config::standard()).unwrap();
    for degree in [0, super::program::MAX_DEGREE + 1, usize::MAX] {
        let changed = super::program::Wire { degree, ..wire };
        let bytes = bincode::encode_to_vec(changed, bincode::config::standard()).unwrap();
        assert!(matches!(
            RootProgram::from_bytes(&bytes),
            Err(Error::Invalid(_))
        ));
    }
    assert!(RootProgram::from_bytes(&vec![0; super::program::MAX_HELPER_BYTES + 1]).is_err());
    let mut trailing = encoded.to_vec();
    trailing.push(0);
    assert!(RootProgram::from_bytes(&trailing).is_err());
    let tag = symbol!("algebraic_probe::selected");
    assert!(root.call(tag, &[Atom::one()]).is_err());
    let mut scope = Scope::default();
    scope.insert(tag, root.clone()).unwrap();
    assert!(scope.insert(tag, root).is_err());
    assert_eq!(scope.owners().count(), 1);
}

#[test]
fn native_tracking_controls() {
    use super::numeric::Number;
    use symbolica::{
        atom::AtomCore,
        domains::float::{ErrorPropagatingFloat, RealLike},
    };
    type E = ErrorPropagatingFloat<f64>;
    let rounded = E::rational_at(&Rational::from((1, 3)), 53);
    assert!(rounded.get_absolute_error().is_finite() && rounded.get_absolute_error() > 0.);
    let integer = E::rational_at(&Rational::from(3), 53);
    assert_eq!(integer.get_absolute_error(), 0.);
    // Native weighted tracking must refuse loss of a nonzero uncertainty.
    assert!(!crate::kernel::precision::accepts_weighted_primary(
        E::new(1e-200, 20.),
        0.,
        1e-200,
        1e-300
    ));
    let root = RootProgram::prepare_from_certificate(
        5,
        Rational::from(0),
        Rational::from(1),
        b"endpoint uncertainty native control".to_vec(),
        OptimizationSettings::default(),
    )
    .unwrap();
    let tag = symbol!("algebraic_probe::endpoint_root");
    let c = symbol!("algebraic_probe::endpoint_coefficient");
    let call = root
        .call(
            tag,
            &[
                Atom::var(c),
                Atom::one(),
                Atom::zero(),
                Atom::zero(),
                Atom::zero(),
                Atom::one(),
            ],
        )
        .unwrap();
    let mut scope = Scope::default();
    scope.insert(tag, root).unwrap();
    let exact = Atom::evaluator_multiple(&[call], &[Atom::var(c)])
        .build()
        .unwrap();
    let mut evaluator = scope.enter(53, || {
        exact
            .clone()
            .map_coeff(&|q| E::new_with_accuracy(q.re.to_f64(), f64::INFINITY))
    });
    let uncertain_zero = E::new(0., 12.);
    let mut output = [E::new_with_accuracy(0., f64::INFINITY)];
    super::attempt(|| evaluator.evaluate(&[uncertain_zero], &mut output)).unwrap();
    assert_eq!(output[0].to_f64(), 0.);
    assert!(
        output[0].get_absolute_error().is_finite() && output[0].get_absolute_error() >= 1e-12,
        "endpoint uncertainty {:?}",
        output[0]
    );
    let mut complex = scope.enter(53, || {
        exact.map_coeff(&|q| {
            symbolica::domains::float::Complex::new(
                E::new_with_accuracy(q.re.to_f64(), f64::INFINITY),
                E::new_with_accuracy(q.im.to_f64(), f64::INFINITY),
            )
        })
    });
    let exact_zero = E::new_with_accuracy(0., f64::INFINITY);
    let mut complex_output = [symbolica::domains::float::Complex::new(
        exact_zero, exact_zero,
    )];
    super::attempt(|| {
        complex.evaluate(
            &[symbolica::domains::float::Complex::new(
                exact_zero,
                E::new(0., 12.),
            )],
            &mut complex_output,
        )
    })
    .unwrap();
    assert_eq!(complex_output[0].im.to_f64(), 0.);
    assert!(
        complex_output[0].im.get_absolute_error() >= 1e-12
            && complex_output[0].im.get_absolute_error().is_finite()
    );
    let invalid = E::new(0., 400.);
    assert!(!invalid.get_absolute_error().is_finite());
    let error = super::attempt(|| evaluator.evaluate(&[invalid], &mut output)).unwrap_err();
    assert!(error.contains("uncertainty"), "{error}");
}
