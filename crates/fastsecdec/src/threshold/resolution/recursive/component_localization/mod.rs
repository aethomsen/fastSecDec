//! A full ambient cover induced by an issued terminal component split.
//! This is local auxiliary recursion, not global center gluing or a real atlas.
use super::*;
mod frontier;
mod sources;
pub use frontier::{
    ComponentLocalizationAdvance, ComponentLocalizationFrontier, ComponentLocalizationState,
};
pub use sources::{LocalizedComponentCenter, LocalizedComponentSources};
#[derive(Clone, Debug)]
pub struct ComponentLocalization {
    terminal: Arc<RecursiveComponentCover>,
    level: Arc<RecursiveLevel>,
    cover: Arc<PhysicalSupportCover>,
}
impl ComponentLocalization {
    /// Restrict the original contact. Higher stacks remain explicitly unresolved.
    pub fn prepare(terminal: Arc<RecursiveComponentCover>, b: &mut Budget) -> Result<Arc<Self>> {
        let [level] = terminal.levels() else {
            return Err(Error::ResourceIncomplete(
                "component localization requires one retained contact level",
            ));
        };
        if level.presentation().power().is_some() {
            return Err(Error::ResourceIncomplete(
                "component localization of a principal presentation remains unsupported",
            ));
        }
        let origin = terminal.origin();
        let contact = level.contact();
        if !Arc::ptr_eq(origin.frame(), &level.frame)
            || !Arc::ptr_eq(origin.source(), level.source())
            || !Arc::ptr_eq(&level.frame, contact.source().frame())
            || !Arc::ptr_eq(level.normalization().source(), level.source())
            || !Arc::ptr_eq(
                level.normalization().normalizer().local(),
                level.frame.local(),
            )
            || level.normalization().normalized().ideal() != contact.source().source().as_ref()
            || level.normalization().normalized().mark() != level.source().mark()
            || !Arc::ptr_eq(terminal.frame(), contact.contact())
            || terminal.source().ideal() != contact.differential_coefficient().ideal()
            || terminal.source().mark() != contact.differential_coefficient().mark()
            || !Arc::ptr_eq(terminal.normalization().source(), terminal.source())
            || terminal.normalization().normalized().mark() != terminal.source().mark()
            || !Arc::ptr_eq(
                terminal.normalization().normalizer().local(),
                terminal.frame().local(),
            )
        {
            return Err(Error::Invalid("component localization original hierarchy"));
        }
        let embedding = SupportEmbedding::contact(contact.clone(), b)?;
        let cover = embedding.physical_cover(
            level.source().clone(),
            Arc::new(terminal.split().cover().clone()),
            b,
        )?;
        Ok(Arc::new(Self {
            level: level.clone(),
            terminal,
            cover,
        }))
    }
    pub fn terminal(&self) -> &Arc<RecursiveComponentCover> {
        &self.terminal
    }
    pub fn level(&self) -> &Arc<RecursiveLevel> {
        &self.level
    }
    pub fn cover(&self) -> &Arc<PhysicalSupportCover> {
        &self.cover
    }
    pub fn origin(&self) -> &Arc<AuxiliaryRecursionOrigin> {
        self.terminal.origin()
    }
}
#[cfg(test)]
mod tests;

pub(super) use sources::pull_ideal;
