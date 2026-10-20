//! Exact, pre-sector interchange; native graph admission remains in input.rs.

use crate::{CliResult, input};
use fastsecdec::{Atom, AtomCore, Symbol, input::GraphSymanzik};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeMap, fs, io::Write, path::Path};
use symbolica::{
    atom::AtomView,
    coefficient::Coefficient,
    domains::{atom::AtomField, rational::Rational},
    id::{Pattern, Replacement},
};

const MAX_SOURCE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_TERMS: usize = 100_000;
const MAX_SYMBOLS: usize = 64;
const MAX_OUTPUT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, Serialize)]
pub struct ExportSymbol {
    pub name: String,
    pub native_symbol: String,
    pub role: &'static str,
}
#[derive(Clone, Debug, Serialize)]
pub struct SparseTerm {
    pub coefficient: String,
    pub exponents: Vec<u32>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SparsePolynomial {
    pub variables: Vec<String>,
    pub terms: Vec<SparseTerm>,
    pub expression: String,
}
#[derive(Deserialize, Default)]
#[serde(deny_unknown_fields)]
struct Point {
    #[serde(default)]
    parameters: BTreeMap<String, toml::Value>,
}

fn rational(atom: &Atom) -> Option<Rational> {
    if atom.is_zero() {
        return Some(Rational::from(0));
    }
    let AtomView::Num(n) = atom.as_view() else {
        return None;
    };
    match n.get_coeff_view().to_owned() {
        Coefficient::Complex(n) if n.im.is_zero() => Some(n.re),
        _ => None,
    }
}

/// Parse only signed decimal integers or a quotient of two such integers.
/// No floating conversion and no eager parsing of arbitrary powers/functions.
fn exact_value(value: &toml::Value) -> CliResult<Atom> {
    let text =
        match value {
            toml::Value::Integer(n) => n.to_string(),
            toml::Value::String(s) => s.trim().to_owned(),
            _ => return Err(
                "exact U/F point values must be rational strings or integers; floats are rejected"
                    .into(),
            ),
        };
    fn integer(s: &str) -> bool {
        let s = s
            .strip_prefix('-')
            .or_else(|| s.strip_prefix('+'))
            .unwrap_or(s);
        !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit())
    }
    let parts: Vec<_> = text.split('/').collect();
    if text.len() > 4096 || !(1..=2).contains(&parts.len()) || !parts.iter().all(|s| integer(s)) {
        return Err("exact value requires a bounded integer or integer/integer literal".into());
    }
    if parts.len() == 2
        && parts[1]
            .trim_start_matches(['+', '-'])
            .bytes()
            .all(|c| c == b'0')
    {
        return Err("zero rational denominator".into());
    }
    let atom = input::expression(&text)?;
    if rational(&atom).is_none() {
        return Err("value is not an exact real rational".into());
    }
    Ok(atom)
}

fn bounded_read(path: &Path) -> CliResult<String> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > MAX_SOURCE_BYTES {
        return Err(format!(
            "input must be a regular file of at most {MAX_SOURCE_BYTES} bytes: {}",
            path.display()
        )
        .into());
    }
    use std::io::Read;
    let mut text = String::new();
    fs::File::open(path)?
        .take(MAX_SOURCE_BYTES + 1)
        .read_to_string(&mut text)?;
    if text.len() as u64 > MAX_SOURCE_BYTES {
        return Err("input grew beyond byte cap".into());
    }
    Ok(text)
}

fn sparse(
    expression: &Atom,
    symbols: &[Symbol],
    aliases: &[ExportSymbol],
) -> CliResult<SparsePolynomial> {
    let variables = symbols.iter().map(|s| Atom::var(*s)).collect::<Vec<_>>();
    let poly = expression.to_polynomial_in_vars_with_field::<u32>(
        &variables,
        &AtomField {
            statistical_zero_test: false,
            ..AtomField::new()
        },
    );
    if poly.nterms() > MAX_TERMS {
        return Err("sparse U/F term cap exceeded".into());
    }
    let mut terms = Vec::with_capacity(poly.nterms());
    let mut reconstructed = Atom::Zero;
    for term in &poly {
        let coefficient = rational(term.coefficient).ok_or("U/F coefficients must be rational; unresolved functions or nonpolynomial dependence are unsupported")?;
        if coefficient.is_zero() {
            return Err("unexpected zero sparse coefficient".into());
        }
        let coefficient_text = coefficient.to_string();
        if coefficient_text.len() > 16384 || term.exponents.iter().any(|e| *e > 65535) {
            return Err("sparse U/F coefficient or exponent cap exceeded".into());
        }
        let mut monomial = Atom::num(coefficient);
        for (variable, exponent) in variables.iter().zip(term.exponents) {
            if *exponent != 0 {
                monomial *= variable.pow(*exponent as i64);
            }
        }
        reconstructed += monomial;
        terms.push(SparseTerm {
            coefficient: coefficient_text,
            exponents: term.exponents.to_vec(),
        });
    }
    if !(reconstructed - expression).expand().is_zero() {
        return Err("sparse U/F exact reconstruction failed".into());
    }
    terms.sort_by(|a, b| a.exponents.cmp(&b.exponents));
    let expression = if terms.is_empty() {
        "0".to_owned()
    } else {
        terms
            .iter()
            .map(|term| {
                let mut s = format!("({})", term.coefficient);
                for (alias, exponent) in aliases.iter().zip(&term.exponents) {
                    if *exponent != 0 {
                        s.push_str(&format!("*{}^{exponent}", alias.name));
                    }
                }
                s
            })
            .collect::<Vec<_>>()
            .join("+")
    };
    if expression.len() > MAX_OUTPUT_BYTES {
        return Err("sparse expression byte cap exceeded".into());
    }
    Ok(SparsePolynomial {
        variables: aliases.iter().map(|s| s.name.clone()).collect(),
        terms,
        expression,
    })
}

fn homogeneous(poly: &SparsePolynomial, start: usize, degree: usize) -> bool {
    !poly.terms.is_empty()
        && poly.terms.iter().all(|t| {
            t.exponents[start..]
                .iter()
                .map(|e| u64::from(*e))
                .sum::<u64>()
                == degree as u64
        })
}

fn binding_index(name: &str, symbols: &[Symbol], aliases: &[ExportSymbol]) -> CliResult<usize> {
    // A point key is a name, never an expression to normalize eagerly.
    let plain = name.len() <= 512
        && name.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':');
    let parsed = if plain {
        input::symbol(name).ok()
    } else {
        None
    };
    let matches = aliases
        .iter()
        .zip(symbols)
        .enumerate()
        .filter(|(_, (alias, symbol))| {
            alias.name == name || alias.native_symbol == name || parsed == Some(**symbol)
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [index] => Ok(*index),
        [] => Err(format!("point symbol {name:?} is not present in U/F").into()),
        _ => Err(format!("point symbol {name:?} is ambiguous between a native name and an alias; use its fully qualified native name").into()),
    }
}

/// Bounded polynomial grammar, parsed by Symbolica before normalizing atoms.
/// This maps native identifiers and aliases without interpreting functions or
/// permitting numeric exponent towers to allocate unbounded coefficients.
fn constraint_side(
    text: &str,
    symbols: &[Symbol],
    aliases: &[ExportSymbol],
    kinematic: usize,
) -> CliResult<Atom> {
    use symbolica::parser::{Operator, ParseSettings, Token};
    if text.len() > 4096 {
        return Err("kinematic constraint byte cap exceeded".into());
    }
    let mut depth = 0usize;
    for c in text.chars() {
        if c == '(' {
            depth += 1;
            if depth > 32 {
                return Err("constraint nesting cap".into());
            }
        }
        if c == ')' {
            depth = depth.checked_sub(1).ok_or("unbalanced constraint")?;
        }
    }
    if depth != 0 {
        return Err("unbalanced constraint".into());
    }
    let token = Token::parse(text, ParseSettings::symbolica().convert_mul_to_atom(false))
        .map_err(|e| format!("invalid constraint: {e}"))?;
    fn mapped(
        t: &Token,
        symbols: &[Symbol],
        aliases: &[ExportSymbol],
        kinematic: usize,
        visits: &mut usize,
    ) -> CliResult<(String, usize, u32, usize)> {
        *visits += 1;
        if *visits > 512 {
            return Err("constraint token cap".into());
        }
        let value = match t {
            Token::Number(n, false) => {
                let value = toml::Value::String(n.to_string());
                if n.len() > 64 || n.contains('/') {
                    return Err("constraint integer literal cap".into());
                }
                let atom = exact_value(&value)?;
                (atom.to_canonical_string(), 1, 0, n.len() * 4)
            }
            Token::ID(name) => {
                let i = binding_index(name, symbols, aliases)?;
                if i >= kinematic {
                    return Err("kinematic constraint cannot restrict Feynman coordinates".into());
                }
                (aliases[i].native_symbol.clone(), 1, 1, 1)
            }
            Token::Op(_, _, op, args) => match op {
                Operator::Add | Operator::Mul => {
                    let mut pieces = Vec::new();
                    let mut terms = if *op == Operator::Add { 0usize } else { 1 };
                    let mut degree = 0u32;
                    let mut height = args.len();
                    for arg in args {
                        let (s, n, d, h) = mapped(arg, symbols, aliases, kinematic, visits)?;
                        height = height.saturating_add(h);
                        pieces.push(format!("({s})"));
                        if *op == Operator::Add {
                            terms = terms.saturating_add(n);
                            degree = degree.max(d);
                        } else {
                            terms = terms.saturating_mul(n);
                            degree = degree.saturating_add(d);
                        }
                    }
                    (
                        pieces.join(if *op == Operator::Add { "+" } else { "*" }),
                        terms,
                        degree,
                        height,
                    )
                }
                Operator::Neg if args.len() == 1 => {
                    let (s, n, d, h) = mapped(&args[0], symbols, aliases, kinematic, visits)?;
                    (format!("-({s})"), n, d, h)
                }
                Operator::Inv if args.len() == 1 => {
                    let (s, n, d, h) = mapped(&args[0], symbols, aliases, kinematic, visits)?;
                    if n != 1 || d != 0 || !matches!(&args[0], Token::Number(_, false)) {
                        return Err("constraint denominator must be an integer literal".into());
                    }
                    if rational(&input::expression(&s)?).is_none_or(|q| q.is_zero()) {
                        return Err("zero constraint denominator".into());
                    }
                    (format!("1/({s})"), 1, 0, h)
                }
                Operator::Pow if args.len() == 2 => {
                    let Token::Number(n, false) = &args[1] else {
                        return Err("constraint exponent must be a nonnegative literal".into());
                    };
                    let e = n.parse::<u32>().map_err(|_| "constraint exponent cap")?;
                    if e > 32 {
                        return Err("constraint exponent cap".into());
                    }
                    let (s, n, d, h) = mapped(&args[0], symbols, aliases, kinematic, visits)?;
                    (
                        format!("({s})^{e}"),
                        n.saturating_pow(e),
                        d.saturating_mul(e),
                        h.saturating_mul(e as usize),
                    )
                }
                _ => return Err("unsupported polynomial constraint syntax".into()),
            },
            _ => {
                return Err(
                    "constraint functions and nonrational coefficients are unsupported".into(),
                );
            }
        };
        if value.1 > 4096 || value.2 > 32 || value.3 > 65536 {
            return Err("constraint expansion cap".into());
        }
        Ok(value)
    }
    let (mapped, _, _, _) = mapped(&token, symbols, aliases, kinematic, &mut 0)?;
    input::expression(&mapped)
}

fn constraint(
    text: &str,
    symbols: &[Symbol],
    aliases: &[ExportSymbol],
    kinematic: usize,
) -> CliResult<String> {
    if text.contains('=') || text.matches('<').count() + text.matches('>').count() != 1 {
        return Err("kinematic constraints require exactly one strict < or > comparison".into());
    }
    let greater = text.contains('>');
    let (left, right) = text.split_once(if greater { '>' } else { '<' }).unwrap();
    let left = constraint_side(left, symbols, aliases, kinematic)?;
    let right = constraint_side(right, symbols, aliases, kinematic)?;
    let difference = if greater { left - right } else { right - left };
    let polynomial = sparse(&difference.expand(), symbols, aliases)?;
    Ok(format!("({}) > 0", polynomial.expression))
}

pub fn export(
    path: &Path,
    output: &Path,
    point_path: Option<&Path>,
    constraints: &[String],
) -> CliResult<serde_json::Value> {
    let started = std::time::Instant::now();
    if output.exists() {
        return Err("export output directory already exists; use a fresh path".into());
    }
    let card_text = bounded_read(path)?;
    let card: crate::config::RunCard = toml::from_str(&card_text)?;
    let source = card
        .input
        .as_ref()
        .ok_or("exact U/F export requires graph input")?;
    if card.direct.is_some() {
        return Err("exact U/F export does not accept direct input".into());
    }
    for value in card.parameters.values() {
        let _ = exact_value(value)?;
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
    for file in [&source.model, &source.graph] {
        bounded_read(&base.join(file))?;
    }
    if let Some(file) = &source.parameter_card {
        bounded_read(&base.join(file))?;
    }
    let (_, admission, sources) = input::load_graph(path)?;
    if sources
        .first()
        .is_none_or(|s| s.blake3 != blake3::hash(card_text.as_bytes()).to_hex().as_str())
    {
        return Err("run card changed during exact export admission".into());
    }
    let graph = &admission.graph;
    if graph.propagator_edges().len() > 32 || graph.diagram().loop_count() > 8 {
        return Err("U/F export v1 is bounded to 32 propagators and 8 loops".into());
    }
    let native = GraphSymanzik::from_graph(
        graph,
        admission.parameters.iter().map(|s| Atom::var(*s)).collect(),
    )?;
    if native.f().is_zero() {
        return Err(
            "degenerate zero F is unsupported by the U/F split export; no split was emitted".into(),
        );
    }
    let mut kinematic = native.u().get_all_symbols(false);
    kinematic.extend(native.f().get_all_symbols(false));
    for parameter in &admission.parameters {
        kinematic.remove(parameter);
    }
    let mut kinematic = kinematic.into_iter().collect::<Vec<_>>();
    kinematic.sort_by_key(|s| Atom::var(*s).to_canonical_string());
    if kinematic.len() + admission.parameters.len() > MAX_SYMBOLS {
        return Err("U/F variable-map cap exceeded".into());
    }
    let mut aliases = kinematic
        .iter()
        .enumerate()
        .map(|(i, s)| ExportSymbol {
            name: format!("k{i}"),
            native_symbol: Atom::var(*s).to_canonical_string(),
            role: "kinematic",
        })
        .collect::<Vec<_>>();
    aliases.extend(
        admission
            .parameters
            .iter()
            .enumerate()
            .map(|(i, s)| ExportSymbol {
                name: format!("x{i}"),
                native_symbol: Atom::var(*s).to_canonical_string(),
                role: "feynman",
            }),
    );
    let symbols = kinematic
        .iter()
        .chain(&admission.parameters)
        .copied()
        .collect::<Vec<_>>();
    let u = sparse(native.u(), &symbols, &aliases)?;
    let f = sparse(native.f(), &symbols, &aliases)?;
    if !homogeneous(&u, kinematic.len(), native.loop_count())
        || !homogeneous(&f, kinematic.len(), native.loop_count() + 1)
    {
        return Err("native U/F failed expected loop-degree homogeneity".into());
    }
    if u.terms
        .iter()
        .any(|t| t.exponents[..kinematic.len()].iter().any(|e| *e != 0))
    {
        return Err("native U unexpectedly depends on a kinematic symbol".into());
    }
    let point_text = point_path.map(bounded_read).transpose()?;
    let point: Point = point_text
        .as_deref()
        .map(toml::from_str)
        .transpose()?
        .unwrap_or_default();
    let mut values = BTreeMap::new();
    let mut substitutions = Vec::new();
    let mut problem_parameters = BTreeMap::new();
    for (name, value) in &point.parameters {
        let index = binding_index(name, &symbols, &aliases)?;
        if index >= kinematic.len() {
            return Err("point cannot bind a Feynman coordinate".into());
        }
        let value = exact_value(value)?;
        if values.insert(symbols[index], value.clone()).is_some() {
            return Err("duplicate point binding through aliases".into());
        }
        let value_text = rational(&value).unwrap().to_string();
        problem_parameters.insert(aliases[index].name.clone(), value_text.clone());
        substitutions.push(json!({"name":aliases[index].name,"native_symbol":aliases[index].native_symbol,"value":value_text}));
    }
    substitutions.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let bind = |atom: &Atom| {
        atom.replace_multiple(values.iter().map(|(s, v)| {
            Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(v.clone()))
        }))
        .expand()
    };
    let specialized_u = sparse(&bind(native.u()), &symbols, &aliases)?;
    let specialized_f = sparse(&bind(native.f()), &symbols, &aliases)?;
    if specialized_f.terms.is_empty() {
        return Err(
            "point gives identically zero F; degenerate split export is unsupported".into(),
        );
    }
    if !homogeneous(&specialized_u, kinematic.len(), native.loop_count())
        || !homogeneous(&specialized_f, kinematic.len(), native.loop_count() + 1)
    {
        return Err("specialized U/F failed expected loop-degree homogeneity".into());
    }
    let feynman = aliases[kinematic.len()..]
        .iter()
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();
    let free_kinematic = aliases[..kinematic.len()]
        .iter()
        .filter(|s| !problem_parameters.contains_key(&s.name))
        .map(|s| s.name.clone())
        .collect::<Vec<_>>();
    if constraints.len() > 64 {
        return Err("too many kinematic constraints".into());
    }
    let constraints = constraints
        .iter()
        .map(|s| constraint(s, &symbols, &aliases, kinematic.len()))
        .collect::<CliResult<Vec<_>>>()?;
    let mut domain_constraints = feynman
        .iter()
        .map(|s| format!("{s} > 0"))
        .collect::<Vec<_>>();
    domain_constraints.extend_from_slice(&constraints);
    let mut groups = Vec::new();
    if !free_kinematic.is_empty() {
        groups.push(free_kinematic.clone());
    }
    groups.push(feynman.clone());
    let variables = free_kinematic
        .iter()
        .chain(&feynman)
        .cloned()
        .collect::<Vec<_>>();
    let problem = json!({"name":"fastSecDec pre-sector U/F", "variables":variables,
        "variable_groups":groups,"constraints":domain_constraints,"split":[f.expression],"parameters":problem_parameters});
    let problem_text = toml::to_string_pretty(&problem)?;
    let mut bindings = Vec::new();
    for (i, edge_id) in graph.propagator_edges().iter().enumerate() {
        let (_, endpoints, edge) = graph
            .diagram()
            .edges()
            .find(|(id, _, _)| id == edge_id)
            .ok_or("native edge binding missing")?;
        bindings.push(
            json!({"edge_id":edge_id.0,"denominator_index":i,"parameter":feynman[i],
            "source":endpoints.source.map(|v|v.0),"target":endpoints.target.map(|v|v.0),
            "particle":format!("{:?}",edge.particle),"power":graph.powers()[i],
            "denominator":graph.family().denominators()[i].to_canonical_string()}),
        );
    }
    let mut gram = Vec::new();
    for (i, p) in graph.family().external_momenta().iter().enumerate() {
        for q in &graph.family().external_momenta()[i..] {
            gram.push(
                json!({"left":p.to_canonical_string(),"right":q.to_canonical_string(),
                "value":graph.family().kinematics().scalar_product(p,q)?.to_canonical_string()}),
            );
        }
    }
    let basis = graph.diagram().loop_momentum_basis();
    let bundle = json!({
        "schema":"fastsecdec.symanzik","version":1,"stage":"pre_sector_projective",
        "symbols":aliases,"symbolic":{"u":u,"f":f},"specialized":{"u":specialized_u,"f":specialized_f},
        "native_symbolic":{"u":native.u().to_canonical_string(),"f":native.f().to_canonical_string()},
        "substitutions":substitutions,
        "domain":{"source":"projective_simplex","analysis":"positive_orthant",
            "feynman_positive":feynman,"kinematic_constraints":constraints,"u_degree":native.loop_count(),"f_degree":native.loop_count()+1},
        "graph":{"stable_dot":graph.diagram().to_dot()?,"loop_count":native.loop_count(),
            "routing":basis,
            "vertices":graph.diagram().vertices().map(|(v,_)|v.0).collect::<Vec<_>>(),"propagators":bindings,
            "loop_edges":basis.loop_edges.iter().map(|e|e.0).collect::<Vec<_>>(),
            "external_edges":basis.external_edges.iter().map(|e|e.0).collect::<Vec<_>>(),
            "dependent_external_edges":basis.dependent_externals.iter().map(|e|e.0).collect::<Vec<_>>(),
            "independent_externals":admission.independent_externals,"dependent_externals":admission.dependent_externals,
            "gram":gram,"scalar_bindings":admission.values.iter().map(|(s,v)|json!({"native_symbol":Atom::var(*s).to_canonical_string(),"value":v.to_canonical_string()})).collect::<Vec<_>>(),
            "numerator_contracted":false,"native_numerator":graph.diagram().numerator().expression().to_canonical_string(),
            "numerator_prefactor":graph.diagram().numerator_prefactor().expression().to_canonical_string(),
            "projector":graph.diagram().projector().expression().to_canonical_string(),"overall_factor":graph.diagram().overall_factor().to_canonical_string(),
            "measure_multiplier":graph.measure_multiplier().to_canonical_string()},
        "problem":{"file":"problem.toml","blake3":blake3::hash(problem_text.as_bytes()).to_hex().to_string()},
        "provenance":{"producer":"fastsecdec","producer_version":env!("CARGO_PKG_VERSION"),
            "source_identity":env!("FASTSECDEC_SOURCE_IDENTITY"),"feynkit_revision":env!("FASTSECDEC_FEYNKIT_REVISION"),
            "symbolica_revision":env!("FASTSECDEC_SYMBOLICA_REVISION"),"numerica_revision":env!("FASTSECDEC_NUMERICA_REVISION"),
            "sources":sources,"exact_point_blake3":point_text.as_ref().map(|s|blake3::hash(s.as_bytes()).to_hex().to_string())},
        "preprocessing_seconds":started.elapsed().as_secs_f64(),
        "limitations":["No sector decomposition or integration", "Graph metadata is provenance, not a sign or CAD certificate", "Positive orthant describes projective rays; simplex normalization is not implemented here"]
    });
    let bytes = serde_json::to_vec_pretty(&bundle)?;
    if bytes.len() > MAX_OUTPUT_BYTES || problem_text.len() > MAX_OUTPUT_BYTES {
        return Err("export byte cap exceeded".into());
    }
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::create_dir(output)?;
    for (name, bytes) in [
        ("problem.toml", problem_text.as_bytes()),
        ("symanzik.json", bytes.as_slice()),
    ] {
        let mut temporary = tempfile::NamedTempFile::new_in(output)?;
        temporary.write_all(bytes)?;
        temporary.as_file().sync_all()?;
        temporary.persist_noclobber(output.join(name))?;
    }
    Ok(
        json!({"bundle":output.join("symanzik.json"),"problem":output.join("problem.toml"),"stage":"pre_sector_projective"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_points_reject_floats_powers_and_zero_denominators() {
        assert_eq!(
            rational(&exact_value(&toml::Value::String("-3/2".into())).unwrap()).unwrap(),
            Rational::from((-3, 2))
        );
        for value in [
            toml::Value::Float(0.5),
            toml::Value::String("2^1000000000".into()),
            toml::Value::String("1/0".into()),
            toml::Value::String("0.5".into()),
        ] {
            assert!(exact_value(&value).is_err());
        }
    }

    #[test]
    fn point_native_names_cannot_silently_shadow_export_aliases() {
        let symbols = [input::symbol("a").unwrap(), input::symbol("k0").unwrap()];
        let aliases = symbols
            .iter()
            .enumerate()
            .map(|(i, s)| ExportSymbol {
                name: format!("k{i}"),
                native_symbol: Atom::var(*s).to_canonical_string(),
                role: "kinematic",
            })
            .collect::<Vec<_>>();
        assert!(binding_index("k0", &symbols, &aliases).is_err());
        assert_eq!(
            binding_index(&aliases[1].native_symbol, &symbols, &aliases).unwrap(),
            1
        );
    }

    #[test]
    fn native_constraints_are_exact_strict_and_bounded() {
        let s = input::symbol("constraint_s").unwrap();
        let aliases = vec![ExportSymbol {
            name: "k0".into(),
            native_symbol: Atom::var(s).to_canonical_string(),
            role: "kinematic",
        }];
        assert_eq!(
            constraint("constraint_s > 4", &[s], &aliases, 1).unwrap(),
            "((-4)+(1)*k0^1) > 0"
        );
        assert_eq!(
            constraint("4 < k0", &[s], &aliases, 1).unwrap(),
            "((-4)+(1)*k0^1) > 0"
        );
        for bad in [
            "k0 >= 4",
            "k0 == 4",
            "k0 > 2^1000000000",
            "k0 > ((2^32)^32)^32^32",
            "unknown > 0",
            "sin(k0) > 0",
        ] {
            assert!(constraint(bad, &[s], &aliases, 1).is_err(), "{bad}");
        }
    }
    #[test]
    fn signed_sparse_roundtrip_keeps_scale_and_cancellation() {
        let x = input::symbol("export_test_x").unwrap();
        let k = input::symbol("export_test_k").unwrap();
        let aliases = vec![
            ExportSymbol {
                name: "k0".into(),
                native_symbol: "k".into(),
                role: "kinematic",
            },
            ExportSymbol {
                name: "x0".into(),
                native_symbol: "x".into(),
                role: "feynman",
            },
        ];
        let p =
            Atom::var(x).pow(2) * Atom::num(Rational::from((-2, 3))) + Atom::var(k) * Atom::var(x);
        let encoded = sparse(&p, &[k, x], &aliases).unwrap();
        assert_eq!(encoded.expression, "(-2/3)*x0^2+(1)*k0^1*x0^1");
        let zero = &p - &p;
        assert_eq!(sparse(&zero, &[k, x], &aliases).unwrap().expression, "0");
    }

    #[test]
    fn native_graph_exports_bind_problem_point_and_source_hashes() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let fixture = repository.join("examples/no_deformation");
        let temporary = tempfile::tempdir().unwrap();
        for (name, expected) in [
            ("bubble", "x0^2-3*x0*x1+x1^2"),
            ("sunrise", "(x0*x1+x0*x2+x1*x2)*(x0+x1+x2)-10*x0*x1*x2"),
        ] {
            let output = temporary.path().join(name);
            export(
                &fixture.join(format!("{name}.toml")),
                &output,
                Some(&fixture.join(format!("{name}-point.toml"))),
                &[],
            )
            .unwrap();
            let bundle: serde_json::Value =
                serde_json::from_slice(&fs::read(output.join("symanzik.json")).unwrap()).unwrap();
            let actual =
                input::expression(bundle["specialized"]["f"]["expression"].as_str().unwrap())
                    .unwrap();
            assert!(
                (actual - input::expression(expected).unwrap())
                    .expand()
                    .is_zero()
            );
            let text = fs::read_to_string(output.join("problem.toml")).unwrap();
            assert_eq!(
                bundle["problem"]["blake3"],
                blake3::hash(text.as_bytes()).to_hex().to_string()
            );
            let problem: toml::Value = toml::from_str(&text).unwrap();
            assert_eq!(
                problem["split"][0].as_str(),
                bundle["symbolic"]["f"]["expression"].as_str()
            );
            assert_eq!(
                problem["parameters"]["k0"].as_str(),
                Some(if name == "bubble" { "5" } else { "10" })
            );
            assert_eq!(bundle["graph"]["numerator_contracted"], false);
            assert!(
                bundle["provenance"]["sources"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|s| s["blake3"].as_str().unwrap().len() == 64)
            );
            assert!(export(&fixture.join(format!("{name}.toml")), &output, None, &[]).is_err());
        }
        let output = temporary.path().join("symbolic");
        export(
            &fixture.join("bubble.toml"),
            &output,
            None,
            &["s > 4".into()],
        )
        .unwrap();
        let problem: toml::Value =
            toml::from_str(&fs::read_to_string(output.join("problem.toml")).unwrap()).unwrap();
        assert_eq!(problem["variables"][0].as_str(), Some("k0"));
        assert_eq!(problem["variable_groups"][0][0].as_str(), Some("k0"));
        assert!(
            problem["constraints"]
                .as_array()
                .unwrap()
                .iter()
                .any(|c| c.as_str() == Some("((-4)+(1)*k0^1) > 0"))
        );
    }

    #[test]
    fn export_rejects_coordinate_bindings_and_degenerate_f() {
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/no_deformation");
        let temporary = tempfile::tempdir().unwrap();
        for (i, point) in [
            "[parameters]\nx0='1'",
            "[parameters]\ns=5.0",
            "[parameters]\nunknown='2'",
        ]
        .iter()
        .enumerate()
        {
            let path = temporary.path().join(format!("point-{i}.toml"));
            fs::write(&path, point).unwrap();
            assert!(
                export(
                    &fixture.join("bubble.toml"),
                    &temporary.path().join(format!("bad-{i}")),
                    Some(&path),
                    &[]
                )
                .is_err()
            );
        }
        assert!(
            export(
                &fixture.join("bubble.toml"),
                &temporary.path().join("bad-domain"),
                None,
                &["x0 > 1".into()]
            )
            .is_err()
        );
        let mut card: toml::Value =
            toml::from_str(&fs::read_to_string(fixture.join("bubble.toml")).unwrap()).unwrap();
        for (key, name) in [
            ("graph", "../graphs/bubble.dot"),
            ("model", "../models/scalar.json"),
        ] {
            card["input"][key] = toml::Value::String(
                fs::canonicalize(fixture.join(name))
                    .unwrap()
                    .to_string_lossy()
                    .into_owned(),
            );
        }
        card["parameters"]["UFO::mt"] = toml::Value::String("0".into());
        let card_path = temporary.path().join("zero.toml");
        fs::write(&card_path, toml::to_string(&card).unwrap()).unwrap();
        let point_path = temporary.path().join("zero-point.toml");
        fs::write(&point_path, "[parameters]\ns='0'\n").unwrap();
        let output = temporary.path().join("degenerate");
        assert!(
            export(&card_path, &output, Some(&point_path), &[])
                .unwrap_err()
                .to_string()
                .contains("zero F")
        );
        assert!(!output.exists());
    }
}
