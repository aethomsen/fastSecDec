use super::*;
#[derive(Clone, Debug)]
enum LocalAdmissibility {
    Monomial(Box<ProducedMonomialCenter>),
    Selection(Arc<MonomialCenterRestriction>),
    EmptyCosupport {
        witness: MonomialWitness,
        progress: MonomialProgress,
    },
    EmptyAmbient,
}
#[derive(Clone, Debug)]
pub struct LocalCenter {
    atlas: Arc<PrincipalCenterAtlas>,
    index: usize,
    proof: LocalAdmissibility,
    presentation: Option<Ideal>,
}
impl LocalCenter {
    pub(super) fn from_selection(restriction: Arc<MonomialCenterRestriction>) -> Arc<Self> {
        Arc::new(Self {
            atlas: restriction.selection().atlas().clone(),
            index: restriction.index(),
            presentation: restriction.ideal().cloned(),
            proof: LocalAdmissibility::Selection(restriction),
        })
    }
    pub fn selection_restriction(&self) -> Option<&Arc<MonomialCenterRestriction>> {
        match &self.proof {
            LocalAdmissibility::Selection(r) => Some(r),
            _ => None,
        }
    }

    pub fn monomial(
        atlas: Arc<PrincipalCenterAtlas>,
        index: usize,
        powers: Vec<usize>,
        quotients: Vec<Poly>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let open = atlas.open(index).ok_or(Error::Invalid(
            "monomial center requires actual nonempty ambient open",
        ))?;
        let source = Arc::new(MarkedIdeal::new(
            open.open().extension().ideal(atlas.source().ideal(), b)?,
            atlas.source().mark(),
            b,
        )?);
        let (proof, presentation) = match produce_monomial_center(
            open.history().ledger().clone(),
            MonomialWitness {
                source,
                powers,
                quotients,
            },
            b,
        )? {
            MonomialProduction::Center(c) => {
                let p = c.center().clone();
                (LocalAdmissibility::Monomial(c), p)
            }
            MonomialProduction::EmptyCosupport {
                witness, progress, ..
            } => {
                let r = open.history().ledger().frame().local().ring();
                (
                    LocalAdmissibility::EmptyCosupport { witness, progress },
                    Ideal::new(r.clone(), vec![r.one()], b)?,
                )
            }
            MonomialProduction::Incomplete { reason, .. } => {
                return Err(Error::ResourceIncomplete(reason));
            }
        };
        Ok(Arc::new(Self {
            atlas,
            index,
            proof,
            presentation: Some(presentation),
        }))
    }
    pub fn empty_ambient(atlas: Arc<PrincipalCenterAtlas>, index: usize) -> Result<Arc<Self>> {
        if atlas.empty_open(index).is_none() {
            return Err(Error::Invalid("missing native empty ambient receipt"));
        }
        Ok(Arc::new(Self {
            atlas,
            index,
            proof: LocalAdmissibility::EmptyAmbient,
            presentation: None,
        }))
    }
    /// Alternative generators are accepted only for this already issued center.
    pub fn with_presentation(
        self: &Arc<Self>,
        presentation: Ideal,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let old = self
            .presentation
            .as_ref()
            .ok_or(Error::Invalid("empty ambient has no center ring"))?;
        let local = self
            .atlas
            .open(self.index)
            .unwrap()
            .history()
            .ledger()
            .frame()
            .local();
        equal_ideals(local, old, &presentation, b)?;
        Ok(Arc::new(Self {
            atlas: self.atlas.clone(),
            index: self.index,
            proof: self.proof.clone(),
            presentation: Some(presentation),
        }))
    }
    pub fn atlas(&self) -> &Arc<PrincipalCenterAtlas> {
        &self.atlas
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn presentation(&self) -> Option<&Ideal> {
        self.presentation.as_ref()
    }
    pub fn monomial_proof(&self) -> Option<&ProducedMonomialCenter> {
        match &self.proof {
            LocalAdmissibility::Monomial(p) => Some(p),
            _ => None,
        }
    }
    pub fn empty_cosupport(&self) -> Option<(&MonomialWitness, &MonomialProgress)> {
        match &self.proof {
            LocalAdmissibility::EmptyCosupport { witness, progress } => Some((witness, progress)),
            _ => None,
        }
    }
}
