//! Optional accounting of issued auxiliary powers beside physical chart work.
//! Accounting completion is not marked-source completion or an integration atlas.
use super::super::super::*;
use super::super::helpers::Result;
use super::*;
use std::sync::Arc;
mod slots;
mod work;
pub use slots::{PhysicalPowerSlot, PhysicalPowerSlotKind};
pub(super) use work::AuxiliaryFrontier;
pub use work::{PhysicalAuxiliaryAdvance, PhysicalAuxiliaryWork, PhysicalPowerPreparation};

/// A previous auxiliary inventory retired by its actual source-bound cycle start.
/// It asserts no new native J completion and supplies no target controlled map.
#[derive(Clone, Debug)]
pub struct RetiredPowerInventory {
    source: Arc<PhysicalPowerInventory>,
    start: Arc<PhysicalCycleStart>,
}
impl RetiredPowerInventory {
    pub fn source(&self) -> &Arc<PhysicalPowerInventory> {
        &self.source
    }
    pub fn start(&self) -> &Arc<PhysicalCycleStart> {
        &self.start
    }
}
#[derive(Clone, Debug)]
enum InventoryOrigin {
    First(Arc<OriginalTreePullback>),
    Continued(Arc<PhysicalPowerInventory>),
}
#[derive(Clone, Debug)]
struct SlotGroup {
    begin: usize,
    end: usize,
}
/// A complete source-bound target accounting, containing both active and
/// completed sources. Its private construction inventories every original slot.
#[derive(Clone, Debug)]
pub struct PhysicalPowerInventory {
    state: Arc<OriginalPhysicalState>,
    origin: InventoryOrigin,
    slots: Vec<Arc<PhysicalPowerSlot>>,
    groups: Vec<SlotGroup>,
    retired: Option<Arc<RetiredPowerInventory>>,
}
impl PhysicalPowerInventory {
    pub fn state(&self) -> &Arc<OriginalPhysicalState> {
        &self.state
    }
    pub fn slots(&self) -> &[Arc<PhysicalPowerSlot>] {
        &self.slots
    }
    pub fn retired(&self) -> Option<&Arc<RetiredPowerInventory>> {
        self.retired.as_ref()
    }
    pub fn first_tree(&self) -> Option<&Arc<OriginalTreePullback>> {
        match &self.origin {
            InventoryOrigin::First(t) => Some(t),
            _ => None,
        }
    }
    pub fn prior(&self) -> Option<&Arc<Self>> {
        match &self.origin {
            InventoryOrigin::Continued(p) => Some(p),
            _ => None,
        }
    }
    /// True only for an actually constructed complete accounting; this does not
    /// mean all source J ideals have empty cosupport.
    pub fn complete_accounting(&self) -> bool {
        self.check_shape().is_ok()
    }
    pub fn active_sources(&self) -> usize {
        self.slots
            .iter()
            .filter(|s| matches!(s.kind(), PhysicalPowerSlotKind::Active(_)))
            .count()
    }
    fn check_shape(&self) -> Result<()> {
        let expected = match &self.origin {
            InventoryOrigin::First(t) => {
                if !Arc::ptr_eq(t.chart(), self.state.chart()) {
                    return Err(Error::Invalid("power inventory first chart"));
                }
                t.levels().iter().try_fold(0usize, |n, l| {
                    n.checked_add(l.opens().len().max(1))
                        .ok_or(Error::ResourceIncomplete("power first slot count"))
                })?
            }
            InventoryOrigin::Continued(p) => {
                let PhysicalLineageEdge::Continued { source, .. } = self.state.edge() else {
                    return Err(Error::Invalid("power inventory continued physical edge"));
                };
                if !Arc::ptr_eq(source, p.state()) {
                    return Err(Error::Invalid("power inventory predecessor state"));
                }
                p.slots.len()
            }
        };
        if self.groups.len() != expected {
            return Err(Error::Invalid("power inventory source slot count"));
        }
        let mut end = 0;
        for g in &self.groups {
            if g.begin != end || g.end <= g.begin || g.end > self.slots.len() {
                return Err(Error::Invalid("power inventory group coverage"));
            }
            end = g.end;
        }
        if end != self.slots.len() {
            return Err(Error::Invalid("power inventory omitted target slots"));
        }
        match &self.origin {
            InventoryOrigin::First(tree) => {
                let mut index = 0;
                for (level, l) in tree.levels().iter().enumerate() {
                    for slot_index in 0..l.opens().len().max(1) {
                        let open = if l.opens().is_empty() {
                            None
                        } else {
                            Some(slot_index)
                        };
                        let g = &self.groups[index];
                        if g.end != g.begin + 1
                            || !self.slots[g.begin].matches_first(tree, level, open)
                        {
                            return Err(Error::Invalid("power first complete slot association"));
                        }
                        index += 1;
                    }
                }
            }
            InventoryOrigin::Continued(prior) => {
                for (i, g) in self.groups.iter().enumerate() {
                    PhysicalPowerSlot::check_group(
                        &self.slots[g.begin..g.end],
                        &prior.slots[i],
                        &self.state,
                    )?;
                }
            }
        }
        for slot in &self.slots {
            slot.check(&self.state)?;
        }
        Ok(())
    }
}

/// Physical SNC termination justifies skipping irrelevant auxiliary transport.
/// The actual source recipe is retained; no target empty-support receipt exists.
#[derive(Clone, Debug)]
pub struct PhysicalPowerTerminal {
    terminal: Arc<PhysicalTerminalInventory>,
    source: AuxiliaryOrigin,
}
impl PhysicalPowerTerminal {
    pub fn terminal(&self) -> &Arc<PhysicalTerminalInventory> {
        &self.terminal
    }
    pub fn original_tree(&self) -> Option<&Arc<OriginalRecursionTree>> {
        self.source.first_tree()
    }
    pub fn prior_inventory(&self) -> Option<&Arc<PhysicalPowerInventory>> {
        self.source.prior()
    }
    pub fn retirement(&self) -> Option<&Arc<RetiredPowerInventory>> {
        self.source.retirement()
    }
}
#[derive(Clone, Debug)]
enum AuxiliaryOrigin {
    First(Arc<OriginalPhysicalProblem>),
    Continued(Arc<PhysicalPowerInventory>),
    Fresh {
        start: Arc<PhysicalCycleStart>,
        retired: Arc<RetiredPowerInventory>,
    },
}
impl AuxiliaryOrigin {
    fn first_tree(&self) -> Option<&Arc<OriginalRecursionTree>> {
        match self {
            Self::First(p) => Some(p.root().owner().original_tree()),
            Self::Fresh { start, .. } => Some(start.origin().original_tree()),
            _ => None,
        }
    }
    fn prior(&self) -> Option<&Arc<PhysicalPowerInventory>> {
        match self {
            Self::Continued(p) => Some(p),
            Self::Fresh { retired, .. } => Some(retired.source()),
            _ => None,
        }
    }
    fn retirement(&self) -> Option<&Arc<RetiredPowerInventory>> {
        match self {
            Self::Continued(p) => p.retired(),
            Self::Fresh { retired, .. } => Some(retired),
            _ => None,
        }
    }
    fn check_state(&self, state: &Arc<OriginalPhysicalState>) -> Result<()> {
        let good = match (self, state.edge()) {
            (Self::First(a), PhysicalLineageEdge::Initial { problem: b, .. }) => Arc::ptr_eq(a, b),
            (Self::Continued(p), PhysicalLineageEdge::Continued { source, .. }) => {
                Arc::ptr_eq(p.state(), source)
            }
            (Self::Fresh { start: a, .. }, PhysicalLineageEdge::FreshCycle { start: b, .. }) => {
                Arc::ptr_eq(a, b)
            }
            _ => false,
        };
        if !good {
            return Err(Error::Invalid("auxiliary exact physical cycle/source"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
