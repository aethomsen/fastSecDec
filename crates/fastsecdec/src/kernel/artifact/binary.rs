//! Native context-aware binserde caches. JSON presentation has no program bytes.
use super::{native, validate_orders};
use crate::contour::functions::dynamic::requests::{ExactRequest, merge_exact_requests};
use crate::{
    kernel::{
        CompilationSettings, KernelError, KernelLoadOptions, KernelSet, PrecisionPolicy,
        RuntimeMassConstraint,
        cancellation::Cancellation,
        metadata::{LegacyMetadata, PortableMetadata},
        program::{self, SectorProgram},
    },
    status::CoefficientComponent,
};
use bincode::{Decode, Encode};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    state::{State, StateMap},
};
mod parameters;
#[cfg(test)]
mod tests;

pub(super) const PREFIX: &[u8] = b"FastSecDec\0binserde";
pub(super) const MAGIC: &[u8] = b"FastSecDec\0binserde\x09";
const MAGIC_V10: &[u8] = b"FastSecDec\0binserde\x0a";
const MAGIC_V11: &[u8] = b"FastSecDec\0binserde\x0b";
const MAGIC_V12: &[u8] = b"FastSecDec\0binserde\x0c";
const MAGIC_V15: &[u8] = b"FastSecDec\0binserde\x0f";
const MAX_ROOT_RECORD_BYTES: usize = 256 * 1024 * 1024;
const MAGIC_V18: &[u8] = b"FastSecDec\0binserde\x12";
const MAGIC_V17: &[u8] = b"FastSecDec\0binserde\x11";
const MAGIC_V16: &[u8] = b"FastSecDec\0binserde\x10";
const MAGIC_V14: &[u8] = b"FastSecDec\0binserde\x0e";
const MAGIC_V8: &[u8] = b"FastSecDec\0binserde\x08";
const MAGIC_V7: &[u8] = b"FastSecDec\0binserde\x07";
const MAGIC_V6: &[u8] = b"FastSecDec\0binserde\x06";
const MAGIC_V5: &[u8] = b"FastSecDec\0binserde\x05";
const CODEC: &str = "symbolica-3:context-binserde-atoms-v1:serde-binserde-evaluators-v1";
#[derive(Encode, Decode)]
struct Envelope {
    content_id: String,
    digest: [u8; 32],
    state: Vec<u8>,
    payload: Vec<u8>,
}
/// Borrow the large transport buffers; native context decoding below owns only
/// the actual decoded Atoms and evaluator programs, not another payload copy.
#[derive(bincode::BorrowDecode)]
struct EnvelopeRef<'a> {
    content_id: &'a str,
    digest: [u8; 32],
    state: &'a [u8],
    payload: &'a [u8],
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct Payload {
    codec: String,
    compiler_policy: String,
    orders: Vec<i32>,
    #[bincode(with_serde)]
    components: Vec<CoefficientComponent>,
    exact: Vec<Atom>,
    runtime_parameters: Vec<Symbol>,
    runtime_mass_constraints: Vec<MassConstraint>,
    #[bincode(with_serde)]
    precision: PrecisionPolicy,
    sectors: Vec<Sector>,
    metadata: Option<PortableMetadata>,
    contour_checks: Vec<crate::kernel::contour::CheckProgram>,
}
/// V9 remains an exact nested layout; only explicit recipes use this wrapper.
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV10 {
    base: Payload,
    #[bincode(with_serde)]
    descriptor: crate::kernel::recipe::SavedProgramDescriptor,
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV11 {
    base: Payload,
    #[bincode(with_serde)]
    descriptor: crate::kernel::recipe::SavedProgramDescriptorV2,
    exact_requests: Vec<ExactRequest>,
}

/// Preserve the old nested metadata layout. Native coefficient definitions
/// are attached by local retained chart index before any semantic admission.
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV12 {
    base: Payload,
    descriptor: Option<SavedDescriptor>,
    exact_requests: Vec<ExactRequest>,
    contour_definitions: Vec<(usize, crate::contour::ContourDefinitions)>,
}

/// A proper source subset retains the unchanged native payload and definitions.
/// Scope is semantic metadata; optional v13 primary caches can wrap this record.
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV14 {
    base: PayloadV12,
    scope: crate::generation::GenerationSourceScope,
}

#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV15 {
    base: PayloadV12,
    threshold: crate::kernel::threshold_owner::Saved,
}

#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV16 {
    base: PayloadV12,
    threshold: crate::kernel::threshold_owner::SavedV16,
}

/// Selected-root arithmetic transport, with an explicit validated-policy capability.
/// Helper IR and branch association are trusted execution, not proof import.
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV17 {
    base: PayloadV12,
    threshold: Option<SavedThreshold>,
    roots: Vec<(Symbol, Vec<u8>)>,
    validated_policy_version: u32,
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
enum SavedThreshold {
    Global(crate::kernel::threshold_owner::Saved),
    Local(crate::kernel::threshold_owner::SavedV16),
}
#[derive(Encode, Decode)]
enum SavedDescriptor {
    V10(#[bincode(with_serde)] crate::kernel::recipe::SavedProgramDescriptor),
    V11(#[bincode(with_serde)] crate::kernel::recipe::SavedProgramDescriptorV2),
}
impl SavedDescriptor {
    fn from_native(value: &crate::kernel::NativeProgramDescriptor) -> Result<Self, KernelError> {
        if value.certificates().is_some() {
            Ok(Self::V11(
                crate::kernel::recipe::SavedProgramDescriptorV2::from_native(value)?,
            ))
        } else {
            Ok(Self::V10(
                crate::kernel::recipe::SavedProgramDescriptor::from_native(value),
            ))
        }
    }
    fn identity(
        &self,
        payload: &Payload,
        exact_requests: &[ExactRequest],
    ) -> Result<String, KernelError> {
        match self {
            Self::V10(descriptor) => {
                if !exact_requests.is_empty() {
                    return Err(failure(
                        "legacy descriptor cannot store exact root associations",
                    ));
                }
                semantic_id_v10(payload, descriptor)
            }
            Self::V11(descriptor) => {
                let mut hash = blake3::Hasher::new();
                hash.update(b"fastsecdec-native-semantic-v11\0");
                hash.update(semantic_id(payload, 9)?.as_bytes());
                serde_json::to_writer(&mut hash, descriptor)?;
                for request in exact_requests {
                    serde_json::to_writer(
                        &mut hash,
                        &(request.root.to_canonical_string(), &request.bundle),
                    )?;
                }
                Ok(hash.finalize().to_hex().to_string())
            }
        }
    }
    fn restore(self) -> Result<crate::kernel::NativeProgramDescriptor, KernelError> {
        match self {
            Self::V10(value) => value.restore(),
            Self::V11(value) => value.restore(),
        }
    }
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV8 {
    codec: String,
    compiler_policy: String,
    orders: Vec<i32>,
    #[bincode(with_serde)]
    components: Vec<CoefficientComponent>,
    exact: Vec<Atom>,
    runtime_parameters: Vec<Symbol>,
    runtime_mass_constraints: Vec<MassConstraint>,
    #[bincode(with_serde)]
    precision: PrecisionPolicy,
    sectors: Vec<Sector>,
    metadata: Option<LegacyMetadata>,
}
impl From<PayloadV8> for Payload {
    fn from(value: PayloadV8) -> Self {
        Self {
            codec: value.codec,
            compiler_policy: value.compiler_policy,
            orders: value.orders,
            components: value.components,
            exact: value.exact,
            runtime_parameters: value.runtime_parameters,
            runtime_mass_constraints: value.runtime_mass_constraints,
            precision: value.precision,
            sectors: value.sectors,
            metadata: value.metadata.map(Into::into),
            contour_checks: Vec::new(),
        }
    }
}
#[cfg(test)]
impl From<Payload> for PayloadV8 {
    fn from(value: Payload) -> Self {
        assert!(value.contour_checks.is_empty());
        Self {
            codec: value.codec,
            compiler_policy: value.compiler_policy,
            orders: value.orders,
            components: value.components,
            exact: value.exact,
            runtime_parameters: value.runtime_parameters,
            runtime_mass_constraints: value.runtime_mass_constraints,
            precision: value.precision,
            sectors: value.sectors,
            metadata: value.metadata.map(Into::into),
        }
    }
}
#[derive(Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV6 {
    codec: String,
    compiler_policy: String,
    orders: Vec<i32>,
    #[bincode(with_serde)]
    components: Vec<CoefficientComponent>,
    exact: Vec<Atom>,
    runtime_parameters: Vec<Symbol>,
    runtime_mass_constraints: Vec<MassConstraint>,
    #[bincode(with_serde)]
    precision: PrecisionPolicy,
    sectors: Vec<LegacySector>,
    metadata: Option<LegacyMetadata>,
}
impl From<PayloadV6> for Payload {
    fn from(value: PayloadV6) -> Self {
        Self {
            codec: value.codec,
            compiler_policy: value.compiler_policy,
            orders: value.orders,
            components: value.components,
            exact: value.exact,
            runtime_parameters: value.runtime_parameters,
            runtime_mass_constraints: value.runtime_mass_constraints,
            precision: value.precision,
            sectors: value.sectors.into_iter().map(Into::into).collect(),
            metadata: value.metadata.map(Into::into),
            contour_checks: Vec::new(),
        }
    }
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct MassConstraint {
    name: String,
    expression: Atom,
}
/// Version five predates runtime mass constraints. Keep its exact owned layout
/// for previously validated templates; no decoder guesses missing fields.
#[derive(Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV5 {
    codec: String,
    compiler_policy: String,
    orders: Vec<i32>,
    #[bincode(with_serde)]
    components: Vec<CoefficientComponent>,
    exact: Vec<Atom>,
    runtime_parameters: Vec<Symbol>,
    #[bincode(with_serde)]
    precision: PrecisionPolicy,
    sectors: Vec<LegacySector>,
    metadata: Option<LegacyMetadata>,
}
impl From<PayloadV5> for Payload {
    fn from(value: PayloadV5) -> Self {
        Self {
            codec: value.codec,
            compiler_policy: value.compiler_policy,
            orders: value.orders,
            components: value.components,
            exact: value.exact,
            runtime_parameters: value.runtime_parameters,
            runtime_mass_constraints: Vec::new(),
            precision: value.precision,
            sectors: value.sectors.into_iter().map(Into::into).collect(),
            metadata: value.metadata.map(Into::into),
            contour_checks: Vec::new(),
        }
    }
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct Sector {
    parameters: Vec<Symbol>,
    // Keep the established native serde/bincode codec for byte compatibility.
    // It originally avoided Numerica 3.0.1's Integer::Large sign-loss issue;
    // the current owner has fixed that issue. Atoms use native StateMap Decode.
    program: Vec<u8>,
    cancellation_degree: usize,
    cancellation_terms: Option<Vec<Vec<usize>>>,
    #[bincode(with_serde)]
    endpoint_profiles: Option<Vec<crate::generation::EndpointProfileRow>>,
}
#[derive(Decode)]
#[bincode(decode_context = "StateMap")]
struct LegacySector {
    parameters: Vec<Symbol>,
    // Keep the established native serde/bincode codec for byte compatibility.
    // It originally avoided Numerica 3.0.1's Integer::Large sign-loss issue;
    // the current owner has fixed that issue. Atoms use native StateMap Decode.
    program: Vec<u8>,
    cancellation_degree: usize,
    cancellation_terms: Option<Vec<Vec<usize>>>,
}
impl From<LegacySector> for Sector {
    fn from(value: LegacySector) -> Self {
        Self {
            parameters: value.parameters,
            program: value.program,
            cancellation_degree: value.cancellation_degree,
            cancellation_terms: value.cancellation_terms,
            endpoint_profiles: None,
        }
    }
}
fn failure(error: impl std::fmt::Display) -> KernelError {
    KernelError::Artifact(format!("native binserde cache: {error}"))
}
fn digest(magic: &[u8], state: &[u8], payload: &[u8]) -> blake3::Hash {
    let mut hash = blake3::Hasher::new();
    hash.update(magic);
    hash.update(&(state.len() as u64).to_le_bytes());
    hash.update(state);
    hash.update(payload);
    hash.finalize()
}
fn semantic_id(payload: &Payload, version: u8) -> Result<String, KernelError> {
    #[derive(serde::Serialize)]
    struct SemanticSector<'a> {
        parameters: Vec<String>,
        program: &'a [u8],
        cancellation_degree: usize,
        cancellation_terms: &'a Option<Vec<Vec<usize>>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        endpoint_profiles: &'a Option<Vec<crate::generation::EndpointProfileRow>>,
    }
    #[derive(serde::Serialize)]
    struct SemanticPayload<'a> {
        codec: &'a str,
        compiler_policy: &'a str,
        orders: &'a [i32],
        components: &'a [CoefficientComponent],
        exact: Vec<String>,
        runtime_parameters: Vec<String>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        runtime_mass_constraints: Vec<(&'a str, String)>,
        precision: &'a PrecisionPolicy,
        sectors: Vec<SemanticSector<'a>>,
        metadata: &'a Option<PortableMetadata>,
        #[serde(skip_serializing_if = "Vec::is_empty")]
        contour_checks: &'a Vec<crate::kernel::contour::CheckProgram>,
    }
    // Native evaluator serde uses portable symbol definitions, while raw Atom
    // binserde uses process-local IDs. Scientific identity must not include the
    // latter IDs or unrelated entries in Symbolica's shared context.
    let semantic = SemanticPayload {
        codec: &payload.codec,
        compiler_policy: &payload.compiler_policy,
        orders: &payload.orders,
        components: &payload.components,
        exact: payload
            .exact
            .iter()
            .map(AtomCore::to_canonical_string)
            .collect(),
        runtime_parameters: super::parameter_names(&payload.runtime_parameters),
        runtime_mass_constraints: payload
            .runtime_mass_constraints
            .iter()
            .map(|constraint| {
                (
                    constraint.name.as_str(),
                    constraint.expression.to_canonical_string(),
                )
            })
            .collect(),
        precision: &payload.precision,
        sectors: payload
            .sectors
            .iter()
            .map(|s| {
                Ok(SemanticSector {
                    parameters: super::parameter_names(&s.parameters),
                    program: &s.program,
                    cancellation_degree: s.cancellation_degree,
                    cancellation_terms: &s.cancellation_terms,
                    endpoint_profiles: &s.endpoint_profiles,
                })
            })
            .collect::<Result<_, KernelError>>()?,
        metadata: &payload.metadata,
        contour_checks: &payload.contour_checks,
    };
    let mut hash = blake3::Hasher::new();
    hash.update(match version {
        5 => b"fastsecdec-native-semantic-v5",
        6 => b"fastsecdec-native-semantic-v6",
        7 => b"fastsecdec-native-semantic-v7",
        8 => b"fastsecdec-native-semantic-v8",
        _ => b"fastsecdec-native-semantic-v9",
    });
    serde_json::to_writer(&mut hash, &semantic)?;
    Ok(hash.finalize().to_hex().to_string())
}
fn semantic_id_v10(
    payload: &Payload,
    descriptor: &crate::kernel::recipe::SavedProgramDescriptor,
) -> Result<String, KernelError> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"fastsecdec-native-semantic-v10\0");
    hash.update(semantic_id(payload, 9)?.as_bytes());
    serde_json::to_writer(&mut hash, descriptor)?;
    Ok(hash.finalize().to_hex().to_string())
}
#[cfg(test)]
fn encode(payload: Payload) -> Result<(String, Vec<u8>), KernelError> {
    encode_with_descriptor(payload, None)
}
#[cfg(test)]
fn encode_with_descriptor(
    payload: Payload,
    descriptor: Option<SavedDescriptor>,
) -> Result<(String, Vec<u8>), KernelError> {
    encode_with_requests(payload, descriptor, Vec::new())
}
fn encode_with_requests(
    payload: Payload,
    descriptor: Option<SavedDescriptor>,
    exact_requests: Vec<ExactRequest>,
) -> Result<(String, Vec<u8>), KernelError> {
    encode_with_threshold(
        payload,
        descriptor,
        exact_requests,
        None,
        true,
        &crate::kernel::algebraic::Scope::default(),
    )
}
fn encode_with_threshold(
    mut payload: Payload,
    descriptor: Option<SavedDescriptor>,
    exact_requests: Vec<ExactRequest>,
    threshold: Option<&crate::kernel::ThresholdMetadata>,
    inline_threshold_parent: bool,
    roots: &crate::kernel::algebraic::Scope,
) -> Result<(String, Vec<u8>), KernelError> {
    let source_scope = payload
        .metadata
        .as_mut()
        .and_then(PortableMetadata::take_source_scope);
    if let Some(scope) = &source_scope {
        scope
            .validate(payload.metadata.as_ref().unwrap().charts_len())
            .map_err(failure)?;
    }
    let mut content_id = match &descriptor {
        Some(descriptor) => descriptor.identity(&payload, &exact_requests)?,
        None => {
            if !exact_requests.is_empty() {
                return Err(failure(
                    "exact root associations require a native v11 descriptor",
                ));
            }
            semantic_id(&payload, 9)?
        }
    };
    let mut symbols = Atom::Zero.get_all_symbols(true);
    let mut collect = |atom: &Atom| {
        symbols.extend(atom.get_all_symbols(true));
    };
    for atom in &payload.exact {
        collect(atom);
    }
    for request in &exact_requests {
        collect(&request.root);
    }
    for constraint in &payload.runtime_mass_constraints {
        collect(&constraint.expression);
    }
    if let Some(metadata) = &payload.metadata {
        metadata.visit_atoms(&mut collect);
    }
    symbols.extend(payload.runtime_parameters.iter().copied());
    symbols.extend(
        payload
            .sectors
            .iter()
            .flat_map(|s| s.parameters.iter().copied()),
    );
    if let Some(threshold) = threshold {
        threshold.visit_atoms(&mut |a| {
            symbols.extend(a.get_all_symbols(true));
        });
    }
    symbols.extend(roots.owners().map(|(tag, _)| tag));
    let mut state = Vec::new();
    State::export_partial(&mut state, symbols).map_err(failure)?;
    let contour_definitions = payload
        .metadata
        .as_mut()
        .map_or_else(Vec::new, PortableMetadata::take_contour_definitions);
    let (magic, payload) = if let Some(owner) =
        threshold.filter(|o| o.lineage().preparation.runtime_chamber.is_some())
    {
        if source_scope.is_some() {
            return Err(failure("uniform template has legacy source scope"));
        }
        content_id = owner.semantic_identity(&payload.compiler_policy, &payload.precision)?;
        let base = PayloadV12 {
            base: payload,
            descriptor,
            exact_requests,
            contour_definitions,
        };
        (
            MAGIC_V18,
            parameters::encode(base, owner, roots, inline_threshold_parent)?,
        )
    } else if !roots.is_empty() {
        if source_scope.is_some()
            || payload.metadata.is_some()
            || descriptor.is_some()
            || !exact_requests.is_empty()
            || !contour_definitions.is_empty()
        {
            return Err(failure(
                "algebraic root format requires threshold or plain native vectors",
            ));
        }
        let threshold = threshold
            .map(|owner| -> Result<SavedThreshold, KernelError> {
                owner.validate_algebraic_helpers(roots)?;
                owner.validate_layout(
                    &payload
                        .sectors
                        .iter()
                        .map(|s| s.parameters.clone())
                        .collect::<Vec<_>>(),
                    &payload.orders,
                    &payload.runtime_parameters,
                )?;
                content_id =
                    owner.semantic_identity(&payload.compiler_policy, &payload.precision)?;
                if owner.is_local() {
                    Ok(SavedThreshold::Local(
                        owner.save_v16_with_parent(inline_threshold_parent)?,
                    ))
                } else {
                    Ok(SavedThreshold::Global(owner.save()?))
                }
            })
            .transpose()?;
        let base = PayloadV12 {
            base: payload,
            descriptor,
            exact_requests,
            contour_definitions,
        };
        (
            MAGIC_V17,
            bincode::encode_to_vec(
                PayloadV17 {
                    base,
                    threshold,
                    roots: roots.saved(),
                    validated_policy_version: 1,
                },
                bincode::config::standard(),
            )
            .map_err(failure)?,
        )
    } else if let Some(threshold) = threshold {
        if source_scope.is_some()
            || payload.metadata.is_some()
            || descriptor.is_some()
            || !exact_requests.is_empty()
            || !contour_definitions.is_empty()
        {
            return Err(failure("mixed threshold and legacy metadata"));
        }
        threshold.validate_layout(
            &payload
                .sectors
                .iter()
                .map(|s| s.parameters.clone())
                .collect::<Vec<_>>(),
            &payload.orders,
            &payload.runtime_parameters,
        )?;
        content_id = threshold.semantic_identity(&payload.compiler_policy, &payload.precision)?;
        let base = PayloadV12 {
            base: payload,
            descriptor,
            exact_requests,
            contour_definitions,
        };
        if threshold.is_local() {
            (
                MAGIC_V16,
                bincode::encode_to_vec(
                    PayloadV16 {
                        base,
                        threshold: threshold.save_v16_with_parent(inline_threshold_parent)?,
                    },
                    bincode::config::standard(),
                )
                .map_err(failure)?,
            )
        } else {
            (
                MAGIC_V15,
                bincode::encode_to_vec(
                    PayloadV15 {
                        base,
                        threshold: threshold.save()?,
                    },
                    bincode::config::standard(),
                )
                .map_err(failure)?,
            )
        }
    } else if let Some(scope) = source_scope {
        content_id = semantic_id_v14(&content_id, &scope)?;
        (
            MAGIC_V14,
            bincode::encode_to_vec(
                PayloadV14 {
                    base: PayloadV12 {
                        base: payload,
                        descriptor,
                        exact_requests,
                        contour_definitions,
                    },
                    scope,
                },
                bincode::config::standard(),
            )
            .map_err(failure)?,
        )
    } else if !contour_definitions.is_empty() {
        content_id = semantic_id_v12(&content_id);
        (
            MAGIC_V12,
            bincode::encode_to_vec(
                PayloadV12 {
                    base: payload,
                    descriptor,
                    exact_requests,
                    contour_definitions,
                },
                bincode::config::standard(),
            )
            .map_err(failure)?,
        )
    } else {
        match descriptor {
            Some(SavedDescriptor::V10(descriptor)) => (
                MAGIC_V10,
                bincode::encode_to_vec(
                    PayloadV10 {
                        base: payload,
                        descriptor,
                    },
                    bincode::config::standard(),
                )
                .map_err(failure)?,
            ),
            Some(SavedDescriptor::V11(descriptor)) => (
                MAGIC_V11,
                bincode::encode_to_vec(
                    PayloadV11 {
                        base: payload,
                        descriptor,
                        exact_requests,
                    },
                    bincode::config::standard(),
                )
                .map_err(failure)?,
            ),
            None => (
                MAGIC,
                bincode::encode_to_vec(payload, bincode::config::standard()).map_err(failure)?,
            ),
        }
    };
    let digest = digest(magic, &state, &payload);
    let envelope = Envelope {
        content_id: content_id.clone(),
        digest: *digest.as_bytes(),
        state,
        payload,
    };
    let mut bytes = magic.to_vec();
    bytes.extend(bincode::encode_to_vec(envelope, bincode::config::standard()).map_err(failure)?);
    Ok((content_id, bytes))
}

fn semantic_id_v12(base_identity: &str) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(b"fastsecdec-native-semantic-v12\0");
    hash.update(base_identity.as_bytes());
    hash.finalize().to_hex().to_string()
}
fn semantic_id_v14(
    base_identity: &str,
    scope: &crate::generation::GenerationSourceScope,
) -> Result<String, KernelError> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"fastsecdec-native-semantic-v14\0");
    hash.update(base_identity.as_bytes());
    serde_json::to_writer(&mut hash, scope)?;
    Ok(hash.finalize().to_hex().to_string())
}
pub(super) fn compiled(kernels: &KernelSet) -> Result<(String, Vec<u8>), KernelError> {
    compiled_with_parent(kernels, true)
}
pub(super) fn compiled_with_parent(
    kernels: &KernelSet,
    inline_threshold_parent: bool,
) -> Result<(String, Vec<u8>), KernelError> {
    let (id, bytes) = encode_with_threshold(
        Payload {
            codec: CODEC.into(),
            compiler_policy: native::compiler_policy_with_settings(kernels.compilation_settings),
            orders: kernels.coefficient_orders.clone(),
            components: kernels.components.clone(),
            exact: kernels.exact_expressions.clone(),
            runtime_parameters: kernels.runtime_parameters.clone(),
            runtime_mass_constraints: kernels
                .runtime_mass_constraints
                .iter()
                .map(|constraint| MassConstraint {
                    name: constraint.name.clone(),
                    expression: constraint.expression.clone(),
                })
                .collect(),
            precision: kernels.precision.clone(),
            sectors: kernels
                .sectors
                .iter()
                .map(|sector| {
                    Ok(Sector {
                        parameters: sector.parameters.clone(),
                        program: sector.program_bytes.to_vec(),
                        cancellation_degree: sector.cancellation.degree(),
                        cancellation_terms: sector.cancellation.terms().map(<[Vec<usize>]>::to_vec),
                        endpoint_profiles: sector
                            .cancellation
                            .endpoint_profiles()
                            .map(<[_]>::to_vec),
                    })
                })
                .collect::<Result<_, KernelError>>()?,
            metadata: kernels.metadata.as_ref().map(PortableMetadata::from_native),
            contour_checks: kernels.contour_checks.clone(),
        },
        kernels
            .program_descriptor
            .as_ref()
            .map(SavedDescriptor::from_native)
            .transpose()?,
        kernels.exact_requests.clone(),
        kernels.threshold.as_deref(),
        inline_threshold_parent,
        &crate::kernel::algebraic::Scope::merged(
            kernels.sectors.iter().map(|sector| &sector.algebraic),
        )
        .map_err(failure)?,
    )?;
    let primaries = kernels
        .sectors
        .iter()
        .map(|sector| sector.saved_primary())
        .collect::<Result<Vec<_>, _>>()?;
    let bytes = super::cached::wrap(&id, bytes, primaries)?;
    Ok((id, bytes))
}

/// One independent archive record. The native evaluator codec is unchanged;
/// only the selected chart group's ordinals are localized for standalone load.
pub(super) fn partition(
    kernels: &KernelSet,
    index: Option<usize>,
) -> Result<(String, Vec<u8>, Vec<usize>), KernelError> {
    if kernels.threshold.is_some() {
        return Err(failure(
            "threshold indexed partition capability is not yet registered",
        ));
    }
    let sector = index
        .map(|index| {
            kernels
                .sectors
                .get(index)
                .ok_or_else(|| failure("unknown partition sector"))
        })
        .transpose()?;
    let mut source_indices = Vec::new();
    let metadata = kernels
        .metadata
        .as_ref()
        .map(|metadata| {
            let mut local = crate::generation::GenerationMetadata {
                source_scope: None,
                domain: metadata.domain.clone(),
                charts: metadata
                    .charts
                    .iter()
                    .filter(|chart| chart.kernel_sector == index)
                    .cloned()
                    .collect(),
            };
            source_indices = local
                .charts
                .iter()
                .map(|chart| chart.source_index)
                .collect();
            local.source_scope = metadata
                .source_scope
                .as_ref()
                .map(|scope| scope.for_charts(&source_indices))
                .transpose()
                .map_err(|error| failure(error.to_string()))?;
            let indices = source_indices
                .iter()
                .enumerate()
                .map(|(local, original)| (*original, local))
                .collect::<std::collections::BTreeMap<_, _>>();
            for (ordinal, chart) in local.charts.iter_mut().enumerate() {
                chart.source_index = ordinal;
                chart.representative = *indices
                    .get(&chart.representative)
                    .ok_or_else(|| failure("partition omits its chart representative"))?;
                chart.kernel_sector = index.map(|_| 0);
            }
            Ok::<_, KernelError>(
                (!local.charts.is_empty() || local.source_scope.is_some())
                    .then(|| PortableMetadata::from_native(&local)),
            )
        })
        .transpose()?
        .flatten();
    let (orders, components) = sector
        .and_then(|s| s.projection.as_ref())
        .map(|projection| {
            (
                projection.local_orders.clone(),
                projection.local_components.clone(),
            )
        })
        .unwrap_or_else(|| {
            (
                kernels.coefficient_orders.clone(),
                kernels.components.clone(),
            )
        });
    let payload = Payload {
        codec: CODEC.into(),
        compiler_policy: native::compiler_policy_with_settings(kernels.compilation_settings),
        exact: if index.is_none() {
            kernels.exact_expressions.clone()
        } else {
            vec![Atom::Zero; orders.len()]
        },
        orders,
        components,
        runtime_parameters: kernels.runtime_parameters.clone(),
        runtime_mass_constraints: kernels
            .runtime_mass_constraints
            .iter()
            .map(|constraint| MassConstraint {
                name: constraint.name.clone(),
                expression: constraint.expression.clone(),
            })
            .collect(),
        precision: kernels.precision.clone(),
        sectors: sector
            .into_iter()
            .map(|sector| Sector {
                parameters: sector.parameters.clone(),
                program: sector.program_bytes.to_vec(),
                cancellation_degree: sector.cancellation.degree(),
                cancellation_terms: sector.cancellation.terms().map(<[Vec<usize>]>::to_vec),
                endpoint_profiles: sector.cancellation.endpoint_profiles().map(<[_]>::to_vec),
            })
            .collect(),
        metadata,
        contour_checks: kernels
            .contour_checks
            .iter()
            .filter_map(|check| {
                source_indices
                    .iter()
                    .position(|index| *index == check.chart_index)
                    .map(|chart_index| crate::kernel::contour::CheckProgram {
                        chart_index,
                        program: check.program.clone(),
                        unsupported: check.unsupported.clone(),
                    })
            })
            .collect(),
    };
    let exact_requests =
        merge_exact_requests(&payload.exact, kernels.exact_requests.clone()).map_err(failure)?;
    let descriptor = kernels
        .program_descriptor
        .as_ref()
        .map(|descriptor| {
            descriptor.for_payload_with_requests(
                &source_indices,
                &payload.exact,
                kernels.metadata.as_ref(),
                &exact_requests,
            )
        })
        .transpose()?;
    let roots =
        crate::kernel::algebraic::Scope::merged(sector.into_iter().map(|sector| &sector.algebraic))
            .map_err(failure)?;
    let (id, bytes) = encode_with_threshold(
        payload,
        descriptor
            .as_ref()
            .map(SavedDescriptor::from_native)
            .transpose()?,
        exact_requests,
        None,
        true,
        &roots,
    )?;
    let primaries = sector
        .into_iter()
        .map(|sector| sector.saved_primary())
        .collect::<Result<Vec<_>, _>>()?;
    let bytes = super::cached::wrap(&id, bytes, primaries)?;
    Ok((id, bytes, source_indices))
}
pub(super) fn generated(
    value: &crate::generation::GeneratedIntegral,
    precision: PrecisionPolicy,
    settings: CompilationSettings,
) -> Result<Vec<u8>, KernelError> {
    precision.validate()?;
    let settings = settings.resolve_contour_jacobian(value.contour_jacobian())?;
    settings.validate()?;
    let runtime_parameters = crate::kernel::compilation::runtime_inputs(value, &[]);
    crate::kernel::compilation::validate_descriptor(value, &runtime_parameters)?;
    let _preparing = crate::kernel::NativeProgramDescriptor::enter_optional(
        value.program_descriptor().map(std::sync::Arc::as_ref),
    );
    let dynamic =
        crate::kernel::compilation::PreparedDynamic::build(value, &runtime_parameters, settings)?;
    let contour_checks = if dynamic.is_some() {
        Vec::new()
    } else {
        crate::kernel::contour::build_checks(value.metadata(), &runtime_parameters, settings)?
    };
    let complex = crate::kernel::compilation::requires_complex(value, &runtime_parameters);
    let sectors = value
        .sectors()
        .iter()
        .map(|sector| {
            let program = program::build_sector_with_lowering(
                sector,
                &runtime_parameters,
                settings,
                dynamic.as_ref().map(|value| value.lookup.as_ref()),
            )?;
            Ok(Sector {
                parameters: program.parameters,
                program: program::encode(&program.exact)?,
                cancellation_degree: program.cancellation.degree(),
                cancellation_terms: program.cancellation.terms().map(<[Vec<usize>]>::to_vec),
                endpoint_profiles: program.cancellation.endpoint_profiles().map(<[_]>::to_vec),
            })
        })
        .collect::<Result<_, KernelError>>()?;
    Ok(encode_with_requests(
        Payload {
            codec: CODEC.into(),
            compiler_policy: native::compiler_policy_with_settings(settings),
            orders: value.orders().to_vec(),
            components: native::component_layout(value.orders().len(), complex),
            exact: value.exact_coefficients().to_vec(),
            runtime_parameters,
            runtime_mass_constraints: Vec::new(),
            precision,
            sectors,
            metadata: Some(PortableMetadata::from_native(value.metadata())),
            contour_checks,
        },
        dynamic
            .as_ref()
            .map(|value| value.descriptor.as_ref())
            .or_else(|| value.program_descriptor().map(std::sync::Arc::as_ref))
            .map(SavedDescriptor::from_native)
            .transpose()?,
        dynamic
            .map(|value| value.exact_requests)
            .unwrap_or_default(),
    )?
    .1)
}
#[cfg(test)]
fn load(bytes: &[u8]) -> Result<KernelSet, KernelError> {
    KernelSet::from_bytes(bytes)
}

pub(super) fn load_with_progress(
    bytes: &[u8],
    options: KernelLoadOptions,
    progress: &mut impl FnMut(&crate::kernel::CompilationProgress) -> std::ops::ControlFlow<()>,
) -> Result<KernelSet, KernelError> {
    load_with_primary(bytes, options, None, None, true, progress)
}

pub(super) fn load_with_primary(
    bytes: &[u8],
    options: KernelLoadOptions,
    primary_caches: Option<Vec<Option<crate::kernel::evaluator::SavedPrimary>>>,
    cache_content_id: Option<&str>,
    retain: bool,
    progress: &mut impl FnMut(&crate::kernel::CompilationProgress) -> std::ops::ControlFlow<()>,
) -> Result<KernelSet, KernelError> {
    load_with_primary_and_parent(
        bytes,
        options,
        primary_caches,
        cache_content_id,
        retain,
        progress,
        None,
    )
}
#[allow(clippy::too_many_arguments)]
pub(super) fn load_with_primary_and_parent(
    bytes: &[u8],
    options: KernelLoadOptions,
    primary_caches: Option<Vec<Option<crate::kernel::evaluator::SavedPrimary>>>,
    cache_content_id: Option<&str>,
    retain: bool,
    progress: &mut impl FnMut(&crate::kernel::CompilationProgress) -> std::ops::ControlFlow<()>,
    threshold_parent: Option<&crate::kernel::ThresholdMetadata>,
) -> Result<KernelSet, KernelError> {
    let (wire, magic, version) = if let Some(wire) = bytes.strip_prefix(MAGIC_V18) {
        (wire, MAGIC_V18, 18)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V17) {
        (wire, MAGIC_V17, 17)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V16) {
        (wire, MAGIC_V16, 16)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V15) {
        (wire, MAGIC_V15, 15)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V14) {
        (wire, MAGIC_V14, 14)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V12) {
        (wire, MAGIC_V12, 12)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V11) {
        (wire, MAGIC_V11, 11)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V10) {
        (wire, MAGIC_V10, 10)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC) {
        (wire, MAGIC, 9)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V8) {
        (wire, MAGIC_V8, 8)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V7) {
        (wire, MAGIC_V7, 7)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V6) {
        (wire, MAGIC_V6, 6)
    } else if let Some(wire) = bytes.strip_prefix(MAGIC_V5) {
        (wire, MAGIC_V5, 5)
    } else {
        return Err(failure("unsupported header"));
    };
    let (envelope, used): (EnvelopeRef<'_>, usize) =
        bincode::borrow_decode_from_slice(wire, bincode::config::standard()).map_err(failure)?;
    if used != wire.len() {
        return Err(failure("trailing envelope bytes"));
    }
    super::validate_content_id(envelope.content_id)?;
    if cache_content_id.is_some_and(|id| id != envelope.content_id) {
        return Err(failure(
            "native primary cache belongs to a different mathematical record",
        ));
    }
    if options.validate
        && digest(magic, envelope.state, envelope.payload).as_bytes() != &envelope.digest
    {
        return Err(failure("content identity mismatch"));
    }
    if version >= 17
        && (envelope.state.len() > MAX_ROOT_RECORD_BYTES
            || envelope.payload.len() > MAX_ROOT_RECORD_BYTES)
    {
        return Err(failure(
            "algebraic native record exceeds transport byte limit",
        ));
    }
    let _ = symbolica::transcendental::gamma();
    crate::contour::functions::register();
    if version >= 10 {
        crate::contour::functions::dynamic::register();
    }
    if version >= 17 {
        crate::kernel::algebraic::register();
    }
    let mut state_source = envelope.state;
    let context = State::import(&mut state_source, None).map_err(failure)?;
    if !state_source.is_empty() {
        return Err(failure("trailing symbol context bytes"));
    }
    let mut saved_descriptor = None;
    let mut exact_requests = Vec::new();
    let mut source_scope = None;
    let mut threshold = None;
    let mut roots = crate::kernel::algebraic::Scope::default();
    let (mut payload, used): (Payload, usize) = if version == 18 {
        let (payload, owner, restored_roots, used) =
            parameters::decode(envelope.payload, context, threshold_parent)?;
        threshold = Some(owner);
        roots = restored_roots;
        (payload, used)
    } else if version == 17 {
        let (payload, used): (PayloadV17, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard().with_limit::<MAX_ROOT_RECORD_BYTES>(),
            context,
        )
        .map_err(failure)?;
        if payload.validated_policy_version != 1
            || payload.roots.is_empty()
            || payload.base.base.metadata.is_some()
            || payload.base.descriptor.is_some()
            || !payload.base.exact_requests.is_empty()
            || !payload.base.contour_definitions.is_empty()
        {
            return Err(failure("invalid selected-root transport schema"));
        }
        roots = crate::kernel::algebraic::Scope::restore(payload.roots).map_err(failure)?;
        threshold = payload
            .threshold
            .map(|owner| match owner {
                SavedThreshold::Global(saved) => crate::kernel::ThresholdMetadata::restore(saved),
                SavedThreshold::Local(saved) => {
                    crate::kernel::ThresholdMetadata::restore_v16_with_parent(
                        saved,
                        threshold_parent,
                    )
                }
            })
            .transpose()?;
        if let Some(owner) = &threshold {
            owner.validate_algebraic_helpers(&roots)?;
            owner.validate_layout(
                &payload
                    .base
                    .base
                    .sectors
                    .iter()
                    .map(|s| s.parameters.clone())
                    .collect::<Vec<_>>(),
                &payload.base.base.orders,
                &payload.base.base.runtime_parameters,
            )?;
        }
        (payload.base.base, used)
    } else if version == 15 || version == 16 {
        let (payload, owner, used) = if version == 15 {
            let (payload, used): (PayloadV15, usize) = bincode::decode_from_slice_with_context(
                envelope.payload,
                bincode::config::standard(),
                context,
            )
            .map_err(failure)?;
            (
                payload.base,
                crate::kernel::ThresholdMetadata::restore(payload.threshold)?,
                used,
            )
        } else {
            let (payload, used): (PayloadV16, usize) = bincode::decode_from_slice_with_context(
                envelope.payload,
                bincode::config::standard(),
                context,
            )
            .map_err(failure)?;
            (
                payload.base,
                crate::kernel::ThresholdMetadata::restore_v16_with_parent(
                    payload.threshold,
                    threshold_parent,
                )?,
                used,
            )
        };
        if payload.base.metadata.is_some()
            || payload.descriptor.is_some()
            || !payload.exact_requests.is_empty()
            || !payload.contour_definitions.is_empty()
        {
            return Err(failure("mixed threshold and legacy payload"));
        }
        owner.validate_layout(
            &payload
                .base
                .sectors
                .iter()
                .map(|s| s.parameters.clone())
                .collect::<Vec<_>>(),
            &payload.base.orders,
            &payload.base.runtime_parameters,
        )?;
        threshold = Some(owner);
        (payload.base, used)
    } else if version == 14 {
        let (mut payload, used): (PayloadV14, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        let metadata = payload
            .base
            .base
            .metadata
            .as_mut()
            .ok_or_else(|| failure("source subset lacks retained native metadata"))?;
        payload
            .scope
            .validate(metadata.charts_len())
            .map_err(failure)?;
        metadata.attach_contour_definitions(payload.base.contour_definitions)?;
        source_scope = Some(payload.scope);
        saved_descriptor = payload.base.descriptor;
        exact_requests = payload.base.exact_requests;
        (payload.base.base, used)
    } else if version == 12 {
        let (mut payload, used): (PayloadV12, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        if payload.contour_definitions.is_empty() {
            return Err(failure("v12 artifact has no compact contour definitions"));
        }
        payload
            .base
            .metadata
            .as_mut()
            .ok_or_else(|| failure("compact contour definitions lack retained metadata"))?
            .attach_contour_definitions(payload.contour_definitions)?;
        exact_requests = payload.exact_requests;
        saved_descriptor = payload.descriptor;
        (payload.base, used)
    } else if version == 11 {
        let (payload, used): (PayloadV11, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        exact_requests = payload.exact_requests;
        saved_descriptor = Some(SavedDescriptor::V11(payload.descriptor));
        (payload.base, used)
    } else if version == 10 {
        let (payload, used): (PayloadV10, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        saved_descriptor = Some(SavedDescriptor::V10(payload.descriptor));
        (payload.base, used)
    } else if version == 5 {
        let (payload, used): (PayloadV5, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        (payload.into(), used)
    } else if version == 6 {
        let (payload, used): (PayloadV6, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        (payload.into(), used)
    } else if version <= 8 {
        let (payload, used): (PayloadV8, usize) = bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?;
        (payload.into(), used)
    } else {
        bincode::decode_from_slice_with_context(
            envelope.payload,
            bincode::config::standard(),
            context,
        )
        .map_err(failure)?
    };
    if used != envelope.payload.len() {
        return Err(failure("trailing payload bytes"));
    }
    let actual_id = if options.validate {
        let identity = match &saved_descriptor {
            Some(descriptor) => descriptor.identity(&payload, &exact_requests)?,
            None => semantic_id(&payload, if version >= 14 { 9 } else { version })?,
        };
        Some(if let Some(threshold) = &threshold {
            threshold.semantic_identity(&payload.compiler_policy, &payload.precision)?
        } else if let Some(scope) = &source_scope {
            semantic_id_v14(&identity, scope)?
        } else if version == 12 {
            semantic_id_v12(&identity)
        } else {
            identity
        })
    } else {
        None
    };
    if actual_id.is_some_and(|id| id != envelope.content_id) {
        return Err(failure("semantic content identity mismatch"));
    }
    if let Some(scope) = source_scope {
        payload
            .metadata
            .as_mut()
            .unwrap()
            .attach_source_scope(scope)?;
    }
    if version >= 11 {
        let canonical =
            merge_exact_requests(&payload.exact, exact_requests.clone()).map_err(failure)?;
        let mut seen = std::collections::BTreeSet::new();
        if canonical.len() != exact_requests.len()
            || exact_requests.iter().any(|request| {
                !seen.insert(&request.root)
                    || !canonical.iter().any(|expected| {
                        expected.root == request.root && expected.bundle == request.bundle
                    })
            })
        {
            return Err(failure("duplicate or surplus exact root associations"));
        }
    }
    // Retain immutable helper owners before any numerical callback can be
    // constructed. Restoration never reruns symbolic evaluator optimization.
    let descriptor = saved_descriptor
        .map(|descriptor| descriptor.restore())
        .transpose()?;
    if let Some(descriptor) = &descriptor {
        if version == 10 && descriptor.recipe().is_dynamic() {
            return Err(failure(
                "dynamic v10 artifacts lack saved certificate and exact-root associations; regenerate with the current compiler",
            ));
        }
        descriptor.recipe().validate_runtime_schema(
            &payload
                .runtime_parameters
                .iter()
                .map(|symbol| symbol.get_name().to_owned())
                .collect::<Vec<_>>(),
        )?;
        descriptor.admit_runtime()?;
    }
    let _preparing = crate::kernel::NativeProgramDescriptor::enter_optional(descriptor.as_ref());
    let settings = native::settings_from_policy(&payload.compiler_policy);
    if payload.codec != CODEC || settings.is_none() {
        return Err(failure("unsupported codec or compiler policy"));
    }
    validate_orders(&payload.orders, payload.exact.len())?;
    payload.precision.validate()?;
    let use_complex = if payload.components == native::component_layout(payload.orders.len(), false)
    {
        false
    } else if payload.components == native::component_layout(payload.orders.len(), true) {
        true
    } else {
        return Err(failure("invalid component layout"));
    };
    let unique = payload
        .runtime_parameters
        .iter()
        .collect::<std::collections::BTreeSet<_>>();
    if unique.len() != payload.runtime_parameters.len() {
        return Err(failure("duplicate runtime parameter"));
    }
    let mut programs = Vec::with_capacity(payload.sectors.len());
    let mut encoded_programs = Vec::with_capacity(payload.sectors.len());
    for sector in payload.sectors {
        let coordinates = sector
            .parameters
            .iter()
            .collect::<std::collections::BTreeSet<_>>();
        if coordinates.is_empty()
            || coordinates.len() != sector.parameters.len()
            || coordinates.iter().any(|p| unique.contains(p))
        {
            return Err(failure("invalid sector coordinates"));
        }
        let program = program::decode(&sector.program)?;
        if version < 17
            && program::callbacks(&program)
                .map_err(failure)?
                .iter()
                .any(|c| crate::kernel::algebraic::is_root(c.symbol))
        {
            return Err(failure("selected root helpers require native format17"));
        }
        if version < 8 && !use_complex && program::legacy_real_branch(&program) {
            return Err(failure(
                "historical real layout has an unproved branch domain; regenerate with the current compiler",
            ));
        }
        if program.get_input_len() != sector.parameters.len() + payload.runtime_parameters.len()
            || program.get_output_len() != payload.orders.len()
        {
            return Err(failure("native program input/output layout mismatch"));
        }
        let mut cancellation = Cancellation::new(
            sector.cancellation_degree,
            sector.cancellation_terms,
            sector.parameters.len(),
        )?;
        if let Some(profiles) = sector.endpoint_profiles {
            cancellation = cancellation.with_endpoint_profiles(profiles)?;
        }
        programs.push(SectorProgram {
            symbolic_endpoint_contour_partials: None,
            parameters: sector.parameters,
            runtime_parameters: payload.runtime_parameters.clone(),
            exact_zero: native::literal_zero::outputs(&program),
            exact: program,
            cancellation,
            real_coefficients: vec![false; payload.orders.len()],
        });
        encoded_programs.push(std::sync::Arc::<[u8]>::from(sector.program));
    }
    let coordinates = programs
        .iter()
        .map(|p| p.parameters.clone())
        .collect::<Vec<_>>();
    let metadata = payload
        .metadata
        .map(|m| m.into_native(&coordinates, options.validate))
        .transpose()?;
    if let Some(descriptor) = &descriptor {
        // This compares the complete ordered native Symbol schema. Saved
        // get_name() labels and canonical Atom spellings can denote the same
        // symbol, so a second raw-string comparison would reject valid files.
        descriptor.validate_generation(metadata.as_ref(), &payload.runtime_parameters)?;
        for certificate in descriptor.certificates().into_iter().flatten() {
            if let Some(metadata) = &metadata {
                certificate.validate_metadata(metadata)?;
            } else if certificate.chart_index.is_some() {
                return Err(failure(
                    "projected certificate lacks its retained chart metadata",
                ));
            }
        }
    }
    if !use_complex
        && payload.exact.iter().any(|coefficient| {
            !program::is_real_expression(coefficient, &payload.runtime_parameters)
        })
    {
        return Err(failure("complex exact offset in real output layout"));
    }
    let mut kernels = roots.enter(53, || {
        KernelSet::from_programs_for_load_with_progress(
            payload.orders,
            programs,
            payload.exact,
            payload.precision,
            metadata,
            use_complex,
            payload.runtime_parameters,
            settings.expect("compiler policy validated"),
            Some(encoded_programs),
            primary_caches,
            options.validate,
            progress,
        )
    })?;
    if version >= 17
        && crate::kernel::algebraic::Scope::merged(kernels.sectors.iter().map(|s| &s.algebraic))
            .map_err(failure)?
            .saved()
            != roots.saved()
    {
        return Err(failure("unused or missing algebraic helper inventory"));
    }
    if let Some(threshold) = threshold {
        kernels.attach_threshold(threshold)?;
    }
    kernels.runtime_mass_constraints = payload
        .runtime_mass_constraints
        .into_iter()
        .map(|constraint| RuntimeMassConstraint {
            name: constraint.name,
            expression: constraint.expression,
        })
        .collect();
    kernels.contour_checks = payload.contour_checks;
    kernels.program_descriptor = descriptor;
    kernels.exact_requests =
        merge_exact_requests(&kernels.exact_expressions, exact_requests).map_err(failure)?;
    kernels.validate_runtime_mass_constraints()?;
    kernels.content_id = envelope.content_id.to_owned();
    if retain {
        kernels.portable_artifact = Some(bytes.to_vec().into());
    }
    Ok(kernels)
}
