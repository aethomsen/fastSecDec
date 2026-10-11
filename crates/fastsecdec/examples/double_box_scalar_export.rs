//! Reproduce the scalar D05 DOT using the native HEPKit graph serializer.
use fastsecdec::{Atom, FeynmanDiagram, Model, ParameterCard};
use std::{error::Error, path::PathBuf, sync::Arc};

fn main() -> Result<(), Box<dyn Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: cargo run -p fastsecdec --example double_box_scalar_export -- OUTPUT.dot")?;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = root.join("examples/gghh_double_box");
    let card = ParameterCard::from_json(&std::fs::read_to_string(source.join("parameters.json"))?)?;
    let mut model = Model::from_json(&std::fs::read_to_string(source.join("model.json"))?)?;
    model.apply_parameter_card(&card)?;
    let model = Arc::new(model);
    let physical = FeynmanDiagram::from_dot(
        model.clone(),
        &std::fs::read_to_string(source.join("graph.dot"))?,
    )?;
    physical.validate()?;
    assert_eq!(physical.overall_factor(), &Atom::num(-1));
    // Remove physical numerator/projector and fermion multiplier. Native loop
    // measure and Feynman parameterization signs are applied later, unchanged.
    let scalar = physical
        .clone()
        .with_projector(Atom::one())
        .with_overall_factor(Atom::one())
        .with_numerator_prefactor(Atom::one())
        .with_numerator(Atom::one())?;
    scalar.validate()?;
    assert_eq!(scalar.id(), physical.id());
    let dot = scalar.to_dot()?;
    let restored = FeynmanDiagram::from_dot(model, &dot)?;
    restored.validate()?;
    assert_eq!(restored.id(), physical.id());
    assert!(restored.numerator().is_one());
    assert!(restored.projector().is_one());
    assert!(restored.overall_factor().is_one());
    assert!(restored.numerator_prefactor().is_one());
    assert_eq!(restored.to_json()?, scalar.to_json()?);
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, dot)?;
    println!(
        "native scalar D05 DOT roundtrip passed: {}",
        output.display()
    );
    Ok(())
}
