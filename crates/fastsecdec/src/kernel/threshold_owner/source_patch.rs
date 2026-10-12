//! Borrowed native preparation facts shared by rational and algebraic lineage.
//! This projects an already admitted owner; it never constructs a new gauge.
use super::*;
use crate::threshold::{
    continued::SourceAuthority,
    gcad::{DomainOrigin, GcadRequest, PositiveProjectiveQuotient},
};

pub(super) struct SourcePatch<'a> {
    pub coordinates: &'a [Symbol],
    pub images: Vec<Atom>,
    pub positive_measure: Atom,
}
impl<'a> SourcePatch<'a> {
    /// The actual coordinate map from anchor ratios to the original simplex.
    /// Issued anchor densities already cancel this Jacobian by homogeneity;
    /// metadata records it without multiplying it into the density again.
    pub fn from_positive_projective(
        owner: &'a PositiveProjectiveQuotient,
    ) -> Result<Self, KernelError> {
        SourceAuthority::PositiveProjective(owner)
            .source_identity()
            .map_err(invalid)?;
        let preparation = owner.preparation();
        let original = preparation.input().parameters();
        let expected = original
            .iter()
            .enumerate()
            .filter_map(|(i, s)| (i != preparation.anchor_index()).then_some(*s))
            .collect::<Vec<_>>();
        if preparation.coordinates() != expected
            || preparation.normalized_images().len() != original.len()
            || preparation.images().len() != original.len()
            || preparation.measure_jacobian() != &Atom::one()
        {
            return Err(invalid(
                "positive-projective source/anchor patch association",
            ));
        }
        Ok(Self {
            coordinates: preparation.coordinates(),
            images: preparation.normalized_images().to_vec(),
            positive_measure: preparation.simplex_jacobian().clone(),
        })
    }
    pub fn from_request(request: &'a GcadRequest) -> Result<Self, KernelError> {
        let coordinates = request.domain().coordinates();
        match request.domain().origin() {
            DomainOrigin::NativeUnitCube
                if request.projective_preparation().is_none()
                    && coordinates == request.input().parameters() =>
            {
                Ok(Self {
                    coordinates,
                    images: coordinates.iter().map(|s| Atom::var(*s)).collect(),
                    positive_measure: Atom::one(),
                })
            }
            DomainOrigin::AffineProjective { eliminated_index } => {
                let p = request.projective_preparation().ok_or_else(|| {
                    invalid("projective lineage lacks its native preparation owner")
                })?;
                if p.eliminated_index() != *eliminated_index
                    || p.coordinates() != coordinates
                    || p.input().parameters() != request.input().parameters()
                    || p.images().len() != request.input().parameters().len()
                {
                    return Err(invalid("projective source/prepared patch association"));
                }
                Ok(Self {
                    coordinates,
                    images: p.images().to_vec(),
                    positive_measure: p.measure_jacobian().clone(),
                })
            }
            _ => Err(invalid("unsupported native source patch origin")),
        }
    }
}
