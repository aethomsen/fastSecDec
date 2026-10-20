//! Exact scientific controls for optional native family preparation.
use idenso::tensor::SymbolicTensor;
use std::{collections::BTreeMap, sync::Arc};

use fastsecdec::{
    Atom, AtomCore, Error, IntegralFamily, Kinematics, Model,
    input::GraphIntegral,
    parametric::{
        FamilyPreparationFallback as Fallback, FamilyPreparationPolicy as Policy,
        FamilyPreparationReport, FamilyPreparationStatus as Status, ParametricIntegrand,
        prepare_family,
    },
};
use feynkit_graph::EdgeId;
use symbolica::{id::Pattern, parse, symbol};

fn family(masses: &[Atom]) -> (IntegralFamily, Atom) {
    let k = parse!("family_prepare::k");
    let kin = Kinematics::in_dimension(&parse!("family_prepare::D"))
        .unwrap()
        .with_momenta([k.clone()])
        .unwrap();
    let k2 = kin.scalar_product(&k, &k).unwrap();
    (
        IntegralFamily::new(
            vec![k],
            vec![],
            masses.iter().map(|m| &k2 - m).collect(),
            &kin,
        )
        .unwrap(),
        k2,
    )
}

fn finite(expression: Atom) -> Atom {
    expression
        .series(symbol!("family_prepare::eps"), 0, 0)
        .unwrap()
        .to_atom()
        .cancel()
}

#[test]
fn projected_gaussian_moment_agrees_with_original_simplex_measure() {
    let (family, k2) = family(&[Atom::one(), Atom::one()]);
    let parameters = vec![symbol!("family_prepare::x"), symbol!("family_prepare::y")];
    let eps = symbol!("family_prepare::eps");
    let dimension = parse!("2-2*family_prepare::eps");
    let numerator = Atom::num(6) * (k2 - 1);
    let original = ParametricIntegrand::from_family(
        &family,
        &[2, 3],
        numerator.clone(),
        parameters.clone(),
        eps,
        dimension.clone(),
    )
    .unwrap();
    let (projected, report) = ParametricIntegrand::from_family_prepared(
        &family,
        &[2, 3],
        numerator,
        parameters.clone(),
        eps,
        dimension,
        Policy::default(),
    )
    .unwrap();
    assert_eq!(report.status, Status::Projected);
    assert_eq!(report.active_original_indices, [1]);
    assert_eq!(report.active_powers, [5]);
    assert_eq!(projected.parameters(), &parameters[1..]);
    let value = finite(
        projected
            .density()
            .replace(Pattern::Literal(Atom::var(parameters[1])))
            .with(1),
    );
    assert_eq!(value, Atom::num(2));
    let original_density = finite(
        original
            .density()
            .replace(Pattern::Literal(Atom::var(parameters[1])))
            .with(Atom::one() - Atom::var(parameters[0])),
    );
    let primitive = original_density
        .to_polynomial_in_vars::<u32>(parameters[0])
        .integrate(0)
        .flatten(false);
    let integrated = primitive
        .replace(Pattern::Literal(Atom::var(parameters[0])))
        .with(1)
        - primitive
            .replace(Pattern::Literal(Atom::var(parameters[0])))
            .with(0);
    assert_eq!(integrated, value);
    let restored: FamilyPreparationReport =
        serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
    assert_eq!(restored, report);
}

#[test]
fn native_noop_and_inadmissible_decompositions_preserve_original_density() {
    for (masses, policy, reason) in [
        (vec![Atom::one()], Policy::default(), Fallback::Independent),
        (
            vec![Atom::one(), Atom::one()],
            Policy::Original,
            Fallback::OriginalRequested,
        ),
        (
            vec![Atom::one(), Atom::one()],
            Policy::SingleUnitTerm { max_states: 1 },
            Fallback::StateLimit,
        ),
        (
            vec![Atom::one(), Atom::num(2)],
            Policy::default(),
            Fallback::MultipleTerms,
        ),
    ] {
        let (family, _) = family(&masses);
        let powers = vec![3; masses.len()];
        let parameters =
            [symbol!("family_prepare::u"), symbol!("family_prepare::v")][..masses.len()].to_vec();
        let dimension = parse!("2-2*family_prepare::eps");
        let original = ParametricIntegrand::from_family(
            &family,
            &powers,
            Atom::num(7),
            parameters.clone(),
            symbol!("family_prepare::eps"),
            dimension.clone(),
        )
        .unwrap();
        let (prepared, report) = ParametricIntegrand::from_family_prepared(
            &family,
            &powers,
            Atom::num(7),
            parameters.clone(),
            symbol!("family_prepare::eps"),
            dimension,
            policy,
        )
        .unwrap();
        assert_eq!(report.status, Status::Original(reason));
        assert_eq!(prepared.parameters(), parameters);
        assert_eq!(prepared.density(), original.density());
    }
    let (base, _) = family(&[Atom::one()]);
    let denominator = &base.denominators()[0];
    let scaled = IntegralFamily::new(
        base.loop_momenta().to_vec(),
        vec![],
        vec![Atom::num(2) * denominator, denominator.clone()],
        base.kinematics(),
    )
    .unwrap();
    let prepared = prepare_family(&scaled, &[2, 2], Policy::default()).unwrap();
    assert_eq!(
        prepared.report().status,
        Status::Original(Fallback::NonUnitCoefficient)
    );
    assert_eq!(prepared.family().denominators(), scaled.denominators());
    assert_eq!(prepared.powers(), [2, 2]);
}

#[test]
fn single_term_extracts_timelike_tree_propagator_exactly_once() {
    let k = parse!("family_prepare::bridge_k");
    let p = parse!("family_prepare::bridge_p");
    let q = parse!("family_prepare::bridge_q");
    let kin = Kinematics::in_dimension(&parse!("family_prepare::D"))
        .unwrap()
        .with_momenta([k.clone(), p.clone(), q.clone()])
        .unwrap()
        .with_scalar_product(&p, &p, Atom::num(0))
        .unwrap()
        .with_scalar_product(&q, &q, Atom::num(0))
        .unwrap()
        .with_scalar_product(&p, &q, parse!("family_prepare::s/2"))
        .unwrap();
    let top_mass_squared = Atom::num((119025, 4));
    let first = kin.scalar_product(&k, &k).unwrap() - &top_mass_squared;
    let second = kin.scalar_product(&(&k + &p), &(&k + &p)).unwrap() - &top_mass_squared;
    let third = kin
        .scalar_product(&(&k + &p + &q), &(&k + &p + &q))
        .unwrap()
        - top_mass_squared;
    let tree = parse!("family_prepare::s-family_prepare::MH^2");
    let original = IntegralFamily::new(
        vec![k.clone()],
        vec![p.clone(), q.clone()],
        vec![first.clone(), second.clone(), tree.clone(), third.clone()],
        &kin,
    )
    .unwrap();
    let control = IntegralFamily::new(
        vec![k.clone()],
        vec![p, q],
        vec![first, second, third],
        &kin,
    )
    .unwrap();
    let parameters = vec![
        symbol!("family_prepare::bridge_x0"),
        symbol!("family_prepare::bridge_x1"),
        symbol!("family_prepare::bridge_x2"),
        symbol!("family_prepare::bridge_x3"),
    ];
    let numerator = Atom::num(7) * kin.scalar_product(&k, &k).unwrap();
    for tree_power in [1, 2] {
        let powers = [1, 1, tree_power, 1];
        let prepared =
            prepare_family(&original, &powers, Policy::SingleTerm { max_states: 32 }).unwrap();
        assert_eq!(prepared.report().status, Status::Projected);
        assert_eq!(prepared.report().active_original_indices, [0, 1, 3]);
        assert_eq!(prepared.coefficient(), &tree.pow(-i64::from(tree_power)));
        assert_eq!(prepared.family().denominators(), control.denominators());
        assert_eq!(prepared.family().loop_momenta(), original.loop_momenta());
        assert_eq!(
            prepared.report().scalar_prefactor.as_deref(),
            Some(prepared.coefficient().to_canonical_string().as_str())
        );
        let (actual, report) = ParametricIntegrand::from_family_prepared(
            &original,
            &powers,
            numerator.clone(),
            parameters.clone(),
            symbol!("family_prepare::eps"),
            parse!("4-2*family_prepare::eps"),
            Policy::SingleTerm { max_states: 32 },
        )
        .unwrap();
        let expected = ParametricIntegrand::from_family(
            &control,
            &[1, 1, 1],
            &numerator / tree.pow(i64::from(tree_power)),
            vec![parameters[0], parameters[1], parameters[3]],
            symbol!("family_prepare::eps"),
            parse!("4-2*family_prepare::eps"),
        )
        .unwrap();
        assert_eq!(actual.density(), expected.density());
        assert!(
            actual
                .density()
                .contains(parse!("family_prepare::s").as_view())
        );
        assert!(
            actual
                .density()
                .contains(parse!("family_prepare::MH").as_view())
        );
        let decoded: FamilyPreparationReport =
            serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(decoded, report);
        assert_eq!(
            prepare_family(&original, &powers, Policy::default())
                .unwrap()
                .report()
                .status,
            Status::Original(Fallback::NonUnitCoefficient)
        );
    }
}

#[test]
fn single_term_reuses_scaled_duplicate_and_keeps_multiterm_fallback() {
    let (base, _) = family(&[Atom::one()]);
    let denominator = &base.denominators()[0];
    let scaled = IntegralFamily::new(
        base.loop_momenta().to_vec(),
        vec![],
        vec![Atom::num(2) * denominator, denominator.clone()],
        base.kinematics(),
    )
    .unwrap();
    let prepared = prepare_family(&scaled, &[2, 2], Policy::SingleTerm { max_states: 32 }).unwrap();
    assert_eq!(prepared.report().status, Status::Projected);
    assert_eq!(prepared.coefficient(), &Atom::num((1, 4)));
    assert_eq!(prepared.powers(), [4]);
    let (shifted, _) = family(&[Atom::one(), Atom::num(2)]);
    assert_eq!(
        prepare_family(&shifted, &[1, 1], Policy::SingleTerm { max_states: 32 })
            .unwrap()
            .report()
            .status,
        Status::Original(Fallback::MultipleTerms)
    );
    assert!(prepare_family(&scaled, &[2, 2], Policy::SingleTerm { max_states: 0 }).is_err());
}

#[test]
fn noncontiguous_source_labels_and_loop_numerator_are_preserved() {
    let k = parse!("family_prepare::q");
    let p = parse!("family_prepare::p");
    let kin = Kinematics::in_dimension(&parse!("family_prepare::D"))
        .unwrap()
        .with_momenta([k.clone(), p.clone()])
        .unwrap()
        .with_scalar_product(&p, &p, Atom::num(-1))
        .unwrap();
    let first = kin.scalar_product(&k, &k).unwrap() - 1;
    let shifted = &k + &p;
    let second = kin.scalar_product(&shifted, &shifted).unwrap() - 1;
    let original = IntegralFamily::new(
        vec![k.clone()],
        vec![p.clone()],
        vec![first.clone(), second.clone(), second.clone()],
        &kin,
    )
    .unwrap();
    let expected =
        IntegralFamily::new(vec![k], vec![p], vec![first, second.clone()], &kin).unwrap();
    let parameters = vec![
        symbol!("family_prepare::a"),
        symbol!("family_prepare::b"),
        symbol!("family_prepare::c"),
    ];
    let (actual, report) = ParametricIntegrand::from_family_prepared(
        &original,
        &[1, 2, 3],
        second.clone(),
        parameters.clone(),
        symbol!("family_prepare::eps"),
        parse!("2-2*family_prepare::eps"),
        Policy::default(),
    )
    .unwrap();
    assert_eq!(report.status, Status::Projected);
    assert_eq!(report.active_original_indices, [0, 2]);
    assert_eq!(report.active_powers, [1, 5]);
    assert_eq!(actual.parameters(), [parameters[0], parameters[2]]);
    let control = ParametricIntegrand::from_family(
        &expected,
        &[1, 5],
        second,
        vec![parameters[0], parameters[2]],
        symbol!("family_prepare::eps"),
        parse!("2-2*family_prepare::eps"),
    )
    .unwrap();
    assert_eq!(actual.density(), control.density());
}

#[test]
fn discarded_parameters_still_obey_the_original_admission_contract() {
    let x = symbol!("family_prepare::collision");
    let y = symbol!("family_prepare::survivor");
    let (regular, _) = family(&[Atom::one(), Atom::one()]);
    let eps = symbol!("family_prepare::eps");
    let dimension = parse!("2-2*family_prepare::eps");
    for (numerator, parameters, dim) in [
        (Atom::var(x), vec![x, y], dimension.clone()),
        (Atom::one(), vec![x, y], &dimension + Atom::var(x)),
        (Atom::one(), vec![x, x], dimension.clone()),
        (Atom::one(), vec![eps, y], dimension.clone()),
    ] {
        assert!(
            ParametricIntegrand::from_family_prepared(
                &regular,
                &[2, 3],
                numerator,
                parameters,
                eps,
                dim,
                Policy::default(),
            )
            .is_err()
        );
    }
    let (collision, _) = family(&[Atom::var(x), Atom::var(x)]);
    assert!(matches!(
        ParametricIntegrand::from_family_prepared(
            &collision,
            &[2, 3],
            Atom::one(),
            vec![x, y],
            eps,
            dimension,
            Policy::default(),
        ),
        Err(Error::IntegralFamily(
            feynkit_graph::IntegralFamilyError::InvalidLabels
        ))
    ));
}

#[test]
fn optional_native_integer_limits_fall_back_without_changing_valid_powers() {
    let (family, _) = family(&[Atom::one(), Atom::one()]);
    for powers in [[u32::MAX, 1], [i32::MAX as u32, i32::MAX as u32]] {
        let prepared = prepare_family(&family, &powers, Policy::default()).unwrap();
        assert_eq!(
            prepared.report().status,
            Status::Original(Fallback::PowerOutsideNativeRange)
        );
        assert_eq!(prepared.powers(), powers);
    }
    assert!(prepare_family(&family, &[1, 1], Policy::SingleUnitTerm { max_states: 0 }).is_err());
    assert!(prepare_family(&family, &[0, 1], Policy::default()).is_err());
    let invalid = IntegralFamily::new(
        family.loop_momenta().to_vec(),
        vec![],
        vec![Atom::Zero],
        family.kinematics(),
    )
    .unwrap();
    for policy in [Policy::Original, Policy::default()] {
        assert!(matches!(
            prepare_family(&invalid, &[1], policy),
            Err(Error::IntegralFamily(
                feynkit_graph::IntegralFamilyError::InvalidBasis(_)
            ))
        ));
    }
}

#[test]
fn native_graph_projection_applies_weight_and_raised_power_measure_once() {
    let model =
        Arc::new(Model::from_json(include_str!("../../../examples/models/scalar.json")).unwrap());
    let kin = Kinematics::in_dimension(&parse!("family_prepare::D")).unwrap();
    let base = GraphIntegral::from_dot(model,
        "digraph repeated { a -> b [id=0,particle=\"phi\",lmb_id=0]; b -> a [id=1,particle=\"phi\"]; }",&kin).unwrap();
    let diagram = base
        .diagram()
        .as_ref()
        .clone()
        .with_numerator(SymbolicTensor::infer(Atom::num(5)).unwrap())
        .unwrap()
        .with_overall_factor(Atom::num(2))
        .with_projector(SymbolicTensor::infer(Atom::num(3)).unwrap());
    let graph = GraphIntegral::new(Arc::new(diagram), &kin)
        .unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"), Atom::one())]))
        .unwrap()
        .with_powers(&BTreeMap::from([(EdgeId(0), 2), (EdgeId(1), 2)]))
        .unwrap()
        .with_measure_multiplier(Atom::num(7));
    let parameters = vec![symbol!("family_prepare::gx"), symbol!("family_prepare::gy")];
    let (prepared, report) = ParametricIntegrand::from_graph_prepared(
        &graph,
        parameters.clone(),
        symbol!("family_prepare::eps"),
        parse!("2-2*family_prepare::eps"),
        Policy::default(),
    )
    .unwrap();
    assert_eq!(report.status, Status::Projected);
    assert_eq!(report.active_powers, [4]);
    let value = finite(
        prepared
            .density()
            .replace(Pattern::Literal(Atom::var(parameters[1])))
            .with(1),
    );
    // 5*2*3*7 * Gamma(4-1)/Gamma(4) = 70, with positive even-power sign.
    assert_eq!(value, Atom::num(70));
    assert_eq!(graph.powers(), [2, 2]);
    assert_eq!(graph.family().denominators().len(), 2);
}
