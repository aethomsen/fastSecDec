//! Durable, untrusted native GCAD preparation evidence for caller-owned workers.
//!
//! This is global preparation staging: independent verification still loads the
//! complete native proof. These records are not selective cell workers, a solver
//! checkpoint, or a complete integration artifact. Caller process limits remain
//! responsible for native solve/verification RSS and wall time.

mod request;
#[cfg(test)]
mod tests;

use super::{GcadError, GcadRequest, NativeDecomposition, VerifiedDecomposition};
use crate::generation::streaming::{
    RecordRef, StreamingError,
    codec::{self, Atoms, invalid},
};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use symgcad::output::{SolveResult, Status};

const EVIDENCE_KIND: &str = "threshold-gcad-evidence-v1";
const OBSERVATION_KIND: &str = "threshold-gcad-verification-observation-v1";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Record(#[from] StreamingError),
    #[error(transparent)]
    Gcad(#[from] GcadError),
    #[error("GCAD evidence record association mismatch")]
    Association,
    #[error("GCAD evidence records exceed the caller's transport-byte limit")]
    RecordLimit,
    #[error("pre-parametric graph replay requires its original native GraphPoint owner")]
    OriginalGraphRequired,
    #[error("pre-parametric family replay requires its original native FamilyPoint owner")]
    OriginalFamilyRequired,
}
impl From<crate::generation::GenerationError> for Error {
    fn from(error: crate::generation::GenerationError) -> Self {
        Self::Record(error.into())
    }
}
pub type Result<T> = std::result::Result<T, Error>;

/// Lightweight native request receipt. The canonical source identity describes
/// the ordered physical input; the record digest also binds domain/kinematics,
/// typed preparation and native solver options. Neither is a coverage proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestRecord {
    pub source_identity: String,
    pub record: RecordRef,
}
/// Raw native solve evidence, including incomplete and failed-status results.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceRecord {
    pub request: RequestRecord,
    pub record: RecordRef,
    pub status: Status,
    pub cells: usize,
}
/// Historical observation only. Restoring it never creates verified authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case", deny_unknown_fields)]
pub enum VerificationObservation {
    Verified {
        cells: usize,
        stacks: usize,
        rejected_branches: usize,
    },
    Incomplete {
        status: Status,
    },
    Rejected {
        diagnostic: String,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationRecord {
    pub evidence: EvidenceRecord,
    pub record: RecordRef,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Evidence<T> {
    request: RequestRecord,
    result: T,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    evidence: EvidenceRecord,
    observation: VerificationObservation,
}

/// One shared request context. Retain this between sequential evidence attempts
/// to avoid importing/copying the native physical density for each attempt.
#[derive(Clone, Debug)]
pub struct StagedRequest {
    receipt: RequestRecord,
    request: Arc<GcadRequest>,
}

/// Fresh verifier result and its already-published diagnostic record. A failure
/// remains distinct from a native incomplete result and leaves raw evidence on disk.
#[derive(Debug)]
pub struct VerificationAttempt {
    pub observation: ObservationRecord,
    pub result: std::result::Result<VerifiedDecomposition, GcadError>,
}

// Length checks precede any decoding/import. This caps accepted transport bytes,
// not the multiple buffers or native algebra memory used while decoding/verifying.
fn preflight(root: &Path, records: &[&RecordRef], maximum_bytes: u64) -> Result<()> {
    let mut total = 0u64;
    for record in records {
        total = total.checked_add(record.bytes).ok_or(Error::RecordLimit)?;
        if total > maximum_bytes {
            return Err(Error::RecordLimit);
        }
        record.resolve(root)?;
    }
    for record in records {
        record.verify(root)?;
    }
    Ok(())
}

impl StagedRequest {
    /// Atomically publish a request, retaining its existing shared owner.
    pub fn write(root: &Path, request: Arc<GcadRequest>) -> Result<Self> {
        let receipt = request::write(root, &request)?;
        Ok(Self { receipt, request })
    }
    /// Reconstruct the native typed request, without solving or importing trust.
    pub fn read(root: &Path, receipt: &RequestRecord, maximum_bytes: u64) -> Result<Self> {
        Self::read_with_representation_limits(
            root,
            receipt,
            maximum_bytes,
            crate::threshold::represented::Limits::default(),
        )
    }
    /// Replay saved represented-value conversion under an independent caller cap.
    pub fn read_with_representation_limits(
        root: &Path,
        receipt: &RequestRecord,
        maximum_bytes: u64,
        conversion_cap: crate::threshold::represented::Limits,
    ) -> Result<Self> {
        preflight(root, &[&receipt.record], maximum_bytes)?;
        Ok(Self {
            receipt: receipt.clone(),
            request: Arc::new(request::read(root, receipt, conversion_cap, None)?),
        })
    }
    /// Reconstruct a pre-parametric graph request from the original native point.
    /// Complete original/conversion associations and exact density must agree.
    /// This redoes bounded conversion and native parameterization, never GCAD solve;
    /// saved raw evidence must still pass `verify_evidence` independently.
    /// Integration artifact loading does not require this preparation owner.
    pub fn read_with_original_graph(
        root: &Path,
        receipt: &RequestRecord,
        maximum_bytes: u64,
        original: Arc<crate::threshold::represented::graph::GraphPoint>,
        conversion_cap: crate::threshold::represented::Limits,
        mut observer: impl FnMut(
            crate::threshold::represented::graph::Progress,
        ) -> std::ops::ControlFlow<()>,
    ) -> Result<Self> {
        preflight(root, &[&receipt.record], maximum_bytes)?;
        Ok(Self {
            receipt: receipt.clone(),
            request: Arc::new(request::read(
                root,
                receipt,
                conversion_cap,
                Some(request::OriginalSource::Graph {
                    original,
                    observer: &mut observer,
                }),
            )?),
        })
    }
    /// Rebuild a represented family request from the caller's original native
    /// family/point, then check complete source/conversion and density association.
    /// This never solves GCAD or imports verification authority.
    pub fn read_with_original_family(
        root: &Path,
        receipt: &RequestRecord,
        maximum_bytes: u64,
        original: Arc<crate::threshold::represented::family::FamilyPoint>,
        conversion_cap: crate::threshold::represented::Limits,
        mut observer: impl FnMut(
            crate::threshold::represented::family::Progress,
        ) -> std::ops::ControlFlow<()>,
    ) -> Result<Self> {
        preflight(root, &[&receipt.record], maximum_bytes)?;
        Ok(Self {
            receipt: receipt.clone(),
            request: Arc::new(request::read(
                root,
                receipt,
                conversion_cap,
                Some(request::OriginalSource::Family {
                    original,
                    observer: &mut observer,
                }),
            )?),
        })
    }
    pub fn request(&self) -> &GcadRequest {
        &self.request
    }
    pub fn request_owner(&self) -> &Arc<GcadRequest> {
        &self.request
    }
    pub fn receipt(&self) -> &RequestRecord {
        &self.receipt
    }

    /// Save raw native evidence before attempting independent verification.
    /// Serialization borrows the result; no second SolveResult clone is made.
    pub fn write_evidence(
        &self,
        root: &Path,
        evidence: &NativeDecomposition,
    ) -> Result<EvidenceRecord> {
        if evidence.identity() != self.request.identity()
            || !self
                .request
                .matches_native_problem(&evidence.result().problem)
        {
            return Err(Error::Association);
        }
        self.receipt.record.verify(root)?;
        let record = codec::write(
            root,
            "gcad-evidence",
            EVIDENCE_KIND,
            &Evidence {
                request: self.receipt.clone(),
                result: evidence.result(),
            },
            Atoms::default(),
            vec![],
        )?;
        Ok(EvidenceRecord {
            request: self.receipt.clone(),
            record,
            status: evidence.result().status.clone(),
            cells: evidence.result().cells.len(),
        })
    }
    /// Load untrusted raw evidence. This neither solves nor verifies the proof.
    pub fn read_evidence(
        &self,
        root: &Path,
        receipt: &EvidenceRecord,
        maximum_bytes: u64,
    ) -> Result<NativeDecomposition> {
        if receipt.request != self.receipt {
            return Err(Error::Association);
        }
        preflight(
            root,
            &[&self.receipt.record, &receipt.record],
            maximum_bytes,
        )?;
        let (stored, atoms, symbols): (Evidence<SolveResult>, _, _) =
            codec::read(root, &receipt.record, EVIDENCE_KIND)?;
        if stored.request != self.receipt
            || !atoms.atoms.is_empty()
            || !symbols.is_empty()
            || !self.request.matches_native_problem(&stored.result.problem)
            || stored.result.status != receipt.status
            || stored.result.cells.len() != receipt.cells
        {
            return Err(Error::Association);
        }
        Ok(NativeDecomposition::from_parts(
            self.request.identity().clone(),
            stored.result,
        ))
    }
    /// Restore and run the current native verifier, recording its observation.
    /// Previously saved accepted observations are deliberately never consulted.
    pub fn verify_evidence(
        &self,
        root: &Path,
        receipt: &EvidenceRecord,
        maximum_bytes: u64,
    ) -> Result<VerificationAttempt> {
        let evidence = self.read_evidence(root, receipt, maximum_bytes)?;
        let result = self.request.verify(evidence);
        let observation = match &result {
            Ok(verified) => {
                let report = verified.verification();
                VerificationObservation::Verified {
                    cells: report.cells,
                    stacks: report.stacks,
                    rejected_branches: report.rejected_branches,
                }
            }
            Err(GcadError::Incomplete(status)) => VerificationObservation::Incomplete {
                status: status.clone(),
            },
            Err(error) => VerificationObservation::Rejected {
                diagnostic: error.to_string(),
            },
        };
        let record = codec::write(
            root,
            "gcad-verification",
            OBSERVATION_KIND,
            &Observation {
                evidence: receipt.clone(),
                observation,
            },
            Atoms::default(),
            vec![],
        )?;
        Ok(VerificationAttempt {
            observation: ObservationRecord {
                evidence: receipt.clone(),
                record,
            },
            result,
        })
    }
}

impl ObservationRecord {
    /// Read only a historical diagnostic. No VerifiedDecomposition is returned.
    pub fn read(&self, root: &Path, maximum_bytes: u64) -> Result<VerificationObservation> {
        preflight(
            root,
            &[
                &self.evidence.request.record,
                &self.evidence.record,
                &self.record,
            ],
            maximum_bytes,
        )?;
        let (stored, atoms, symbols): (Observation, _, _) =
            codec::read(root, &self.record, OBSERVATION_KIND)?;
        if stored.evidence != self.evidence || !atoms.atoms.is_empty() || !symbols.is_empty() {
            return Err(invalid("GCAD observation association mismatch").into());
        }
        Ok(stored.observation)
    }
}
