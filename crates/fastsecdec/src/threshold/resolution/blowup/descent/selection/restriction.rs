use super::*;
/// Empty selected locus is distinct from empty original marked cosupport.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectedCenterState {
    EmptyAmbient,
    EmptyCosupport,
    EmptySelectedCenter,
    SelectedCenter,
}
#[derive(Debug)]
pub struct MonomialCenterRestriction {
    selection: Arc<MonomialMaximumSelection>,
    index: usize,
    state: SelectedCenterState,
    ideal: Option<Ideal>,
    cosupport: Option<Arc<MarkedCosupport>>,
}
impl MonomialCenterRestriction {
    pub fn selection(&self) -> &Arc<MonomialMaximumSelection> {
        &self.selection
    }
    pub fn index(&self) -> usize {
        self.index
    }
    pub fn state(&self) -> SelectedCenterState {
        self.state
    }
    pub fn ideal(&self) -> Option<&Ideal> {
        self.ideal.as_ref()
    }
    pub fn cosupport(&self) -> Option<&Arc<MarkedCosupport>> {
        self.cosupport.as_ref()
    }
}
pub(super) fn prepare(
    selection: Arc<MonomialMaximumSelection>,
    index: usize,
    b: &mut Budget,
) -> Result<MonomialCenterRestriction> {
    let atlas = selection.atlas();
    let Some(open) = atlas.open(index) else {
        if atlas.empty_open(index).is_none() {
            return Err(Error::Invalid("selected monomial atlas index"));
        }
        b.reserve_slots(1)?;
        return Ok(MonomialCenterRestriction {
            selection,
            index,
            state: SelectedCenterState::EmptyAmbient,
            ideal: None,
            cosupport: None,
        });
    };
    let local = open.history().ledger().frame().local();
    let extension = open.open().extension();
    let ideal = match selection.native_proof() {
        MonomialProduction::Center(center) => extension.ideal(center.center(), b)?,
        MonomialProduction::EmptyCosupport { .. } => {
            Ideal::new(local.ring().clone(), vec![local.ring().one()], b)?
        }
        MonomialProduction::Incomplete { .. } => {
            return Err(Error::Invalid("incomplete monomial selection"));
        }
    };
    for f in ideal.generators() {
        local.supports(f)?;
    }
    let empty_selected =
        local
            .ideal()
            .sum(&ideal, b)?
            .contains(&local.ring().one(), local.unit_relations(), b)?;
    // Even a native global empty-cosupport outcome is locally reconstructed:
    // this getter is an actual proof of the current pulled marked source.
    let source = Arc::new(MarkedIdeal::new(
        extension.ideal(atlas.source().ideal(), b)?,
        atlas.source().mark(),
        b,
    )?);
    let cosupport = MarkedCosupport::prove(open.history().ledger().frame().clone(), source, b)?;
    if cosupport.empty() && !empty_selected {
        return Err(Error::Invalid("selected center outside marked cosupport"));
    }
    if matches!(
        selection.native_proof(),
        MonomialProduction::EmptyCosupport { .. }
    ) && !cosupport.empty()
    {
        return Err(Error::Invalid(
            "global monomial empty cosupport restriction",
        ));
    }
    let state = if cosupport.empty() {
        SelectedCenterState::EmptyCosupport
    } else if empty_selected {
        SelectedCenterState::EmptySelectedCenter
    } else {
        SelectedCenterState::SelectedCenter
    };
    b.reserve_slots(1)?;
    Ok(MonomialCenterRestriction {
        selection,
        index,
        state,
        ideal: Some(ideal),
        cosupport: Some(cosupport),
    })
}
