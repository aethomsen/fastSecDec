use super::*;
use symbolica::{state::State, symbol};

fn source() -> Source {
    let (p, q, rp, rq, y) = symbol!(
        "guard_actual::p",
        "guard_actual::q",
        "guard_actual::raw_p",
        "guard_actual::raw_q",
        "guard_actual::y"
    );
    let variables = Arc::new(vec![
        PolyVariable::Symbol(rp),
        PolyVariable::Symbol(rq),
        PolyVariable::Symbol(y),
    ]);
    let poly = |a: Atom| {
        a.try_to_polynomial::<_, u32>(&Q, variables.clone())
            .unwrap()
    };
    Source {
        public_parameters: vec![q, p],
        variables: variables.clone(),
        prefix_to_public: vec![1, 0],
        axes: vec![
            Axis {
                lower: Some(LinearBound {
                    source_polynomial: 10,
                    positive_selector: true,
                    numerator: poly(Atom::num(-1)),
                    denominator: poly(Atom::num(-4)),
                }),
                upper: Some(LinearBound {
                    source_polynomial: 11,
                    positive_selector: false,
                    numerator: poly(Atom::num(3)),
                    denominator: poly(Atom::num(4)),
                }),
            },
            Axis {
                lower: Some(LinearBound {
                    source_polynomial: 12,
                    positive_selector: false,
                    numerator: poly(Atom::var(rp).pow(2)),
                    denominator: poly(Atom::one()),
                }),
                upper: Some(LinearBound {
                    source_polynomial: 13,
                    positive_selector: false,
                    numerator: poly(Atom::one()),
                    denominator: poly(Atom::one()),
                }),
            },
        ],
        positive: vec![poly(Atom::var(rq) - Atom::var(rp))],
        nonzero: vec![poly(Atom::var(rp) - Atom::num(Rational::from((5, 8))))],
    }
}
fn point(p: Rational, q: Rational) -> BTreeMap<Symbol, Rational> {
    let source = source();
    [
        (source.public_parameters[1], p),
        (source.public_parameters[0], q),
    ]
    .into_iter()
    .collect()
}
fn prepare(source: Source) -> ExactParameterGuard {
    ExactParameterGuard::prepare(source, OptimizationSettings::default(), Limits::default())
        .unwrap()
}
#[test]
fn exact_open_chamber_and_native_polynomial_differential() {
    let source = source();
    let raw = [
        Rational::from((1, 2)),
        Rational::from((3, 4)),
        Rational::zero(),
    ];
    let expected = source
        .positive
        .iter()
        .chain(&source.nonzero)
        .map(|p| p.replace_all(&raw))
        .collect::<Vec<_>>();
    assert_eq!(expected, [Rational::from((1, 4)), Rational::from((-1, 8))]);
    let guard = prepare(source);
    assert_eq!(
        guard.admit(&point(raw[0].clone(), raw[1].clone())).unwrap(),
        raw[..2]
    );
    assert!(matches!(
        guard.admit(&point(Rational::from((1, 4)), raw[1].clone())),
        Err(Error::Outside { axis: 0 })
    ));
    assert!(matches!(
        guard.admit(&point(Rational::from((3, 4)), raw[1].clone())),
        Err(Error::Outside { axis: 0 })
    ));
    assert!(matches!(
        guard.admit(&point(raw[0].clone(), Rational::one())),
        Err(Error::Outside { axis: 1 })
    ));
    assert!(matches!(
        guard.admit(&point(raw[0].clone(), Rational::from((1, 4)))),
        Err(Error::Outside { axis: 1 })
    ));
    assert!(matches!(
        guard.admit(&point(raw[0].clone(), raw[0].clone())),
        Err(Error::Predicate {
            exceptional: false,
            ..
        })
    ));
    assert!(matches!(
        guard.admit(&point(Rational::from((5, 8)), raw[1].clone())),
        Err(Error::Predicate {
            exceptional: true,
            ..
        })
    ));
    assert!(matches!(guard.admit(&BTreeMap::new()), Err(Error::KeySet)));
    // Rejection does not poison or change the immutable saved instructions.
    assert_eq!(
        guard.admit(&point(raw[0].clone(), raw[1].clone())).unwrap(),
        raw[..2]
    );
}
#[test]
fn degree_drop_and_positive_selector_remain_distinct() {
    let mut s = source();
    let PolyVariable::Symbol(p) = s.variables[0] else {
        unreachable!()
    };
    s.axes[1].lower.as_mut().unwrap().denominator = (Atom::var(p)
        - Atom::num(Rational::from((1, 2))))
    .try_to_polynomial(&Q, s.variables.clone())
    .unwrap();
    let guard = prepare(s);
    assert!(matches!(
        guard.admit(&point(Rational::from((1, 2)), Rational::from((3, 4)))),
        Err(Error::DegreeDrop { polynomial: 12 })
    ));
    let mut s = source();
    s.axes[0].lower.as_mut().unwrap().numerator = Atom::Zero
        .try_to_polynomial(&Q, s.variables.clone())
        .unwrap();
    let guard = prepare(s);
    assert!(matches!(
        guard.admit(&point(Rational::from((1, 2)), Rational::from((3, 4)))),
        Err(Error::Selector { polynomial: 10 })
    ));
}
#[test]
fn malformed_context_and_budgets_refuse_before_native_access() {
    let mut s = source();
    s.positive[0].exponents.pop();
    assert!(matches!(
        ExactParameterGuard::prepare(s, OptimizationSettings::default(), Limits::default()),
        Err(Error::Invalid(_))
    ));
    let mut s = source();
    s.prefix_to_public = vec![0, 0];
    assert!(matches!(
        ExactParameterGuard::prepare(s, OptimizationSettings::default(), Limits::default()),
        Err(Error::Invalid(_))
    ));
    let mut s = source();
    let PolyVariable::Symbol(y) = s.variables[2] else {
        unreachable!()
    };
    s.positive[0] = Atom::var(y)
        .try_to_polynomial(&Q, s.variables.clone())
        .unwrap();
    assert!(matches!(
        ExactParameterGuard::prepare(s, OptimizationSettings::default(), Limits::default()),
        Err(Error::Invalid(_))
    ));
    let mut s = source();
    s.positive[0].exponents[0] = u32::MAX;
    assert!(matches!(
        ExactParameterGuard::prepare(s, OptimizationSettings::default(), Limits::default()),
        Err(Error::Resource(_))
    ));
    let limits = Limits {
        polynomials: 1,
        ..Limits::default()
    };
    assert!(matches!(
        ExactParameterGuard::prepare(source(), OptimizationSettings::default(), limits),
        Err(Error::Resource(_))
    ));
    let guard = prepare(source());
    let mut stored = guard.stored().clone();
    stored.axes[0].lower.as_mut().unwrap().numerator = usize::MAX;
    assert!(matches!(
        ExactParameterGuard::restore(stored, Limits::default()),
        Err(Error::Invalid(_))
    ));
    let stored = guard.stored().clone();
    let cap = stored.program.len() - 1;
    assert!(matches!(
        ExactParameterGuard::restore(
            stored,
            Limits {
                program_bytes: cap,
                ..Limits::default()
            }
        ),
        Err(Error::Resource(_))
    ));
    let mut stored = guard.stored().clone();
    stored.prefix_to_public[0] = usize::MAX;
    assert!(matches!(
        ExactParameterGuard::restore(stored, Limits::default()),
        Err(Error::Invalid(_))
    ));
}
#[test]
fn actual_f64_meaning_and_unguarded_chamber() {
    let mut s = source();
    s.axes.iter_mut().for_each(|a| {
        a.lower = None;
        a.upper = None;
    });
    s.positive.clear();
    s.nonzero.clear();
    let guard = prepare(s);
    let exact = Rational::try_from(0.1f64).unwrap();
    assert_ne!(exact, Rational::from((1, 10)));
    assert_eq!(
        guard
            .admit(&point(exact.clone(), Rational::from(10)))
            .unwrap()[0],
        exact
    );
}
#[test]
fn native_record_fresh_process_remaps_symbols_without_rebuilding_ir() {
    let guard = prepare(source());
    let dir = tempfile::tempdir().unwrap();
    let mut symbols = Atom::Zero.get_all_symbols(true);
    symbols.extend(guard.stored().symbols());
    guard
        .stored()
        .visit_atoms(&mut |a| symbols.extend(a.get_all_symbols(true)));
    let mut state = Vec::new();
    State::export_partial(&mut state, symbols).unwrap();
    std::fs::write(dir.path().join("state"), state).unwrap();
    std::fs::write(
        dir.path().join("guard"),
        bincode::encode_to_vec(guard.stored(), bincode::config::standard()).unwrap(),
    )
    .unwrap();
    let staged = staging::write(dir.path(), &guard).unwrap();
    std::fs::write(
        dir.path().join("staged.json"),
        serde_json::to_vec(&staged).unwrap(),
    )
    .unwrap();
    std::fs::write(
        dir.path().join("semantic"),
        guard.semantic_identity().unwrap(),
    )
    .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "kernel::parameter_guard::tests::fresh_helper",
            "--ignored",
            "--test-threads=1",
        ])
        .env("FSD_ACTUAL_GUARD", dir.path())
        .status()
        .unwrap();
    assert!(status.success());
}
#[test]
#[ignore]
fn fresh_helper() {
    let root = std::path::PathBuf::from(std::env::var("FSD_ACTUAL_GUARD").unwrap());
    for i in 0..29 {
        Symbol::parse(format!("guard_fresh_noise::n{i}"), "").unwrap();
    }
    for name in [
        "guard_actual::raw_q",
        "guard_actual::raw_p",
        "guard_actual::q",
        "guard_actual::p",
    ] {
        Symbol::parse(name, "").unwrap();
    }
    let bytes = std::fs::read(root.join("state")).unwrap();
    let context = State::import(&mut bytes.as_slice(), None).unwrap();
    let bytes = std::fs::read(root.join("guard")).unwrap();
    let (stored, used): (Stored, usize) =
        bincode::decode_from_slice_with_context(&bytes, bincode::config::standard(), context)
            .unwrap();
    assert_eq!(used, bytes.len());
    let original_program = stored.program.clone();
    let guard = ExactParameterGuard::restore(stored, Limits::default()).unwrap();
    assert_eq!(guard.stored().program, original_program);
    let staged: staging::StagedGuard =
        serde_json::from_slice(&std::fs::read(root.join("staged.json")).unwrap()).unwrap();
    let semantic = std::fs::read_to_string(root.join("semantic")).unwrap();
    let local = staging::read(&root, &staged, &semantic, staged.record.bytes).unwrap();
    assert_eq!(local.stored().program, original_program);
    assert_eq!(local.semantic_identity().unwrap(), semantic);
    assert_eq!(
        local
            .admit(&point(Rational::from((1, 2)), Rational::from((3, 4))))
            .unwrap(),
        [Rational::from((1, 2)), Rational::from((3, 4))]
    );
    assert_eq!(
        guard
            .admit(&point(Rational::from((1, 2)), Rational::from((3, 4))))
            .unwrap(),
        [Rational::from((1, 2)), Rational::from((3, 4))]
    );
    assert!(matches!(
        guard.admit(&point(Rational::from((5, 8)), Rational::from((3, 4)))),
        Err(Error::Predicate {
            exceptional: true,
            ..
        })
    ));
}

#[test]
fn predicate_identity_excludes_raw_aliases_program_and_diagnostic_indices() {
    use symbolica::id::{Pattern, Replacement};
    let guard = prepare(source());
    let identity = guard.semantic_identity().unwrap();
    let mut stored = guard.stored().clone();
    let fresh = [symbol!("guard_renamed::r1"), symbol!("guard_renamed::r0")];
    for a in &mut stored.expressions {
        *a = a.replace_multiple(stored.raw_parameters.iter().zip(fresh).map(|(s, t)| {
            Replacement::new(
                Pattern::Literal(Atom::var(*s)),
                Pattern::Literal(Atom::var(t)),
            )
        }));
    }
    stored.raw_parameters = fresh.to_vec();
    stored.axes[0].lower.as_mut().unwrap().source_polynomial = 999;
    assert_eq!(stored.semantic_identity().unwrap(), identity);
    let restored = ExactParameterGuard::restore(stored.clone(), Limits::default()).unwrap();
    assert_eq!(
        restored
            .admit(&point(Rational::from((1, 2)), Rational::from((3, 4))))
            .unwrap(),
        [Rational::from((1, 2)), Rational::from((3, 4))]
    );
    let unoptimized = ExactParameterGuard::prepare(
        source(),
        OptimizationSettings::default()
            .horner_iterations(0)
            .cpe_iterations(Some(0)),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(unoptimized.semantic_identity().unwrap(), identity);
    stored.axes[0].lower.as_mut().unwrap().positive_selector = false;
    assert_ne!(stored.semantic_identity().unwrap(), identity);
    let mut stored = guard.stored().clone();
    stored.prefix_to_public.swap(0, 1);
    assert_ne!(stored.semantic_identity().unwrap(), identity);
    // Original physical names are meaningful and are never fresh-role aliases.
    let mut stored = guard.stored().clone();
    stored.public_parameters[0] = symbol!("guard_actual::different_physical_q");
    assert_ne!(stored.semantic_identity().unwrap(), identity);
}

#[test]
fn staged_guard_caps_association_and_exact_program_roundtrip() {
    let guard = prepare(source());
    let dir = tempfile::tempdir().unwrap();
    let staged = staging::write(dir.path(), &guard).unwrap();
    assert!(
        staging::read(
            dir.path(),
            &staged,
            &staged.semantic,
            staged.record.bytes - 1
        )
        .is_err()
    );
    assert!(staging::read(dir.path(), &staged, &"0".repeat(64), u64::MAX).is_err());
    let restored =
        staging::read(dir.path(), &staged, &staged.semantic, staged.record.bytes).unwrap();
    assert_eq!(restored.stored().program, guard.stored().program);
    assert_eq!(
        restored.semantic_identity().unwrap(),
        guard.semantic_identity().unwrap()
    );
    assert_eq!(
        restored
            .admit(&point(Rational::from((1, 2)), Rational::from((3, 4))))
            .unwrap(),
        [Rational::from((1, 2)), Rational::from((3, 4))]
    );
    assert!(
        restored
            .admit(&point(Rational::from((1, 4)), Rational::from((3, 4))))
            .is_err()
    );
    let path = staged.record.resolve(dir.path()).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.push(0);
    std::fs::write(path, bytes).unwrap();
    assert!(staging::read(dir.path(), &staged, &staged.semantic, u64::MAX).is_err());
}
