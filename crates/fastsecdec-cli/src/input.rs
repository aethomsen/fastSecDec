use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};

use fastsecdec::{
    Atom, AtomCore, EdgeId, Kinematics, Model, ParameterCard, Symbol,
    input::{GraphIntegral, RuntimeModelBindings},
    parametric::{
        FactorRole, FamilyPreparationPolicy, FamilyPreparationReport, ParametricDomain,
        ParametricIntegrand, ParametricTerm, PolynomialFactor,
    },
};
use feynkit_model::resolve_scalar_bindings;
use symbolica::{
    atom::AtomView,
    domains::rational::Rational,
    id::{Pattern, Replacement},
    parser::ParseSettings,
};

use crate::{
    CliResult,
    config::{MomentumName, RunCard},
};

impl MomentumName {
    fn atom(&self) -> CliResult<Atom> {
        match self {
            Self::External(index) => Ok(feynkit_graph::symbols::external_momentum().call(*index)),
            Self::Expression(text) => expression(text),
        }
    }
}

pub struct LoadedInput {
    pub card: RunCard,
    pub integrand: ParametricIntegrand,
    pub runtime_parameters: Vec<Symbol>,
    pub runtime_mass_constraints: Vec<fastsecdec::kernel::RuntimeMassConstraint>,
    pub model_parameter_defaults: BTreeMap<String, f64>,
    pub label: String,
    pub loops: Option<usize>,
    pub propagators: usize,
    pub sources: Vec<crate::artifact::SourceFile>,
    pub family_preparation: Option<FamilyPreparationReport>,
    pub independent_externals: Vec<String>,
    pub dependent_externals: Vec<String>,
    pub input_seconds: f64,
    pub parametrization_seconds: f64,
}

pub fn expression(text: &str) -> CliResult<Atom> {
    Atom::parse(text, "feynkit_graph", ParseSettings::default())
        .map_err(|error| format!("invalid Symbolica expression {text:?}: {error}").into())
}

pub fn symbol(text: &str) -> CliResult<Symbol> {
    match expression(text)?.as_view() {
        AtomView::Var(variable) => Ok(variable.get_symbol()),
        _ => Err(format!("expected a plain symbol, got {text:?}").into()),
    }
}

fn bind(expression: &Atom, values: &BTreeMap<Symbol, Atom>) -> Atom {
    expression.replace_multiple(values.iter().map(|(symbol, value)| {
        Replacement::new(
            Pattern::Literal(Atom::var(*symbol)),
            Pattern::Literal(value.clone()),
        )
    }))
}

pub fn value_expression(value: &toml::Value) -> CliResult<Atom> {
    match value {
        toml::Value::String(text) => expression(text),
        toml::Value::Integer(value) => Ok(Atom::num(*value)),
        toml::Value::Float(value) if value.is_finite() => {
            Ok(Atom::num(Rational::try_from(*value)?))
        }
        _ => Err("parameter values must be Symbolica strings or finite numbers".into()),
    }
}

/// Native admission shared by generation and exact U/F export. No numerator
/// contraction or sector preparation takes place in this stage.
pub struct GraphAdmission {
    pub graph: GraphIntegral,
    pub parameters: Vec<Symbol>,
    pub runtime_parameters: Vec<Symbol>,
    pub runtime_model: Option<RuntimeModelBindings>,
    pub values: BTreeMap<Symbol, Atom>,
    pub independent_externals: Vec<String>,
    pub dependent_externals: Vec<String>,
}

fn admit_graph(
    card: &RunCard,
    base: &Path,
    values: BTreeMap<Symbol, Atom>,
    sources: &mut Vec<crate::artifact::SourceFile>,
) -> CliResult<GraphAdmission> {
    let input = card
        .input
        .as_ref()
        .ok_or("U/F export requires native graph input")?;
    if card.direct.is_some() {
        return Err("graph input cannot also contain direct data".into());
    }
    let regulator = symbol(&card.integral.regulator)?;
    let read = |path: &Path, sources: &mut Vec<crate::artifact::SourceFile>| {
        read_source(path, base, sources)
    };
    let mut model = Model::from_json(&read(&base.join(&input.model), sources)?)?;
    let restriction = if let Some(parameter_card) = &input.parameter_card {
        let restriction = ParameterCard::from_json(&read(&base.join(parameter_card), sources)?)?;
        model.apply_parameter_card(&restriction)?;
        restriction
    } else {
        ParameterCard::new()
    };
    let graph_text = read(&base.join(&input.graph), sources)?;
    let diagram = feynkit_graph::FeynmanDiagram::from_dot(Arc::new(model), &graph_text)?;
    let runtime_model = match input.model_parameters {
        crate::config::ModelParameters::Runtime => Some(RuntimeModelBindings::new(
            &diagram,
            Some(&restriction),
            &values,
        )?),
        crate::config::ModelParameters::Fixed => None,
    };
    let values = match &runtime_model {
        Some(runtime) => runtime.values().clone(),
        None => diagram
            .model()
            .scalar_bindings(Some(&restriction), &values)?,
    };
    let model_symbols = runtime_model
        .as_ref()
        .map_or_else(Vec::new, RuntimeModelBindings::symbols);
    if model_symbols.contains(&regulator) {
        return Err("runtime model inputs must differ from the regulator".into());
    }
    let mut kinematics = Kinematics::in_dimension(&expression("D")?)?;
    let mut runtime_parameters = Vec::new();
    for product in &card.kinematics.products {
        let product_value = match (&product.value, &product.symbol) {
            (Some(value), None) => bind(&expression(value)?, &values),
            (None, Some(name)) => {
                let parameter = symbol(name)?;
                if parameter == regulator
                    || values.contains_key(&parameter)
                    || model_symbols.contains(&parameter)
                {
                    return Err(format!(
                        "runtime kinematic symbol {name} conflicts with a fixed scalar or regulator"
                    )
                    .into());
                }
                if !runtime_parameters.contains(&parameter) {
                    runtime_parameters.push(parameter);
                }
                Atom::var(parameter)
            }
            _ => {
                return Err(
                    "each kinematic product requires exactly one of symbol or value".into(),
                );
            }
        };
        kinematics = kinematics.with_scalar_product(
            &product.left.atom()?,
            &product.right.atom()?,
            product_value,
        )?;
    }
    let powers = card
        .integral
        .powers
        .iter()
        .map(|(edge, power)| edge.parse::<usize>().map(|edge| (EdgeId(edge), *power)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let graph = GraphIntegral::new_with_runtime_scalar_values(
        Arc::new(diagram),
        &kinematics,
        &values,
        &model_symbols,
    )?
    .with_auxiliary_external_momenta(
        &card
            .kinematics
            .auxiliary_momenta
            .iter()
            .map(|name| expression(name))
            .collect::<CliResult<Vec<_>>>()?,
    )?
    .with_powers(&powers)?
    .with_measure_multiplier(expression(&card.integral.measure_multiplier)?);
    let propagators = graph.powers().len();
    let independent_externals = graph
        .family()
        .external_momenta()
        .iter()
        .map(Atom::to_canonical_string)
        .collect();
    let basis = graph.diagram().loop_momentum_basis();
    let dependent_externals = basis
        .external_edges
        .iter()
        .enumerate()
        .filter(|(_, edge)| basis.dependent_externals.contains(edge))
        .map(|(index, _)| {
            feynkit_graph::symbols::external_momentum()
                .call(index)
                .to_canonical_string()
        })
        .collect();
    let parameters = (0..propagators)
        .map(|i| symbol(&format!("fastsecdec::x{i}")))
        .collect::<CliResult<Vec<_>>>()?;
    if runtime_parameters
        .iter()
        .any(|parameter| parameters.contains(parameter))
    {
        return Err("runtime kinematic symbols must differ from integration coordinates".into());
    }
    Ok(GraphAdmission {
        graph,
        parameters,
        runtime_parameters,
        runtime_model,
        values,
        independent_externals,
        dependent_externals,
    })
}

fn read_source(
    path: &Path,
    base: &Path,
    sources: &mut Vec<crate::artifact::SourceFile>,
) -> CliResult<String> {
    let text = fs::read_to_string(path)?;
    sources.push(crate::artifact::SourceFile {
        path: crate::artifact::relative_path(path, base)?
            .to_string_lossy()
            .into_owned(),
        blake3: blake3::hash(text.as_bytes()).to_hex().to_string(),
        fingerprint: crate::artifact::SourceFingerprint::Bytes,
    });
    Ok(text)
}

pub fn load_graph(
    path: &Path,
) -> CliResult<(RunCard, GraphAdmission, Vec<crate::artifact::SourceFile>)> {
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut sources = Vec::new();
    let text = read_source(path, base, &mut sources)?;
    let card: RunCard = toml::from_str(&text)?;
    let values = card
        .parameters
        .iter()
        .map(|(name, value)| Ok((symbol(name)?, value_expression(value)?)))
        .collect::<CliResult<BTreeMap<_, _>>>()?;
    let admission = admit_graph(&card, base, values, &mut sources)?;
    Ok((card, admission, sources))
}

pub enum LoadProgress<'a> {
    Parsed(&'a RunCard),
    Parametrization,
}

pub fn load(path: &Path) -> CliResult<LoadedInput> {
    load_observed(path, |_| Ok(()))
}

pub fn load_observed(
    path: &Path,
    observe: impl FnMut(LoadProgress<'_>) -> CliResult<()>,
) -> CliResult<LoadedInput> {
    load_observed_with_overrides(path, Default::default(), observe)
}

pub(crate) fn load_observed_with_overrides(
    path: &Path,
    overrides: crate::config::GenerationOverrides,
    mut observe: impl FnMut(LoadProgress<'_>) -> CliResult<()>,
) -> CliResult<LoadedInput> {
    let started = Instant::now();
    let mut sources = Vec::new();
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let read = |path: &Path, sources: &mut Vec<crate::artifact::SourceFile>| -> CliResult<String> {
        let text = fs::read_to_string(path)?;
        sources.push(crate::artifact::SourceFile {
            path: crate::artifact::relative_path(path, base)?
                .to_string_lossy()
                .into_owned(),
            blake3: blake3::hash(text.as_bytes()).to_hex().to_string(),
            fingerprint: crate::artifact::SourceFingerprint::Bytes,
        });
        Ok(text)
    };
    let text = read(path, &mut sources)?;
    sources[0].fingerprint = crate::artifact::SourceFingerprint::RunCardScientificInput;
    sources[0].blake3 = sources[0].fingerprint.hash(text.as_bytes())?;
    let mut card: RunCard = toml::from_str(&text)?;
    overrides.apply(&mut card);
    card.generation.resolve_jacobian()?;
    observe(LoadProgress::Parsed(&card))?;
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    let mut values = BTreeMap::new();
    for (name, value) in &card.parameters {
        values.insert(symbol(name)?, value_expression(value)?);
    }
    let regulator = symbol(&card.integral.regulator)?;
    match (&card.input, &card.direct) {
        (Some(_), None) => {
            let GraphAdmission {
                graph,
                parameters,
                mut runtime_parameters,
                mut runtime_model,
                values,
                independent_externals,
                dependent_externals,
            } = admit_graph(&card, base, values, &mut sources)?;
            let loops = Some(graph.diagram().loop_count());
            let propagators = graph.powers().len();
            let input_seconds = started.elapsed().as_secs_f64();
            observe(LoadProgress::Parametrization)?;
            let parametrization_started = Instant::now();
            let policy = card.generation.family_preparation;
            let dimension = bind(&expression(&card.integral.dimension)?, &values);
            let numerator = graph
                .scalar_numerator(&card.generation.contraction_mode.algebra_settings())?
                * graph.measure_multiplier();
            let (integrand, family_preparation) = if policy == FamilyPreparationPolicy::Original {
                (
                    ParametricIntegrand::from_family(
                        graph.family(),
                        graph.powers(),
                        numerator,
                        parameters,
                        regulator,
                        dimension,
                    )?,
                    None,
                )
            } else {
                let (integrand, report) = ParametricIntegrand::from_family_prepared(
                    graph.family(),
                    graph.powers(),
                    numerator,
                    parameters,
                    regulator,
                    dimension,
                    policy,
                )?;
                (integrand, Some(report))
            };
            let (runtime_mass_constraints, model_parameter_defaults) =
                if let Some(runtime) = &mut runtime_model {
                    runtime.retain_used(&integrand);
                    runtime_parameters.extend(runtime.symbols());
                    (runtime.mass_constraints().to_vec(), runtime.defaults())
                } else {
                    (Vec::new(), BTreeMap::new())
                };
            Ok(LoadedInput {
                card,
                integrand,
                runtime_parameters,
                runtime_mass_constraints,
                model_parameter_defaults,
                label: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                loops,
                propagators,
                sources,
                family_preparation,
                independent_externals,
                dependent_externals,
                input_seconds,
                parametrization_seconds: parametrization_started.elapsed().as_secs_f64(),
            })
        }
        (None, Some(direct)) => {
            if card.generation.contraction_mode != fastsecdec::input::NumeratorContraction::Minimal
            {
                return Err(
                    "contraction_mode requires native graph input, not direct parametric data"
                        .into(),
                );
            }
            if card.generation.family_preparation != FamilyPreparationPolicy::Original {
                return Err(
                    "family preparation requires native graph input, not direct parametric data"
                        .into(),
                );
            }
            let values = resolve_scalar_bindings(values)?;
            let domain =
                match direct.domain.as_str() {
                    "unit_cube" => ParametricDomain::UnitCube,
                    "projective_simplex" => ParametricDomain::ProjectiveSimplex,
                    "positive_orthant" => ParametricDomain::PositiveOrthant,
                    _ => return Err(
                        "direct domain must be unit_cube, projective_simplex or positive_orthant"
                            .into(),
                    ),
                };
            let parameters = direct
                .parameters
                .iter()
                .map(|name| symbol(name))
                .collect::<CliResult<Vec<_>>>()?;
            if parameters
                .iter()
                .chain(std::iter::once(&regulator))
                .any(|symbol| values.contains_key(symbol))
            {
                return Err(
                    "[parameters] cannot bind an integration parameter or its regulator".into(),
                );
            }
            let runtime_parameters = direct
                .runtime_parameters
                .iter()
                .map(|name| symbol(name))
                .collect::<CliResult<Vec<_>>>()?;
            let mut seen = std::collections::BTreeSet::new();
            for symbol in &runtime_parameters {
                if !seen.insert(*symbol) {
                    return Err("direct runtime parameters contain duplicate symbol aliases".into());
                }
                if *symbol == regulator || parameters.contains(symbol) {
                    return Err(
                        "direct runtime parameter overlaps an integration parameter or regulator"
                            .into(),
                    );
                }
                if values.contains_key(symbol) {
                    return Err(
                        "[parameters] cannot bind a declared direct runtime parameter".into(),
                    );
                }
            }
            let mut terms = Vec::new();
            for term in &direct.terms {
                let monomial_powers = if term.monomial_powers.is_empty() {
                    vec![Atom::Zero; parameters.len()]
                } else {
                    term.monomial_powers
                        .iter()
                        .map(|text| Ok(bind(&expression(text)?, &values)))
                        .collect::<CliResult<Vec<_>>>()?
                };
                let mut factors = Vec::new();
                for factor in &term.factors {
                    let polynomial = match (&factor.polynomial, &factor.polynomial_file) {
                        (Some(text), None) => expression(text)?,
                        (None, Some(path)) => expression(&read(&base.join(path), &mut sources)?)?,
                        _ => {
                            return Err(
                                "each factor requires exactly one of polynomial or polynomial_file"
                                    .into(),
                            );
                        }
                    };
                    let role = match factor.role.as_str() {
                        "singularity" => FactorRole::Singularity,
                        "polynomial" => FactorRole::Polynomial,
                        _ => return Err("factor role must be singularity or polynomial".into()),
                    };
                    factors.push(
                        PolynomialFactor::new(
                            bind(&polynomial, &values),
                            bind(&expression(&factor.exponent)?, &values),
                            role,
                        )
                        .with_semantics(factor.semantics),
                    );
                }
                terms.push(ParametricTerm::new(
                    bind(
                        &(expression(&term.prefactor)?
                            * expression(&card.integral.measure_multiplier)?),
                        &values,
                    ),
                    monomial_powers,
                    factors,
                ));
            }
            let propagators = parameters.len();
            let input_seconds = started.elapsed().as_secs_f64();
            observe(LoadProgress::Parametrization)?;
            let parametrization_started = Instant::now();
            let integrand = ParametricIntegrand::new(parameters, regulator, domain, terms)?;
            Ok(LoadedInput {
                card,
                integrand,
                label: path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                runtime_parameters,
                runtime_mass_constraints: Vec::new(),
                model_parameter_defaults: BTreeMap::new(),
                loops: None,
                propagators,
                sources,
                family_preparation: None,
                independent_externals: Vec::new(),
                dependent_externals: Vec::new(),
                input_seconds,
                parametrization_seconds: parametrization_started.elapsed().as_secs_f64(),
            })
        }
        _ => Err(
            "run card requires exactly one of [input] graph data or [direct] parametric data"
                .into(),
        ),
    }
}

pub fn artifact_path(card: &Path) -> PathBuf {
    PathBuf::from("output").join(format!(
        "{}.fsd",
        card.file_stem().unwrap_or_default().to_string_lossy()
    ))
}

#[cfg(test)]
mod on_shell_tests;

#[cfg(test)]
mod runtime_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_example_points_preserve_frozen_reference_densities() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixtures = repository.join("crates/fastsecdec-cli/tests/fixtures/historical-run-cards");
        let directory = tempfile::tempdir().unwrap();
        // Preserve each card's ordinary relative paths without writing absolute
        // paths into either historical evidence or temporary cards.
        for name in ["graphs", "models", "parametric", "runs"] {
            fs::create_dir(directory.path().join(name)).unwrap();
            if name != "runs" {
                for entry in fs::read_dir(repository.join("examples").join(name)).unwrap() {
                    let entry = entry.unwrap();
                    if entry.file_type().unwrap().is_file() {
                        fs::copy(
                            entry.path(),
                            directory.path().join(name).join(entry.file_name()),
                        )
                        .unwrap();
                    }
                }
            }
        }
        let mut entries = fs::read_dir(fixtures)
            .unwrap()
            .map(Result::unwrap)
            .collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if entry
                .path()
                .extension()
                .is_none_or(|extension| extension != "toml")
            {
                continue;
            }
            let name = entry.file_name();
            let mut historical: toml::Value =
                toml::from_str(&fs::read_to_string(entry.path()).unwrap()).unwrap();
            if let Some(input) = historical.get_mut("input") {
                input
                    .as_table_mut()
                    .unwrap()
                    .insert("model_parameters".into(), "fixed".into());
            }
            let old_path = directory.path().join("runs").join(&name);
            fs::write(&old_path, toml::to_string(&historical).unwrap()).unwrap();
            let old = load(&old_path).unwrap();
            assert!(old.runtime_parameters.is_empty());
            let current = load(&repository.join("examples/runs").join(&name)).unwrap();
            let point = current
                .card
                .integration
                .parameters
                .iter()
                .map(|(name, value)| {
                    (
                        symbol(name).unwrap(),
                        Atom::num(Rational::try_from(*value).unwrap()),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            assert!(
                current
                    .runtime_parameters
                    .iter()
                    .all(|symbol| point.contains_key(symbol)),
                "missing point for {name:?}"
            );
            assert_eq!(
                current.integrand.parameters(),
                old.integrand.parameters(),
                "coordinate ordering for {name:?}"
            );
            assert_eq!(
                current.integrand.domain(),
                old.integrand.domain(),
                "domain for {name:?}"
            );
            assert_eq!(
                current.integrand.regulator(),
                old.integrand.regulator(),
                "regulator for {name:?}"
            );
            assert_eq!(
                bind(&current.integrand.density(), &point).expand(),
                old.integrand.density().expand(),
                "bound native density for {name:?}"
            );
        }
    }

    #[test]
    fn caller_parameter_names_are_literal_not_pattern_wildcards() {
        let values = BTreeMap::from([(symbol("user_value_").unwrap(), expression("2").unwrap())]);
        assert_eq!(
            bind(&expression("user_value_+untouched").unwrap(), &values),
            expression("2+untouched").unwrap()
        );
    }

    #[test]
    fn numeric_model_parameters_keep_native_imaginary_coefficients() {
        let mut json: serde_json::Value =
            serde_json::from_str(include_str!("../../../examples/models/scalar.json")).unwrap();
        json["parameters"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name":"complex_external", "nature":"external", "parameter_type":"complex",
            "value":[2.5,3.25], "expression":null,
            }));
        let model = Model::from_json(&serde_json::to_string(&json).unwrap()).unwrap();
        let values = model.scalar_bindings(None, &BTreeMap::new()).unwrap();
        assert_eq!(
            values[&symbol("UFO::complex_external").unwrap()],
            Atom::num((5, 2)) + Atom::num((13, 4)) * Atom::i(),
        );
        assert_eq!(
            value_expression(&toml::Value::Float(172.5)).unwrap(),
            Atom::num((345, 2)),
        );
    }

    #[test]
    fn inline_zero_width_is_admitted_before_the_default_model_point() {
        let directory = tempfile::tempdir().unwrap();
        let mut model: serde_json::Value =
            serde_json::from_str(include_str!("../../../examples/models/massless_phi3.json"))
                .unwrap();
        model["parameters"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name":"width", "nature":"external", "parameter_type":"real",
                "value":[0.1,0.0], "expression":null,
            }));
        model["particles"][0]["width"] = "width".into();
        fs::write(directory.path().join("model.json"), model.to_string()).unwrap();
        fs::write(
            directory.path().join("bubble.dot"),
            include_str!("../../../examples/graphs/bubble.dot"),
        )
        .unwrap();
        let card = r#"
[input]
model = 'model.json'
graph = 'bubble.dot'
[kinematics]
products = [{left=1,right=1,value='-1'}]
[parameters]
'UFO::width' = '0'
"#;
        let path = directory.path().join("input.toml");
        fs::write(&path, card).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.propagators, 2);
        assert!(!loaded.integrand.terms().is_empty());
        for card in [
            card.replace("'UFO::width' = '0'", ""),
            card.replace("'UFO::width' = '0'", "'UFO::width' = '1'"),
        ] {
            fs::write(&path, card).unwrap();
            match load(&path) {
                Err(error) => assert!(error.to_string().contains("width")),
                Ok(_) => panic!("unsupported width was silently omitted"),
            }
        }
    }

    #[test]
    fn numerator_vectors_use_native_expression_labels_and_scalar_bindings() {
        let directory = tempfile::tempdir().unwrap();
        fs::write(
            directory.path().join("model.json"),
            include_str!("../../../examples/models/massless_phi3.json"),
        )
        .unwrap();
        let model = Arc::new(
            Model::from_json(include_str!("../../../examples/models/massless_phi3.json")).unwrap(),
        );
        let kin = Kinematics::in_dimension(&expression("D").unwrap()).unwrap();
        let k = feynkit_graph::symbols::loop_momentum().call(0);
        let e = Atom::var(spenso::vector_symbol!("helicity::e"));
        let numerator = kin.scalar_product(&k, &e).unwrap().pow(2);
        let diagram = feynkit_graph::FeynmanDiagram::from_dot(
            model,
            include_str!("../../../examples/graphs/bubble.dot"),
        )
        .unwrap()
        .with_numerator(numerator)
        .unwrap();
        fs::write(
            directory.path().join("graph.dot"),
            diagram.to_dot().unwrap(),
        )
        .unwrap();
        let card = r#"
[input]
graph = "graph.dot"
model = "model.json"
[kinematics]
auxiliary_momenta = ["helicity::e"]
products = [
  {left=1, right=1, value="-1"},
  {left=1, right="helicity::e", value="0"},
  {left="helicity::e", right="helicity::e", value="norm"},
]
[parameters]
norm = "-1"
"#;
        // Native canonical spelling retains the tensor/rank-one metadata in
        // a fresh process, before any graph has registered the vector head.
        let encoded = serde_json::to_string(&e.to_canonical_string()).unwrap();
        let card = card.replace("\"helicity::e\"", &encoded);
        let path = directory.path().join("input.toml");
        fs::write(&path, &card).unwrap();
        let loaded = load(&path).unwrap();
        assert_eq!(loaded.loops, Some(1));
        assert_eq!(loaded.propagators, 2);
        assert!(
            loaded
                .independent_externals
                .contains(&e.to_canonical_string())
        );
        let density = loaded.integrand.density();
        assert!(!density.is_zero());
        assert!(!density.contains(expression("norm").unwrap().as_view()));
        assert!(
            !density
                .get_all_symbols(true)
                .contains(&feynkit_graph::symbols::loop_momentum())
        );

        // The numerator must not silently lose an undeclared vector.
        fs::write(
            &path,
            card.replace(&format!("auxiliary_momenta = [{encoded}]"), ""),
        )
        .unwrap();
        assert!(load(&path).is_err());
    }

    #[test]
    fn native_model_cached_dependents_follow_exact_overrides_and_restrictions() {
        let directory = tempfile::tempdir().unwrap();
        let mut model: serde_json::Value =
            serde_json::from_str(include_str!("../../../examples/models/scalar.json")).unwrap();
        model["parameters"].as_array_mut().unwrap().extend([
            serde_json::json!({"name":"a","nature":"external","parameter_type":"real","value":[2.0,0.0],"expression":null}),
            serde_json::json!({"name":"b","nature":"internal","parameter_type":"real","value":[4.0,0.0],"expression":"2*UFO::a"}),
            serde_json::json!({"name":"fixed","nature":"internal","parameter_type":"real","value":[11.0,0.0],"expression":null}),
        ]);
        model["couplings"] = serde_json::json!([
            {"name":"GC_check","expression":"UFO::b","orders":[],"value":[4.0,0.0]}
        ]);
        fs::write(
            directory.path().join("model.json"),
            serde_json::to_vec(&model).unwrap(),
        )
        .unwrap();
        fs::write(
            directory.path().join("bubble.dot"),
            include_str!("../../../examples/graphs/bubble.dot"),
        )
        .unwrap();
        for (inline, restriction, symbolic, scale, fixed_expected) in [
            (
                "'UFO::a' = '3'",
                None,
                "12+model::fixed",
                "23",
                "23*gamma(eps)",
            ),
            (
                "'UFO::a' = 'symbolica::pi'",
                None,
                "model::fixed+4*symbolica::pi",
                "11+4*symbolica::pi",
                "(11+4*symbolica::pi)*gamma(eps)",
            ),
            (
                "'UFO::a' = '3'",
                Some(r#"{"b":[5,0]}"#),
                "12+model::fixed",
                "23",
                "21*gamma(eps)",
            ),
            (
                "'UFO::a' = '3'\n'UFO::b' = '7'",
                Some(r#"{"b":[5,0]}"#),
                "14+model::fixed",
                "25",
                "25*gamma(eps)",
            ),
        ] {
            let restriction_field = if let Some(restriction) = restriction {
                fs::write(directory.path().join("restriction.json"), restriction).unwrap();
                "parameter_card = 'restriction.json'"
            } else {
                ""
            };
            let card = format!(
                r#"
[input]
graph = 'bubble.dot'
model = 'model.json'
{restriction_field}
[kinematics]
products = [{{left=1,right=1,value='-1'}}]
[parameters]
'UFO::mt' = '0'
{inline}
[integral]
measure_multiplier = 'UFO::b+UFO::GC_check+UFO::fixed'
"#
            );
            let path = directory.path().join("input.toml");
            fs::write(&path, &card).unwrap();
            let loaded = load(&path).unwrap();
            let fixed = symbol("model::fixed").unwrap();
            assert_eq!(loaded.runtime_parameters, [fixed]);
            assert_eq!(loaded.model_parameter_defaults["model::fixed"], 11.0);
            assert_eq!(
                loaded.integrand.terms()[0].prefactor(),
                &expression(&format!("({symbolic})*gamma(eps)")).unwrap(),
                "inline {inline}, restriction {restriction:?}"
            );
            // Cached internal/card values are metadata in runtime mode. The
            // native dependent definition follows explicit scalar overrides;
            // the independent expressionless leaf is supplied to the evaluator.
            let generated =
                fastsecdec::generation::generate(&loaded.integrand, &Default::default(), |_| {
                    std::ops::ControlFlow::Continue(())
                })
                .unwrap();
            let mut kernels = generated
                .compile_with_parameters_and_progress(&loaded.runtime_parameters, |_| {
                    std::ops::ControlFlow::Continue(())
                })
                .unwrap();
            let template = kernels.to_bytes().unwrap();
            assert_eq!(kernels.sectors().len(), 1);
            assert_eq!(kernels.sectors()[0].dimension(), 1);
            let scale = expression(scale)
                .unwrap()
                .evaluate(&std::collections::HashMap::<Atom, f64>::new())
                .unwrap();
            let mut prior = Vec::<f64>::new();
            for (value, expected_scale) in [(11.0, scale), (13.0, scale + 2.0)] {
                kernels
                    .bind_parameters(&BTreeMap::from([(fixed, value)]))
                    .unwrap();
                let mut total = kernels.exact_coefficients().to_vec();
                for sector in kernels.sectors_mut() {
                    let mut output = vec![0.0; total.len()];
                    sector
                        .evaluate(&vec![0.37; sector.dimension()], &mut output)
                        .unwrap();
                    for (sum, value) in total.iter_mut().zip(output) {
                        *sum += value;
                    }
                }
                // The two projective bubble charts combine to the pole
                // density 2/(1+t)^2, whose integral is one. This is a
                // pointwise evaluator check, not an integrated pole value.
                let expected_pole_density = expected_scale * 2.0 / (1.0_f64 + 0.37).powi(2);
                assert!(
                    (total[0] - expected_pole_density).abs() < 1e-12,
                    "pole density {} != {expected_pole_density}",
                    total[0]
                );
                if !prior.is_empty() {
                    for (old, new) in prior.iter().zip(&total) {
                        assert!((new - old * expected_scale / scale).abs() < 1e-11);
                    }
                }
                prior = total;
                assert_eq!(kernels.to_bytes().unwrap(), template);
            }
            // Explicit fixed mode retains the old scalar-card restriction
            // behavior; this is distinct from runtime defaults above.
            fs::write(
                &path,
                card.replace("[input]", "[input]\nmodel_parameters='fixed'"),
            )
            .unwrap();
            assert_eq!(
                load(&path).unwrap().integrand.terms()[0].prefactor(),
                &expression(fixed_expected).unwrap()
            );
        }
    }
}
