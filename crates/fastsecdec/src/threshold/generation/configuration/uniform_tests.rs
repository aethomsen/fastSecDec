use super::*;
use crate::parametric::*;
use symbolica::symbol;

fn fixture(runtime: bool) -> (PreparationOptions, Request) {
    let (p, x, y, e, s, t) = symbol!(
        "uniform_config::p",
        "uniform_config::x",
        "uniform_config::y",
        "uniform_config::eps",
        "uniform_config::s",
        "uniform_config::t"
    );
    let input = ParametricIntegrand::new(
        vec![x, y],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    Atom::one() + Atom::var(y),
                    -Atom::one() - Atom::var(e),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let kinematics = if runtime {
        gcad::GcadKinematics {
            runtime_parameters: vec![p],
            strict_positive: vec![Atom::var(p) - Atom::num(2), Atom::num(3) - Atom::var(p)],
            ..Default::default()
        }
    } else {
        Default::default()
    };
    let request = gcad::GcadRequest::unit_cube(
        &input,
        kinematics,
        Default::default(),
        gcad::GcadRequest::default_limits(),
    )
    .unwrap();
    let mut options = PreparationOptions::new(t);
    options.regular_family = Some(RegularFamilyPreparation {
        coordinates: Some(vec![s, t]),
        ..Default::default()
    });
    (options, Request::Affine(Arc::new(request)))
}

#[test]
fn uniform_configuration_versions_cannot_transplant_fixed_or_missing_roles() {
    let root = tempfile::tempdir().unwrap();
    let (options, uniform) = fixture(true);
    let (_, fixed) = fixture(false);
    let reference = write(root.path(), &options, Route::Uniform).unwrap();
    let (stored, atoms, symbols): (Stored, _, _) =
        codec::read(root.path(), &reference, KIND).unwrap();
    assert_eq!(
        serde_json::to_value(&stored).unwrap()["regular_family"]["version"],
        2
    );
    let (mut restored, route) = read(root.path(), &reference, u64::MAX).unwrap();
    assert_eq!(route, Route::Uniform);
    super::super::regular::restore(&mut restored, &uniform, route).unwrap();
    assert!(matches!(
        super::super::regular::restore(&mut restored, &fixed, route),
        Err(Error::Association(_))
    ));
    for version in [1, 3, u32::MAX] {
        let mut metadata = serde_json::to_value(&stored).unwrap();
        metadata["regular_family"]["version"] = serde_json::json!(version);
        let modified: Stored = serde_json::from_value(metadata).unwrap();
        let forged = codec::write(
            root.path(),
            "transplanted-regular",
            KIND,
            &modified,
            Atoms {
                atoms: atoms.atoms.clone(),
            },
            symbols.clone(),
        )
        .unwrap();
        if version == 1 || version == 3 {
            let (mut options, route) = read(root.path(), &forged, u64::MAX).unwrap();
            assert_eq!(
                route,
                if version == 1 {
                    Route::FixedCube
                } else {
                    Route::ResolvedProjective
                }
            );
            assert!(matches!(
                super::super::regular::restore(&mut options, &uniform, route),
                Err(Error::Association(_))
            ));
        } else {
            assert!(matches!(
                read(root.path(), &forged, u64::MAX),
                Err(Error::Association(_))
            ));
        }
    }
    let mut missing = PreparationOptions::new(options.unit);
    assert!(matches!(
        super::super::regular::restore(&mut missing, &uniform, Route::LegacyFiber),
        Err(Error::Association(_))
    ));
    let fixed_record = write(root.path(), &options, Route::FixedCube).unwrap();
    let (mut fixed_options, fixed_route) = read(root.path(), &fixed_record, u64::MAX).unwrap();
    super::super::regular::restore(&mut fixed_options, &fixed, fixed_route).unwrap();
    assert_eq!(fixed_route, Route::FixedCube);
}
