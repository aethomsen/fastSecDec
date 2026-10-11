//! Recipe-addressable transport. Selection precedes native decoding/JIT, and
//! every active numerical owner still contains exactly one mathematical recipe.
mod catalogue;
mod reader;
mod resident;
#[cfg(test)]
mod tests;
mod writer;

pub use catalogue::{ProgramArchiveCatalogue, ProgramRecipe, ProgramRecipeCatalogue};
pub use reader::{ProgramArchiveReader, SelectedProgramReader};
pub(crate) use resident::ProgramResidentAssembly;
pub use writer::ProgramArchiveWriter;

pub(crate) const MAGIC: &[u8] = b"FastSecDec\0indexed\x02";
pub(crate) const MAGIC_V3: &[u8] = b"FastSecDec\0indexed\x03";
pub(crate) const MAGIC_V4: &[u8] = b"FastSecDec\0indexed\x04";
const FOOTER_V4: &[u8] = b"FastSecDec\0catalogue\x04";
const FOOTER_V3: &[u8] = b"FastSecDec\0catalogue\x03";
const FOOTER: &[u8] = b"FastSecDec\0catalogue\x02";

impl crate::kernel::KernelSet {
    /// Publish the current unbound template as a complete singleton program
    /// archive and retain its portable bytes. Numerical objects stay resident;
    /// adoption performs neither decoding nor JIT. The caller owns writing and
    /// atomically publishing the returned archive via [`Self::artifact_bytes`].
    pub fn retain_program_archive(
        &mut self,
        source_identity: String,
    ) -> Result<ProgramArchiveCatalogue, crate::kernel::KernelError> {
        let recipe = self.program_recipe();
        let mut writer =
            ProgramArchiveWriter::new(std::io::Cursor::new(Vec::new()), source_identity, [recipe])?;
        writer.append_kernels(recipe, self)?;
        let (writer, catalogue) = writer.finish()?;
        let selected = catalogue.recipe(recipe)?;
        if self.orders != selected.orders
            || self.components != selected.components
            || self.sectors.len() != selected.sector_count()
        {
            return Err(super::failure(
                "resident program layout differs from its saved archive",
            ));
        }
        self.content_id = selected.content_id.clone();
        self.portable_artifact = Some(writer.into_inner().into());
        Ok(catalogue)
    }
}

/// Standalone native owner restoration requires an unambiguous single recipe.
/// Full archives are opened through ProgramArchiveReader and selected explicitly.
pub(crate) fn from_bytes(
    bytes: &[u8],
    options: crate::kernel::KernelLoadOptions,
    progress: &mut impl FnMut(&crate::kernel::KernelLoadProgress) -> std::ops::ControlFlow<()>,
) -> Result<crate::kernel::KernelSet, crate::kernel::KernelError> {
    let mut reader = ProgramArchiveReader::from_reader(std::io::Cursor::new(bytes), options)?;
    if reader.catalogue().recipes.len() != 1 {
        return Err(super::failure(
            "multi-recipe artifact requires explicit ProgramArchiveReader::select before loading",
        ));
    }
    let recipe = reader.catalogue().recipes[0].recipe;
    reader.select(recipe)?.load_all_with_progress(progress)
}
