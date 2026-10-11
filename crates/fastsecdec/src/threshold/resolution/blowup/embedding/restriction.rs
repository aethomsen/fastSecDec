//! Exact base restriction of an already-issued support and its lower cover.
use super::super::refinement::frame::{JoinedFrameProduction, join};
use super::*;

pub(super) struct RestrictedGeometry {
    pub ambient: Arc<EtaleFrame>,
    pub frame: Arc<EtaleFrame>,
    pub extension: RingExtension,
    pub equations: Arc<Ideal>,
    pub map: super::super::refinement::SupportFrameMap,
}
pub(super) enum RestrictionGeometry {
    Complete(Box<RestrictedGeometry>),
    Empty {
        equations: Ideal,
        unit_relations: Vec<Poly>,
    },
}

/// Shared by the old physical-cover method and the actual source-open method.
pub(super) fn restrict(
    parent: &Arc<SupportEmbedding>,
    localization: &Arc<LocalizedCoverHistory>,
    namespace: &str,
    b: &mut Budget,
) -> Result<RestrictionGeometry> {
    if !Arc::ptr_eq(
        localization.open().source().ledger().frame(),
        parent.ambient(),
    ) {
        return Err(Error::Invalid("ambient restriction original source owner"));
    }
    let ambient = localization.history().ledger().frame().clone();
    let joined = match join(
        ambient.clone(),
        parent.frame().clone(),
        parent.ambient().local().ring(),
        namespace,
        b,
    )? {
        JoinedFrameProduction::Complete(joined) => joined,
        JoinedFrameProduction::Empty {
            equations,
            unit_relations,
        } => {
            return Ok(RestrictionGeometry::Empty {
                equations,
                unit_relations,
            });
        }
    };
    let frame = joined.frame.clone();
    let extension = super::nested::extension_to(ambient.local().ring(), frame.local().ring(), b)?;
    let equations = Arc::new(
        localization
            .open()
            .extension()
            .ideal(parent.equations(), b)?,
    );
    super::nested::full_equations(&ambient, &frame, &extension, &equations, b)?;
    if ambient
        .free_axes()
        .len()
        .checked_sub(frame.free_axes().len())
        != Some(parent.codimension())
    {
        return Err(Error::Invalid("ambient restriction support codimension"));
    }
    Ok(RestrictionGeometry::Complete(Box::new(
        RestrictedGeometry {
            ambient,
            frame,
            extension,
            equations,
            map: joined.child_map.clone(),
        },
    )))
}

#[derive(Clone, Debug)]
pub struct EmptyAmbientSupport {
    parent: Arc<SupportEmbedding>,
    localization: Arc<LocalizedCoverHistory>,
    equations: Ideal,
    unit_relations: Vec<Poly>,
}
impl EmptyAmbientSupport {
    pub fn parent(&self) -> &Arc<SupportEmbedding> {
        &self.parent
    }
    pub fn localization(&self) -> &Arc<LocalizedCoverHistory> {
        &self.localization
    }
    pub fn equations(&self) -> &Ideal {
        &self.equations
    }
    pub fn unit_relations(&self) -> &[Poly] {
        &self.unit_relations
    }
}
#[derive(Clone, Debug)]
pub enum AmbientSupportRestriction {
    Support(Arc<SupportEmbedding>),
    /// Empty lower support, never an empty physical principal open.
    EmptySupport(Arc<EmptyAmbientSupport>),
}
impl SupportEmbedding {
    /// Restrict along an actual original-ambient localization. The caller still
    /// owns the full source cover and every sibling; this is one base change.
    pub fn restrict_ambient_open(
        self: &Arc<Self>,
        localization: Arc<LocalizedCoverHistory>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<AmbientSupportRestriction> {
        let g = match restrict(self, &localization, namespace, b)? {
            RestrictionGeometry::Complete(g) => g,
            RestrictionGeometry::Empty {
                equations,
                unit_relations,
            } => {
                return Ok(AmbientSupportRestriction::EmptySupport(Arc::new(
                    EmptyAmbientSupport {
                        parent: self.clone(),
                        localization,
                        equations,
                        unit_relations,
                    },
                )));
            }
        };
        Ok(AmbientSupportRestriction::Support(Arc::new(Self {
            ambient: g.ambient,
            frame: g.frame,
            extension: g.extension,
            equations: g.equations,
            codimension: self.codimension(),
            origin: EmbeddingOrigin::SourceOpen {
                parent: self.clone(),
                localization,
                map: g.map,
            },
        })))
    }
    pub fn source_open(
        &self,
    ) -> Option<(
        &Arc<SupportEmbedding>,
        &Arc<LocalizedCoverHistory>,
        &super::super::refinement::SupportFrameMap,
    )> {
        match &self.origin {
            EmbeddingOrigin::SourceOpen {
                parent,
                localization,
                map,
            } => Some((parent, localization, map)),
            _ => None,
        }
    }
}

/// Complete original-cover/index association under the actual support map.
#[derive(Clone, Debug)]
pub struct PulledSupportCover {
    embedding: Arc<SupportEmbedding>,
    original: Arc<VerifiedOpenCover>,
    pulled: Arc<VerifiedOpenCover>,
    map: super::super::refinement::SupportFrameMap,
    join_prefix: Arc<Ring>,
    original_history: Option<Arc<ResolutionHistory>>,
}
impl PulledSupportCover {
    pub fn prepare(
        embedding: Arc<SupportEmbedding>,
        original: Arc<VerifiedOpenCover>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let (parent, _, map) = embedding.source_open().ok_or(Error::Invalid(
            "cover pullback needs issued source restriction",
        ))?;
        if !Arc::ptr_eq(original.algebra(), parent.frame().local())
            || !Arc::ptr_eq(map.source(), parent.frame())
            || !Arc::ptr_eq(map.target(), embedding.frame())
        {
            return Err(Error::Invalid("support cover original/map association"));
        }
        let prefix = parent.ambient().local().ring().clone();
        b.reserve_slots(map.coordinate_indices().len())?;
        let map = map.clone();
        Self::prepare_with_map(embedding, original, map, prefix, None, b)
    }
    /// Only an actual matched support can license the next lower-cover step.
    pub(crate) fn prepare_matched(
        embedding: Arc<SupportEmbedding>,
        original: Arc<VerifiedOpenCover>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let matched = embedding
            .matched_origin()
            .ok_or(Error::Invalid("next cover missing matched refinement"))?;
        if !Arc::ptr_eq(
            original.algebra(),
            matched.lower().history().ledger().frame().local(),
        ) {
            return Err(Error::Invalid("next cover actual lower history frame"));
        }
        b.reserve_slots(matched.lower_map().coordinate_indices().len())?;
        let map = matched.lower_map().clone();
        let prefix = matched.join_prefix().clone();
        let history = matched.lower().history().clone();
        Self::prepare_with_map(embedding, original, map, prefix, Some(history), b)
    }
    fn prepare_with_map(
        embedding: Arc<SupportEmbedding>,
        original: Arc<VerifiedOpenCover>,
        map: super::super::refinement::SupportFrameMap,
        join_prefix: Arc<Ring>,
        original_history: Option<Arc<ResolutionHistory>>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !Arc::ptr_eq(original.algebra(), map.source().local())
            || !Arc::ptr_eq(map.target(), embedding.frame())
        {
            return Err(Error::Invalid("support cover consumed map association"));
        }
        b.reserve_slots(
            original
                .opens()
                .len()
                .checked_add(original.support().generators().len())
                .ok_or(Error::ResourceIncomplete("support cover pullback slots"))?,
        )?;
        let support = Ideal::new(
            embedding.frame().local().ring().clone(),
            original
                .support()
                .generators()
                .iter()
                .map(|p| map.pull(p, b))
                .collect::<Result<Vec<_>>>()?,
            b,
        )?;
        let opens = original
            .opens()
            .iter()
            .map(|p| map.pull(p, b))
            .collect::<Result<Vec<_>>>()?;
        let pulled = Arc::new(
            OpenCoverCertificate {
                algebra: embedding.frame().local().clone(),
                support,
                opens,
            }
            .verify(b)?,
        );
        if pulled.opens().len() != original.opens().len() {
            return Err(Error::Invalid("pulled lower cover changed index inventory"));
        }
        Ok(Arc::new(Self {
            embedding,
            original,
            pulled,
            map,
            join_prefix,
            original_history,
        }))
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    pub fn original(&self) -> &Arc<VerifiedOpenCover> {
        &self.original
    }
    pub fn pulled(&self) -> &Arc<VerifiedOpenCover> {
        &self.pulled
    }
    pub(super) fn map(&self) -> &super::super::refinement::SupportFrameMap {
        &self.map
    }
    pub(super) fn join_prefix(&self) -> &Arc<Ring> {
        &self.join_prefix
    }
    pub(super) fn original_history(&self) -> Option<&Arc<ResolutionHistory>> {
        self.original_history.as_ref()
    }
}
