use super::*;
use std::time::Duration;

/// Native limits are not serde owners; this DTO stores only their scalar policy.
/// Exact margin and coordinate roles remain in the shared native Atom/state table.
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredRegular {
    version: u32,
    coordinates: usize,
    discovery: DiscoveryLimits,
    degree: usize,
    terms: usize,
    rational_bits: u64,
    refinements: usize,
    wall_time: Duration,
    certificates: symgcad::domain::CertificateOptions,
    source_nodes: usize,
    source_depth: usize,
    density_degree: u16,
    density_terms: usize,
    integer_power: i64,
    witness_candidates: usize,
}
impl StoredRegular {
    pub(super) fn new(settings: &RegularFamilyPreparation) -> Result<Self> {
        let coordinates = settings
            .coordinates
            .as_ref()
            .ok_or(Error::Association("unresolved regular coordinates"))?;
        let g = &settings.limits.geometry;
        let d = settings.limits.density;
        Ok(Self {
            version: 1,
            coordinates: coordinates.len(),
            discovery: settings.discovery.clone(),
            degree: g.max_degree,
            terms: g.max_terms,
            rational_bits: g.max_rational_bits,
            refinements: g.max_refinements,
            wall_time: g.wall_time,
            certificates: g.certificates.clone(),
            source_nodes: d.source_nodes,
            source_depth: d.source_depth,
            density_degree: d.degree,
            density_terms: d.terms,
            integer_power: d.integer_power,
            witness_candidates: d.witness_candidates,
        })
    }
    pub(super) fn coordinate_count(&self) -> Result<usize> {
        if self.version != 1 || self.coordinates < 2 {
            return Err(Error::Association(
                "unsupported regular-family configuration version/dimension",
            ));
        }
        Ok(self.coordinates)
    }
    pub(super) fn restore(
        self,
        coordinates: Vec<Symbol>,
        margin: &Atom,
    ) -> Result<RegularFamilyPreparation> {
        if coordinates.len() != self.coordinate_count()? {
            return Err(Error::Association("regular-family coordinate table"));
        }
        let unit_margin = Rational::try_from(margin.as_view())
            .map_err(|_| Error::Association("nonrational regular margin"))?;
        Ok(RegularFamilyPreparation {
            coordinates: Some(coordinates),
            discovery: self.discovery,
            limits: regularization::secant::Limits {
                geometry: crate::threshold::maps::regular::Limits {
                    max_degree: self.degree,
                    max_terms: self.terms,
                    max_rational_bits: self.rational_bits,
                    max_refinements: self.refinements,
                    wall_time: self.wall_time,
                    certificates: self.certificates,
                },
                density: regularization::meromorphic::Limits {
                    source_nodes: self.source_nodes,
                    source_depth: self.source_depth,
                    degree: self.density_degree,
                    terms: self.density_terms,
                    integer_power: self.integer_power,
                    witness_candidates: self.witness_candidates,
                },
                unit_margin,
                cancellation: None,
            },
        })
    }
}
