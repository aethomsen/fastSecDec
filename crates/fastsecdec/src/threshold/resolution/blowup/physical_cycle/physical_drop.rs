//! A fresh companion only after the active G completes and the full residual
//! maximum strictly drops. Retired G is neither transformed nor rechecked here.
use super::super::super::*;
use super::super::helpers::Result;
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct PhysicalDropInventory {
    state: Arc<OriginalPhysicalState>,
    factors: Arc<CompletedComponentFactors>,
    companion: Arc<RelativeMarkedTransform>,
    cosupport: Arc<MarkedCosupport>,
    records: Vec<AmbientDropRecord>,
    maximum: Option<usize>,
    zero_component: bool,
}
impl PhysicalDropInventory {
    pub fn prove(
        state: Arc<OriginalPhysicalState>,
        factors: Arc<CompletedComponentFactors>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let original = state
            .active_original_cycle()
            .ok_or(Error::ResourceIncomplete(
                "bootstrap drop uses original ambient completion",
            ))?;
        let old = original
            .center()
            .coefficient()
            .source()
            .order()
            .algebraic_maximum_on_cosupport();
        let companion = state.prepare_active_companion(namespace, b)?;
        if !Arc::ptr_eq(companion.geometry(), state.chart().geometry()) {
            return Err(Error::Invalid("physical active companion geometry"));
        }
        let cosupport = MarkedCosupport::prove(
            state.chart().geometry().frame().clone(),
            companion.target().clone(),
            b,
        )?;
        if !cosupport.empty() {
            return Err(Error::Invalid(
                "physical residual drop active companion unresolved",
            ));
        }
        let (records, maximum, zero_component) = super::drop::prove_residual_inventory(
            state.chart().parent().target(),
            state.chart().history(),
            old,
            &factors,
            b,
        )?;
        Ok(Arc::new(Self {
            state,
            factors,
            companion,
            cosupport,
            records,
            maximum,
            zero_component,
        }))
    }
    pub fn state(&self) -> &Arc<OriginalPhysicalState> {
        &self.state
    }
    pub fn factors(&self) -> &Arc<CompletedComponentFactors> {
        &self.factors
    }
    pub fn companion(&self) -> &Arc<RelativeMarkedTransform> {
        &self.companion
    }
    pub fn cosupport(&self) -> &Arc<MarkedCosupport> {
        &self.cosupport
    }
    pub fn records(&self) -> &[AmbientDropRecord] {
        &self.records
    }
    pub fn proved_maximum(&self) -> Option<usize> {
        self.maximum
    }
    pub fn unresolved_zero_component(&self) -> bool {
        self.zero_component
    }
    pub fn positive_drop(self: &Arc<Self>, index: usize) -> Result<Arc<PhysicalResidualDrop>> {
        if self.maximum.is_none() {
            return Err(Error::Invalid("physical residual inventory incomplete"));
        }
        let Some(AmbientDropRecord::Positive(current)) = self.records.get(index) else {
            return Err(Error::Invalid("physical positive residual record required"));
        };
        if !Arc::ptr_eq(
            current.factor().data().history(),
            self.state.chart().history(),
        ) {
            return Err(Error::ResourceIncomplete(
                "physical residual factor needs full source localization",
            ));
        }
        Ok(Arc::new(PhysicalResidualDrop {
            inventory: self.clone(),
            current: current.clone(),
        }))
    }
}
#[derive(Clone, Debug)]
pub struct PhysicalResidualDrop {
    inventory: Arc<PhysicalDropInventory>,
    current: Arc<ComponentResidualOrder>,
}
impl PhysicalResidualDrop {
    pub fn inventory(&self) -> &Arc<PhysicalDropInventory> {
        &self.inventory
    }
    pub fn current(&self) -> &Arc<ComponentResidualOrder> {
        &self.current
    }
}
/// Exact ownership adapter for a genuinely new positive residual cycle.
#[derive(Clone, Debug)]
pub struct PhysicalContinuationCenter {
    drop: Arc<PhysicalResidualDrop>,
    center: Arc<CompanionCenter>,
    source_open: Arc<LocalizedCoverHistory>,
}
impl PhysicalContinuationCenter {
    pub fn prepare(
        drop: Arc<PhysicalResidualDrop>,
        center: Arc<CompanionCenter>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let cycle = center
            .coefficient()
            .coefficient()
            .cycle()
            .ok_or(Error::Invalid("physical continuation missing cycle"))?;
        if !matches!(cycle.origin(),CycleOrigin::AfterPhysicalDrop(d)if Arc::ptr_eq(d,&drop))
            || !Arc::ptr_eq(center.coefficient().source().order(), drop.current())
        {
            return Err(Error::Invalid("physical continuation actual residual drop"));
        }
        let source_open = center.coefficient().source().restriction().clone();
        let prior = drop.inventory().state().chart();
        if !Arc::ptr_eq(source_open.open().source(), prior.history())
            || !Arc::ptr_eq(center.frame(), source_open.history().ledger().frame())
            || source_open.history().births() != prior.history().births()
            || !source_open.history().same_root(prior.history())
        {
            return Err(Error::Invalid("physical continuation actual localization"));
        }
        let expected = source_open
            .open()
            .extension()
            .ideal(prior.parent().target().ideal(), b)?;
        if center.parent_source().mark() != prior.parent().target().mark()
            || center.parent_source().ideal() != &expected
        {
            return Err(Error::Invalid(
                "physical continuation original marked restriction",
            ));
        }
        Ok(Arc::new(Self {
            drop,
            center,
            source_open,
        }))
    }
    pub fn residual_drop(&self) -> &Arc<PhysicalResidualDrop> {
        &self.drop
    }
    pub fn center(&self) -> &Arc<CompanionCenter> {
        &self.center
    }
    pub fn source_open(&self) -> &Arc<LocalizedCoverHistory> {
        &self.source_open
    }
}
