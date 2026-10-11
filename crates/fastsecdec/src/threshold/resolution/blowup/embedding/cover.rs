//! Physical opens induced by an actual lower support cover. The complement is
//! retained as a lower-mark-order locus, never as a zero-density conclusion.
use super::*;

#[derive(Clone, Debug)]
pub enum PhysicalSupportRestriction {
    Support(Arc<SupportEmbedding>),
    EmptySupport {
        cover: Arc<PhysicalSupportCover>,
        localization: Arc<LocalizedCoverHistory>,
        equations: Ideal,
        unit_relations: Vec<Poly>,
    },
}

#[derive(Clone, Debug)]
pub enum PhysicalSupportOpen {
    Lower {
        index: usize,
        clearing: UnitClearing,
    },
    BelowMark {
        differential_generator: usize,
    },
}

#[derive(Clone, Debug)]
pub struct PhysicalSupportCover {
    embedding: Arc<SupportEmbedding>,
    source: Arc<MarkedIdeal>,
    lower: Arc<VerifiedOpenCover>,
    cosupport: Ideal,
    cosupport_cover: Arc<VerifiedOpenCover>,
    full_cover: Arc<VerifiedOpenCover>,
    opens: Vec<PhysicalSupportOpen>,
}
impl PhysicalSupportCover {
    /// Restrict the actual support along an issued physical-cover open.
    pub fn restrict_support(
        self: &Arc<Self>,
        localization: Arc<LocalizedCoverHistory>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<PhysicalSupportRestriction> {
        if !Arc::ptr_eq(localization.open().cover(), &self.full_cover)
            || !Arc::ptr_eq(
                localization.open().source().ledger().frame(),
                self.embedding.ambient(),
            )
        {
            return Err(Error::Invalid(
                "physical support restriction actual cover/open owner",
            ));
        }
        let geometry =
            match super::restriction::restrict(&self.embedding, &localization, namespace, b)? {
                super::restriction::RestrictionGeometry::Complete(g) => g,
                super::restriction::RestrictionGeometry::Empty {
                    equations,
                    unit_relations,
                } => {
                    return Ok(PhysicalSupportRestriction::EmptySupport {
                        cover: self.clone(),
                        localization,
                        equations,
                        unit_relations,
                    });
                }
            };
        Ok(PhysicalSupportRestriction::Support(Arc::new(
            SupportEmbedding {
                ambient: geometry.ambient,
                frame: geometry.frame,
                extension: geometry.extension,
                equations: geometry.equations,
                codimension: self.embedding.codimension(),
                origin: EmbeddingOrigin::AmbientOpen {
                    parent: self.embedding.clone(),
                    cover: self.clone(),
                    localization,
                    map: geometry.map,
                },
            },
        )))
    }
    pub(crate) fn prepare(
        embedding: Arc<SupportEmbedding>,
        source: Arc<MarkedIdeal>,
        lower: Arc<VerifiedOpenCover>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let frame = embedding.ambient();
        if !Arc::ptr_eq(embedding.frame().local(), lower.algebra())
            || source.ideal().ring() != frame.local().ring()
        {
            return Err(Error::Invalid("physical support-cover source owners"));
        }
        // This is the actual marked cosupport ideal, with relative derivatives
        // in the physical frame. Lower support/cover relations are not used as
        // substitutes for the final exact coverage check.
        let mut cosupport = source.ideal().clone();
        for _ in 1..source.mark() {
            cosupport = super::super::super::differential::relative_differential_ideal(
                frame, &cosupport, b,
            )?;
        }
        let count = lower
            .opens()
            .len()
            .checked_add(cosupport.generators().len())
            .ok_or(Error::ResourceIncomplete("physical support cover size"))?;
        b.reserve_slots(
            count
                .checked_mul(2)
                .ok_or(Error::ResourceIncomplete("physical support cover slots"))?,
        )?;
        let mut opens = Vec::new();
        let mut factors = Vec::new();
        for (index, f) in lower.opens().iter().enumerate() {
            let clearing = super::nested::lift_unit_fraction(&embedding, f, b)?;
            factors.push(clearing.numerator.clone());
            opens.push(PhysicalSupportOpen::Lower { index, clearing });
        }
        let cosupport_cover = Arc::new(
            OpenCoverCertificate {
                algebra: frame.local().clone(),
                support: cosupport.clone(),
                opens: factors.clone(),
            }
            .verify(b)?,
        );
        for (differential_generator, f) in cosupport.generators().iter().enumerate() {
            factors.push(f.clone());
            opens.push(PhysicalSupportOpen::BelowMark {
                differential_generator,
            });
        }
        let full_cover = Arc::new(
            OpenCoverCertificate {
                algebra: frame.local().clone(),
                support: Ideal::new(frame.local().ring().clone(), vec![], b)?,
                opens: factors,
            }
            .verify(b)?,
        );
        Ok(Arc::new(Self {
            embedding,
            source,
            lower,
            cosupport,
            cosupport_cover,
            full_cover,
            opens,
        }))
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn lower(&self) -> &Arc<VerifiedOpenCover> {
        &self.lower
    }
    pub fn cosupport(&self) -> &Ideal {
        &self.cosupport
    }
    pub fn cosupport_cover(&self) -> &Arc<VerifiedOpenCover> {
        &self.cosupport_cover
    }
    pub fn full_cover(&self) -> &Arc<VerifiedOpenCover> {
        &self.full_cover
    }
    pub fn opens(&self) -> &[PhysicalSupportOpen] {
        &self.opens
    }
    /// Issue one covered physical localization, retaining below-mark complements.
    pub fn localize(
        &self,
        history: Arc<ResolutionHistory>,
        index: usize,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<CoverLocalizationProduction> {
        if !Arc::ptr_eq(history.ledger().frame(), self.embedding.ambient()) {
            return Err(Error::Invalid("physical support cover history owner"));
        }
        let kind = self
            .opens
            .get(index)
            .ok_or(Error::Invalid("physical support open index"))?;
        let result =
            localize_verified_cover_open(history, self.full_cover.clone(), index, namespace, b)?;
        if let (
            PhysicalSupportOpen::BelowMark { .. },
            CoverLocalizationProduction::Complete(open),
        ) = (kind, &result)
        {
            let frame = open.history().ledger().frame();
            let pulled = open.open().extension().ideal(&self.cosupport, b)?;
            if !frame.local().ideal().sum(&pulled, b)?.contains(
                &frame.local().ring().one(),
                frame.local().unit_relations(),
                b,
            )? {
                return Err(Error::Invalid(
                    "physical support complement did not lower marked order",
                ));
            }
        }
        Ok(result)
    }
}
impl SupportEmbedding {
    /// Verify that pulled lower opens cover this marked cosupport, and include
    /// its derivative-ideal complements to cover the entire physical chart.
    pub fn physical_cover(
        self: &Arc<Self>,
        source: Arc<MarkedIdeal>,
        lower: Arc<VerifiedOpenCover>,
        b: &mut Budget,
    ) -> Result<Arc<PhysicalSupportCover>> {
        PhysicalSupportCover::prepare(self.clone(), source, lower, b)
    }
    pub fn physical_localization(
        &self,
    ) -> Option<(
        &Arc<PhysicalSupportCover>,
        &Arc<LocalizedCoverHistory>,
        &super::super::refinement::SupportFrameMap,
    )> {
        match &self.origin {
            EmbeddingOrigin::AmbientOpen {
                cover,
                localization,
                map,
                ..
            } => Some((cover, localization, map)),
            _ => None,
        }
    }
}
