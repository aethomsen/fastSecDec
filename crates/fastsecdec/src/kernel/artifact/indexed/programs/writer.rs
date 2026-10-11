use super::super::{RecordDescriptor, RecordReceipt, failure, transport};
use super::{ProgramArchiveCatalogue, ProgramRecipe, ProgramRecipeCatalogue};
use crate::kernel::{KernelError, KernelSet};
use std::{
    collections::BTreeMap,
    io::{Read, Seek, Write},
};

/// Caller-owned append-only storage for a declared complete recipe set. Records
/// are native worker output, copied in bounded blocks without decoding or JIT.
/// A failed append poisons this staging writer. The caller owns durable atomic
/// publication, just as with the version-one writer.
pub struct ProgramArchiveWriter<W> {
    writer: W,
    source_identity: String,
    recipes: BTreeMap<ProgramRecipe, Vec<RecordDescriptor>>,
    failed: bool,
    threshold: Option<super::super::ThresholdArchiveSummary>,
}
impl<W: Write + Seek> ProgramArchiveWriter<W> {
    pub fn new(
        mut writer: W,
        source_identity: String,
        recipes: impl IntoIterator<Item = ProgramRecipe>,
    ) -> Result<Self, KernelError> {
        super::super::super::validate_content_id(&source_identity)?;
        let mut requested = BTreeMap::new();
        for recipe in recipes {
            if requested.insert(recipe, Vec::new()).is_some() {
                return Err(failure("duplicate requested recipe"));
            }
        }
        if requested.contains_key(&ProgramRecipe::ThresholdV1) {
            return Err(failure(
                "threshold writer requires a closed native publication plan",
            ));
        }
        if requested.is_empty() || writer.stream_position().map_err(failure)? != 0 {
            return Err(failure(
                "program writer requires recipes and must start at byte zero",
            ));
        }
        writer.write_all(super::MAGIC).map_err(failure)?;
        Ok(Self {
            writer,
            source_identity,
            recipes: requested,
            failed: false,
            threshold: None,
        })
    }
    #[cfg(feature = "native")]
    pub(crate) fn new_threshold(
        mut writer: W,
        summary: super::super::ThresholdArchiveSummary,
    ) -> Result<Self, KernelError> {
        if writer.stream_position().map_err(failure)? != 0 {
            return Err(failure("threshold writer must start at byte zero"));
        }
        writer
            .write_all(if summary.runtime_guard.is_some() {
                super::MAGIC_V4
            } else {
                super::MAGIC_V3
            })
            .map_err(failure)?;
        Ok(Self {
            writer,
            source_identity: summary.source_identity.clone(),
            recipes: BTreeMap::from([(ProgramRecipe::ThresholdV1, Vec::new())]),
            failed: false,
            threshold: Some(summary),
        })
    }
    pub fn append_record(
        &mut self,
        recipe: ProgramRecipe,
        reader: &mut impl Read,
        receipt: RecordReceipt,
    ) -> Result<(), KernelError> {
        if self.failed {
            return Err(failure("cannot append to a failed archive writer"));
        }
        let records = self
            .recipes
            .get_mut(&recipe)
            .ok_or_else(|| failure("record recipe was not requested"))?;
        receipt.validate()?;
        if receipt.program_recipe() != recipe {
            return Err(failure(
                "record runtime schema differs from its advertised recipe",
            ));
        }
        self.failed = true;
        let offset = self.writer.stream_position().map_err(failure)?;
        transport::copy_record(&mut self.writer, reader, &receipt)?;
        records.push(RecordDescriptor {
            offset,
            receipt,
            output_indices: Vec::new(),
            sector: None,
        });
        self.failed = false;
        Ok(())
    }
    /// Append a resident template through exactly the same record partitioner
    /// as streamed generation. Only one temporary native sector record is held
    /// at a time; no evaluator is decoded, restored or reoptimized.
    pub fn append_kernels(
        &mut self,
        recipe: ProgramRecipe,
        kernels: &KernelSet,
    ) -> Result<(), KernelError> {
        if kernels.program_recipe() != recipe || kernels.template_content_id.is_some() {
            return Err(failure(
                "resident archive input must be an unbound template of the requested recipe",
            ));
        }
        for sector in std::iter::once(None).chain((0..kernels.sectors().len()).map(Some)) {
            let (bytes, receipt) = super::super::writer::partition(kernels, sector)
                .inspect_err(|_| self.failed = true)?;
            self.append_record(recipe, &mut bytes.as_slice(), receipt)?;
        }
        Ok(())
    }

    /// Append one complete native representative with local-to-source chart
    /// indices. Partitioning preserves independent exact and stochastic records.
    /// Only one temporary serialized record is resident at a time.
    pub fn append_unit(
        &mut self,
        recipe: ProgramRecipe,
        kernels: &KernelSet,
        source_indices: &[usize],
    ) -> Result<(), KernelError> {
        if kernels.program_recipe() != recipe || kernels.template_content_id.is_some() {
            return Err(failure(
                "archive unit must be an unbound template of the requested recipe",
            ));
        }
        super::super::writer::for_each_unit_record(kernels, source_indices, |mut bytes, receipt| {
            self.append_record(recipe, &mut bytes, receipt)
        })
        .inspect_err(|_| self.failed = true)
    }
    pub fn finish(mut self) -> Result<(W, ProgramArchiveCatalogue), KernelError> {
        if self.failed {
            return Err(failure("cannot finish a failed archive writer"));
        }
        let end = self.writer.stream_position().map_err(failure)?;
        let recipes = self
            .recipes
            .into_iter()
            .map(|(recipe, records)| {
                if let Some(summary) = self.threshold.take() {
                    ProgramRecipeCatalogue::finish_threshold(records, summary)
                } else {
                    ProgramRecipeCatalogue::finish(recipe, records, &self.source_identity)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        let catalogue = ProgramArchiveCatalogue::finish(self.source_identity, recipes, end)?;
        let footer = if catalogue.version == 4 {
            super::FOOTER_V4
        } else if catalogue.version == 3 {
            super::FOOTER_V3
        } else {
            super::FOOTER
        };
        transport::write_footer(&mut self.writer, &catalogue, footer)?;
        Ok((self.writer, catalogue))
    }
}
