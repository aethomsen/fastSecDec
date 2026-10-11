//! Retained original strict-support preparation, before any guard enlargement.
use super::super::*;
use super::{
    embedding::SupportEmbedding,
    general_transform::RelativeBlowupGeometry,
    helpers::*,
    saturation::{SaturatedSupport, saturate_support},
};
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct SupportGuardPullback {
    source: Poly,
    total: Poly,
    ambient_unit: bool,
}
impl SupportGuardPullback {
    pub fn source(&self) -> &Poly {
        &self.source
    }
    pub fn total(&self) -> &Poly {
        &self.total
    }
    pub fn ambient_unit(&self) -> bool {
        self.ambient_unit
    }
}
#[derive(Clone, Debug)]
pub struct StrictSupportPreparation {
    geometry: Arc<RelativeBlowupGeometry>,
    embedding: Arc<SupportEmbedding>,
    saturation: Arc<SaturatedSupport>,
    guards: Vec<SupportGuardPullback>,
}
impl StrictSupportPreparation {
    pub fn prepare(
        geometry: Arc<RelativeBlowupGeometry>,
        embedding: Arc<SupportEmbedding>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let ambient = geometry.center().history().ledger().frame();
        if !Arc::ptr_eq(ambient, embedding.ambient()) {
            return Err(Error::Invalid(
                "strict embedded support current ambient owner",
            ));
        }
        b.reserve_slots(embedding.equations().generators().len())?;
        let totals = embedding
            .equations()
            .generators()
            .iter()
            .map(|f| geometry.pull(f, b))
            .collect::<Result<Vec<_>>>()?;
        let local = geometry.frame().local();
        let pulled = Arc::new(Ideal::new(local.ring().clone(), totals, b)?);
        let factor = geometry
            .exceptional()
            .cloned()
            .unwrap_or_else(|| local.ring().one());
        let saturation = saturate_support(
            local.clone(),
            pulled,
            factor,
            &format!("{namespace}_sat"),
            b,
        )?;
        let mut guards = Vec::new();
        if !saturation.empty() {
            b.reserve_slots(embedding.frame().local().guards().len())?;
            for g in embedding.frame().local().guards() {
                if (embedding.extension().source().len()..g.factor.nvars())
                    .any(|i| g.factor.degree(i) > 0)
                {
                    return Err(Error::ResourceIncomplete(
                        "strict support needs source guard lift",
                    ));
                }
                let source =
                    super::super::elimination::remap(&g.factor, ambient.local().ring(), b)?;
                let total = geometry.pull(&source, b)?;
                let ambient_unit = unit(local, &total, b)?;
                guards.push(SupportGuardPullback {
                    source,
                    total,
                    ambient_unit,
                });
            }
        }
        Ok(Arc::new(Self {
            geometry,
            embedding,
            saturation,
            guards,
        }))
    }
    pub fn geometry(&self) -> &Arc<RelativeBlowupGeometry> {
        &self.geometry
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    pub fn saturation(&self) -> &Arc<SaturatedSupport> {
        &self.saturation
    }
    pub fn guards(&self) -> &[SupportGuardPullback] {
        &self.guards
    }
    pub(super) fn ambient_units(&self, b: &mut Budget) -> Result<Vec<Poly>> {
        if self.guards.iter().any(|g| !g.ambient_unit) {
            return Err(Error::ResourceIncomplete(
                "strict support needs original contact localization",
            ));
        }
        b.reserve_slots(self.guards.len())?;
        Ok(self.guards.iter().map(|g| g.total.clone()).collect())
    }
    /// Full ambient cover from units on THIS complete saturated support. This is
    /// not a cover of some other ancestor's marked cosupport.
    pub fn physical_cover(&self, b: &mut Budget) -> Result<Arc<VerifiedOpenCover>> {
        if self.saturation.empty() {
            return Err(Error::Invalid(
                "empty strict support needs no guard refinement",
            ));
        }
        let local = self.geometry.frame().local();
        let support = LocalizedAlgebra::new(
            local.ideal().sum(self.saturation.result(), b)?,
            local.axes().to_vec(),
            local.guards().to_vec(),
            b,
        )?;
        let mut product = local.ring().one();
        let mut required = 0usize;
        for g in &self.guards {
            if !g.ambient_unit {
                if !unit(&support, &g.total, b)? {
                    return Err(Error::ResourceIncomplete(
                        "original guard is nonunit on strict support",
                    ));
                }
                product = b.mul(&product, &g.total)?;
                required = required
                    .checked_add(1)
                    .ok_or(Error::ResourceIncomplete("support guard count"))?;
            }
        }
        if required == 0 {
            return Err(Error::Invalid(
                "strict support has no pending original guard",
            ));
        }
        b.reserve_slots(
            self.saturation
                .result()
                .generators()
                .len()
                .checked_add(1)
                .ok_or(Error::ResourceIncomplete("support complement count"))?,
        )?;
        let mut opens = vec![product];
        opens.extend(self.saturation.result().generators().iter().cloned());
        Ok(Arc::new(
            OpenCoverCertificate {
                algebra: local.clone(),
                support: Ideal::new(local.ring().clone(), vec![], b)?,
                opens,
            }
            .verify(b)?,
        ))
    }
}
