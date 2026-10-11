//! Input-only D05 point export. No contraction, sector generation or sampling.
//!
//! Build as an example in the fastsecdec package (see this folder's README).

use fastsecdec::input::RuntimeModelBindings;
use feynkit_graph::FeynmanDiagram;
use feynkit_kinematics::FourMomentum;
use feynkit_model::{Model, ParameterCard};
use numerica::domains::rational::Rational;
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};
use symbolica::atom::{Atom, AtomCore};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[allow(dead_code)]
#[path = "../../../crates/fastsecdec/examples/gghh_double_box/point.rs"]
mod point;
#[allow(dead_code)]
#[path = "../../../crates/fastsecdec/examples/gghh_double_box/select.rs"]
mod select;

fn require(ok: bool, message: &str) -> Result<()> {
    if !ok {
        return Err(message.into());
    }
    Ok(())
}
fn number(value: &toml::Value) -> Result<f64> {
    value
        .as_float()
        .ok_or_else(|| "point entries must be TOML floats".into())
}
fn scalar<'a>(point: &'a point::Point, left: &str, right: &str) -> Result<&'a str> {
    let left = point::expression(left)?;
    let right = point::expression(right)?;
    let matches = point
        .products
        .iter()
        .filter_map(|p| {
            let a = point::expression(&p.left).ok()?;
            let b = point::expression(&p.right).ok()?;
            ((a == left && b == right) || (a == right && b == left)).then_some(p.value.as_str())
        })
        .collect::<Vec<_>>();
    require(
        matches.len() == 1,
        "Gram entry must match exactly one native product",
    )?;
    Ok(matches[0])
}
fn run(root: &Path, output: &Path) -> Result<()> {
    run_at_energy(root, output, 400)
}
fn run_at_energy(root: &Path, output: &Path, sqrt_s_gev: u32) -> Result<()> {
    run_at_point(root, output, sqrt_s_gev, None)
}
fn run_at_point(
    root: &Path,
    output: &Path,
    sqrt_s_gev: u32,
    mandelstam_t: Option<Rational>,
) -> Result<()> {
    require(!output.exists(), "use a fresh output directory")?;
    let source = root.join("examples/gghh_double_box");
    let run_text = std::fs::read_to_string(source.join("run.toml"))?;
    let run: toml::Value = toml::from_str(&run_text)?;
    let old: toml::Value = toml::from_str(&std::fs::read_to_string(source.join("point.toml"))?)?;
    let old = old["parameters"].as_table().ok_or("old point table")?;
    let model_text = std::fs::read_to_string(source.join("model.json"))?;
    let card_text = std::fs::read_to_string(source.join("parameters.json"))?;
    let card = ParameterCard::from_json(&card_text)?;
    let mut model = Model::from_json(&model_text)?;
    let model_fingerprint = model.fingerprint();
    let model_names = model
        .parameters()
        .iter()
        .map(|p| p.name.as_str())
        .collect::<BTreeSet<_>>();
    let card_json: Value = serde_json::from_str(&card_text)?;
    let card_keys = card_json
        .as_object()
        .ok_or("parameter card object")?
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    require(
        card_keys
            .iter()
            .all(|name| model_names.contains(name.as_str())),
        "parameter card contains non-model inputs",
    )?;
    model.apply_parameter_card(&card)?;
    require(
        model.fingerprint() == model_fingerprint,
        "existing model/card identity changed",
    )?;
    let diagram = FeynmanDiagram::from_dot(
        Arc::new(model),
        &std::fs::read_to_string(source.join("graph.dot"))?,
    )?;
    let topology =
        select::double_box(&diagram)?.ok_or("expected native D05 s-channel double box")?;
    let model_inputs = RuntimeModelBindings::new(&diagram, Some(&card), &BTreeMap::new())?;
    let defaults = model_inputs.defaults();
    let old_native = point::Point::with_sqrt_s(&diagram, 300)?;
    let new_native = match &mandelstam_t {
        Some(t) => point::Point::with_mandelstam_t(&diagram, sqrt_s_gev, t.clone())?,
        None => point::Point::with_sqrt_s(&diagram, sqrt_s_gev)?,
    };
    require(
        new_native.polarization_gram_checks_complete,
        "native polarization checks incomplete",
    )?;
    let products = run["kinematics"]["products"]
        .as_array()
        .ok_or("run Gram entries")?;
    require(
        products.len() == 15 && new_native.products.len() == 15,
        "expected complete 5-vector Gram data",
    )?;
    let mut parameters = toml::map::Map::new();
    let mut regenerated = Vec::new();
    let mut fixed = Vec::new();
    for product in products {
        let left = product["left"].as_str().ok_or("left vector")?;
        let right = product["right"].as_str().ok_or("right vector")?;
        let before = scalar(&old_native, left, right)?;
        let after = scalar(&new_native, left, right)?;
        if let Some(name) = product.get("symbol").and_then(toml::Value::as_str) {
            require(
                number(old.get(name).ok_or("missing old Gram value")?)?.to_bits()
                    == point::real_value(before)?.to_bits(),
                "300 GeV native exporter no longer reproduces old runtime input",
            )?;
            let value = point::real_value(after)?;
            parameters.insert(name.to_owned(), toml::Value::Float(value));
            regenerated.push(json!({"name":name,"exact":after,"value":value,"previous_value":number(&old[name])?}));
        } else {
            let expected = point::expression(product["value"].as_str().ok_or("fixed Gram value")?)?;
            require(
                (point::expression(before)? - &expected).expand().is_zero()
                    && (point::expression(after)? - &expected).expand().is_zero(),
                "fixed on-shell Gram condition changed",
            )?;
            fixed.push(json!({"left":left,"right":right,"value":expected.to_canonical_string()}));
        }
    }
    let mut inherited = BTreeMap::new();
    for (name, value) in old {
        if name.starts_with("model::") {
            let supplied = number(value)?;
            require(
                defaults
                    .get(name)
                    .is_some_and(|v| v.to_bits() == supplied.to_bits()),
                "old used-model input does not match native model/card",
            )?;
            inherited.insert(name.clone(), supplied);
            parameters.insert(name.clone(), toml::Value::Float(supplied));
        }
    }
    require(
        regenerated.len() == 13
            && fixed.len() == 2
            && inherited.len() == 6
            && parameters.len() == old.len(),
        "runtime schema changed",
    )?;
    let reference: Value = serde_json::from_slice(&std::fs::read(
        root.join("example/gg_hh_one_loop_ME/threshold/point.json"),
    )?)?;
    for name in ["MT", "ymt", "MH", "Gf", "MZ", "aEWM1", "aS", "WT", "WH"] {
        require(
            card_json[name][0].as_f64() == reference["parameters"][name].as_f64()
                && card_json[name][1].as_f64() == Some(0.0),
            "model values differ from established 400 GeV point",
        )?;
    }
    require(
        reference["sqrt_s_GeV"] == json!(400.0) && reference["helicities"] == json!([1, 1, 0, 0]),
        "shared point energy or helicity mismatch",
    )?;
    let mut vectors = Vec::new();
    require(
        new_native
            .physical_momenta
            .keys()
            .copied()
            .collect::<Vec<_>>()
            == [0, 1, 2, 3],
        "expected incoming external labels 0,1 and outgoing labels 2,3",
    )?;
    for (i, components) in new_native.physical_momenta.values().enumerate() {
        let values = components
            .iter()
            .map(|c| point::expression(c))
            .collect::<Result<Vec<_>>>()?;
        if sqrt_s_gev == 400 && mandelstam_t.is_none() {
            for (j, component) in components.iter().enumerate() {
                let expected = reference["physical_momenta_GeV"][i][j]
                    .as_f64()
                    .ok_or("reference momentum")?;
                require(
                    point::real_value(component)?.to_bits() == expected.to_bits(),
                    "native physical momentum differs from shared 400 GeV point",
                )?;
            }
        }
        let p = FourMomentum::from_args(
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            values[3].clone(),
        );
        require(
            (p.dot(&p) - Atom::num(if i < 2 { 0 } else { 125 * 125 }))
                .expand()
                .is_zero(),
            "native external on-shell identity failed",
        )?;
        vectors.push(p);
    }
    require(vectors.len() == 4, "expected four external momenta")?;
    for j in 0..4 {
        require(
            (vectors[0].components()[j].clone() + vectors[1].components()[j].clone()
                - vectors[2].components()[j].clone()
                - vectors[3].components()[j].clone())
            .expand()
            .is_zero(),
            "exact physical momentum conservation failed",
        )?;
    }
    require(
        (vectors[0].dot(&vectors[1]) * Atom::num(2) - Atom::num(u64::from(sqrt_s_gev).pow(2)))
            .expand()
            .is_zero(),
        "s invariant mismatch",
    )?;
    if let Some(t) = &mandelstam_t {
        require(
            (Atom::num(125 * 125)
                - vectors[0].dot(&vectors[2]) * Atom::num(2)
                - Atom::num(t.clone()))
            .expand()
            .is_zero(),
            "requested t invariant mismatch",
        )?;
    }
    let mut generated_run = run.clone();
    for (key, file) in [
        ("graph", "graph.dot"),
        ("model", "model.json"),
        ("parameter_card", "parameters.json"),
    ] {
        generated_run["input"][key] = toml::Value::String(format!("../../gghh_double_box/{file}"));
    }
    generated_run["generation"]
        .as_table_mut()
        .ok_or("generation table")?
        .insert("contour".into(), toml::Value::Boolean(true));
    let mut hashes = BTreeMap::new();
    for path in [
        "examples/gghh_double_box/graph.dot",
        "examples/gghh_double_box/model.json",
        "examples/gghh_double_box/parameters.json",
        "examples/gghh_double_box/run.toml",
        "examples/gghh_double_box/point.toml",
        "crates/fastsecdec/examples/gghh_double_box/point.rs",
        "crates/fastsecdec/examples/gghh_double_box/select.rs",
        "example/gg_hh_one_loop_ME/threshold/point.json",
    ] {
        hashes.insert(
            path,
            blake3::hash(&std::fs::read(root.join(path))?)
                .to_hex()
                .to_string(),
        );
    }
    let mut report = json!({
        "schema":1,"status":"native input checks passed; generation and numerical integration not run",
        "sqrt_s_GeV":sqrt_s_gev,"model_fingerprint":model_fingerprint,"source_blake3":hashes,
        "native_topology":topology,"card_model_keys":card_keys,"card_contains_only_model_parameters":true,
        "fixed_gram_products":fixed,"regenerated_runtime_products":regenerated,"inherited_runtime_model_defaults":inherited,
        "native_300_GeV_values_reproduced_bitwise":true,"matches_shared_400_GeV_momenta_bitwise":(sqrt_s_gev == 400 && mandelstam_t.is_none()).then_some(true),
        "exact_on_shell_and_conservation":true,"native_incoming_plus_plus_wavefunction_checks":true,
        "normalization":{"color_projector":"unnormalized delta_ab","measure_multiplier":"1","loop_measure":"prod_l d^D k_l / (i*pi^(D/2))","dimension":"4-2*eps","individual_diagram":"D05 s-channel","spin_or_color_average":false,"diagram_sum":false},
        "not_run":["tensor contraction","parametric or sector generation","contour admission","numerical integration","independent physical reference"]
    });
    if let Some(t) = &mandelstam_t {
        report["mandelstam_t_GeV2"] = json!(t.to_string());
        report["mandelstam_u_GeV2"] = json!(
            (Atom::num(2 * 125 * 125)
                - Atom::num(u64::from(sqrt_s_gev).pow(2))
                - Atom::num(t.clone()))
            .to_canonical_string()
        );
    }
    let angle_description = mandelstam_t
        .as_ref()
        .map_or_else(|| "cos(theta)=4/5".to_owned(), |t| format!("t={t} GeV^2"));
    std::fs::create_dir_all(output)?;
    std::fs::write(
        output.join("run.toml"),
        format!(
            "# D05 input with native {sqrt_s_gev} GeV runtime data; shared assets remain unchanged.\n{}",
            toml::to_string_pretty(&generated_run)?
        ),
    )?;
    std::fs::write(
        output.join("point.toml"),
        format!(
            "# Native incoming++ point at sqrt(s)={sqrt_s_gev} GeV, {angle_description}.\n{}",
            toml::to_string_pretty(&toml::Value::Table(
                [(String::from("parameters"), toml::Value::Table(parameters))]
                    .into_iter()
                    .collect()
            ))?
        ),
    )?;
    for (name, value) in [
        ("point-exact.json", serde_json::to_value(&new_native)?),
        ("validation.json", report),
    ] {
        std::fs::write(
            output.join(name),
            format!("{}\n", serde_json::to_string_pretty(&value)?),
        )?;
    }
    println!(
        "native D05 input checks passed: 15 Gram products, 13 regenerated runtime values, 6 unchanged model inputs"
    );
    Ok(())
}
fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    require(
        args.len() == 2
            || (args.len() == 4 && args[2] == "--sqrt-s")
            || (args.len() == 6 && args[2] == "--sqrt-s" && args[4] == "--t"),
        "usage: prepare REPOSITORY_ROOT FRESH_OUTPUT_DIRECTORY [--sqrt-s GEV [--t EXACT_RATIONAL]]",
    )?;
    if args.len() == 2 {
        run(Path::new(&args[0]), Path::new(&args[1]))
    } else if args.len() == 4 {
        run_at_energy(Path::new(&args[0]), Path::new(&args[1]), args[3].parse()?)
    } else {
        let t = Rational::try_from(point::expression(&args[5])?)?;
        run_at_point(
            Path::new(&args[0]),
            Path::new(&args[1]),
            args[3].parse()?,
            Some(t),
        )
    }
}

#[cfg(test)]
mod rational_point_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_input_reproduction_matches_committed_fixture() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let temporary = tempfile::tempdir()?;
        let output = temporary.path().join("fixture");
        run(&root, &output)?;
        let fixture = root.join("examples/contour/gghh_double_box_400");
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
        assert!(
            run(&root, &output).is_err(),
            "existing inputs must not be overwritten"
        );
        Ok(())
    }

    #[test]
    fn thousand_gev_preserves_generation_input_and_regenerates_runtime_point() -> Result<()> {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let temporary = tempfile::tempdir()?;
        let output = temporary.path().join("fixture");
        run_at_energy(&root, &output, 1000)?;
        let fixture = root.join("examples/contour/gghh_double_box_1000");
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
        let generated_run: toml::Value =
            toml::from_str(&std::fs::read_to_string(output.join("run.toml"))?)?;
        let old_run: toml::Value = toml::from_str(&std::fs::read_to_string(
            root.join("examples/contour/gghh_double_box_400/run.toml"),
        )?)?;
        assert_eq!(generated_run, old_run, "energy is a runtime binding only");
        let point: toml::Value =
            toml::from_str(&std::fs::read_to_string(output.join("point.toml"))?)?;
        assert_eq!(point["parameters"].as_table().unwrap().len(), 19);
        assert_eq!(point["parameters"]["p0p1"].as_float(), Some(500_000.0));
        let report: Value =
            serde_json::from_slice(&std::fs::read(output.join("validation.json"))?)?;
        assert_eq!(report["sqrt_s_GeV"], 1000);
        assert!(report["matches_shared_400_GeV_momenta_bitwise"].is_null());
        assert_eq!(report["exact_on_shell_and_conservation"], true);
        assert_eq!(
            report["native_incoming_plus_plus_wavefunction_checks"],
            true
        );
        assert!(run_at_energy(&root, &output, 1000).is_err());
        assert!(run_at_energy(&root, &temporary.path().join("threshold"), 250).is_err());
        Ok(())
    }
}
