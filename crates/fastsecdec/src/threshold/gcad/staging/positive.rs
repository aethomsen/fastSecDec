//! Durable original homogeneous requests. No imported bytes issue proof authority.
use super::*;
use crate::{
    generation::streaming::input::Input,
    threshold::{
        gcad::{PositiveProjectiveQuotient, PositiveProjectiveRequest, SourceProvenance},
        represented::{
            self,
            graph::{ExactRepresentedGraphInput, GraphPayloadPolicy, GraphPoint},
        },
    },
};
use std::ops::ControlFlow;
use symgcad::Problem;

const KIND: &str = "threshold-positive-projective-request-v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Stored {
    source_identity: String,
    input: Input,
    anchor_index: usize,
    problem: Problem,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    graph: Option<StoredGraph>,
}

// Association with a caller-supplied HEPKit owner, never a replacement graph codec.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredGraph {
    meaning: represented::NumericalMeaning,
    limits: represented::Limits,
    payloads: Option<GraphPayloadPolicy>,
    source_witness: String,
}

/// Caller-owned staging for an original homogeneous request and its raw results.
/// Reconstruction never solves, and verification never trusts a saved observation.
#[derive(Clone, Debug)]
pub struct StagedPositiveRequest {
    receipt: RequestRecord,
    request: Arc<PositiveProjectiveRequest>,
}

#[derive(Debug)]
pub struct PositiveVerificationAttempt {
    pub observation: ObservationRecord,
    pub result: std::result::Result<PositiveProjectiveQuotient, GcadError>,
}

impl StagedPositiveRequest {
    pub fn write(root: &Path, request: Arc<PositiveProjectiveRequest>) -> Result<Self> {
        let source_identity = request.source_identity()?;
        let mut atoms = Atoms::default();
        let input = request.preparation().input();
        let stored = Stored {
            source_identity: source_identity.clone(),
            input: Input::encode(input, &mut atoms),
            anchor_index: request.preparation().anchor_index(),
            problem: request.problem().clone(),
            graph: match request.provenance() {
                SourceProvenance::Exact => None,
                SourceProvenance::PreparametricGraph(graph) => Some(StoredGraph {
                    meaning: graph.meaning(),
                    limits: graph.limits(),
                    payloads: graph.payload_policy(),
                    source_witness: graph.source_witness().to_owned(),
                }),
                _ => return Err(Error::Association),
            },
        };
        let symbols = input
            .parameters()
            .iter()
            .copied()
            .chain([input.regulator()])
            .collect();
        let record = codec::write(root, "gcad-positive-request", KIND, &stored, atoms, symbols)?;
        Ok(Self {
            receipt: RequestRecord {
                source_identity,
                record,
            },
            request,
        })
    }

    /// Read an exact source. Graph sources require `read_with_original_graph`;
    /// decoding an exact density cannot substitute for their original ownership.
    pub fn read(root: &Path, receipt: &RequestRecord, maximum_bytes: u64) -> Result<Self> {
        Self::restore(root, receipt, maximum_bytes, None)
    }

    /// Bounded re-conversion/parameterization of the caller's actual graph point.
    /// This does not solve GCAD, and the original raw proof still needs replay.
    pub fn read_with_original_graph(
        root: &Path,
        receipt: &RequestRecord,
        maximum_bytes: u64,
        original: Arc<GraphPoint>,
        conversion_cap: represented::Limits,
        mut observer: impl FnMut(represented::graph::Progress) -> ControlFlow<()>,
    ) -> Result<Self> {
        Self::restore(
            root,
            receipt,
            maximum_bytes,
            Some((original, conversion_cap, &mut observer)),
        )
    }

    fn restore(
        root: &Path,
        receipt: &RequestRecord,
        maximum_bytes: u64,
        original: Option<OriginalGraph<'_>>,
    ) -> Result<Self> {
        preflight(root, &[&receipt.record], maximum_bytes)?;
        let (stored, atoms, symbols): (Stored, _, _) = codec::read(root, &receipt.record, KIND)?;
        let input = stored.input.decode(&atoms, &symbols)?;
        let request = match (stored.graph, original) {
            (None, None) => PositiveProjectiveRequest::exact(
                Arc::new(input),
                stored.anchor_index,
                stored.problem.solver.clone(),
                stored.problem.limits.clone(),
            )?,
            (Some(_), None) => return Err(Error::OriginalGraphRequired),
            (None, Some(_)) => return Err(Error::Association),
            (Some(record), Some((original, cap, observer))) => {
                if !record.limits.is_within(cap) {
                    return Err(
                        invalid("pre-parametric conversion exceeds caller resource cap").into(),
                    );
                }
                let graph = ExactRepresentedGraphInput::prepare_with_payloads(
                    original,
                    record.meaning,
                    record.limits,
                    record.payloads,
                    observer,
                )
                .map_err(GcadError::from)?;
                if graph.source_witness() != record.source_witness
                    || graph.exact().as_ref() != &input
                {
                    return Err(Error::Association);
                }
                PositiveProjectiveRequest::graph(
                    Arc::new(graph),
                    stored.anchor_index,
                    stored.problem.solver.clone(),
                    stored.problem.limits.clone(),
                )?
            }
        };
        if stored.source_identity != receipt.source_identity
            || request.source_identity()? != receipt.source_identity
            || !request.matches_native_problem(&stored.problem)
        {
            return Err(Error::Association);
        }
        Ok(Self {
            receipt: receipt.clone(),
            request: Arc::new(request),
        })
    }

    pub fn receipt(&self) -> &RequestRecord {
        &self.receipt
    }
    pub fn request(&self) -> &PositiveProjectiveRequest {
        &self.request
    }
    pub fn request_owner(&self) -> &Arc<PositiveProjectiveRequest> {
        &self.request
    }

    /// Persist even incomplete solver results before any verification attempt.
    /// Serialization borrows the large native result instead of cloning it.
    pub fn write_evidence(&self, root: &Path, result: &SolveResult) -> Result<EvidenceRecord> {
        if !self.request.matches_native_problem(&result.problem) {
            return Err(Error::Association);
        }
        self.receipt.record.verify(root)?;
        let record = codec::write(
            root,
            "gcad-evidence",
            EVIDENCE_KIND,
            &Evidence {
                request: self.receipt.clone(),
                result,
            },
            Atoms::default(),
            vec![],
        )?;
        Ok(EvidenceRecord {
            request: self.receipt.clone(),
            record,
            status: result.status.clone(),
            cells: cell_count(result),
        })
    }

    pub fn read_evidence(
        &self,
        root: &Path,
        receipt: &EvidenceRecord,
        maximum_bytes: u64,
    ) -> Result<Arc<SolveResult>> {
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
            || cell_count(&stored.result) != receipt.cells
        {
            return Err(Error::Association);
        }
        Ok(Arc::new(stored.result))
    }

    pub fn verify_evidence(
        &self,
        root: &Path,
        receipt: &EvidenceRecord,
        maximum_bytes: u64,
    ) -> Result<PositiveVerificationAttempt> {
        let raw = self.read_evidence(root, receipt, maximum_bytes)?;
        let result = self.request.verify(raw);
        let observation = match &result {
            Ok(owner) => {
                let report = owner.verification();
                VerificationObservation::Verified {
                    cells: owner.cell_count(),
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
        Ok(PositiveVerificationAttempt {
            observation: record_observation(root, receipt, observation)?,
            result,
        })
    }
}

type OriginalGraph<'a> = (
    Arc<GraphPoint>,
    represented::Limits,
    &'a mut dyn FnMut(represented::graph::Progress) -> ControlFlow<()>,
);

// The native original homogeneous result keeps the actual cells in its quotient.
// This is diagnostic counting only; authority always requires full native replay.
fn cell_count(result: &SolveResult) -> usize {
    result
        .homogeneous
        .as_ref()
        .map_or(result.cells.len(), |h| h.reduced.cells.len())
}

#[cfg(test)]
mod tests;
