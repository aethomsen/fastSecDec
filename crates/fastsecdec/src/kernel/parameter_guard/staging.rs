//! One shared guard record through the existing native Atom/State codec. The
//! descriptor is untrusted transport; a live family remains the issuer.
use super::*;
use crate::generation::streaming::{
    RecordRef, StreamingError,
    codec::{self, Atoms, invalid},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

const KIND: &str = "threshold-exact-parameter-guard-v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct StagedGuard {
    pub record: RecordRef,
    pub semantic: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transport {
    version: u32,
    parameters: usize,
    prefix_to_public: Vec<usize>,
    axes: Vec<AxisSlots>,
    positive: Vec<usize>,
    nonzero: Vec<usize>,
    program: Vec<u8>,
}
pub(crate) fn write(
    root: &Path,
    guard: &ExactParameterGuard,
) -> std::result::Result<StagedGuard, StreamingError> {
    let s = guard.stored();
    let dto = Transport {
        version: s.version,
        parameters: s.public_parameters.len(),
        prefix_to_public: s.prefix_to_public.clone(),
        axes: s.axes.clone(),
        positive: s.positive.clone(),
        nonzero: s.nonzero.clone(),
        program: s.program.clone(),
    };
    let record = codec::write(
        root,
        "threshold-parameter-guard",
        KIND,
        &dto,
        Atoms {
            atoms: s.expressions.clone(),
        },
        s.symbols().collect(),
    )?;
    Ok(StagedGuard {
        record,
        semantic: guard.semantic_identity().map_err(invalid)?,
    })
}
pub(crate) fn read(
    root: &Path,
    staged: &StagedGuard,
    expected: &str,
    maximum_bytes: u64,
) -> std::result::Result<Arc<ExactParameterGuard>, StreamingError> {
    if staged.semantic != expected || staged.record.bytes > maximum_bytes {
        return Err(invalid(
            "parameter guard semantic association or byte allowance",
        ));
    }
    // Record byte allowance precedes State/native import. Native allocations
    // inside decoding remain subject to the caller's process/resource limits.
    let (dto, atoms, symbols): (Transport, _, _) = codec::read(root, &staged.record, KIND)?;
    let limits = Limits::default();
    let count = dto
        .parameters
        .checked_mul(2)
        .ok_or_else(|| invalid("guard symbol count overflow"))?;
    if dto.parameters == 0 || dto.parameters > limits.variables || symbols.len() != count {
        return Err(invalid("staged guard native symbol layout"));
    }
    let stored = Stored {
        version: dto.version,
        public_parameters: symbols[..dto.parameters].to_vec(),
        raw_parameters: symbols[dto.parameters..].to_vec(),
        prefix_to_public: dto.prefix_to_public,
        axes: dto.axes,
        positive: dto.positive,
        nonzero: dto.nonzero,
        expressions: atoms.atoms,
        program: dto.program,
    };
    stored.validate(&limits).map_err(invalid)?;
    if stored.semantic_identity().map_err(invalid)? != expected {
        return Err(invalid("restored guard predicate identity"));
    }
    Ok(Arc::new(
        ExactParameterGuard::restore(stored, limits).map_err(invalid)?,
    ))
}
