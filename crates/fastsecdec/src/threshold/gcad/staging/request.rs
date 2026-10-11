//! Native request reconstruction; no solving and no trusted proof decoding.
use super::{Error, RequestRecord, Result};
use crate::{
    generation::{
        identity::{CanonicalAtom, CanonicalSymbol},
        streaming::{
            codec::{self, Atoms, invalid},
            input::Input,
        },
    },
    parametric::FactorRole,
    threshold::{
        gcad::{AliasRole, DomainOrigin, GcadKinematics, GcadRequest, PreparedDomain, Problem},
        projective::{AffineProjectivePreparation, FactorOrigin},
    },
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};
use symbolica::{
    atom::{Atom, AtomCore},
    prelude::Rational,
};

const KIND: &str = "threshold-gcad-request-v1";

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Origin {
    // Pinned Serde ignores unknown fields on internally tagged unit variants.
    // An empty struct keeps the same wire representation and rejects them.
    UnitCube {},
    AffineProjective { eliminated_index: usize },
    ExplicitPrepared { provenance: String },
}
impl From<&DomainOrigin> for Origin {
    fn from(origin: &DomainOrigin) -> Self {
        match origin {
            DomainOrigin::NativeUnitCube => Self::UnitCube {},
            DomainOrigin::AffineProjective { eliminated_index } => Self::AffineProjective {
                eliminated_index: *eliminated_index,
            },
            DomainOrigin::ExplicitPrepared { provenance } => Self::ExplicitPrepared {
                provenance: provenance.clone(),
            },
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRequest {
    source_identity: String,
    witness: String,
    input: Input,
    origin: Origin,
    coordinates: Vec<usize>,
    domain_positive: Vec<usize>,
    exact_values: Vec<(usize, usize)>,
    runtime_parameters: Vec<usize>,
    parameter_positive: Vec<usize>,
    problem: Problem,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    represented: Option<StoredRepresentation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preparametric_graph: Option<StoredOriginal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preparametric_family: Option<StoredOriginal>,
}

/// Association bytes only: replay requires the original native graph owner and
/// re-runs bounded conversion/parameterization. This is not a graph decoder.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredOriginal {
    version: u32,
    meaning: crate::threshold::represented::NumericalMeaning,
    limits: crate::threshold::represented::Limits,
    source_witness: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    graph_payloads: Option<crate::threshold::represented::graph::GraphPayloadPolicy>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredLiteral {
    location: crate::threshold::represented::Location,
    original: usize,
    exact: usize,
    precision_bits: [u32; 2],
    binary_exponents: [Option<i32>; 2],
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredRepresentation {
    version: u32,
    meaning: crate::threshold::represented::NumericalMeaning,
    limits: crate::threshold::represented::Limits,
    exact_input: Input,
    literals: Vec<StoredLiteral>,
}

fn origin(origin: FactorOrigin) -> (u8, usize) {
    match origin {
        FactorOrigin::OriginalFactor { factor_index } => (0, factor_index),
        FactorOrigin::OriginalMonomial { parameter_index } => (1, parameter_index),
    }
}

/// A canonical association witness, not an alternate symbolic representation.
/// Native canonical serializers feed a streaming hash, without building a
/// second transformed density or expanding any factor.
fn witness(request: &GcadRequest, source_identity: &str) -> Result<String> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"fastsecdec-threshold-gcad-request-association-v1\0");
    fn item<T: Serialize>(hash: &mut blake3::Hasher, value: &T) -> Result<()> {
        serde_json::to_writer(&mut *hash, value).map_err(invalid)?;
        hash.update(b"\0");
        Ok(())
    }
    item(&mut hash, &source_identity)?;
    if let Some(r) = request.represented_input() {
        item(&mut hash, &r.limits())?;
    }
    if let Some(r) = request.preparametric_graph_input() {
        if let Some(policy) = r.payload_policy() {
            item(&mut hash, &(2u32, r.limits(), r.source_witness(), policy))?;
        } else {
            item(&mut hash, &(1u32, r.limits(), r.source_witness()))?;
        }
    }
    if let Some(r) = request.preparametric_family_input() {
        item(&mut hash, &("family-v1", r.limits(), r.source_witness()))?;
    }
    item(&mut hash, &Origin::from(request.domain().origin()))?;
    item(
        &mut hash,
        &request
            .domain()
            .coordinates()
            .iter()
            .copied()
            .map(CanonicalSymbol)
            .collect::<Vec<_>>(),
    )?;
    item(
        &mut hash,
        &request
            .domain()
            .strict_positive()
            .iter()
            .map(CanonicalAtom)
            .collect::<Vec<_>>(),
    )?;
    item(
        &mut hash,
        &request
            .kinematics()
            .runtime_parameters
            .iter()
            .copied()
            .map(CanonicalSymbol)
            .collect::<Vec<_>>(),
    )?;
    let mut exact_values = request.kinematics().exact_values.iter().collect::<Vec<_>>();
    exact_values.sort_by_cached_key(|(symbol, _)| Atom::var(**symbol).to_canonical_string());
    for (symbol, value) in exact_values {
        item(
            &mut hash,
            &(
                CanonicalSymbol(*symbol),
                CanonicalAtom(&Atom::num(value.clone())),
            ),
        )?;
    }
    item(
        &mut hash,
        &request
            .kinematics()
            .strict_positive
            .iter()
            .map(CanonicalAtom)
            .collect::<Vec<_>>(),
    )?;
    item(&mut hash, request.problem())?;
    for alias in request.aliases() {
        item(
            &mut hash,
            &(
                CanonicalSymbol(alias.symbol),
                &alias.name,
                alias.role == AliasRole::RuntimeParameter,
            ),
        )?;
    }
    for term in request.prepared_terms() {
        item(&mut hash, &CanonicalAtom(term.prefactor()))?;
        item(
            &mut hash,
            &term
                .monomial_powers()
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
        )?;
        item(&mut hash, &term.factors().len())?;
        for factor in term.factors() {
            item(
                &mut hash,
                &(
                    CanonicalAtom(factor.polynomial()),
                    CanonicalAtom(factor.exponent()),
                    factor.role() == FactorRole::Polynomial,
                    factor.semantics(),
                ),
            )?;
        }
    }
    if let Some(preparation) = request.projective_preparation() {
        item(
            &mut hash,
            &preparation
                .images()
                .iter()
                .map(CanonicalAtom)
                .collect::<Vec<_>>(),
        )?;
        item(&mut hash, &CanonicalAtom(preparation.measure_jacobian()))?;
        item(
            &mut hash,
            &preparation
                .factor_origins()
                .iter()
                .map(|terms| terms.iter().copied().map(origin).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
        )?;
    }
    for factor in request.signed_factors() {
        item(
            &mut hash,
            &(
                factor.term_index,
                factor.factor_index,
                origin(factor.origin),
                factor.split_index,
                CanonicalAtom(&factor.original_polynomial),
                CanonicalAtom(&factor.prepared_polynomial),
                CanonicalAtom(&factor.exponent),
                factor.semantics,
            ),
        )?;
    }
    Ok(hash.finalize().to_hex().to_string())
}

pub(super) fn write(root: &Path, request: &GcadRequest) -> Result<RequestRecord> {
    let source_identity = request.source_identity()?;
    let mut atoms = Atoms::default();
    let input = Input::encode(request.input(), &mut atoms);
    let mut symbols = request
        .input()
        .parameters()
        .iter()
        .copied()
        .chain([request.input().regulator()])
        .collect::<Vec<_>>();
    let mut symbol = |symbol| {
        let index = symbols.len();
        symbols.push(symbol);
        index
    };
    let represented = request.represented_input().map(|r| StoredRepresentation {
        version: 1,
        meaning: r.meaning(),
        limits: r.limits(),
        exact_input: Input::encode(r.exact(), &mut atoms),
        literals: r
            .conversions()
            .iter()
            .map(|c| StoredLiteral {
                location: c.location.clone(),
                original: atoms.push(&c.original),
                exact: atoms.push(&c.exact),
                precision_bits: c.precision_bits,
                binary_exponents: c.binary_exponents,
            })
            .collect(),
    });
    let stored = StoredRequest {
        source_identity: source_identity.clone(),
        witness: witness(request, &source_identity)?,
        input,
        represented,
        preparametric_graph: request.preparametric_graph_input().map(|r| StoredOriginal {
            version: if r.payload_policy().is_some() { 2 } else { 1 },
            meaning: r.meaning(),
            limits: r.limits(),
            source_witness: r.source_witness().to_owned(),
            graph_payloads: r.payload_policy(),
        }),
        preparametric_family: request
            .preparametric_family_input()
            .map(|r| StoredOriginal {
                version: 1,
                meaning: r.meaning(),
                limits: r.limits(),
                source_witness: r.source_witness().to_owned(),
                graph_payloads: None,
            }),
        origin: request.domain().origin().into(),
        coordinates: request
            .domain()
            .coordinates()
            .iter()
            .copied()
            .map(&mut symbol)
            .collect(),
        domain_positive: request
            .domain()
            .strict_positive()
            .iter()
            .map(|x| atoms.push(x))
            .collect(),
        exact_values: request
            .kinematics()
            .exact_values
            .iter()
            .map(|(s, r)| (symbol(*s), atoms.push(&Atom::num(r.clone()))))
            .collect(),
        runtime_parameters: request
            .kinematics()
            .runtime_parameters
            .iter()
            .copied()
            .map(&mut symbol)
            .collect(),
        parameter_positive: request
            .kinematics()
            .strict_positive
            .iter()
            .map(|x| atoms.push(x))
            .collect(),
        problem: request.problem().clone(),
    };
    let record = codec::write(root, "gcad-request", KIND, &stored, atoms, symbols)?;
    Ok(RequestRecord {
        source_identity,
        record,
    })
}

pub(super) enum OriginalSource<'a> {
    Graph {
        original: std::sync::Arc<crate::threshold::represented::graph::GraphPoint>,
        observer: &'a mut dyn FnMut(
            crate::threshold::represented::graph::Progress,
        ) -> std::ops::ControlFlow<()>,
    },
    Family {
        original: std::sync::Arc<crate::threshold::represented::family::FamilyPoint>,
        observer: &'a mut dyn FnMut(
            crate::threshold::represented::family::Progress,
        ) -> std::ops::ControlFlow<()>,
    },
}

pub(super) fn read(
    root: &Path,
    receipt: &RequestRecord,
    conversion_cap: crate::threshold::represented::Limits,
    original: Option<OriginalSource<'_>>,
) -> Result<GcadRequest> {
    let (stored, atoms, symbols): (StoredRequest, _, _) = codec::read(root, &receipt.record, KIND)?;
    if usize::from(stored.represented.is_some())
        + usize::from(stored.preparametric_graph.is_some())
        + usize::from(stored.preparametric_family.is_some())
        > 1
    {
        return Err(Error::Association);
    }
    if matches!(&original, Some(OriginalSource::Graph { .. }))
        && stored.preparametric_graph.is_none()
        || matches!(&original, Some(OriginalSource::Family { .. }))
            && stored.preparametric_family.is_none()
    {
        return Err(Error::Association);
    }
    let (original_graph, original_family) = match original {
        Some(OriginalSource::Graph { original, observer }) => (Some((original, observer)), None),
        Some(OriginalSource::Family { original, observer }) => (None, Some((original, observer))),
        None => (None, None),
    };
    let symbol = |i: usize| {
        symbols
            .get(i)
            .copied()
            .ok_or_else(|| invalid("GCAD request Symbol index out of bounds"))
    };
    let prefix = stored
        .input
        .parameters
        .checked_add(1)
        .ok_or_else(|| invalid("GCAD input symbol count overflow"))?;
    let input_symbols = symbols
        .get(..prefix)
        .ok_or_else(|| invalid("GCAD input symbol layout"))?;
    let input = stored.input.decode(&atoms, input_symbols)?;
    let graph = if let Some(r) = stored.preparametric_graph {
        if !((r.version == 1 && r.graph_payloads.is_none())
            || (r.version == 2 && r.graph_payloads.is_some()))
            || !r.limits.is_within(conversion_cap)
        {
            return Err(invalid("pre-parametric conversion version or caller resource cap").into());
        }
        let (original, observer) = original_graph.ok_or(Error::OriginalGraphRequired)?;
        let rebuilt = crate::threshold::represented::graph::ExactRepresentedGraphInput::prepare_with_payloads(
            original, r.meaning, r.limits, r.graph_payloads, observer,
        )
        .map_err(super::GcadError::from)?;
        if rebuilt.source_witness() != r.source_witness || rebuilt.exact().as_ref() != &input {
            return Err(Error::Association);
        }
        Some(std::sync::Arc::new(rebuilt))
    } else {
        None
    };
    let family = if let Some(r) = stored.preparametric_family {
        if r.version != 1 || r.graph_payloads.is_some() || !r.limits.is_within(conversion_cap) {
            return Err(
                invalid("pre-parametric family conversion version or caller resource cap").into(),
            );
        }
        let (original, observer) = original_family.ok_or(Error::OriginalFamilyRequired)?;
        let rebuilt = crate::threshold::represented::family::ExactRepresentedFamilyInput::prepare(
            original, r.meaning, r.limits, observer,
        )
        .map_err(super::GcadError::from)?;
        if rebuilt.source_witness() != r.source_witness || rebuilt.exact().as_ref() != &input {
            return Err(Error::Association);
        }
        Some(std::sync::Arc::new(rebuilt))
    } else {
        None
    };
    let represented = if let Some(r) = stored.represented {
        if r.version != 1 || !r.limits.is_within(conversion_cap) {
            return Err(invalid("represented conversion version or caller resource cap").into());
        }
        let rebuilt = crate::threshold::represented::ExactRepresentedInput::prepare(
            std::sync::Arc::new(input.clone()),
            r.meaning,
            r.limits,
            |_| std::ops::ControlFlow::Continue(()),
        )
        .map_err(super::GcadError::from)?;
        let exact = r.exact_input.decode(&atoms, input_symbols)?;
        if rebuilt.conversions().is_empty()
            || rebuilt.exact().as_ref() != &exact
            || rebuilt.conversions().len() != r.literals.len()
        {
            return Err(Error::Association);
        }
        for (actual, stored) in rebuilt.conversions().iter().zip(r.literals) {
            if actual.location != stored.location
                || actual.original != atoms.take(stored.original)?
                || actual.exact != atoms.take(stored.exact)?
                || actual.precision_bits != stored.precision_bits
                || actual.binary_exponents != stored.binary_exponents
            {
                return Err(Error::Association);
            }
        }
        Some(std::sync::Arc::new(rebuilt))
    } else {
        None
    };
    let geometry_input = represented.as_ref().map_or(&input, |r| r.exact().as_ref());
    let coordinates = stored
        .coordinates
        .into_iter()
        .map(symbol)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let positive = stored
        .domain_positive
        .into_iter()
        .map(|i| atoms.take(i))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut exact_values = BTreeMap::new();
    for (s, value) in stored.exact_values {
        let value = Rational::try_from(atoms.take(value)?.as_view()).map_err(invalid)?;
        if exact_values.insert(symbol(s)?, value).is_some() {
            return Err(invalid("duplicate GCAD exact kinematic binding").into());
        }
    }
    let kinematics = GcadKinematics {
        exact_values,
        runtime_parameters: stored
            .runtime_parameters
            .into_iter()
            .map(symbol)
            .collect::<std::result::Result<_, _>>()?,
        strict_positive: stored
            .parameter_positive
            .into_iter()
            .map(|i| atoms.take(i))
            .collect::<std::result::Result<_, _>>()?,
    };
    let request = match stored.origin {
        Origin::UnitCube {} => GcadRequest::unit_cube(
            geometry_input,
            kinematics,
            stored.problem.solver.clone(),
            stored.problem.limits.clone(),
        )?,
        Origin::AffineProjective { eliminated_index } => {
            let preparation =
                AffineProjectivePreparation::eliminate(geometry_input, eliminated_index)
                    .map_err(invalid)?;
            GcadRequest::projective(
                preparation,
                kinematics,
                stored.problem.solver.clone(),
                stored.problem.limits.clone(),
            )?
        }
        Origin::ExplicitPrepared { provenance } => GcadRequest::prepared(
            geometry_input,
            PreparedDomain::explicit(coordinates.clone(), positive.clone(), provenance)?,
            kinematics,
            stored.problem.solver.clone(),
            stored.problem.limits.clone(),
        )?,
    };
    let request = if let Some(r) = represented {
        request.retain_represented(r)?
    } else if let Some(r) = graph {
        request.retain_preparametric(r)?
    } else if let Some(r) = family {
        request.retain_preparametric_family(r)?
    } else {
        request
    };
    let actual_source = request.source_identity()?;
    if request.domain().coordinates() != coordinates
        || request.domain().strict_positive() != positive
        || !request.matches_native_problem(&stored.problem)
        || stored.source_identity != receipt.source_identity
        || stored.source_identity != actual_source
        || stored.witness != witness(&request, &actual_source)?
    {
        return Err(Error::Association);
    }
    Ok(request)
}

#[cfg(test)]
mod tests {
    use super::Origin;

    #[test]
    fn request_origin_wire_rejects_unknown_metadata_without_changing_valid_bytes() {
        assert_eq!(
            serde_json::to_string(&Origin::UnitCube {}).unwrap(),
            r#"{"kind":"unit_cube"}"#
        );
        assert!(matches!(
            serde_json::from_str::<Origin>(r#"{"kind":"unit_cube"}"#).unwrap(),
            Origin::UnitCube {}
        ));
        for mut value in [
            serde_json::json!({"kind": "unit_cube"}),
            serde_json::json!({"kind": "affine_projective", "eliminated_index": 1}),
            serde_json::json!({"kind": "explicit_prepared", "provenance": "native"}),
        ] {
            assert!(serde_json::from_value::<Origin>(value.clone()).is_ok());
            value["unrecognized_domain_setting"] = serde_json::json!(17);
            assert!(serde_json::from_value::<Origin>(value).is_err());
        }
    }
}

#[cfg(test)]
mod represented_tests;

#[cfg(test)]
mod graph_wire_tests {
    use super::*;
    use crate::{
        parametric::{ParametricDomain, ParametricIntegrand, ParametricTerm},
        threshold::{
            gcad::staging::StagedRequest,
            represented::{Limits, NumericalMeaning},
        },
    };
    use std::sync::Arc;
    use symbolica::symbol;

    #[test]
    fn graph_wire_cannot_mix_source_kinds_or_downgrade_to_exact() {
        let (x, eps) = symbol!("graph_wire::x", "graph_wire::eps");
        let input = ParametricIntegrand::new(
            vec![x],
            eps,
            ParametricDomain::UnitCube,
            vec![ParametricTerm::new(Atom::one(), vec![Atom::Zero], vec![])],
        )
        .unwrap();
        let request = GcadRequest::unit_cube(
            &input,
            Default::default(),
            Default::default(),
            GcadRequest::default_limits(),
        )
        .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let saved = StagedRequest::write(dir.path(), Arc::new(request.clone())).unwrap();
        let restored = StagedRequest::read(dir.path(), saved.receipt(), 10_000_000).unwrap();
        assert_eq!(restored.request().identity(), request.identity());
        assert_eq!(
            request.source_identity().unwrap(),
            crate::generation::source_identity(&input, &[], &[]).unwrap()
        );
        for case in 0..5 {
            let (mut stored, mut atoms, symbols): (StoredRequest, _, _) =
                codec::read(dir.path(), &saved.receipt().record, KIND).unwrap();
            let ordinary = serde_json::to_value(&stored).unwrap();
            assert!(ordinary.get("preparametric_graph").is_none());
            assert!(ordinary.get("preparametric_family").is_none());
            assert!(ordinary.get("represented").is_none());
            stored.preparametric_graph = Some(StoredOriginal {
                version: if case == 2 || case == 4 { 2 } else { 1 },
                meaning: NumericalMeaning::RepresentedValues,
                limits: Limits::default(),
                source_witness: "not native authority".into(),
                graph_payloads: (case >= 3).then_some(
                    crate::threshold::represented::graph::GraphPayloadPolicy::PrimitiveFragmentsV1,
                ),
            });
            if case == 1 {
                stored.represented = Some(StoredRepresentation {
                    version: 1,
                    meaning: NumericalMeaning::RepresentedValues,
                    limits: Limits::default(),
                    exact_input: Input::encode(&input, &mut atoms),
                    literals: vec![],
                });
            }
            let record = codec::write(
                dir.path(),
                "graph-wire-tamper",
                KIND,
                &stored,
                atoms,
                symbols,
            )
            .unwrap();
            let receipt = RequestRecord {
                source_identity: saved.receipt().source_identity.clone(),
                record,
            };
            let result = StagedRequest::read(dir.path(), &receipt, 10_000_000);
            match case {
                0 => assert!(matches!(result, Err(Error::OriginalGraphRequired))),
                1 => assert!(matches!(result, Err(Error::Association))),
                _ => assert!(result.is_err()),
            }
        }
        for case in 0..5 {
            let (mut stored, mut atoms, symbols): (StoredRequest, _, _) =
                codec::read(dir.path(), &saved.receipt().record, KIND).unwrap();
            let original = || StoredOriginal {
                version: 1,
                meaning: NumericalMeaning::RepresentedValues,
                limits: Limits::default(),
                source_witness: "not authority".into(),
                graph_payloads: None,
            };
            stored.preparametric_family = Some(original());
            if case == 1 {
                stored.preparametric_graph = Some(original());
            }
            if case == 2 {
                stored.represented = Some(StoredRepresentation {
                    version: 1,
                    meaning: NumericalMeaning::RepresentedValues,
                    limits: Limits::default(),
                    exact_input: Input::encode(&input, &mut atoms),
                    literals: vec![],
                });
            }
            if case == 3 {
                stored.preparametric_family.as_mut().unwrap().version = 2;
            }
            if case == 4 {
                stored.preparametric_family.as_mut().unwrap().graph_payloads = Some(
                    crate::threshold::represented::graph::GraphPayloadPolicy::PrimitiveFragmentsV1,
                );
            }
            let record = codec::write(
                dir.path(),
                "family-wire-tamper",
                KIND,
                &stored,
                atoms,
                symbols,
            )
            .unwrap();
            let receipt = RequestRecord {
                source_identity: saved.receipt().source_identity.clone(),
                record,
            };
            let result = StagedRequest::read(dir.path(), &receipt, 10_000_000);
            if case == 0 {
                assert!(matches!(result, Err(Error::OriginalFamilyRequired)));
            } else {
                assert!(result.is_err());
            }
        }
        let mut graph = serde_json::json!({"version":1,"meaning":"represented_values","limits":Limits::default(),"source_witness":"none"});
        assert!(serde_json::from_value::<StoredOriginal>(graph.clone()).is_ok());
        graph["unknown_source"] = serde_json::json!(true);
        assert!(serde_json::from_value::<StoredOriginal>(graph).is_err());
    }
    #[test]
    fn graph_payload_wire_is_exclusive_and_legacy_original_metadata_is_unchanged() {
        #[derive(Serialize, Deserialize)]
        #[serde(deny_unknown_fields)]
        struct LegacyOriginal {
            version: u32,
            meaning: crate::threshold::represented::NumericalMeaning,
            limits: crate::threshold::represented::Limits,
            source_witness: String,
        }
        let original = StoredOriginal {
            version: 1,
            meaning: crate::threshold::represented::NumericalMeaning::RepresentedValues,
            limits: Default::default(),
            source_witness: "original".into(),
            graph_payloads: None,
        };
        let bytes = serde_json::to_vec(&original).unwrap();
        let legacy: LegacyOriginal = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&legacy).unwrap());
        let policy = crate::threshold::represented::graph::GraphPayloadPolicy::PrimitiveFragmentsV1;
        let new = StoredOriginal {
            version: 2,
            graph_payloads: Some(policy),
            ..original
        };
        assert!(
            serde_json::from_slice::<LegacyOriginal>(&serde_json::to_vec(&new).unwrap()).is_err()
        );
        assert!(
            serde_json::from_str::<crate::threshold::represented::graph::GraphPayloadPolicy>(
                r#"{"primitive_fragments_v1":{"ignored":true}}"#
            )
            .is_err()
        );
    }
}
