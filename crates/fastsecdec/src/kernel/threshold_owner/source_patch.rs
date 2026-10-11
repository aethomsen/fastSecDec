//! Borrowed native preparation facts shared by rational and algebraic lineage.
//! This projects an already admitted request; it never constructs a new gauge.
use super::*;
use crate::threshold::gcad::{DomainOrigin, GcadRequest};

pub(super) struct SourcePatch<'a> {
    pub coordinates: &'a [Symbol],
    pub images: Vec<Atom>,
    pub positive_measure: Atom,
}
impl<'a> SourcePatch<'a> {
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
