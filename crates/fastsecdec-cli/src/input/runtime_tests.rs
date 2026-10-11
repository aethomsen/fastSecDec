use super::*;

fn card(roles: &str, extra: &str) -> String {
    format!(
        "[direct]\ndomain='unit_cube'\nparameters=['x']\n{roles}\n[[direct.terms]]\nprefactor='p'\n[[direct.terms.factors]]\npolynomial='x+p'\nexponent='-eps'\n{extra}"
    )
}

#[test]
fn direct_runtime_roles_are_explicit_and_keep_the_original_symbolic_point() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.toml");
    fs::write(&path, card("runtime_parameters=['p']", "")).unwrap();
    let declared = load(&path).unwrap();
    assert_eq!(declared.runtime_parameters, [symbol("p").unwrap()]);
    assert_eq!(declared.integrand.parameters(), [symbol("x").unwrap()]);
    fs::write(&path, card("", "")).unwrap();
    let undeclared = load(&path).unwrap();
    assert!(undeclared.runtime_parameters.is_empty());
    assert_eq!(declared.integrand, undeclared.integrand);
}

#[test]
fn direct_runtime_roles_reject_alias_collisions_and_fixed_values() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.toml");
    for (roles, extra, expected) in [
        ("['p','feynkit_graph::p']", "", "duplicate symbol aliases"),
        ("['x']", "", "integration parameter or regulator"),
        ("['eps']", "", "integration parameter or regulator"),
        ("['p']", "[parameters]\np=3\n", "cannot bind a declared"),
        ("['p+1']", "", "expected a plain symbol"),
    ] {
        fs::write(&path, card(&format!("runtime_parameters={roles}"), extra)).unwrap();
        let error = load(&path).err().expect("invalid role must refuse");
        assert!(error.to_string().contains(expected), "{error}");
    }
}
