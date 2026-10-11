//! Original recursive coefficient hierarchy, issued before physical transport.
//! Later carried ideals cannot manufacture these original construction owners.
use super::super::*;
use super::{embedding::SupportEmbedding, helpers::Result};
use std::sync::Arc;
mod power;
mod transport;
pub use power::PowerLevelTransform;
pub use transport::{OriginalLevelOpen, OriginalLevelPullback, OriginalTreePullback};

#[derive(Clone, Debug)]
pub struct OriginalLowerLevel {
    center: Arc<RecursiveCenter>,
    embedding: Arc<SupportEmbedding>,
    history: Arc<ResolutionHistory>,
    companion: Option<Arc<MarkedIdeal>>,
    effective_companion: Option<Arc<MarkedIdeal>>,
}
impl OriginalLowerLevel {
    pub fn center(&self) -> &Arc<RecursiveCenter> {
        &self.center
    }
    pub fn embedding(&self) -> &Arc<SupportEmbedding> {
        &self.embedding
    }
    pub fn history(&self) -> &Arc<ResolutionHistory> {
        &self.history
    }
    pub fn companion(&self) -> Option<&Arc<MarkedIdeal>> {
        self.companion.as_ref()
    }
    pub fn effective_companion(&self) -> Option<&Arc<MarkedIdeal>> {
        self.effective_companion.as_ref()
    }
    pub fn normalization(&self) -> Option<&Arc<VerifiedQuotientNormalization>> {
        self.center
            .lift_receipt()
            .map(|r| r.level().normalization())
    }
}
#[derive(Clone, Debug)]
pub struct OriginalRecursionTree {
    center: Arc<CompanionCenter>,
    levels: Vec<Arc<OriginalLowerLevel>>,
}
impl OriginalRecursionTree {
    /// Carry every original level through its actual first physical blowup.
    /// All strict-support and common-refinement opens remain in the receipt.
    pub fn pull_first(
        self: &Arc<Self>,
        chart: Arc<RelativeRecursiveChart>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<OriginalTreePullback>> {
        OriginalTreePullback::prepare(self.clone(), chart, namespace, b)
    }
    pub(crate) fn new(
        center: Arc<CompanionCenter>,
        root: Arc<ResolutionHistory>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !Arc::ptr_eq(root.ledger().frame(), center.child().frame())
            || root.stage() != 0
            || !root.ledger().divisors().is_empty()
            || !root.births().is_empty()
            || !root.chart_path().is_empty()
        {
            return Err(Error::Invalid(
                "original recursive hierarchy initial lower root",
            ));
        }
        let mut current = center.child().clone();
        let mut history = root;
        let mut embedding = SupportEmbedding::contact(center.coefficient().contact().clone(), b)?;
        let mut levels = Vec::new();
        loop {
            b.count(1)?;
            b.reserve_slots(1)?;
            if !Arc::ptr_eq(current.frame(), embedding.frame())
                || !Arc::ptr_eq(history.ledger().frame(), current.frame())
            {
                return Err(Error::Invalid(
                    "original recursive hierarchy frame ownership",
                ));
            }
            // Independently retain the exact restriction of the PHYSICAL
            // center at each level, not just an isomorphic universal property.
            let restricted = embedding.extension().ideal(center.ideal(), b)?;
            let local = current.frame().local();
            for (a, z) in [
                (&restricted, current.ideal()),
                (current.ideal(), &restricted),
            ] {
                let target = local.ideal().sum(z, b)?;
                for f in a.generators() {
                    if !target.contains(f, local.unit_relations(), b)? {
                        return Err(Error::Invalid("original recursive center restriction"));
                    }
                }
            }
            let Some(receipt) = current.lift_receipt() else {
                for f in current.source().ideal().generators() {
                    if !local.zero(f, b)? {
                        return Err(Error::Invalid(
                            "original recursive terminal is not zero auxiliary ideal",
                        ));
                    }
                }
                if !current.ideal().generators().is_empty() {
                    return Err(Error::Invalid("original recursive terminal center"));
                }
                levels.push(Arc::new(OriginalLowerLevel {
                    center: current,
                    embedding,
                    history,
                    companion: None,
                    effective_companion: None,
                }));
                break;
            };
            let level = receipt.level();
            let q = level.contact();
            level
                .presentation()
                .verify(current.frame(), current.source())?;
            if let Some(origin) = level.presentation().origin()
                && !Arc::ptr_eq(origin.history(), &history)
            {
                return Err(Error::Invalid(
                    "original power hierarchy changed its issued history",
                ));
            }
            if !Arc::ptr_eq(level.source(), current.source())
                || !Arc::ptr_eq(level.normalization().source(), current.source())
                || level.presentation().effective_source().ideal() != q.source().source().as_ref()
                || !Arc::ptr_eq(q.source().frame(), current.frame())
                || !Arc::ptr_eq(receipt.child().frame(), q.contact())
                || receipt.child().source().ideal() != q.differential_coefficient().ideal()
                || receipt.child().source().mark() != q.differential_coefficient().mark()
            {
                return Err(Error::Invalid(
                    "original recursive normalization/contact/child association",
                ));
            }
            let effective_companion = Arc::new(MarkedIdeal::new(
                (**q.source().source()).clone(),
                q.source().algebraic_maximum_order(),
                b,
            )?);
            let companion = if level.presentation().power().is_some() {
                Arc::new(MarkedIdeal::new(
                    level.normalization().normalized().ideal().clone(),
                    level
                        .presentation()
                        .original_maximum_order(q.source().algebraic_maximum_order())?,
                    b,
                )?)
            } else {
                effective_companion.clone()
            };
            let next_embedding = SupportEmbedding::nested(embedding.clone(), q.clone(), b)?;
            let next_history = match verify_initial_relative_snc(q.contact().clone(), vec![], b)? {
                SncProduction::Verified(ledger) => ResolutionHistory::initial(ledger)?,
                SncProduction::Incomplete { reason, .. } => {
                    return Err(Error::ResourceIncomplete(reason));
                }
                SncProduction::Unresolved { .. } => {
                    return Err(Error::Invalid("original recursive empty boundary frame"));
                }
            };
            let next = receipt.child().clone();
            levels.push(Arc::new(OriginalLowerLevel {
                center: current,
                embedding,
                history,
                companion: Some(companion),
                effective_companion: Some(effective_companion),
            }));
            current = next;
            embedding = next_embedding;
            history = next_history;
        }
        Ok(Arc::new(Self { center, levels }))
    }
    pub fn center(&self) -> &Arc<CompanionCenter> {
        &self.center
    }
    pub fn levels(&self) -> &[Arc<OriginalLowerLevel>] {
        &self.levels
    }
}

mod anchor;
pub use anchor::{AnchoredOriginalTree, OriginalAnchorCenter};
