//! A complete physical factor inventory and exact original-G completion.
use super::super::super::*;
use super::super::helpers::Result;
use super::lineage::AmbientCycleCarry;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub enum AmbientDropRecord {
    Empty(ComponentResidualProduction),
    Zero(Arc<ComponentResidualOrder>),
    Positive(Arc<ComponentResidualOrder>),
    Pending(ComponentResidualProduction),
}
#[derive(Clone, Debug)]
pub struct AmbientDropInventory {
    carry: Arc<AmbientCycleCarry>,
    factors: Arc<CompletedComponentFactors>,
    records: Vec<AmbientDropRecord>,
    maximum: Option<usize>,
    zero_component: bool,
}
impl AmbientDropInventory {
    pub fn prove(
        carry: Arc<AmbientCycleCarry>,
        factors: Arc<CompletedComponentFactors>,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        let chart = carry.chart();
        let root = factors
            .nodes()
            .get(&Vec::new())
            .ok_or(Error::Invalid("ambient drop missing factor root"))?;
        if !Arc::ptr_eq(root.data().source(), chart.parent().target())
            || !Arc::ptr_eq(root.data().history(), chart.history())
        {
            return Err(Error::Invalid(
                "ambient drop actual original source factors",
            ));
        }
        if carry.companions().is_empty()
            || carry.companions().iter().any(|g| !g.cosupport().empty())
        {
            return Err(Error::Invalid("ambient drop original companion unresolved"));
        }
        b.reserve_slots(factors.nodes().len())?;
        let mut records = Vec::new();
        let mut maximum = Some(0usize);
        let mut zero_component = false;
        for node in factors.nodes().values() {
            if node.zero_ideal().is_some() {
                zero_component = true;
                maximum = None;
            }
            let Some(leaf) = node.leaf() else { continue };
            let data = leaf.data();
            if !Arc::ptr_eq(data.source(), chart.parent().target())
                || !data.history().same_root(chart.history())
                || !data
                    .history()
                    .chart_path()
                    .starts_with(chart.history().chart_path())
                || data.history().births() != chart.history().births()
            {
                return Err(Error::Invalid("ambient drop factor ancestry"));
            }
            match produce_component_residual_order(leaf.clone(), b)? {
                ComponentResidualProduction::Order(order) => {
                    let order = Arc::new(*order);
                    let m = order.algebraic_maximum_on_cosupport();
                    if let Some(value) = &mut maximum {
                        *value = (*value).max(m);
                    }
                    records.push(if m == 0 {
                        AmbientDropRecord::Zero(order)
                    } else {
                        AmbientDropRecord::Positive(order)
                    });
                }
                p @ ComponentResidualProduction::EmptyCosupport { .. } => {
                    records.push(AmbientDropRecord::Empty(p))
                }
                p => {
                    maximum = None;
                    records.push(AmbientDropRecord::Pending(p));
                }
            }
        }
        let old = carry
            .original()
            .center()
            .coefficient()
            .source()
            .order()
            .algebraic_maximum_on_cosupport();
        if maximum.is_some_and(|m| m >= old) {
            return Err(Error::Invalid(
                "ambient residual maximum did not strictly drop",
            ));
        }
        Ok(Arc::new(Self {
            carry,
            factors,
            records,
            maximum,
            zero_component,
        }))
    }
    pub fn carry(&self) -> &Arc<AmbientCycleCarry> {
        &self.carry
    }
    pub fn factors(&self) -> &Arc<CompletedComponentFactors> {
        &self.factors
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
    pub fn positive_drop(self: &Arc<Self>, index: usize) -> Result<Arc<AmbientResidualDrop>> {
        if self.maximum.is_none() {
            return Err(Error::Invalid("ambient residual inventory incomplete"));
        }
        let Some(AmbientDropRecord::Positive(current)) = self.records.get(index) else {
            return Err(Error::Invalid("ambient positive residual record required"));
        };
        if !Arc::ptr_eq(
            current.factor().data().history(),
            self.carry.chart().history(),
        ) {
            return Err(Error::ResourceIncomplete(
                "ambient residual factor needs physical localization",
            ));
        }
        Ok(Arc::new(AmbientResidualDrop {
            inventory: self.clone(),
            current: current.clone(),
        }))
    }
}
#[derive(Clone, Debug)]
pub struct AmbientResidualDrop {
    inventory: Arc<AmbientDropInventory>,
    current: Arc<ComponentResidualOrder>,
}
impl AmbientResidualDrop {
    pub fn inventory(&self) -> &Arc<AmbientDropInventory> {
        &self.inventory
    }
    pub fn current(&self) -> &Arc<ComponentResidualOrder> {
        &self.current
    }
}
