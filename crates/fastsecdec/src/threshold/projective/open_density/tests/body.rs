use super::*;
use std::{
    sync::atomic::AtomicBool,
    time::{Duration, Instant},
};

fn remap(map: &CompiledOpenMap, unit: Symbol, power: Option<u32>) -> Arc<CompiledOpenMap> {
    let source = map.owner().source();
    let cell = PositiveProjectiveCellMap::new(
        source.decomposition().clone(),
        source.cell_index(),
        vec![unit],
    )
    .unwrap();
    let prefix = NormalizedPrefix::new(cell, &budget()).unwrap();
    let prefix = match power {
        Some(n) => prefix.power(n, &budget()).unwrap(),
        None => prefix.open(&budget()).unwrap(),
    };
    Arc::new(CompiledOpenMap::prepare(prefix, settings(), &budget()).unwrap())
}
fn bytes(body: &CompiledDensityBody) -> Vec<u8> {
    bincode::serde::encode_to_vec(body.exact(), bincode::config::standard()).unwrap()
}

#[test]
fn density_body_shares_owned_configuration_across_distinct_maps_and_measures_stages() {
    let (_, _, eps, c, _) = roles();
    let (k, u) = symbol!("density_body_test::k", "density_body_test::other_unit");
    let mut functions = FunctionMap::new();
    functions
        .add_aliases([
            (Atom::var(c), Atom::num(2) * Atom::var(k) + Atom::i()),
            (Atom::var(Symbol::PI), Atom::zero()),
        ])
        .unwrap();
    let mut timings = Vec::new();
    for first in fixture(None) {
        let source = first.owner().source();
        let negative =
            source.decomposition().native_result().cells[source.cell_index()].signs[0] < 0;
        let second = remap(&first, u, (!negative).then_some(2));
        assert!(!Arc::ptr_eq(&first, &second));
        let started = Instant::now();
        let body = Arc::new(
            CompiledDensityBody::prepare(
                &first,
                &[eps],
                &[k],
                functions.clone(),
                settings(),
                &budget(),
            )
            .unwrap(),
        );
        let body_seconds = started.elapsed().as_secs_f64();
        assert_eq!(
            format!("{:?}", body.function_map()),
            format!("{functions:?}")
        );
        let before = bytes(&body);
        let mut attached_seconds = Vec::new();
        let mut delegated_seconds = Vec::new();
        for map in [first, second] {
            let started = Instant::now();
            let reused = Arc::new(
                CompiledOpenDensity::prepare_with_body(
                    map.clone(),
                    body.clone(),
                    settings(),
                    &budget(),
                )
                .unwrap(),
            );
            attached_seconds.push(started.elapsed().as_secs_f64());
            let started = Instant::now();
            let delegated = Arc::new(
                CompiledOpenDensity::prepare(
                    map.clone(),
                    &[eps],
                    &[k],
                    functions.clone(),
                    settings(),
                    &budget(),
                )
                .unwrap(),
            );
            delegated_seconds.push(started.elapsed().as_secs_f64());
            assert!(Arc::ptr_eq(reused.body_owner(), &body));
            assert!(Arc::ptr_eq(reused.map_owner(), &map));
            assert_eq!(reused.term_phases(), delegated.term_phases());
            assert_eq!(reused.term_bodies(), delegated.term_bodies());
            let encoded =
                bincode::serde::encode_to_vec(reused.exact(), bincode::config::standard()).unwrap();
            assert_eq!(
                encoded,
                bincode::serde::encode_to_vec(delegated.exact(), bincode::config::standard())
                    .unwrap()
            );
            let (restored, used): (ExpressionEvaluator<Complex<Rational>>, usize) =
                bincode::serde::decode_from_slice(&encoded, bincode::config::standard()).unwrap();
            assert_eq!(used, encoded.len());
            for bits in [128, 256] {
                let real = |q: &Rational| Float::rational_at(q, bits);
                let mut actual = reused.map::<Float>(bits).unwrap();
                let mut expected = delegated.map::<Float>(bits).unwrap();
                let mut decoded = map
                    .scope()
                    .map_checked(&restored, bits, |q| Complex::new(real(&q.re), real(&q.im)))
                    .unwrap();
                for (u, e) in [((1, 4), (1, 8)), ((2, 3), (1, 4))] {
                    let unit = real(&Rational::from(u));
                    let eps = real(&Rational::from(e));
                    let coefficient = Complex::new(
                        real(&Rational::from((3, 2))),
                        real(&Rational::from((-1, 3))),
                    );
                    let a = actual
                        .evaluate(
                            std::slice::from_ref(&unit),
                            std::slice::from_ref(&eps),
                            std::slice::from_ref(&coefficient),
                        )
                        .unwrap();
                    let b = expected
                        .evaluate(
                            std::slice::from_ref(&unit),
                            std::slice::from_ref(&eps),
                            std::slice::from_ref(&coefficient),
                        )
                        .unwrap();
                    assert_eq!(a.terms, b.terms);
                    assert_eq!(a.total, b.total);
                    assert_eq!(a.coordinates, b.coordinates);
                    assert_eq!(a.jacobian, b.jacobian);
                    let mut outputs = vec![Complex::new(Float::new(bits), Float::new(bits)); 6];
                    decoded.evaluate(
                        &[
                            Complex::new(unit, Float::new(bits)),
                            Complex::new(eps, Float::new(bits)),
                            coefficient,
                        ],
                        &mut outputs,
                    );
                    assert_eq!(outputs[0], a.total);
                    assert_eq!(outputs[1..3], a.terms);
                }
            }
        }
        assert_eq!(bytes(&body), before);
        timings.push(serde_json::json!({"negative_phase":negative,"body_prepare_seconds":body_seconds,"attach_including_composer_finish_seconds":attached_seconds,"delegate_body_plus_attach_seconds":delegated_seconds,"body_preparations_reused":1,"attached_maps":2}));
    }
    println!(
        "density_body_sharing_study {}",
        serde_json::json!({"matched_settings":true,"stages":timings,"scope":"tiny native fixtures; includes final whole-IR optimization; no catalogue speed or memory claim"})
    );
}

#[test]
fn density_body_refuses_foreign_sign_owner_and_late_unit_collision_atomically() {
    let maps = fixture(None);
    let (_, _, eps, c, _) = roles();
    let late = symbol!("density_body_test::late_unit");
    let body = Arc::new(
        CompiledDensityBody::prepare(
            &maps[0],
            &[eps],
            &[c, late],
            FunctionMap::new(),
            settings(),
            &budget(),
        )
        .unwrap(),
    );
    let before = bytes(&body);
    for map in [
        maps[1].clone(),
        fixture(None).remove(0),
        remap(&maps[0], late, None),
    ] {
        assert!(matches!(
            CompiledOpenDensity::prepare_with_body(map, body.clone(), settings(), &budget()),
            Err(Error::Invalid(_))
        ));
    }
    let cancelled = budget().with_cancellation(Arc::new(AtomicBool::new(true)));
    let limited = Budget::new(Limits {
        wall_time: Duration::ZERO,
        ..Default::default()
    });
    for b in [&cancelled, &limited] {
        assert!(
            CompiledDensityBody::prepare(
                &maps[0],
                &[eps],
                &[c, late],
                FunctionMap::new(),
                settings(),
                b
            )
            .is_err()
        );
        assert!(
            CompiledOpenDensity::prepare_with_body(maps[0].clone(), body.clone(), settings(), b)
                .is_err()
        );
    }
    assert_eq!(bytes(&body), before);
    let result = CompiledOpenDensity::prepare_with_body(
        maps[0].clone(),
        body,
        settings().cpe_iterations(Some(1)),
        &budget(),
    )
    .unwrap();
    assert_ne!(
        format!("{:?}", result.composition_settings()),
        format!("{:?}", result.body_owner().optimization_settings())
    );
    assert_eq!(
        result.map_owner().owner().source().cell_index(),
        maps[0].owner().source().cell_index()
    );
}

#[test]
fn density_body_rechecks_late_normalized_root_and_chart_roles() {
    use crate::threshold::maps::normalized::tests::fixture as normalized_fixture;
    let (power, first, second) = normalized_fixture();
    let mut b = budget();
    let base = power
        .section(first.clone(), &mut b)
        .unwrap()
        .section(second.clone(), &mut b)
        .unwrap();
    let baseline = Arc::new(CompiledOpenMap::prepare(base, settings(), &b).unwrap());
    let eps = power
        .source()
        .decomposition()
        .preparation()
        .input()
        .regulator();
    for root_role in [true, false] {
        let role = symbol!("density_body_test::late_geometric_role");
        let body = Arc::new(
            CompiledDensityBody::prepare(
                &baseline,
                &[eps],
                &[role],
                FunctionMap::new(),
                settings(),
                &budget(),
            )
            .unwrap(),
        );
        let before = bytes(&body);
        let mut a = first.clone();
        let mut z = second.clone();
        if root_role {
            let old = a.root_symbol;
            a.root_symbol = role;
            for image in &mut z.chart.parameter_images {
                *image = image.replace(Atom::var(old)).with(Atom::var(role));
            }
        } else {
            let old = a.chart.parameters[0];
            a.chart.parameters[0] = role;
            for image in &mut a.chart.source_prefix {
                *image = image.replace(Atom::var(old)).with(Atom::var(role));
            }
            a.scale = a.scale.replace(Atom::var(old)).with(Atom::var(role));
        }
        let prefix = power
            .section(a, &mut b)
            .unwrap()
            .section(z, &mut b)
            .unwrap();
        let other = Arc::new(CompiledOpenMap::prepare(prefix, settings(), &b).unwrap());
        assert!(matches!(
            CompiledOpenDensity::prepare_with_body(other, body.clone(), settings(), &b),
            Err(Error::Invalid(_))
        ));
        assert_eq!(bytes(&body), before);
        assert!(
            CompiledOpenDensity::prepare_with_body(baseline.clone(), body, settings(), &b).is_ok()
        );
    }
}
