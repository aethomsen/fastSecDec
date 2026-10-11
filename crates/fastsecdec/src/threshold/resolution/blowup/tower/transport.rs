//! One physical blowup transports every ORIGINAL level. Common minor covers
//! relate their frames; no lower level is independently blown up or restarted.
use super::super::{
    coefficient::{SupportedMarkedTransform, controlled_supported},
    refinement::RefinedSupportCover,
    support::{StrictContactSupport, transport_embedding},
};
use super::*;

#[derive(Clone, Debug)]
pub struct OriginalLevelOpen {
    source: Arc<SupportedMarkedTransform>,
    companion: Option<Arc<SupportedMarkedTransform>>,
    power: Option<Arc<PowerLevelTransform>>,
    center: Arc<SupportedMarkedTransform>,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn power_hierarchy_refuses_missing_receipt_and_restarted_history() {
        let mut b = Budget::new(Limits::default());
        let center = super::super::super::super::recursive::power_tests::cusp_center(&mut b);
        let origin = EmbeddedChildCycle::new(center.clone(), &mut b).unwrap();
        let SncProduction::Verified(ledger) =
            verify_initial_relative_snc(center.child().frame().clone(), vec![], &mut b).unwrap()
        else {
            panic!()
        };
        let restarted = ResolutionHistory::initial(ledger).unwrap();
        assert!(matches!(
            OriginalRecursionTree::new(center.clone(), restarted, &mut b),
            Err(Error::Invalid(_))
        ));
        let checked =
            CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(center), &mut b).unwrap();
        let adapted = adapt_recursive_center(checked, "power_mutation_adapt", &mut b).unwrap();
        let maps = blowup_recursive_center(adapted, "power_mutation_maps", &mut b).unwrap();
        for (i, chart) in maps.charts().iter().enumerate() {
            let tree = origin
                .original_tree()
                .pull_first(chart.clone(), &format!("power_mutation_tree{i}"), &mut b)
                .unwrap();
            for (j, level) in tree.levels.iter().enumerate() {
                if let Some(k) = level.opens.iter().position(|o| o.power.is_some()) {
                    let mut missing = tree.as_ref().clone();
                    let mut level = level.as_ref().clone();
                    let mut open = level.opens[k].as_ref().clone();
                    open.power = None;
                    level.opens[k] = Arc::new(open);
                    missing.levels[j] = Arc::new(level);
                    assert!(matches!(missing.histories(&mut b), Err(Error::Invalid(_))));
                    return;
                }
            }
        }
        panic!("missing genuine transformed power open");
    }
    #[test]
    fn original_hierarchy_rejects_missing_or_foreign_level_receipts() {
        let mut b = Budget::new(Limits {
            max_operations: 2_000_000,
            max_total_ideal_slots: 200_000,
            ..Limits::default()
        });
        let center = super::super::super::relative_tests::nonmonomial_probe::nested_source_centers(
            1, &mut b,
        )
        .remove(0);
        let origin = EmbeddedChildCycle::new(center.clone(), &mut b).unwrap();
        let foreign = EmbeddedChildCycle::new(center.clone(), &mut b).unwrap();
        let checked =
            CheckedRecursiveCenter::new(RecursiveCenterOrigin::Companion(center), &mut b).unwrap();
        let cover = adapt_recursive_center(checked, "tree_mutation_adapt", &mut b).unwrap();
        let blowup = blowup_recursive_center(cover, "tree_mutation_blowup", &mut b).unwrap();
        let chart = blowup.charts()[0].clone();
        let mut exhausted = Budget::new(Limits {
            max_operations: 0,
            ..Limits::default()
        });
        assert!(matches!(
            OriginalTreePullback::prepare(
                origin.original_tree().clone(),
                chart.clone(),
                "tree_mutation_refused",
                &mut exhausted
            ),
            Err(Error::ResourceIncomplete(_))
        ));
        let source = OriginalTreePullback::prepare(
            origin.original_tree().clone(),
            chart,
            "tree_mutation_pull",
            &mut b,
        )
        .unwrap();
        let mut missing = source.as_ref().clone();
        missing.levels.pop();
        assert!(matches!(missing.histories(&mut b), Err(Error::Invalid(_))));
        let mut other = source.as_ref().clone();
        let mut level = other.levels[0].as_ref().clone();
        level.original = foreign.original_tree().levels()[0].clone();
        other.levels[0] = Arc::new(level);
        assert!(matches!(other.histories(&mut b), Err(Error::Invalid(_))));
        let mut missing = source.as_ref().clone();
        missing.refinements.clear();
        assert!(matches!(missing.histories(&mut b), Err(Error::Invalid(_))));
    }
}
impl OriginalLevelOpen {
    pub fn source(&self) -> &Arc<SupportedMarkedTransform> {
        &self.source
    }
    pub fn companion(&self) -> Option<&Arc<SupportedMarkedTransform>> {
        self.companion.as_ref()
    }
    pub fn power(&self) -> Option<&Arc<PowerLevelTransform>> {
        self.power.as_ref()
    }
    pub fn center(&self) -> &Arc<SupportedMarkedTransform> {
        &self.center
    }
}
#[derive(Clone, Debug)]
pub struct OriginalLevelPullback {
    original: Arc<OriginalLowerLevel>,
    support: Arc<StrictContactSupport>,
    opens: Vec<Arc<OriginalLevelOpen>>,
}
impl OriginalLevelPullback {
    pub fn original(&self) -> &Arc<OriginalLowerLevel> {
        &self.original
    }
    pub fn support(&self) -> &Arc<StrictContactSupport> {
        &self.support
    }
    pub fn opens(&self) -> &[Arc<OriginalLevelOpen>] {
        &self.opens
    }
}
#[derive(Clone, Debug)]
pub struct OriginalTreePullback {
    original: Arc<OriginalRecursionTree>,
    anchor: Option<Arc<AnchoredOriginalTree>>,
    chart: Arc<RelativeRecursiveChart>,
    levels: Vec<Arc<OriginalLevelPullback>>,
    refinements: Vec<Arc<RefinedSupportCover>>,
}
impl OriginalTreePullback {
    pub(crate) fn prepare(
        original: Arc<OriginalRecursionTree>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        match chart.geometry().center().origin() {
            RecursiveCenterOrigin::Companion(c) if Arc::ptr_eq(c, original.center()) => {}
            _ => {
                return Err(Error::Invalid(
                    "original hierarchy first physical transition owner",
                ));
            }
        }
        Self::prepare_inner(original, None, chart, namespace, b)
    }
    pub(super) fn prepare_anchored(
        anchor: Arc<AnchoredOriginalTree>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        anchor.check_chart(&chart)?;
        Self::prepare_inner(anchor.original().clone(), Some(anchor), chart, namespace, b)
    }
    fn prepare_inner(
        original: Arc<OriginalRecursionTree>,
        anchor: Option<Arc<AnchoredOriginalTree>>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        b.reserve_slots(original.levels().len())?;
        let mut levels = Vec::new();
        for (i, level) in original.levels().iter().enumerate() {
            let support = transport_embedding(
                chart.geometry().clone(),
                anchor
                    .as_ref()
                    .map_or_else(|| level.embedding().clone(), |a| a.embeddings()[i].clone()),
                &format!("{namespace}_support{i}"),
                b,
            )?;
            let center_source = Arc::new(MarkedIdeal::new(level.center().ideal().clone(), 1, b)?);
            let mut opens = Vec::new();
            b.reserve_slots(support.opens().len())?;
            for (j, open) in support.opens().iter().enumerate() {
                let source = controlled_supported(
                    support.clone(),
                    open.clone(),
                    level.center().source().clone(),
                    &format!("{namespace}_source{i}_{j}"),
                    b,
                )?;
                let companion = level
                    .companion()
                    .map(|g| {
                        controlled_supported(
                            support.clone(),
                            open.clone(),
                            g.clone(),
                            &format!("{namespace}_G{i}_{j}"),
                            b,
                        )
                    })
                    .transpose()?;
                let center = controlled_supported(
                    support.clone(),
                    open.clone(),
                    center_source.clone(),
                    &format!("{namespace}_center{i}_{j}"),
                    b,
                )?;
                let local = open.frame().local();
                if !local.ideal().sum(center.target().ideal(), b)?.contains(
                    &local.ring().one(),
                    local.unit_relations(),
                    b,
                )? {
                    return Err(Error::Invalid(
                        "hierarchy source center pullback not exceptional principal",
                    ));
                }
                let power = if level
                    .center()
                    .lift_receipt()
                    .is_some_and(|r| r.level().presentation().power().is_some())
                {
                    Some(PowerLevelTransform::prepare(
                        level,
                        support.clone(),
                        open.clone(),
                        source.clone(),
                        companion
                            .clone()
                            .ok_or(Error::Invalid("power hierarchy companion absent"))?,
                        &format!("{namespace}_power{i}_{j}"),
                        b,
                    )?)
                } else {
                    None
                };
                opens.push(Arc::new(OriginalLevelOpen {
                    source,
                    companion,
                    power,
                    center,
                }));
            }
            levels.push(Arc::new(OriginalLevelPullback {
                original: level.clone(),
                support,
                opens,
            }));
        }
        let mut refinements = Vec::new();
        b.reserve_slots(levels.len().saturating_sub(1))?;
        for i in 1..levels.len() {
            refinements.push(RefinedSupportCover::prepare(
                levels[i - 1].support.clone(),
                levels[i].support.clone(),
                &format!("{namespace}_refine{i}"),
                b,
            )?);
        }
        Ok(Arc::new(Self {
            original,
            anchor,
            chart,
            levels,
            refinements,
        }))
    }
    pub fn original(&self) -> &Arc<OriginalRecursionTree> {
        &self.original
    }
    pub fn chart(&self) -> &Arc<RelativeRecursiveChart> {
        &self.chart
    }
    pub fn levels(&self) -> &[Arc<OriginalLevelPullback>] {
        &self.levels
    }
    pub fn refinements(&self) -> &[Arc<RefinedSupportCover>] {
        &self.refinements
    }
    /// Verify the complete retained inventory and issue each induced history.
    pub fn histories(&self, b: &mut Budget) -> Result<Vec<Vec<Arc<ResolutionHistory>>>> {
        if self.levels.len() != self.original.levels.len()
            || self.refinements.len() != self.levels.len().saturating_sub(1)
        {
            return Err(Error::Invalid("original hierarchy complete inventory"));
        }
        if let Some(anchor) = &self.anchor {
            anchor.check_chart(&self.chart)?;
            if !Arc::ptr_eq(anchor.original(), &self.original)
                || anchor.embeddings().len() != self.levels.len()
            {
                return Err(Error::Invalid("anchored hierarchy inventory/source owner"));
            }
        }
        b.reserve_slots(self.levels.len())?;
        let mut out = Vec::new();
        for (i, level) in self.levels.iter().enumerate() {
            if !Arc::ptr_eq(&level.original, &self.original.levels[i])
                || !Arc::ptr_eq(
                    level.support.embedding(),
                    self.anchor
                        .as_ref()
                        .map_or(level.original.embedding(), |a| &a.embeddings()[i]),
                )
                || !Arc::ptr_eq(level.support.geometry(), self.chart.geometry())
                || level.opens.len() != level.support.opens().len()
            {
                return Err(Error::Invalid(
                    "original hierarchy level/source/physical ownership",
                ));
            }
            if i > 0
                && (!Arc::ptr_eq(
                    self.refinements[i - 1].parent(),
                    &self.levels[i - 1].support,
                ) || !Arc::ptr_eq(self.refinements[i - 1].child(), &level.support))
            {
                return Err(Error::Invalid(
                    "original hierarchy common-refinement ownership",
                ));
            }
            b.reserve_slots(level.opens.len())?;
            let mut histories = Vec::new();
            for (j, open) in level.opens.iter().enumerate() {
                if !Arc::ptr_eq(open.source.source(), level.original.center().source())
                    || !Arc::ptr_eq(open.source.open(), &level.support.opens()[j])
                    || !Arc::ptr_eq(open.center.open(), open.source.open())
                {
                    return Err(Error::Invalid(
                        "original hierarchy actual source transform association",
                    ));
                }
                let power_required = level
                    .original
                    .center()
                    .lift_receipt()
                    .is_some_and(|r| r.level().presentation().power().is_some());
                if power_required != open.power.is_some()
                    || open.power.as_ref().is_some_and(|p| {
                        !open
                            .companion
                            .as_ref()
                            .is_some_and(|g| p.associated(&level.original, &open.source, g))
                    })
                {
                    return Err(Error::Invalid(
                        "original hierarchy controlled power inventory",
                    ));
                }
                histories.push(level.original.history().advance_first_embedded(
                    level.original.center(),
                    &self.chart,
                    &open.source,
                    &open.center,
                    b,
                )?);
            }
            out.push(histories);
        }
        Ok(out)
    }
}
