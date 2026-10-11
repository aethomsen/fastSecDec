//! Caller-owned process boundary. Serialized work is untrusted transport, never
//! a verified continuation or permission to publish a complete integral.
use super::*;
use crate::generation::streaming::{
    RecordRef,
    codec::{self, Atoms},
};
use crate::kernel::indexed::{ProgramArchiveWriter, RecordReceipt};
use std::io::{Read, Seek, Write};

const KIND: &str = "threshold-compilation-work-plan-v1";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdCompilationWork {
    plan: RecordRef,
    index: usize,
}
/// A worker receipt binds the immutable preparation record actually decoded.
/// The live plan validates it before copying the exact hashed program bytes.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdWorkReceipt {
    work: ThresholdCompilationWork,
    record: RecordReceipt,
}
/// Compact child response. This is untrusted data, not a serialized proof owner.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThresholdPublicationDescriptor {
    pub work_plan: RecordRef,
    pub source_identity: String,
    pub prepared_identity: String,
}
/// Structural admission for a caller-authenticated native preparer result.
/// It holds no global proof, native map Atoms, or evaluator programs.
pub struct ThresholdPublicationPlan {
    descriptor: ThresholdPublicationDescriptor,
    summary: crate::kernel::indexed::ThresholdArchiveSummary,
    shape: PreparationShape,
}
/// Derived structural facts only; no extra all-chart manifest or native Atoms.
struct PreparationShape {
    original_dimensions: usize,
    integration_dimensions: BTreeSet<usize>,
    cells: usize,
    endpoint_charts: usize,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    tables: crate::kernel::threshold_owner::StagedTablePlan,
    specifications: Vec<Specification>,
    precision: PrecisionPolicy,
    settings: CompilationSettings,
}
pub(super) fn write_plan(prepared: &Prepared) -> Result<RecordRef, KernelError> {
    let (tables, symbols) = prepared.tables.descriptor();
    codec::write(
        &prepared.root,
        "threshold-work-plan",
        KIND,
        &Plan {
            tables,
            specifications: prepared.specifications.clone(),
            precision: prepared.precision.clone(),
            settings: prepared.settings,
        },
        Atoms::default(),
        symbols,
    )
    .map_err(failure)
}
impl ThresholdCompilationWork {
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn plan_record(&self) -> &RecordRef {
        &self.plan
    }
    /// Run one native job synchronously and write one native record. The limit
    /// covers imported transport bytes, not decoded RSS or compiler allocations.
    /// The caller owns process lifetime, concurrency, RSS and durable publication.
    pub fn compile_record(
        &self,
        root: &Path,
        maximum_bytes: u64,
        writer: &mut impl Write,
    ) -> Result<ThresholdWorkReceipt, KernelError> {
        let remaining = maximum_bytes
            .checked_sub(self.plan.bytes)
            .ok_or_else(|| failure("native plan transport limit"))?;
        let (plan, atoms, symbols): (Plan, _, _) =
            codec::read(root, &self.plan, KIND).map_err(failure)?;
        if !atoms.atoms.is_empty() {
            return Err(failure("native work plan contains nonlocal Atom payload"));
        }
        plan.precision.validate()?;
        plan.settings.validate()?;
        let tables = StagedTables::from_descriptor(plan.tables, symbols)?;
        let specification = plan
            .specifications
            .get(self.index)
            .ok_or_else(|| failure("worker index out of range"))?;
        match (&specification.kind, &specification.vector) {
            (ThresholdJobKind::Setup {}, None) => {}
            (ThresholdJobKind::Exact { .. }, Some(vector))
                if matches!(vector.kind, record::VectorKind::Exact {}) => {}
            (ThresholdJobKind::Stochastic { contribution }, Some(vector))
                if matches!(vector.kind, record::VectorKind::Stochastic {})
                    && contribution.0 == vector.chart => {}
            _ => return Err(failure("worker kind/vector association")),
        }
        let prepared = Arc::new(Prepared {
            token: Arc::new(()),
            root: root.into(),
            tables,
            specifications: plan.specifications,
            precision: plan.precision,
            settings: plan.settings,
            work_record: None,
        });
        let completion = ThresholdCompilationJob {
            prepared,
            index: self.index,
        }
        .run_bounded(remaining)?;
        let record = completion.write_record(writer)?;
        Ok(ThresholdWorkReceipt {
            work: self.clone(),
            record,
        })
    }
}
impl ThresholdWorkReceipt {
    pub fn work(&self) -> &ThresholdCompilationWork {
        &self.work
    }
    pub fn record(&self) -> &RecordReceipt {
        &self.record
    }
}
impl ThresholdCompilationPlan {
    pub fn publication_descriptor(&self) -> Result<ThresholdPublicationDescriptor, KernelError> {
        let summary = self
            .prepared
            .tables
            .publication(self.prepared.settings, &self.prepared.precision)?;
        Ok(ThresholdPublicationDescriptor {
            work_plan: self
                .prepared
                .work_record
                .clone()
                .ok_or_else(|| failure("unissued plan"))?,
            source_identity: summary.source_identity,
            prepared_identity: summary.complete_content_id,
        })
    }
    pub fn work(&self, index: usize) -> Result<ThresholdCompilationWork, KernelError> {
        if index >= self.job_count() {
            return Err(failure("job index out of range"));
        }
        Ok(ThresholdCompilationWork {
            plan: self
                .prepared
                .work_record
                .clone()
                .ok_or_else(|| failure("unissued worker plan"))?,
            index,
        })
    }
    /// Accept durable output only against this live, natively authorized plan.
    /// Resume must reconstruct/reverify preparation before calling this method;
    /// neither the receipt nor the work descriptor reconstitutes authority.
    pub fn validate_work_receipt(&self, receipt: &ThresholdWorkReceipt) -> Result<(), KernelError> {
        if receipt.work != self.work(receipt.work.index)? {
            return Err(failure("foreign worker preparation record"));
        }
        receipt.record.validate()?;
        let summary = self
            .prepared
            .tables
            .publication(self.prepared.settings, &self.prepared.precision)?;
        let expected = summary
            .expected
            .get(receipt.work.index)
            .ok_or_else(|| failure("unexpected worker result"))?;
        let actual = receipt
            .record
            .threshold
            .as_ref()
            .ok_or_else(|| failure("worker omitted threshold lineage"))?;
        if actual.runtime_guard != summary.runtime_guard
            || actual.parent != summary.parent
            || actual.carrier != expected.carrier
            || actual.lineage != expected.lineage
            || receipt.record.native_content_id != expected.native_content_id
        {
            return Err(failure(
                "worker result differs from issued mathematical job",
            ));
        }
        Ok(())
    }
    pub fn append_work_record<W: Write + Seek>(
        &self,
        writer: &mut ProgramArchiveWriter<W>,
        reader: &mut impl Read,
        receipt: ThresholdWorkReceipt,
    ) -> Result<(), KernelError> {
        self.validate_work_receipt(&receipt)?;
        // Native archive copying checks the digest of exactly the copied bytes.
        writer.append_record(
            crate::kernel::ProgramRecipe::ThresholdV1,
            reader,
            receipt.record,
        )
    }
}

impl ThresholdPublicationPlan {
    /// The caller must authenticate the preparer child/lease and original input
    /// receipt. This validates transport and closed publication semantics only;
    /// it never claims to replay geometry/continuation proof in the coordinator.
    pub fn from_trusted_preparer(
        root: &Path,
        descriptor: ThresholdPublicationDescriptor,
        expected_source: &str,
        expected_prepared: &str,
        maximum_bytes: u64,
    ) -> Result<Self, KernelError> {
        if descriptor.source_identity != expected_source
            || descriptor.prepared_identity != expected_prepared
        {
            return Err(failure("prepared source or semantic identity mismatch"));
        }
        if descriptor.work_plan.bytes > maximum_bytes {
            return Err(failure("publication transport limit"));
        }
        let (plan, atoms, symbols): (Plan, _, _) =
            codec::read(root, &descriptor.work_plan, KIND).map_err(failure)?;
        if !atoms.atoms.is_empty() {
            return Err(failure("publication plan contains global native Atoms"));
        }
        plan.precision.validate()?;
        plan.settings.validate()?;
        let tables = StagedTables::from_descriptor(plan.tables, symbols)?;
        let summary = tables.publication(plan.settings, &plan.precision)?;
        if summary.source_identity != descriptor.source_identity
            || summary.complete_content_id != descriptor.prepared_identity
            || summary.expected.len() != plan.specifications.len()
        {
            return Err(failure("publication descriptor/native plan association"));
        }
        for (index, spec) in plan.specifications.iter().enumerate() {
            let expected = &summary.expected[index];
            let matches = match (&spec.kind, &expected.lineage) {
                (ThresholdJobKind::Setup {}, None) => expected.carrier,
                (ThresholdJobKind::Exact { contributions }, Some(lineage)) => {
                    expected.carrier
                        && *contributions == lineage.contributions
                        && matches!(lineage.kind, m::RecordKind::Exact)
                }
                (ThresholdJobKind::Stochastic { contribution }, Some(lineage)) => {
                    !expected.carrier
                        && lineage.contributions == [*contribution]
                        && matches!(lineage.kind, m::RecordKind::Stochastic { .. })
                }
                _ => false,
            };
            if !matches {
                return Err(failure("publication work inventory mismatch"));
            }
        }
        let manifest = tables.manifest();
        let shape = PreparationShape {
            original_dimensions: manifest.preparation.original_coordinates.len(),
            integration_dimensions: manifest
                .endpoint_charts
                .iter()
                .map(|chart| chart.map.geometry().coordinates.len())
                .collect(),
            cells: manifest.cells.len(),
            endpoint_charts: manifest.endpoint_charts.len(),
        };
        Ok(Self {
            descriptor,
            summary,
            shape,
        })
    }
    /// Original input roles in the structurally checked native parent directory.
    pub fn original_dimensions(&self) -> usize {
        self.shape.original_dimensions
    }
    /// Distinct endpoint-chart dimensions, without loading native expression tables.
    /// These counts do not restore a geometry or continuation certificate.
    pub fn integration_dimensions(&self) -> impl Iterator<Item = usize> + '_ {
        self.shape.integration_dimensions.iter().copied()
    }
    pub fn cell_count(&self) -> usize {
        self.shape.cells
    }
    pub fn endpoint_chart_count(&self) -> usize {
        self.shape.endpoint_charts
    }
    pub fn descriptor(&self) -> &ThresholdPublicationDescriptor {
        &self.descriptor
    }
    pub fn job_count(&self) -> usize {
        self.summary.expected.len()
    }
    pub fn work(&self, index: usize) -> Result<ThresholdCompilationWork, KernelError> {
        if index >= self.job_count() {
            return Err(failure("publication job out of range"));
        }
        Ok(ThresholdCompilationWork {
            plan: self.descriptor.work_plan.clone(),
            index,
        })
    }
    pub fn archive_writer<W: Write + Seek>(
        &self,
        writer: W,
    ) -> Result<ProgramArchiveWriter<W>, KernelError> {
        ProgramArchiveWriter::new_threshold(writer, self.summary.clone())
    }
    pub fn validate_work_receipt(&self, receipt: &ThresholdWorkReceipt) -> Result<(), KernelError> {
        if receipt.work != self.work(receipt.work.index)? {
            return Err(failure("foreign prepared worker record"));
        }
        receipt.record.validate()?;
        let expected = &self.summary.expected[receipt.work.index];
        let actual = receipt
            .record
            .threshold
            .as_ref()
            .ok_or_else(|| failure("missing worker lineage"))?;
        if actual.runtime_guard != self.summary.runtime_guard
            || actual.parent != self.summary.parent
            || actual.carrier != expected.carrier
            || actual.lineage != expected.lineage
            || receipt.record.native_content_id != expected.native_content_id
        {
            return Err(failure("worker differs from prepared publication"));
        }
        Ok(())
    }
    pub fn append_work_record<W: Write + Seek>(
        &self,
        writer: &mut ProgramArchiveWriter<W>,
        reader: &mut impl Read,
        receipt: ThresholdWorkReceipt,
    ) -> Result<(), KernelError> {
        self.validate_work_receipt(&receipt)?;
        writer.append_record(
            crate::kernel::ProgramRecipe::ThresholdV1,
            reader,
            receipt.record,
        )
    }
}
