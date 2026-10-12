use super::*;
#[test]
fn bounded_shift_preparation_and_native_pair_codec_association() {
    for degree in [usize::MAX, 4096] {
        assert!(
            super::super::shifted::preflight(degree, &Rational::zero(), &Rational::one()).is_err()
        );
    }
    assert!(
        super::super::shifted::preflight(33, &Rational::zero(), &Rational::from(2).pow(300))
            .is_err()
    );
    assert!(
        super::super::shifted::preflight(33, &Rational::from((1, 3)), &Rational::from((4, 5)))
            .is_ok()
    );
    let root = RootProgram::prepare_shifted_from_certificate(
        5,
        Rational::zero(),
        Rational::one(),
        b"v3 pair codec".to_vec(),
        OptimizationSettings::default().cores(1),
    )
    .unwrap();
    let (wire, _): (super::super::program::Wire<'_>, usize) =
        bincode::borrow_decode_from_slice(root.bytes(), bincode::config::standard()).unwrap();
    for version in [1, 2] {
        let bytes = bincode::encode_to_vec(
            super::super::program::Wire {
                version,
                degree: wire.degree,
                contract: wire.contract,
                bracket: wire.bracket,
                exact: wire.exact,
            },
            bincode::config::standard(),
        )
        .unwrap();
        assert!(RootProgram::from_bytes(&bytes).is_err());
    }
    // A valid native body cannot stand in for the differently shaped shift.
    let changed =
        bincode::serde::encode_to_vec((&root.exact, &root.exact), bincode::config::standard())
            .unwrap();
    let bytes = bincode::encode_to_vec(
        super::super::program::Wire {
            exact: &changed,
            ..wire
        },
        bincode::config::standard(),
    )
    .unwrap();
    assert!(RootProgram::from_bytes(&bytes).is_err());
    let x = symbol!("shifted_v3_test::head_input");
    let coefficients = vec![
        Atom::var(x),
        -Atom::one(),
        Atom::zero(),
        Atom::zero(),
        Atom::zero(),
        Atom::one(),
    ];
    let tag = symbol!("shifted_v3_test::wrong_head");
    let call = root.call(tag, &coefficients).unwrap();
    let arguments = call
        .as_view()
        .as_fun_view()
        .unwrap()
        .iter()
        .map(|a| a.to_owned())
        .collect::<Vec<_>>();
    let wrong = (*super::super::numeric::ROOT_V2).call_args(arguments);
    let mut scope = Scope::default();
    scope.insert(tag, root.clone()).unwrap();
    let mut eval = scope.enter(53, || {
        Atom::evaluator_multiple(&[wrong], &[Atom::var(x)])
            .build()
            .unwrap()
            .map_coeff(&|q| q.re.to_f64())
    });
    assert!(
        attempt(|| eval.evaluate(&[0.5], &mut [0.]))
            .unwrap_err()
            .contains("recipe")
    );
    for prepare in [
        RootProgram::prepare_from_certificate,
        RootProgram::prepare_affine_from_certificate,
    ] {
        let old = prepare(
            5,
            Rational::zero(),
            Rational::one(),
            b"legacy decode remains".to_vec(),
            OptimizationSettings::default(),
        )
        .unwrap();
        assert_eq!(
            old.bytes(),
            RootProgram::from_bytes(old.bytes()).unwrap().bytes()
        );
    }
}

#[test]
fn exact_small_dyadic_shift_coefficients_survive_degree32() {
    for lower in [Rational::from((65, 64)), Rational::from((33, 32))] {
        let width = Rational::from((1, 128));
        let root = RootProgram::prepare_shifted_from_certificate(
            32,
            lower.clone(),
            &lower + &width,
            b"exact dyadic shift degree32".to_vec(),
            OptimizationSettings::default().cores(1),
        )
        .unwrap();
        let restored = RootProgram::from_bytes(root.bytes()).unwrap();
        let mut shift = restored.shift.clone().unwrap().map_coeff_with_prec(
            &|q| {
                Complex::new(
                    Float::rational_at(&q.re, 256),
                    Float::rational_at(&q.im, 256),
                )
            },
            256,
        );
        let zero = Complex::new(Float::exact_zero_at(256), Float::exact_zero_at(256));
        let mut inputs = vec![zero.clone(); 33];
        inputs[32] = Complex::new(Float::one_at(256), Float::exact_zero_at(256));
        let mut values = vec![zero; 33];
        shift.evaluate(&inputs, &mut values);
        assert_eq!(values[0].re.to_rational(), lower.pow(32));
        assert_eq!(values[32].re.to_rational(), width.pow(32));
        assert!(
            values
                .iter()
                .all(|c| c.im == Float::exact_zero_at(256) && c.re > Float::exact_zero_at(256))
        );
        // Test the retained shift against the original monomial at an exact
        // rational interior point, including every small positive coefficient.
        // All dyadic coefficients in this fixture fit exactly within 256 bits.
        let at = Rational::from((1, 3));
        let value = values
            .iter()
            .rev()
            .fold(Rational::zero(), |v, c| v * &at + c.re.to_rational());
        assert_eq!(value, (&lower + &(&width * &at)).pow(32));
    }
}
