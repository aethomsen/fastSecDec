//! Threshold contribution transport alongside, never in place of, legacy source IDs.
use super::{RecordDescriptor, RecordReceipt, failure};
#[cfg(feature = "threshold-decomposition")]
use crate::kernel::KernelSet;
use crate::kernel::{KernelError, ThresholdMetadata, threshold_metadata as m};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicit v4 runtime capability, shared by the compact parent and records.
/// This is arithmetic association, never a serialized uniform geometry proof.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdRuntimeGuard {
    pub semantic: m::Digest,
    pub parameters: Vec<String>,
}
impl ThresholdRuntimeGuard {
    pub(crate) fn from_owner(owner: &ThresholdMetadata) -> Result<Option<Self>, KernelError> {
        owner
            .lineage()
            .preparation
            .runtime_chamber
            .as_ref()
            .map(|chamber| {
                Ok(Self {
                    semantic: chamber.guard_semantic.clone(),
                    parameters: owner
                        .runtime_parameters()?
                        .iter()
                        .map(|s| s.get_name().to_owned())
                        .collect(),
                })
            })
            .transpose()
    }
    fn validate(&self) -> Result<(), KernelError> {
        self.semantic.validate().map_err(failure)?;
        if self.parameters.is_empty()
            || self.parameters.iter().any(String::is_empty)
            || self.parameters.iter().collect::<BTreeSet<_>>().len() != self.parameters.len()
        {
            return Err(failure("uniform template runtime schema"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdRecordReceipt {
    pub parent: m::Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_guard: Option<ThresholdRuntimeGuard>,
    /// The unique exact/setup carrier contains the inline compact native parent.
    pub carrier: bool,
    /// None is permitted only for the explicit zero-offset setup carrier.
    pub lineage: Option<m::RecordLineageV1>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdExpectedRecord {
    pub carrier: bool,
    pub lineage: Option<m::RecordLineageV1>,
    pub native_content_id: String,
}
/// Compact structural publication directory. It contains no native Atom or
/// global proof. The native carrier independently checks its complete parent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdArchiveSummary {
    pub parent: m::Digest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_guard: Option<ThresholdRuntimeGuard>,
    pub source_identity: String,
    pub source_extent: m::SourceExtent,
    pub contributions: Vec<m::Contribution>,
    pub complete_content_id: String,
    pub expected: Vec<ThresholdExpectedRecord>,
}
impl ThresholdRecordReceipt {
    pub(super) fn validate(&self, receipt: &RecordReceipt) -> Result<(), KernelError> {
        self.parent.validate().map_err(failure)?;
        match &self.runtime_guard {
            Some(guard) => {
                guard.validate()?;
                if receipt.version != 4 || receipt.runtime_parameters != guard.parameters {
                    return Err(failure("guarded threshold receipt capability"));
                }
            }
            None if receipt.version == 3 && receipt.runtime_parameters.is_empty() => {}
            None => return Err(failure("fixed threshold receipt capability")),
        }
        if !matches!(receipt.version, 3 | 4)
            || receipt.recipe != Some(super::ProgramRecipe::ThresholdV1)
            || !receipt.source_indices.is_empty()
            || receipt.source_scope.is_some()
        {
            return Err(failure(
                "threshold receipt used legacy source/runtime schema",
            ));
        }
        match &self.lineage {
            None if self.carrier && receipt.dimension.is_none() => {}
            Some(lineage) => {
                if lineage.manifest != self.parent
                    || lineage.contributions.is_empty()
                    || lineage.contributions.windows(2).any(|p| p[0] >= p[1])
                {
                    return Err(failure("threshold receipt contribution association"));
                }
                match &lineage.kind {
                    m::RecordKind::Exact if self.carrier && receipt.dimension.is_none() => {}
                    m::RecordKind::Stochastic { coordinates }
                        if !self.carrier
                            && receipt.dimension == Some(coordinates.len())
                            && lineage.contributions.len() == 1 => {}
                    _ => return Err(failure("threshold receipt kind/dimension")),
                }
            }
            _ => return Err(failure("missing threshold scientific contribution")),
        }
        Ok(())
    }
    pub(crate) fn sort_key(&self) -> (bool, Vec<m::ContributionId>) {
        (
            !self.carrier,
            self.lineage
                .as_ref()
                .map_or_else(Vec::new, |r| r.contributions.clone()),
        )
    }
}
impl ThresholdArchiveSummary {
    pub(crate) fn validate(&self, records: &[RecordDescriptor]) -> Result<(), KernelError> {
        self.parent.validate().map_err(failure)?;
        if let Some(guard) = &self.runtime_guard {
            guard.validate()?;
        }
        super::super::validate_content_id(&self.source_identity)?;
        super::super::validate_content_id(&self.complete_content_id)?;
        if self.contributions.is_empty()
            || self.contributions.windows(2).any(|p| p[0].id >= p[1].id)
            || self.expected.len() != records.len()
            || self.expected.iter().filter(|e| e.carrier).count() != 1
            || !self.expected.first().is_some_and(|e| e.carrier)
        {
            return Err(failure("incomplete threshold publication directory"));
        }
        let mut observed = BTreeSet::new();
        for (expected, record) in self.expected.iter().zip(records) {
            let receipt = record
                .receipt
                .threshold
                .as_ref()
                .ok_or_else(|| failure("threshold directory has legacy record"))?;
            receipt.validate(&record.receipt)?;
            if receipt.runtime_guard != self.runtime_guard
                || receipt.parent != self.parent
                || receipt.carrier != expected.carrier
                || receipt.lineage != expected.lineage
                || record.receipt.native_content_id != expected.native_content_id
            {
                return Err(failure(
                    "threshold record differs from closed publication plan",
                ));
            }
            if let Some(lineage) = &receipt.lineage {
                for id in &lineage.contributions {
                    if !observed.insert(*id) {
                        return Err(failure("duplicate threshold contribution"));
                    }
                    let contribution = self
                        .contributions
                        .iter()
                        .find(|c| c.id == *id)
                        .ok_or_else(|| failure("unknown threshold contribution"))?;
                    match (&contribution.kind, &lineage.kind) {
                        (m::ContributionKind::Exact, m::RecordKind::Exact) => {}
                        (
                            m::ContributionKind::Stochastic { coordinates: a },
                            m::RecordKind::Stochastic { coordinates: b },
                        ) if a == b => {}
                        _ => return Err(failure("threshold contribution kind mismatch")),
                    }
                }
            }
        }
        let expected: BTreeSet<_> = self
            .contributions
            .iter()
            .filter_map(|c| match &c.kind {
                m::ContributionKind::CertifiedZero { .. } => None,
                _ => Some(c.id),
            })
            .collect();
        if observed != expected {
            return Err(failure("missing threshold scientific contribution"));
        }
        for c in &self.contributions {
            if let m::ContributionKind::CertifiedZero { certificate } = &c.kind {
                certificate.validate().map_err(failure)?;
            }
        }
        Ok(())
    }
    pub(crate) fn check_parent(&self, owner: &ThresholdMetadata) -> Result<(), KernelError> {
        if ThresholdRuntimeGuard::from_owner(owner)? != self.runtime_guard
            || owner.lineage().descriptor_digest().map_err(failure)? != self.parent
            || owner.lineage().preparation.source_identity.0 != self.source_identity
            || owner.lineage().preparation.extent != self.source_extent
            || owner.lineage().contributions != self.contributions
        {
            return Err(failure(
                "native threshold parent differs from archive inventory",
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "threshold-decomposition")]
pub(crate) fn write_record(
    writer: &mut impl std::io::Write,
    kernels: &KernelSet,
    carrier: bool,
) -> Result<RecordReceipt, KernelError> {
    let owner = kernels
        .threshold_metadata()
        .ok_or_else(|| failure("missing threshold owner"))?;
    if kernels.template_content_id.is_some() || kernels.sectors().len() > 1 || !owner.is_local() {
        return Err(failure(
            "threshold record must be one unbound local native owner",
        ));
    }
    let threshold = ThresholdRecordReceipt {
        parent: owner.resident().manifest.clone(),
        runtime_guard: ThresholdRuntimeGuard::from_owner(owner)?,
        carrier,
        lineage: owner.combined_record_lineage()?,
    };
    let (id, bytes) = super::super::binary::compiled_with_parent(kernels, carrier)?;
    if id != kernels.content_id() {
        return Err(failure("threshold record identity changed"));
    }
    let receipt = RecordReceipt {
        source_scope: None,
        version: if threshold.runtime_guard.is_some() {
            4
        } else {
            3
        },
        recipe: Some(super::ProgramRecipe::ThresholdV1),
        length: bytes.len() as u64,
        digest: blake3::hash(&bytes).to_hex().to_string(),
        native_content_id: id,
        orders: kernels.orders().to_vec(),
        components: kernels.components().to_vec(),
        runtime_parameters: kernels
            .runtime_parameters
            .iter()
            .map(|s| s.get_name().to_owned())
            .collect(),
        dimension: kernels.sectors().first().map(|s| s.dimension()),
        statistics: kernels.sectors().first().map(|s| s.statistics().clone()),
        source_indices: Vec::new(),
        threshold: Some(threshold),
    };
    receipt.validate()?;
    writer.write_all(&bytes).map_err(failure)?;
    Ok(receipt)
}
