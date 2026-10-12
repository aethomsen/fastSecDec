use super::{
    EndpointTransport, Error, ExtractedFactor, FaceSide, FactorLink, Result, Work,
    clearing::{Ring, pull},
    native,
};
use crate::generation::{add_scaled_powers, coordinates_from_parts};
use fastsecdec_sectors::{ParametricDomain, SectorMap};
use symbolica::atom::{Atom, AtomCore, Symbol};

/// Powers for one original term, never a combined or dropped density row.
#[derive(Debug)]
pub struct TermPowers {
    source_index: usize,
    powers: Vec<Atom>,
}
impl TermPowers {
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn powers(&self) -> &[Atom] {
        &self.powers
    }
}
/// One chart of the retained complete native fan. Every closed face is pending;
/// the real selected root equations/ranges remain in the shared parent owner.
#[derive(Debug)]
pub struct EndpointChart {
    ordinal: usize,
    unit_images: Vec<Atom>,
    extra_jacobian: Atom,
    source_images: Vec<Atom>,
    root_equations: Vec<Atom>,
    factors: Vec<ExtractedFactor>,
    measure: ExtractedFactor,
    terms: Vec<TermPowers>,
}
impl EndpointChart {
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }
    /// Parent unit inputs in their original CAD/unit order, expressed in the
    /// transport target units. Native root callbacks must receive these inputs.
    pub fn unit_images(&self) -> &[Atom] {
        &self.unit_images
    }
    pub fn extra_jacobian(&self) -> &Atom {
        &self.extra_jacobian
    }
    /// Preparation/source order, with selected-root slots still composed.
    pub fn source_images(&self) -> &[Atom] {
        &self.source_images
    }
    pub fn root_equations(&self) -> &[Atom] {
        &self.root_equations
    }
    pub fn factors(&self) -> &[ExtractedFactor] {
        &self.factors
    }
    /// Parent positive measure times the absolute native sector determinant,
    /// factored exactly once in the full target-unit inventory.
    pub fn measure(&self) -> &ExtractedFactor {
        &self.measure
    }
    pub fn terms(&self) -> &[TermPowers] {
        &self.terms
    }
    /// Generators for ALL closed cube faces. Their compatible intersections
    /// also remain pending; no exponentially sized face list is materialized.
    pub fn pending_face_generators(&self) -> usize {
        2 * self.unit_images.len()
    }
    pub fn closed_faces_certified(&self) -> bool {
        false
    }
}
fn validate_map(owner: &EndpointTransport, map: &SectorMap) -> Result<()> {
    map.validate(ParametricDomain::UnitCube).map_err(native)?;
    if map.source_dimension() != owner.faces.len()
        || map.dimension() != owner.faces.len()
        || map.fixed_parameter.is_some()
        || map.factor_valuations.len() != owner.supports.len()
        || map
            .factor_valuations
            .iter()
            .any(|v| v.len() != owner.faces.len())
    {
        return Err(Error::Invalid(
            "native endpoint fan inventory dimensions differ".into(),
        ));
    }
    Ok(())
}
pub(super) fn prepare(
    owner: &EndpointTransport,
    ordinal: usize,
    work: &mut Work<'_>,
) -> Result<EndpointChart> {
    work.check()?;
    let map = &owner.geometry.sectors[ordinal];
    validate_map(owner, map)?;
    let selected_source = owner.faces.iter().map(|f| f.unit).collect::<Vec<_>>();
    let selected_target = owner
        .selected
        .iter()
        .map(|&i| owner.target_units[i])
        .collect::<Vec<_>>();
    let coordinates = coordinates_from_parts(
        &selected_source,
        ParametricDomain::UnitCube,
        map,
        &selected_target,
    );
    let mut unit_images = owner
        .target_units
        .iter()
        .copied()
        .map(Atom::var)
        .collect::<Vec<_>>();
    for ((face, &i), image) in owner
        .faces
        .iter()
        .zip(&owner.selected)
        .zip(coordinates.images())
    {
        unit_images[i] = if face.side == FaceSide::Upper {
            Atom::one() - image
        } else {
            image.clone()
        };
    }
    prepare_images(
        owner,
        ChartImages {
            ordinal,
            target_units: &owner.target_units,
            unit_images,
            extra_jacobian: coordinates.measure_jacobian().clone(),
            valuation: Some(ValuationCheck {
                selected: &owner.selected,
                support_indices: &owner.support_indices,
                map,
            }),
        },
        work,
    )
}

pub(super) struct ValuationCheck<'a> {
    pub selected: &'a [usize],
    pub support_indices: &'a [usize],
    pub map: &'a SectorMap,
}

pub(super) struct ChartImages<'a> {
    pub ordinal: usize,
    pub target_units: &'a [Symbol],
    pub unit_images: Vec<Atom>,
    pub extra_jacobian: Atom,
    pub valuation: Option<ValuationCheck<'a>>,
}

/// Shared sealed construction from the ancestor identities. Callers establish
/// open-map coverage and positive measure; this issues no closed-face proof.
pub(super) fn prepare_images(
    owner: &EndpointTransport,
    images: ChartImages<'_>,
    work: &mut Work<'_>,
) -> Result<EndpointChart> {
    let ChartImages {
        ordinal,
        target_units,
        unit_images,
        extra_jacobian,
        valuation,
    } = images;
    work.check()?;
    let parent_units = owner.parent.source().unit_coordinates();
    if target_units.len() != parent_units.len() || unit_images.len() != parent_units.len() {
        return Err(Error::Invalid(
            "composed chart coordinate inventory differs".into(),
        ));
    }
    let ring = Ring::new(
        target_units
            .iter()
            .copied()
            .chain(owner.roots.iter().map(|r| r.section.root_symbol()))
            .collect(),
    )?;
    let mut factors = Vec::with_capacity(owner.identities.len());
    for (index, identity) in owner.identities.iter().enumerate() {
        let n = pull(identity.numerator(), parent_units, &unit_images);
        let d = pull(identity.denominator(), parent_units, &unit_images);
        let (factor, minima) = ring.extract(&n, &d, target_units, work)?;
        if let Some(check) = &valuation {
            let expected = &check.map.factor_valuations[check.support_indices[index]];
            for (&axis, e) in check.selected.iter().zip(expected) {
                if *e != i64::from(minima[axis]) {
                    return Err(Error::Invalid(
                        "native support-index valuation differs from exact pullback".into(),
                    ));
                }
            }
        }
        factors.push(factor);
    }
    let identity = &owner.identities[owner.measure];
    let n = pull(identity.numerator(), parent_units, &unit_images) * &extra_jacobian;
    let d = pull(identity.denominator(), parent_units, &unit_images);
    let measure = ring.extract_measure(&n, &d, target_units, work)?;
    let preparation = owner.parent.source().decomposition().preparation();
    let mut terms = Vec::with_capacity(owner.links.len());
    for (source_index, (term, links)) in preparation.terms().iter().zip(&owner.links).enumerate() {
        work.check()?;
        if links.coordinates.len() != term.monomial_powers().len()
            || links.factors.len() != term.factors().len()
        {
            return Err(Error::Invalid(
                "complete original term link inventory differs".into(),
            ));
        }
        let mut powers = measure
            .orders()
            .iter()
            .cloned()
            .map(Atom::num)
            .collect::<Vec<_>>();
        for (exponent, index) in term.monomial_powers().iter().zip(&links.coordinates) {
            if let Some(i) = index {
                add_scaled_powers(&mut powers, exponent, factors[*i].orders()).map_err(native)?;
            } else if !exponent.is_zero() {
                return Err(Error::Invalid(
                    "active original coordinate link missing".into(),
                ));
            }
        }
        for (factor_index, (source, link)) in term.factors().iter().zip(&links.factors).enumerate()
        {
            use crate::threshold::gcad::ProjectiveFactorAuthority;
            let authority = &owner.parent.source().decomposition().factor_authorities()
                [source_index][factor_index];
            match link {
                FactorLink::Cleared(i)
                    if !source.exponent().is_zero()
                        && source.role() == crate::parametric::FactorRole::Singularity
                        && matches!(
                            authority,
                            ProjectiveFactorAuthority::Split { .. }
                                | ProjectiveFactorAuthority::Positive { .. }
                        ) =>
                {
                    add_scaled_powers(
                        &mut powers,
                        source.exponent(),
                        factors
                            .get(*i)
                            .ok_or_else(|| Error::Invalid("foreign factor identity".into()))?
                            .orders(),
                    )
                    .map_err(native)?;
                }
                FactorLink::LazyPolynomial
                    if source.role() == crate::parametric::FactorRole::Polynomial
                        && matches!(authority, ProjectiveFactorAuthority::Polynomial) => {}
                FactorLink::Inactive
                    if source.exponent().is_zero()
                        && matches!(authority, ProjectiveFactorAuthority::Inactive) => {}
                _ => {
                    return Err(Error::Invalid(
                        "original factor disposition/authority differs".into(),
                    ));
                }
            }
        }
        terms.push(TermPowers {
            source_index,
            powers: powers.into_iter().map(|p| p.expand()).collect(),
        });
    }
    let source_images = owner
        .source_images
        .iter()
        .map(|p| pull(p, parent_units, &unit_images))
        .collect();
    let root_equations = owner
        .roots
        .iter()
        .map(|r| pull(&r.equation, parent_units, &unit_images))
        .collect();
    work.check()?;
    Ok(EndpointChart {
        ordinal,
        unit_images,
        extra_jacobian,
        source_images,
        root_equations,
        factors,
        measure,
        terms,
    })
}
