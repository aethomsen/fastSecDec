//! Indexed transport around the existing native evaluator codec. Neither this
//! module nor its readers/writers own threads, processes or a scheduling loop.
pub(crate) mod assembly;
mod catalogue;
mod layout;
pub(crate) mod programs;
mod reader;
mod record_reader;
#[cfg(test)]
mod tests;
mod transport;
mod writer;

pub use catalogue::{KernelCatalogue, RecordDescriptor, RecordReceipt};
pub(crate) use programs::ProgramResidentAssembly;
pub use programs::{
    ProgramArchiveCatalogue, ProgramArchiveReader, ProgramArchiveWriter, ProgramRecipe,
    ProgramRecipeCatalogue, SelectedProgramReader,
};
pub use reader::IndexedReader;
pub use writer::{IndexedWriter, write_record, write_unit};

use crate::kernel::{KernelError, KernelLoadOptions, KernelLoadProgress, KernelSet};
use std::{io::Cursor, ops::ControlFlow};

pub(crate) const MAGIC: &[u8] = b"FastSecDec\0indexed\x01";
const FOOTER: &[u8] = b"FastSecDec\0catalogue\x01";

fn failure(error: impl std::fmt::Display) -> KernelError {
    KernelError::Artifact(format!("indexed native artifact: {error}"))
}

/// Encode an ordinary resident kernel set through the same record writer used
/// by disk-spooled generation. Each temporary native record is released before
/// serializing the next; the returned archive is necessarily resident.
pub fn to_bytes(kernels: &KernelSet) -> Result<(Vec<u8>, KernelCatalogue), KernelError> {
    let mut writer = IndexedWriter::new(Cursor::new(Vec::new()))?;
    for sector in std::iter::once(None).chain((0..kernels.sectors().len()).map(Some)) {
        let (bytes, receipt) = writer::partition(kernels, sector)?;
        writer.append_record(&mut bytes.as_slice(), receipt)?;
    }
    let (writer, catalogue) = writer.finish()?;
    Ok((writer.into_inner(), catalogue))
}

impl KernelSet {
    /// Adopt the indexed representation produced from this resident template.
    /// This keeps a generate-and-integrate run identical to loading its saved
    /// archive, without restoring or cloning any numerical evaluator.
    pub fn adopt_indexed_catalogue(
        &mut self,
        catalogue: &KernelCatalogue,
    ) -> Result<(), KernelError> {
        catalogue.validate(true)?;
        if self.template_content_id.is_some()
            || self.orders != catalogue.orders
            || self.components != catalogue.components
            || self.sectors.len() != catalogue.sector_count()
            || self
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.source_scope())
                .map(|scope| scope.selection())
                != catalogue.source_selection()
        {
            return Err(failure(
                "indexed catalogue does not describe this unbound resident template",
            ));
        }
        for (index, sector) in self.sectors.iter().enumerate() {
            let record = catalogue.sector(index)?;
            let sources = self
                .metadata
                .as_ref()
                .map(|metadata| {
                    metadata
                        .charts
                        .iter()
                        .filter(|chart| chart.kernel_sector == Some(index))
                        .map(|chart| chart.source_index)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if record.receipt.dimension != Some(sector.dimension())
                || sources != record.receipt.source_indices
            {
                return Err(failure("indexed catalogue reordered resident sectors"));
            }
        }
        self.content_id = catalogue.content_id.clone();
        self.portable_artifact = None;
        Ok(())
    }
}

pub(crate) fn from_bytes(
    bytes: &[u8],
    options: KernelLoadOptions,
    progress: &mut impl FnMut(&KernelLoadProgress) -> ControlFlow<()>,
) -> Result<KernelSet, KernelError> {
    let mut reader = IndexedReader::from_reader(Cursor::new(bytes), options)?;
    let mut kernels = reader.load_all_with_progress(progress)?;
    kernels.portable_artifact = Some(bytes.to_vec().into());
    Ok(kernels)
}

pub(crate) mod threshold;
pub use threshold::{
    ThresholdArchiveSummary, ThresholdExpectedRecord, ThresholdRecordReceipt, ThresholdRuntimeGuard,
};
