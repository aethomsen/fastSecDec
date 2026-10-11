use super::EndpointChartCover;
use crate::threshold::maps::{
    endpoint_transport::{EndpointChart, EndpointTransport, Error, Result, clearing::pull},
    normalized::{NormalizedPrefix, NormalizedSection},
};
use std::sync::Arc;
use symbolica::atom::{Atom, Symbol};

#[derive(Clone, Debug)]
enum Owner {
    Original(Arc<EndpointTransport>),
    Refined(Arc<EndpointChartCover>),
}
/// Sealed source view for an issued original chart or a complete-cover leaf.
/// It grants exact source/branch association, not real closed-face authority.
#[derive(Clone, Debug)]
pub struct IssuedEndpointChart {
    owner: Owner,
    ordinal: usize,
}
impl IssuedEndpointChart {
    pub fn transport(&self) -> &Arc<EndpointTransport> {
        match &self.owner {
            Owner::Original(p) => p,
            Owner::Refined(p) => p.parent(),
        }
    }
    pub fn parent_prefix(&self) -> &Arc<NormalizedPrefix> {
        self.transport().parent_prefix()
    }
    pub fn target_units(&self) -> &[Symbol] {
        match &self.owner {
            Owner::Original(p) => p.target_units(),
            Owner::Refined(p) => p.target_units(),
        }
    }
    /// The chart's `unit_images()` are polynomial maps from the closed target
    /// cube into the closed ancestor unit cube: original nonnegative sector
    /// monomials/reflections composed with half maps and nonnegative child
    /// monomials. Retained normalized coefficient-range/minor certificates
    /// therefore pull back by continuous composition without a new root solve.
    /// This property does not extend to `source_images()`, whose rays may have
    /// poles on the boundary, or assert closed regularity of physical factors.
    pub fn chart(&self) -> &EndpointChart {
        match &self.owner {
            Owner::Original(p) => &p.charts()[self.ordinal],
            Owner::Refined(p) => p.leaves()[self.ordinal].chart(),
        }
    }
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }
    pub fn cover(&self) -> Option<&Arc<EndpointChartCover>> {
        match &self.owner {
            Owner::Original(_) => None,
            Owner::Refined(p) => Some(p),
        }
    }
    /// Actual retained section parameter images composed through this chart.
    /// Closed coefficient-box containment is a separate consumer obligation.
    pub fn section_parameter_images(&self, section: &Arc<NormalizedSection>) -> Result<Vec<Atom>> {
        if !self
            .transport()
            .root_relations()
            .iter()
            .any(|r| Arc::ptr_eq(r.section(), section))
        {
            return Err(Error::Invalid(
                "section is not owned by this issued chart".into(),
            ));
        }
        let units = self.parent_prefix().source().unit_coordinates();
        Ok(section
            .chart()
            .parameter_images
            .iter()
            .map(|p| pull(p, units, self.chart().unit_images()))
            .collect())
    }
}
impl EndpointTransport {
    pub fn issued_chart(self: &Arc<Self>, ordinal: usize) -> Result<IssuedEndpointChart> {
        if ordinal >= self.charts().len() {
            return Err(Error::Invalid("endpoint chart ordinal out of range".into()));
        }
        Ok(IssuedEndpointChart {
            owner: Owner::Original(self.clone()),
            ordinal,
        })
    }
}
impl EndpointChartCover {
    pub fn issued_chart(self: &Arc<Self>, ordinal: usize) -> Result<IssuedEndpointChart> {
        if ordinal >= self.leaves().len() {
            return Err(Error::Invalid("refined chart ordinal out of range".into()));
        }
        Ok(IssuedEndpointChart {
            owner: Owner::Refined(self.clone()),
            ordinal,
        })
    }
}
