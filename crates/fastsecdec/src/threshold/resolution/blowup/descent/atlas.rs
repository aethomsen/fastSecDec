use super::*;
#[derive(Clone, Debug)]
pub(super) enum AmbientOpen {
    Nonempty(Arc<LocalizedCoverHistory>),
    Empty { clearing: UnitClearing },
}
/// Full ambient principal-open atlas, based on one immutable current problem.
/// The current marked source is not asserted to be an original physics density.
#[derive(Clone, Debug)]
pub struct PrincipalCenterAtlas {
    history: Arc<ResolutionHistory>,
    source: Arc<MarkedIdeal>,
    cover: Arc<VerifiedOpenCover>,
    pub(super) opens: Vec<AmbientOpen>,
}
impl PrincipalCenterAtlas {
    pub fn prepare(
        history: Arc<ResolutionHistory>,
        source: Arc<MarkedIdeal>,
        cover: Arc<VerifiedOpenCover>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let local = history.ledger().frame().local();
        if !Arc::ptr_eq(local, cover.algebra()) || source.ideal().ring() != local.ring() {
            return Err(Error::Invalid(
                "ambient atlas source/history/cover association",
            ));
        }
        if !cover.support().generators().is_empty() {
            return Err(Error::Invalid(
                "ambient atlas requires full zero-support cover",
            ));
        }
        for f in source.ideal().generators() {
            local.supports(f)?;
        }
        b.reserve_slots(cover.opens().len())?;
        let mut opens = Vec::new();
        for i in 0..cover.opens().len() {
            match localize_verified_cover_open(
                history.clone(),
                cover.clone(),
                i,
                &format!("{namespace}_{i}"),
                b,
            )? {
                CoverLocalizationProduction::Complete(o) => {
                    let o = Arc::new(*o);
                    Self::check_history(&history, &o, b)?;
                    opens.push(AmbientOpen::Nonempty(o));
                }
                CoverLocalizationProduction::Empty { clearing, .. } => {
                    opens.push(AmbientOpen::Empty { clearing })
                }
                CoverLocalizationProduction::Incomplete { reason, .. } => {
                    return Err(Error::ResourceIncomplete(reason));
                }
            }
        }
        Ok(Arc::new(Self {
            history,
            source,
            cover,
            opens,
        }))
    }
    fn check_history(
        source: &Arc<ResolutionHistory>,
        open: &LocalizedCoverHistory,
        b: &mut Budget,
    ) -> Result<()> {
        let h = open.history();
        if !Arc::ptr_eq(open.open().source(), source)
            || !h.same_root(source)
            || h.stage() != source.stage()
            || h.births() != source.births()
            || h.old_snapshot() != source.old_snapshot()
        {
            return Err(Error::Invalid("ambient atlas global history association"));
        }
        let old = source.ledger().divisors();
        let new = h.ledger().divisors();
        if old.len() != new.len() {
            return Err(Error::Invalid("ambient atlas complete boundary inventory"));
        }
        for (a, z) in old.iter().zip(new) {
            if a.id != z.id || h.birth_context(a.id) != source.birth_context(a.id) {
                return Err(Error::Invalid("ambient atlas boundary birth/ordering"));
            }
            let f = open.open().extension().pull(&a.equation, b)?;
            if !h.ledger().frame().local().zero(&(&f - &z.equation), b)? {
                return Err(Error::Invalid("ambient atlas boundary pullback"));
            }
        }
        Ok(())
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn source(&self) -> &Arc<MarkedIdeal> {
        &self.source
    }
    pub fn cover(&self) -> &Arc<VerifiedOpenCover> {
        &self.cover
    }
    pub fn len(&self) -> usize {
        self.opens.len()
    }
    pub fn is_empty(&self) -> bool {
        self.opens.is_empty()
    }
    pub fn open(&self, i: usize) -> Option<&Arc<LocalizedCoverHistory>> {
        match self.opens.get(i)? {
            AmbientOpen::Nonempty(o) => Some(o),
            AmbientOpen::Empty { .. } => None,
        }
    }
    pub fn empty_open(&self, i: usize) -> Option<&UnitClearing> {
        match self.opens.get(i)? {
            AmbientOpen::Empty { clearing } => Some(clearing),
            _ => None,
        }
    }
}
