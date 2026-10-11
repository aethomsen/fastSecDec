use super::*;
impl CarriedPowerState {
    /// Consume a selected slot only after the full original hierarchy inventory
    /// and its actual first histories have been verified. Selection stays local.
    pub fn from_first(
        tree: Arc<OriginalTreePullback>,
        level_index: usize,
        open_index: usize,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let histories = tree.histories(b)?;
        let level = tree
            .levels()
            .get(level_index)
            .ok_or(Error::Invalid("carried power first level"))?;
        let slot = level
            .opens()
            .get(open_index)
            .ok_or(Error::Invalid("carried power first open"))?;
        let initial = slot
            .power()
            .cloned()
            .ok_or(Error::Invalid("carried power slot has no issued edge"))?;
        let history = histories[level_index][open_index].clone();
        let open = initial.original().open().clone();
        let original = initial.original().target().clone();
        let normalized = initial.normalized().target().clone();
        let effective = initial.effective().target().clone();
        if !Arc::ptr_eq(history.ledger().frame(), open.frame())
            || !Arc::ptr_eq(initial.original().support(), level.support())
        {
            return Err(Error::Invalid("carried power first frame/history"));
        }
        let unit = initial
            .unit()
            .generators()
            .first()
            .ok_or(Error::Invalid("carried power original unit transform"))?
            .quotient
            .clone();
        let power = initial
            .presentation()
            .power()
            .ok_or(Error::Invalid("carried power original presentation"))?
            .power();
        let normalizer = QuotientNormalizer::prepare(open.frame().local().clone(), b)?;
        if !normalizer.verify_candidate(&original, &normalized, b)? {
            return Err(Error::Invalid("carried power original normalization"));
        }
        check_identity(open.frame(), &normalized, &effective, &unit, power, b)?;
        let gp = initial.original_companion().target().clone();
        let gg = initial.effective_companion().target().clone();
        check_identity(open.frame(), &gp, &gg, &unit, power, b)?;
        let companion = CarriedPowerCompanion::from_proof(PowerCosupportProof::prove(
            open.frame().clone(),
            gp,
            gg,
            b,
        )?);
        let source_cosupport = PowerCosupportProof::prove(
            open.frame().clone(),
            original.clone(),
            effective.clone(),
            b,
        )?;
        Ok(Arc::new(Self {
            origin: PowerStateOrigin::First {
                tree: tree.clone(),
                level: level_index,
                open: open_index,
            },
            initial,
            chart: tree.chart().clone(),
            support: level.support().clone(),
            open,
            history,
            original,
            normalized,
            effective,
            unit,
            normalizer,
            companion,
            source_cosupport,
            depth: 0,
        }))
    }
}
