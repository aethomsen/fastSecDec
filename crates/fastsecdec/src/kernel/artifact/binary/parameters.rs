//! Explicit v18 arithmetic capability. Existing v15/v16/v17 layouts remain
//! unchanged; parent references are meaningful only inside an admitted archive.
use super::*;
use crate::kernel::{
    ThresholdMetadata,
    algebraic::Scope,
    parameter_guard::{ExactParameterGuard, Limits, Stored},
};
use std::sync::Arc;

#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
struct PayloadV18 {
    base: PayloadV12,
    threshold: SavedThreshold,
    roots: Vec<(Symbol, Vec<u8>)>,
    validated_policy_version: u32,
    guard: SavedGuard,
}
#[derive(Encode, Decode)]
#[bincode(decode_context = "StateMap")]
enum SavedGuard {
    Inline(Stored),
    Parent { semantic: String },
}
fn admitted_base(base: &PayloadV12) -> Result<(), KernelError> {
    if base.base.runtime_parameters.is_empty()
        || base.base.metadata.is_some()
        || base.descriptor.is_some()
        || !base.exact_requests.is_empty()
        || !base.contour_definitions.is_empty()
        || !base.base.contour_checks.is_empty()
    {
        return Err(failure("invalid uniform template transport schema"));
    }
    Ok(())
}
fn admitted_owner(
    owner: &ThresholdMetadata,
    base: &PayloadV12,
    roots: &Scope,
) -> Result<(), KernelError> {
    owner.validate_layout(
        &base
            .base
            .sectors
            .iter()
            .map(|s| s.parameters.clone())
            .collect::<Vec<_>>(),
        &base.base.orders,
        &base.base.runtime_parameters,
    )?;
    owner.validate_algebraic_helpers(roots)
}
pub(super) fn encode(
    base: PayloadV12,
    owner: &ThresholdMetadata,
    roots: &Scope,
    inline: bool,
) -> Result<Vec<u8>, KernelError> {
    admitted_base(&base)?;
    admitted_owner(owner, &base, roots)?;
    let guard = owner
        .parameter_guard()
        .ok_or_else(|| failure("uniform template lacks exact guard"))?;
    let saved_guard = if inline {
        SavedGuard::Inline(guard.stored().clone())
    } else {
        SavedGuard::Parent {
            semantic: guard.semantic_identity().map_err(failure)?,
        }
    };
    let threshold = if owner.is_local() {
        SavedThreshold::Local(owner.save_v16_with_parent(inline)?)
    } else {
        if !inline {
            return Err(failure("flat uniform owner requires inline parent"));
        }
        SavedThreshold::Global(owner.save()?)
    };
    bincode::encode_to_vec(
        PayloadV18 {
            base,
            threshold,
            roots: roots.saved(),
            validated_policy_version: 1,
            guard: saved_guard,
        },
        bincode::config::standard(),
    )
    .map_err(failure)
}
pub(super) fn decode(
    bytes: &[u8],
    context: StateMap,
    parent: Option<&ThresholdMetadata>,
) -> Result<(Payload, ThresholdMetadata, Scope, usize), KernelError> {
    let (payload, used): (PayloadV18, usize) = bincode::decode_from_slice_with_context(
        bytes,
        bincode::config::standard().with_limit::<MAX_ROOT_RECORD_BYTES>(),
        context,
    )
    .map_err(failure)?;
    if payload.validated_policy_version != 1 {
        return Err(failure("uniform template validated policy version"));
    }
    admitted_base(&payload.base)?;
    let roots = Scope::restore(payload.roots).map_err(failure)?;
    let mut owner = match payload.threshold {
        SavedThreshold::Global(saved) => ThresholdMetadata::restore(saved)?,
        SavedThreshold::Local(saved) => ThresholdMetadata::restore_v16_with_parent(saved, parent)?,
    };
    let guard = match payload.guard {
        SavedGuard::Inline(stored) => {
            Arc::new(ExactParameterGuard::restore(stored, Limits::default()).map_err(failure)?)
        }
        SavedGuard::Parent { semantic } => {
            let parent =
                parent.ok_or_else(|| failure("guard reference needs native archive parent"))?;
            if owner.lineage() != parent.lineage() {
                return Err(failure("parameter guard parent transplant"));
            }
            let guard = parent
                .parameter_guard()
                .ok_or_else(|| failure("native archive parent lacks guard"))?;
            if guard.semantic_identity().map_err(failure)? != semantic {
                return Err(failure("parameter guard reference identity"));
            }
            guard.clone()
        }
    };
    owner.attach_parameter_guard(guard)?;
    admitted_owner(&owner, &payload.base, &roots)?;
    Ok((payload.base.base, owner, roots, used))
}

#[cfg(all(test, feature = "threshold-decomposition"))]
#[path = "parameters_tests.rs"]
mod tests;
