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
