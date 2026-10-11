use super::*;
/// Exact restrictions through one issued open and its original contact join.
#[derive(Clone, Debug)]
pub struct LocalizedComponentSources {
    owner: Arc<ComponentLocalization>,
    localization: Arc<LocalizedCoverHistory>,
    support: Arc<SupportEmbedding>,
    parent: Arc<MarkedIdeal>,
    parent_normalized: Ideal,
    child: Arc<MarkedIdeal>,
    child_normalized: Ideal,
}
impl LocalizedComponentSources {
    pub fn owner(&self) -> &Arc<ComponentLocalization> {
        &self.owner
    }
    pub fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        &self.localization
    }
    pub fn support(&self) -> &Arc<SupportEmbedding> {
        &self.support
    }
    pub fn parent(&self) -> &Arc<MarkedIdeal> {
        &self.parent
    }
    pub fn child(&self) -> &Arc<MarkedIdeal> {
        &self.child
    }
    pub fn parent_normalized(&self) -> &Ideal {
        &self.parent_normalized
    }
    pub fn child_normalized(&self) -> &Ideal {
        &self.child_normalized
    }
    pub(super) fn prepare(
        owner: Arc<ComponentLocalization>,
        localization: Arc<LocalizedCoverHistory>,
        support: Arc<SupportEmbedding>,
        parent: Arc<MarkedIdeal>,
        parent_normalized: Ideal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let (cover, open, map) = support
            .physical_localization()
            .ok_or(Error::Invalid("component restricted contact origin"))?;
        if !Arc::ptr_eq(cover, owner.cover())
            || !Arc::ptr_eq(open, &localization)
            || !Arc::ptr_eq(support.ambient(), localization.history().ledger().frame())
        {
            return Err(Error::Invalid("component contact/open association"));
        }
        let terminal = owner.terminal();
        let local = support.frame().local();
        let child_ideal = pull_ideal(map, terminal.source().ideal(), b)?;
        let child_normalized = pull_ideal(map, terminal.normalization().normalized().ideal(), b)?;
        equal(local, &child_ideal, &child_normalized, b)?;
        let child = Arc::new(MarkedIdeal::new(child_ideal, terminal.source().mark(), b)?);
        Ok(Arc::new(Self {
            owner,
            localization,
            support,
            parent,
            parent_normalized,
            child,
            child_normalized,
        }))
    }
}
/// A center on exactly this issued open, with the complete component cover retained.
#[derive(Clone, Debug)]
pub struct LocalizedComponentCenter {
    sources: Arc<LocalizedComponentSources>,
    ideal: Ideal,
    normals: Vec<Poly>,
    clearings: Vec<UnitClearing>,
}
impl LocalizedComponentCenter {
    pub fn sources(&self) -> &Arc<LocalizedComponentSources> {
        &self.sources
    }
    pub fn frame(&self) -> &Arc<EtaleFrame> {
        self.sources.support.ambient()
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        self.sources.localization.history()
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.sources.parent
    }
    pub fn ideal(&self) -> &Ideal {
        &self.ideal
    }
    pub fn normals(&self) -> &[Poly] {
        &self.normals
    }
    pub fn clearings(&self) -> &[UnitClearing] {
        &self.clearings
    }
    pub(super) fn prepare(
        sources: Arc<LocalizedComponentSources>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let support = &sources.support;
        for f in sources.child.ideal().generators() {
            if !support.frame().local().zero(f, b)? {
                return Err(Error::Invalid(
                    "localized component center requires zero child",
                ));
            }
        }
        let (ideal, normals, clearings) = super::super::lift::lift_embedded_geometry(
            support.ambient(),
            &sources.parent,
            support.frame(),
            support.extension(),
            support.equations().generators().to_vec(),
            sources.child.ideal(),
            &[],
            b,
        )?;
        if normals.is_empty() {
            return Err(Error::Invalid(
                "localized component center requires positive codimension",
            ));
        }
        Ok(Arc::new(Self {
            sources,
            ideal,
            normals,
            clearings,
        }))
    }
}
// No new CAS: the retained native normalization receipts imply equality; verify
// their restrictions again in the actual joined localization.
pub(super) fn equal(local: &LocalizedAlgebra, a: &Ideal, c: &Ideal, b: &mut Budget) -> Result<()> {
    for (left, right) in [(a, c), (c, a)] {
        let full = local.ideal().sum(right, b)?;
        for f in left.generators() {
            if !full.contains(f, local.unit_relations(), b)? {
                return Err(Error::Invalid("localized original normalization identity"));
            }
        }
    }
    Ok(())
}

/// Shared restriction through an existing checked support-frame map.
pub(in super::super) fn pull_ideal(
    map: &SupportFrameMap,
    ideal: &Ideal,
    b: &mut Budget,
) -> Result<Ideal> {
    b.reserve_slots(ideal.generators().len())?;
    Ideal::new(
        map.target().local().ring().clone(),
        ideal
            .generators()
            .iter()
            .map(|f| map.pull(f, b))
            .collect::<Result<Vec<_>>>()?,
        b,
    )
}
