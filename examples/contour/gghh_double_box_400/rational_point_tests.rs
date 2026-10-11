use super::*;

#[test]
fn rational_t_regenerates_consistent_external_data() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir()?;
    let output = temporary.path().join("rational");
    run_at_point(&root, &output, 1000, Some(Rational::from(-97077)))?;
    let fixture = root.join("examples/contour/gghh_double_box_1000_rational");
    for file in [
        "run.toml",
        "point.toml",
        "point-exact.json",
        "validation.json",
    ] {
        assert_eq!(
            std::fs::read(output.join(file))?,
            std::fs::read(fixture.join(file))?,
            "{file}"
        );
    }
    let report: Value = serde_json::from_slice(&std::fs::read(output.join("validation.json"))?)?;
    assert_eq!(report["mandelstam_t_GeV2"], "-97077");
    assert_eq!(report["mandelstam_u_GeV2"], "-871673");
    assert_eq!(report["exact_on_shell_and_conservation"], true);
    assert_eq!(
        report["native_incoming_plus_plus_wavefunction_checks"],
        true
    );
    let point: toml::Value = toml::from_str(&std::fs::read_to_string(output.join("point.toml"))?)?;
    assert_eq!(point["parameters"]["p0p1"].as_float(), Some(500000.0));
    assert_eq!(point["parameters"]["p0p2"].as_float(), Some(56351.0));
    assert_eq!(point["parameters"]["p1p2"].as_float(), Some(443649.0));
    assert!(run_at_point(&root, &output, 1000, Some(Rational::from(-97077))).is_err());
    for (sqrt_s, t) in [(250, -97077), (1000, 0), (1000, -1000000)] {
        let invalid = temporary.path().join(format!("invalid-{sqrt_s}-{t}"));
        assert!(run_at_point(&root, &invalid, sqrt_s, Some(Rational::from(t))).is_err());
        assert!(!invalid.exists());
    }
    Ok(())
}

#[test]
fn rational_t_matches_the_original_angle_when_the_invariant_is_rational() -> Result<()> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temporary = tempfile::tempdir()?;
    let original = temporary.path().join("angle");
    let rational = temporary.path().join("invariant");
    run_at_energy(&root, &original, 650)?;
    run_at_point(&root, &rational, 650, Some(Rational::from(-39625)))?;
    assert_eq!(
        std::fs::read(original.join("point-exact.json"))?,
        std::fs::read(rational.join("point-exact.json"))?,
    );
    let read = |directory: &Path| -> Result<toml::Value> {
        Ok(toml::from_str(&std::fs::read_to_string(
            directory.join("point.toml"),
        )?)?)
    };
    assert_eq!(read(&original)?, read(&rational)?);
    Ok(())
}
