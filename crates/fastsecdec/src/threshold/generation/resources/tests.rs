use super::*;
use crate::threshold::generation::{configuration, regular::Route};
use symbolica::symbol;

fn options() -> PreparationOptions {
    let (s, t) = symbol!("endpoint_resources::s", "endpoint_resources::t");
    let mut options = PreparationOptions::new(t);
    options.regular_family = Some(RegularFamilyPreparation {
        coordinates: Some(vec![s, t]),
        ..Default::default()
    });
    let settings = options.regular_family.as_mut().unwrap();
    settings.limits.unit_margin = Rational::from((1, 8));
    settings.limits.geometry.max_terms = 731;
    settings.limits.geometry.certificates.polya_order = 2;
    settings.discovery.max_family_attempts = 9;
    options
}

#[test]
fn omitted_resources_and_automatic_enable_preserve_explicit_native_policy_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let mut options = options();
    let before = configuration::write(dir.path(), &options, Route::FixedCube).unwrap();
    options.enable_automatic_endpoint_routes();
    options
        .apply_endpoint_resources(&EndpointPreparationResources::default())
        .unwrap();
    let after = configuration::write(dir.path(), &options, Route::FixedCube).unwrap();
    assert_eq!(before, after);
    let mut legacy = PreparationOptions::new(options.unit);
    assert!(legacy.regular_family.is_none());
    assert!(
        legacy
            .apply_endpoint_resources(&EndpointPreparationResources::default())
            .is_err()
    );
    assert!(legacy.regular_family.is_none());
    legacy.enable_automatic_endpoint_routes();
    assert!(legacy.regular_family.is_some());
}

#[test]
fn partial_resources_are_independent_of_gcad_and_persist_effective_native_policy() {
    let dir = tempfile::tempdir().unwrap();
    let mut options = options();
    let gcad = serde_json::to_value(&options.threshold.gcad_limits).unwrap();
    let resources: EndpointPreparationResources = serde_json::from_str(
        r#"{
        "geometry":{"wall_time_secs":17700,"certificates":{"work_limit":16000000000}},
        "discovery":{"wall_time_secs":17701},
        "prefactors":{"source_nodes":8192},"max_charts":1024
    }"#,
    )
    .unwrap();
    resources.validate().unwrap();
    options.apply_endpoint_resources(&resources).unwrap();
    assert_eq!(
        serde_json::to_value(&options.threshold.gcad_limits).unwrap(),
        gcad
    );
    let record = configuration::write(dir.path(), &options, Route::FixedCube).unwrap();
    let (restored, route) = configuration::read(dir.path(), &record, u64::MAX).unwrap();
    assert_eq!(route, Route::FixedCube);
    assert_eq!(restored.regularization.max_charts, 1024);
    assert_eq!(restored.regularization.prefactors.source_nodes, 8192);
    let settings = restored.regular_family.unwrap();
    assert_eq!(
        settings.limits.geometry.wall_time,
        Duration::from_secs(17700)
    );
    assert_eq!(settings.discovery.wall_time, Duration::from_secs(17701));
    assert_eq!(settings.discovery.max_family_attempts, 9);
    assert_eq!(settings.limits.geometry.max_terms, 731);
    assert_eq!(settings.limits.geometry.certificates.polya_order, 2);
    assert_eq!(
        settings.limits.geometry.certificates.work_limit,
        16_000_000_000usize
    );
    assert_eq!(settings.limits.unit_margin, Rational::from((1, 8)));
    assert_eq!(
        settings.coordinates,
        options.regular_family.unwrap().coordinates
    );
}

#[test]
fn invalid_overrides_are_atomic_and_unknown_or_overflowed_scalars_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let mut options = options();
    let before = configuration::write(dir.path(), &options, Route::FixedCube).unwrap();
    for seconds in [0., -1., f64::NAN, f64::INFINITY, f64::MAX] {
        let resources = EndpointPreparationResources {
            max_charts: Some(17),
            geometry: GeometryResources {
                max_terms: Some(999),
                wall_time_secs: Some(seconds),
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(options.apply_endpoint_resources(&resources).is_err());
        assert_eq!(
            configuration::write(dir.path(), &options, Route::FixedCube).unwrap(),
            before
        );
    }
    for input in [
        r#"{"discovery":{"max_dyadic_level":31}}"#,
        r#"{"prefactors":{"integer_power":-1}}"#,
        r#"{"prefactors":{"degree":65535}}"#,
    ] {
        let resources: EndpointPreparationResources = serde_json::from_str(input).unwrap();
        assert!(options.apply_endpoint_resources(&resources).is_err());
        assert_eq!(
            configuration::write(dir.path(), &options, Route::FixedCube).unwrap(),
            before
        );
    }
    for input in [
        r#"{"gcad_limits":{"wall_time_secs":17700}}"#,
        r#"{"geometry":{"certificates":{"unknown":1}}}"#,
        r#"{"geometry":{"wall_seconds":10}}"#,
        r#"{"prefactors":{"degree":65536}}"#,
    ] {
        assert!(serde_json::from_str::<EndpointPreparationResources>(input).is_err());
    }
    if usize::BITS > 32 {
        let mut resources = EndpointPreparationResources::default();
        resources.geometry.max_degree = Some(usize::MAX);
        assert!(options.apply_endpoint_resources(&resources).is_err());
        assert_eq!(
            configuration::write(dir.path(), &options, Route::FixedCube).unwrap(),
            before
        );
    }
}
