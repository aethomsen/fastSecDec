//! One local Laurent vector using the existing native Atom/State record codec.
//! A record is untrusted transport, not independent continuation authority.
use crate::{
    generation::{
        self,
        streaming::{
            RecordRef, StreamingError,
            codec::{self, Atoms, invalid},
        },
    },
    kernel::algebraic,
    threshold::continued::ContinuedSource,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
use symbolica::{
    atom::{AliasedAtom, Atom, AtomCore, Symbol},
    evaluate::{FunctionMap, FunctionRegistrationOptions, InliningPolicy},
};

const KIND: &str = "threshold-local-laurent-v1";
const ALGEBRAIC_KIND: &str = "threshold-algebraic-local-laurent-v1";
fn is_false(value: &bool) -> bool {
    !*value
}
type Result<T> = std::result::Result<T, StreamingError>;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Aliased {
    root: usize,
    aliases: Vec<(usize, usize)>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Definition {
    head: usize,
    tags: Vec<usize>,
    formals: Vec<usize>,
    body: usize,
    derivative_order: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mixed_orders: Option<Vec<usize>>,
    #[serde(default, skip_serializing_if = "is_false")]
    always_inline: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum VectorKind {
    Stochastic {},
    Exact {},
    ZeroInLayout {},
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    parent: String,
    chart: usize,
    maximum: i32,
    coordinates: Vec<usize>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    runtime_parameters: Vec<usize>,
    kind: VectorKind,
    coefficients: Vec<(i32, Aliased)>,
    definitions: Vec<Definition>,
    profiles: Vec<generation::EndpointProfileRow>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    coefficient_zeros: Vec<StoredZero>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    roots: Vec<(usize, Vec<u8>)>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredZero {
    order: i32,
    variables: Vec<usize>,
    atoms: Vec<usize>,
    gap_power: u32,
    branches: [String; 2],
    inverse_units: Vec<(usize, u32)>,
    digest: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StagedVector {
    pub record: RecordRef,
    pub chart: usize,
    pub minimum: i32,
    pub maximum: i32,
    pub kind: VectorKind,
    #[serde(default, skip_serializing_if = "is_false")]
    pub algebraic: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coefficient_zeros: Vec<(i32, String)>,
}
pub(crate) struct LocalVector {
    pub roots: algebraic::Scope,
    pub coordinates: Vec<Symbol>,
    pub runtime_parameters: Vec<Symbol>,
    pub coefficients: BTreeMap<i32, AliasedAtom>,
    pub functions: FunctionMap,
    pub profiles: Vec<generation::EndpointProfileRow>,
    pub kind: VectorKind,
}
fn symbol_index(symbols: &mut Vec<Symbol>, symbol: Symbol) -> usize {
    if let Some(index) = symbols.iter().position(|s| *s == symbol) {
        index
    } else {
        let index = symbols.len();
        symbols.push(symbol);
        index
    }
}
fn alias(value: &AliasedAtom, atoms: &mut Atoms) -> Aliased {
    let mut aliases = value.get_aliases().iter().collect::<Vec<_>>();
    aliases.sort_by_cached_key(|(a, _)| a.to_canonical_string());
    Aliased {
        root: atoms.push(value.get_root()),
        aliases: aliases
            .into_iter()
            .map(|(a, b)| (atoms.push(a), atoms.push(b)))
            .collect(),
    }
}

pub(crate) fn write(
    root: &Path,
    parent: &str,
    source: ContinuedSource<'_>,
    chart: usize,
    maximum: i32,
) -> Result<StagedVector> {
    let expression = source
        .charts()
        .get(chart)
        .ok_or_else(|| invalid("continued chart index"))?;
    let mut coefficients = generation::threshold_expand_vector(
        expression,
        &source.coordinates(),
        source.regulators()[0],
        maximum,
    )?;
    let mut coefficient_zeros = Vec::new();
    if let ContinuedSource::Algebraic(bound) = source {
        for (order, value) in &mut coefficients {
            if let Some(proof) =
                crate::threshold::regularization::secant::zeros::prove(bound, chart, *order, value)
                    .map_err(invalid)?
            {
                *value = AliasedAtom::from(Atom::zero());
                coefficient_zeros.push(proof);
            }
        }
    }
    let kind = if coefficients.is_empty() {
        VectorKind::ZeroInLayout {}
    } else if source.rational().is_some()
        && coefficients
            .values()
            .all(|c| generation::coordinate_independent(c, &source.coordinates()))
    {
        for coefficient in coefficients.values_mut() {
            let materialized = source
                .rational()
                .expect("rational source checked")
                .materialize_exact(&coefficient.clone().into_inner())
                .map_err(invalid)?;
            *coefficient = AliasedAtom::from(materialized);
        }
        VectorKind::Exact {}
    } else {
        VectorKind::Stochastic {}
    };
    let minimum = coefficients
        .keys()
        .next()
        .copied()
        .unwrap_or(maximum)
        .min(maximum.min(0));
    let mut atoms = Atoms::default();
    let mut symbols = Vec::new();
    let coordinates = source
        .coordinates()
        .iter()
        .map(|s| symbol_index(&mut symbols, *s))
        .collect();
    let runtime_parameters = source
        .runtime_parameters()
        .iter()
        .map(|s| symbol_index(&mut symbols, *s))
        .collect();
    let zero_digests = coefficient_zeros
        .iter()
        .map(|p| (p.order, p.digest.clone()))
        .collect();
    let coefficient_zeros = coefficient_zeros
        .into_iter()
        .map(|p| StoredZero {
            order: p.order,
            variables: p
                .variables
                .into_iter()
                .map(|s| symbol_index(&mut symbols, s))
                .collect(),
            atoms: p.atoms.iter().map(|a| atoms.push(a)).collect(),
            gap_power: p.gap_power,
            branches: p.branches,
            inverse_units: p.inverse_units,
            digest: p.digest,
        })
        .collect();
    let coefficients = coefficients
        .iter()
        .map(|(order, value)| (*order, alias(value, &mut atoms)))
        .collect();
    let mut definitions = Vec::new();
    if kind == (VectorKind::Stochastic {}) {
        for definition in source.definitions(chart) {
            definitions.push(Definition {
                head: symbol_index(&mut symbols, definition.head),
                tags: definition.tags.iter().map(|a| atoms.push(a)).collect(),
                formals: definition
                    .formals
                    .iter()
                    .map(|s| symbol_index(&mut symbols, *s))
                    .collect(),
                body: atoms.push(&definition.body),
                derivative_order: source
                    .rational()
                    .and_then(|_| definition.orders.as_ref().map(|o| o[1])),
                mixed_orders: if source.rational().is_none() {
                    definition.orders
                } else {
                    None
                },
                always_inline: source.rational().is_none() && definition.always_inline,
            });
        }
    }
    let algebraic = source.rational().is_none();
    let roots = source
        .roots(chart)
        .saved()
        .into_iter()
        .map(|(tag, bytes)| (symbol_index(&mut symbols, tag), bytes))
        .collect();
    let record = codec::write(
        root,
        &format!("threshold-chart-{chart}"),
        if algebraic { ALGEBRAIC_KIND } else { KIND },
        &Stored {
            parent: parent.into(),
            chart,
            maximum,
            coordinates,
            runtime_parameters,
            kind,
            coefficients,
            definitions,
            coefficient_zeros,
            roots,
            profiles: source
                .profiles()
                .get(chart)
                .ok_or_else(|| invalid("continued chart profiles"))?
                .clone(),
        },
        atoms,
        symbols,
    )?;
    Ok(StagedVector {
        record,
        chart,
        minimum,
        maximum,
        kind,
        algebraic,
        coefficient_zeros: zero_digests,
    })
}

pub(crate) fn read(
    root: &Path,
    record: &StagedVector,
    parent: &str,
    maximum_bytes: u64,
) -> Result<LocalVector> {
    if record.record.bytes > maximum_bytes {
        return Err(invalid("local vector transport limit"));
    }
    if record.algebraic {
        algebraic::register();
    }
    let (stored, atoms, symbols): (Stored, _, _) = codec::read(
        root,
        &record.record,
        if record.algebraic {
            ALGEBRAIC_KIND
        } else {
            KIND
        },
    )?;
    if !record.algebraic
        && (!stored.coefficient_zeros.is_empty()
            || !record.coefficient_zeros.is_empty()
            || !stored.roots.is_empty()
            || !stored.runtime_parameters.is_empty()
            || stored
                .definitions
                .iter()
                .any(|d| d.mixed_orders.is_some() || d.always_inline))
    {
        return Err(invalid("algebraic helpers in historical rational record"));
    }
    if stored.parent != parent
        || stored.chart != record.chart
        || stored.maximum != record.maximum
        || stored.kind != record.kind
    {
        return Err(invalid("local vector parent/chart/layout association"));
    }
    let symbol = |index: usize| {
        symbols
            .get(index)
            .copied()
            .ok_or_else(|| invalid("local vector symbol index"))
    };
    let roots = algebraic::Scope::restore(
        stored
            .roots
            .into_iter()
            .map(|(tag, bytes)| symbol(tag).map(|tag| (tag, bytes)))
            .collect::<Result<Vec<_>>>()?,
    )
    .map_err(invalid)?;
    let zero_digests = stored
        .coefficient_zeros
        .iter()
        .map(|p| (p.order, p.digest.clone()))
        .collect::<Vec<_>>();
    if zero_digests != record.coefficient_zeros
        || zero_digests
            .iter()
            .map(|p| p.0)
            .collect::<BTreeSet<_>>()
            .len()
            != zero_digests.len()
    {
        return Err(invalid("local coefficient-zero inventory"));
    }
    for p in stored.coefficient_zeros {
        let proof = crate::threshold::regularization::secant::zeros::Receipt {
            order: p.order,
            variables: p
                .variables
                .into_iter()
                .map(symbol)
                .collect::<Result<Vec<_>>>()?,
            atoms: p
                .atoms
                .into_iter()
                .map(|i| atoms.take(i))
                .collect::<Result<Vec<_>>>()?,
            gap_power: p.gap_power,
            branches: p.branches,
            inverse_units: p.inverse_units,
            digest: p.digest,
        };
        if !proof
            .branches
            .iter()
            .all(|id| roots.owners().any(|(_, p)| p.semantic_identity() == id))
            || crate::threshold::regularization::secant::zeros::validate(&proof)
                .map_err(invalid)?
                .as_ref()
                != Some(&proof.digest)
        {
            return Err(invalid(
                "local coefficient-zero identity/branch association",
            ));
        }
    }
    let coordinates = stored
        .coordinates
        .into_iter()
        .map(symbol)
        .collect::<Result<Vec<_>>>()?;
    let runtime_parameters = stored
        .runtime_parameters
        .into_iter()
        .map(symbol)
        .collect::<Result<Vec<_>>>()?;
    if coordinates
        .iter()
        .chain(&runtime_parameters)
        .copied()
        .collect::<BTreeSet<_>>()
        .len()
        != coordinates.len() + runtime_parameters.len()
    {
        return Err(invalid("local vector runtime/coordinate roles"));
    }
    if coordinates.is_empty()
        || coordinates.iter().copied().collect::<BTreeSet<_>>().len() != coordinates.len()
    {
        return Err(invalid("local vector coordinate layout"));
    }
    let mut coefficients = BTreeMap::new();
    for (order, value) in stored.coefficients {
        if order > stored.maximum {
            return Err(invalid("local coefficient exceeds requested maximum"));
        }
        let mut coefficient = AliasedAtom::from(atoms.take(value.root)?);
        let mut seen = BTreeSet::new();
        for (handle, body) in value.aliases {
            let handle = atoms.take(handle)?;
            if !seen.insert(handle.clone()) {
                return Err(invalid("duplicate local alias"));
            }
            coefficient.register_alias(handle, atoms.take(body)?);
        }
        if coefficients.insert(order, coefficient).is_some() {
            return Err(invalid("duplicate local Laurent order"));
        }
    }
    if coefficients
        .keys()
        .next()
        .copied()
        .unwrap_or(stored.maximum)
        .min(stored.maximum.min(0))
        != record.minimum
    {
        return Err(invalid("local Laurent minimum"));
    }
    if (stored.kind == (VectorKind::ZeroInLayout {})) != coefficients.is_empty() {
        return Err(invalid("zero-layout record mismatch"));
    }
    if stored.kind != (VectorKind::Stochastic {}) && !stored.definitions.is_empty() {
        return Err(invalid("nonstochastic helper definitions"));
    }
    if record.coefficient_zeros.iter().any(|(order, _)| {
        coefficients
            .get(order)
            .is_none_or(|a| !a.get_root().is_zero() || !a.get_aliases().is_empty())
    }) {
        return Err(invalid("coefficient-zero output association"));
    }
    let mut functions = FunctionMap::new();
    for definition in stored.definitions {
        let head = symbol(definition.head)?;
        let tags = definition
            .tags
            .into_iter()
            .map(|i| atoms.take(i))
            .collect::<Result<Vec<_>>>()?;
        let formals = definition
            .formals
            .into_iter()
            .map(symbol)
            .collect::<Result<Vec<_>>>()?;
        let body = atoms.take(definition.body)?;
        if definition.derivative_order.is_some() && definition.mixed_orders.is_some() {
            return Err(invalid("mixed derivative schema versions"));
        }
        if let Some(orders) = definition.mixed_orders {
            if head != Symbol::DERIVATIVE
                || orders.len() != formals.len() + 1
                || orders.first() != Some(&0)
                || !orders.iter().any(|n| *n > 0)
                || tags.len() != orders.len() + 2
                || tags.iter().zip(&orders).any(|(a, n)| a != &Atom::num(*n))
            {
                return Err(invalid("mixed derivative definition signature"));
            }
            functions
                .add_tagged_function_with_options(
                    head,
                    tags,
                    formals,
                    body,
                    FunctionRegistrationOptions::new().inlining(InliningPolicy::Always),
                )
                .map_err(invalid)?;
        } else if let Some(order) = definition.derivative_order {
            if head != Symbol::DERIVATIVE
                || order == 0
                || tags.len() != 4
                || !tags[0].is_zero()
                || tags[1] != Atom::num(order)
            {
                return Err(invalid("local derivative definition signature"));
            }
            functions
                .add_tagged_function_with_options(
                    head,
                    tags,
                    formals,
                    body,
                    FunctionRegistrationOptions::new().inlining(InliningPolicy::Always),
                )
                .map_err(invalid)?;
        } else {
            functions
                .add_tagged_function_with_options(
                    head,
                    tags,
                    formals,
                    body,
                    if definition.always_inline {
                        FunctionRegistrationOptions::new().inlining(InliningPolicy::Always)
                    } else {
                        FunctionRegistrationOptions::new()
                    },
                )
                .map_err(invalid)?;
        }
    }
    Ok(LocalVector {
        roots,
        coordinates,
        runtime_parameters,
        coefficients,
        functions,
        profiles: stored.profiles,
        kind: stored.kind,
    })
}

/// Exact offsets are one complete vector/group. The caller has already checked
/// each local exact contribution; all original contribution IDs stay in metadata.
pub(crate) fn write_exact(
    root: &Path,
    parent: &str,
    coordinates: &[Symbol],
    runtime_parameters: &[Symbol],
    coefficients: &BTreeMap<i32, Atom>,
    maximum: i32,
) -> Result<StagedVector> {
    let mut atoms = Atoms::default();
    let values = coefficients
        .iter()
        .map(|(order, value)| (*order, alias(&AliasedAtom::from(value.clone()), &mut atoms)))
        .collect();
    let minimum = coefficients
        .keys()
        .next()
        .copied()
        .unwrap_or(maximum)
        .min(maximum.min(0));
    let stored = Stored {
        parent: parent.into(),
        chart: usize::MAX,
        maximum,
        coordinates: (0..coordinates.len()).collect(),
        runtime_parameters: (coordinates.len()..coordinates.len() + runtime_parameters.len())
            .collect(),
        kind: VectorKind::Exact {},
        coefficients: values,
        definitions: Vec::new(),
        profiles: Vec::new(),
        roots: Vec::new(),
        coefficient_zeros: Vec::new(),
    };
    let record = codec::write(
        root,
        "threshold-exact-sum",
        if runtime_parameters.is_empty() {
            KIND
        } else {
            ALGEBRAIC_KIND
        },
        &stored,
        atoms,
        coordinates
            .iter()
            .chain(runtime_parameters)
            .copied()
            .collect(),
    )?;
    Ok(StagedVector {
        record,
        chart: usize::MAX,
        minimum,
        maximum,
        kind: VectorKind::Exact {},
        algebraic: !runtime_parameters.is_empty(),
        coefficient_zeros: Vec::new(),
    })
}
