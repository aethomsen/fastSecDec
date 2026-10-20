use fastsecdec::{
    Atom, AtomCore, EdgeId, FeynmanDiagram, Kinematics, Model,
    input::{GraphIntegral, GraphSymanzik},
};
use feynkit_graph::symbols;
use idenso::tensor::SymbolicTensor;
use std::{collections::BTreeMap, sync::Arc};
use symbolica::parse;

const MODEL: &str = include_str!("../../../examples/models/scalar.json");
const BUBBLE: &str = include_str!("../../../examples/graphs/bubble.dot");

fn initialize_model_symbols() {
    // Native model symbol/print registration is process-global. Initialize
    // shared names once before concurrent tests construct independent models.
    static INITIALIZED: std::sync::Once = std::sync::Once::new();
    INITIALIZED.call_once(|| {
        Model::from_json(MODEL).unwrap();
    });
}

fn admitted(dot: &str, mass: i64, s: Atom) -> GraphIntegral {
    initialize_model_symbols();
    let diagram =
        FeynmanDiagram::from_dot(Arc::new(Model::from_json(MODEL).unwrap()), dot).unwrap();
    let k = Kinematics::in_dimension(&parse!("D"))
        .unwrap()
        .with_mass_squared(&symbols::external_momentum().call(1), s)
        .unwrap();
    GraphIntegral::new_with_scalar_values(
        Arc::new(diagram),
        &k,
        &BTreeMap::from([(symbolica::symbol!("UFO::mt"), Atom::num(mass))]),
    )
    .unwrap()
}

#[test]
fn massive_bubble_retains_physical_sign_and_ignores_numerator_weights() {
    let graph = admitted(BUBBLE, 1, parse!("s"));
    let uf = GraphSymanzik::from_graph(&graph, vec![parse!("x"), parse!("y")]).unwrap();
    assert_eq!(uf.u(), &parse!("x+y"));
    assert!((uf.f() - parse!("(x+y)^2-s*x*y")).expand().is_zero());
    assert_eq!(uf.propagator_edges(), &[EdgeId(2), EdgeId(3)]);
    let weighted = graph
        .clone()
        .with_powers(&BTreeMap::from([(EdgeId(3), 3)]))
        .unwrap()
        .with_measure_multiplier(parse!("7*normalization"));
    let weighted_uf = GraphSymanzik::from_graph(&weighted, vec![parse!("x"), parse!("y")]).unwrap();
    assert_eq!(weighted_uf.f(), uf.f());
    let loop_graph = graph
        .diagram()
        .as_ref()
        .clone()
        .with_numerator(
            SymbolicTensor::infer(parse!(
                "gammalooprs::K(0,spenso::mink(4,mu))*gammalooprs::K(0,spenso::mink(4,mu))"
            ))
            .unwrap(),
        )
        .unwrap();
    let loop_graph = GraphIntegral::new_with_scalar_values(
        Arc::new(loop_graph),
        graph.family().kinematics(),
        &BTreeMap::from([(symbolica::symbol!("UFO::mt"), Atom::num(1))]),
    )
    .unwrap();
    assert_eq!(
        GraphSymanzik::from_graph(&loop_graph, vec![parse!("x"), parse!("y")])
            .unwrap()
            .f(),
        uf.f()
    );
    let physical = GraphSymanzik::from_graph(
        &admitted(BUBBLE, 1, Atom::num(5)),
        vec![parse!("x"), parse!("y")],
    )
    .unwrap();
    assert!((physical.f() - parse!("x^2-3*x*y+y^2")).expand().is_zero());
}

#[test]
fn two_loop_sunrise_matches_independent_hand_polynomials() {
    let dot = r#"digraph sunrise {
      incoming [style=invis]; outgoing [style=invis];
      incoming -> a [id=0,particle="phi"]; b -> outgoing [id=1,particle="phi"];
      a -> b [id=2,particle="phi",lmb_id=0];
      a -> b [id=3,particle="phi",lmb_id=1]; b -> a [id=4,particle="phi"];
    }"#;
    let graph = admitted(dot, 1, Atom::num(10));
    let uf =
        GraphSymanzik::from_graph(&graph, vec![parse!("x"), parse!("y"), parse!("z")]).unwrap();
    assert_eq!(uf.loop_count(), 2);
    assert!((uf.u() - parse!("x*y+x*z+y*z")).expand().is_zero());
    assert!(
        (uf.f() - parse!("(x*y+x*z+y*z)*(x+y+z)-10*x*y*z"))
            .expand()
            .is_zero()
    );
    let stable = graph.diagram().to_dot().unwrap();
    let restored = admitted(&stable, 1, Atom::num(10));
    let again = GraphSymanzik::from_graph(&restored, uf.parameters().to_vec()).unwrap();
    assert_eq!(again.u(), uf.u());
    assert_eq!(again.f(), uf.f());
    assert_eq!(again.propagator_edges(), uf.propagator_edges());
}

#[test]
fn native_label_validation_and_zero_f_are_explicit() {
    let graph = admitted(BUBBLE, 0, Atom::num(0));
    assert!(GraphSymanzik::from_graph(&graph, vec![parse!("x")]).is_err());
    assert!(GraphSymanzik::from_graph(&graph, vec![parse!("x"), parse!("x")]).is_err());
    let uf = GraphSymanzik::from_graph(&graph, vec![parse!("x"), parse!("y")]).unwrap();
    assert!(!uf.u().is_zero());
    assert!(uf.f().is_zero());
}

#[test]
fn unequal_masses_bind_to_the_native_edge_slots() {
    initialize_model_symbols();
    let mut model: serde_json::Value = serde_json::from_str(MODEL).unwrap();
    let mut mass = model["parameters"][1].clone();
    mass["name"] = serde_json::json!("mh");
    mass["value"] = serde_json::json!([2.0, 0.0]);
    model["parameters"].as_array_mut().unwrap().push(mass);
    let mut particle = model["particles"][0].clone();
    particle["name"] = serde_json::json!("heavy");
    particle["antiname"] = serde_json::json!("heavy");
    particle["mass"] = serde_json::json!("mh");
    particle["pdg_code"] = serde_json::json!(1001);
    model["particles"].as_array_mut().unwrap().push(particle);
    let mut propagator = model["propagators"][0].clone();
    propagator["name"] = serde_json::json!("heavy_prop");
    propagator["particle"] = serde_json::json!("heavy");
    propagator["denominator"] = serde_json::json!("(UFO::P(UFO::idx(1,1)))^2-UFO::mh^2");
    model["propagators"]
        .as_array_mut()
        .unwrap()
        .push(propagator);
    let mut vertex = model["vertex_rules"][1].clone();
    vertex["name"] = serde_json::json!("phi_phi_heavy");
    vertex["particles"] = serde_json::json!(["phi", "phi", "heavy"]);
    model["vertex_rules"].as_array_mut().unwrap().push(vertex);
    let dot = BUBBLE.replace("id=3, particle=\"phi\"", "id=3, particle=\"heavy\"");
    let diagram = FeynmanDiagram::from_dot(
        Arc::new(Model::from_json(&model.to_string()).unwrap()),
        &dot,
    )
    .unwrap();
    let k = Kinematics::in_dimension(&parse!("D"))
        .unwrap()
        .with_mass_squared(&symbols::external_momentum().call(1), Atom::num(10))
        .unwrap();
    let graph = GraphIntegral::new_with_scalar_values(
        Arc::new(diagram),
        &k,
        &BTreeMap::from([
            (symbolica::symbol!("UFO::mt"), Atom::num(1)),
            (symbolica::symbol!("UFO::mh"), Atom::num(2)),
        ]),
    )
    .unwrap();
    let uf = GraphSymanzik::from_graph(&graph, vec![parse!("x"), parse!("y")]).unwrap();
    assert!((uf.f() - parse!("(x-y)*(x-4*y)")).expand().is_zero());
    let swapped = GraphSymanzik::from_graph(&graph, vec![parse!("y"), parse!("x")]).unwrap();
    assert!((swapped.f() - parse!("(y-x)*(y-4*x)")).expand().is_zero());
    assert!(!(swapped.f() - uf.f()).expand().is_zero());
}
