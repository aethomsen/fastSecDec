use super::*;
use crate::generation::{EndpointProfileRow, GenerationOptions};

pub struct ContinuedProjectivePairFamily<'a> {
    certificate: &'a ResolvedProjectivePairFamily,
    charts: Vec<Atom>,
    expression: Atom,
    profiles: Vec<Vec<EndpointProfileRow>>,
    options: GenerationOptions,
}
impl ResolvedProjectivePairFamily {
    pub fn continue_symbolically(
        &self,
        options: &GenerationOptions,
        mut observer: impl FnMut(Progress) -> ControlFlow<()>,
    ) -> Result<ContinuedProjectivePairFamily<'_>> {
        if options.mode != crate::generation::GenerationMode::Symbolic
            || options.contour_enabled()
            || options.contour_jacobian != crate::contour::ContourJacobian::Symbolic
            || options.source_sectors.is_some()
        {
            return Err(unsupported(
                "complete symbolic endpoint continuation without contour/source selection required",
            ));
        }
        let mut charts = Vec::new();
        let mut profiles = Vec::new();
        for (i, chart) in self.charts.iter().enumerate() {
            if observer(Progress::Continue(i)).is_break() {
                return Err(Error::Cancelled);
            }
            let continued = crate::generation::threshold_subtract(
                chart
                    .terms
                    .iter()
                    .map(|t| (t.prefactor.clone(), t.regular.clone(), t.powers.clone()))
                    .collect(),
                &self.coordinates,
                &self.regulators,
                options,
            )
            .map_err(native)?;
            charts.push(continued.expression);
            profiles.push(continued.endpoint_profiles);
        }
        Ok(ContinuedProjectivePairFamily {
            certificate: self,
            expression: charts.iter().cloned().sum(),
            charts,
            profiles,
            options: options.clone(),
        })
    }
}
impl ContinuedProjectivePairFamily<'_> {
    pub fn certificate(&self) -> &ResolvedProjectivePairFamily {
        self.certificate
    }
    pub fn charts(&self) -> &[Atom] {
        &self.charts
    }
    pub fn expression(&self) -> &Atom {
        &self.expression
    }
    pub fn profiles(&self) -> &[Vec<EndpointProfileRow>] {
        &self.profiles
    }
    pub fn options(&self) -> &GenerationOptions {
        &self.options
    }
    pub(crate) fn roots(&self, chart: usize) -> Scope {
        self.certificate.charts[chart].roots.clone()
    }
    pub(crate) fn definitions(
        &self,
        _chart: usize,
    ) -> Vec<crate::threshold::continued::Definition> {
        Vec::new()
    }
}
