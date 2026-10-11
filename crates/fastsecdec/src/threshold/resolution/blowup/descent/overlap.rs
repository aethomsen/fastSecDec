use super::super::refinement::frame::{JoinedFrameProduction, JoinedSupportFrame, join};
use super::*;
#[derive(Clone, Debug)]
enum OverlapGeometry {
    EmptyAmbient,
    EmptyIntersection {
        equations: Ideal,
        relations: Vec<Poly>,
    },
    Nonempty(Arc<JoinedSupportFrame>),
}
#[derive(Clone, Debug)]
pub struct CenterOverlap {
    left: Arc<LocalCenter>,
    right: Arc<LocalCenter>,
    geometry: OverlapGeometry,
}
impl CenterOverlap {
    pub fn verify(
        a: Arc<LocalCenter>,
        z: Arc<LocalCenter>,
        namespace: &str,
        b: &mut Budget,
    ) -> Result<Arc<Self>> {
        if !Arc::ptr_eq(a.atlas(), z.atlas()) || a.index() == z.index() {
            return Err(Error::Invalid("ambient overlap source/pair association"));
        }
        let (left, right) = if a.index() < z.index() {
            (a, z)
        } else {
            (z, a)
        };
        let atlas = left.atlas();
        let geometry = match (atlas.open(left.index()), atlas.open(right.index())) {
            (Some(a), Some(z)) => match join(
                a.history().ledger().frame().clone(),
                z.history().ledger().frame().clone(),
                atlas.history().ledger().frame().local().ring(),
                namespace,
                b,
            )? {
                JoinedFrameProduction::Empty {
                    equations,
                    unit_relations,
                } => OverlapGeometry::EmptyIntersection {
                    equations,
                    relations: unit_relations,
                },
                JoinedFrameProduction::Complete(common) => {
                    let local = common.frame.local();
                    // The new determinant inverse is already a unit on the
                    // actual child open: this join does not shrink the overlap.
                    let old = common
                        .child_map
                        .pull(z.history().ledger().frame().determinant(), b)?;
                    if !local.zero(&(common.frame.determinant() - &old), b)? {
                        return Err(Error::Invalid("ambient overlap determinant association"));
                    }
                    b.reserve_slots(left.presentation().unwrap().generators().len())?;
                    let ai = Ideal::new(
                        local.ring().clone(),
                        left.presentation()
                            .unwrap()
                            .generators()
                            .iter()
                            .map(|p| common.parent_map.pull(p, b))
                            .collect::<Result<Vec<_>>>()?,
                        b,
                    )?;
                    b.reserve_slots(right.presentation().unwrap().generators().len())?;
                    let zi = Ideal::new(
                        local.ring().clone(),
                        right
                            .presentation()
                            .unwrap()
                            .generators()
                            .iter()
                            .map(|p| common.child_map.pull(p, b))
                            .collect::<Result<Vec<_>>>()?,
                        b,
                    )?;
                    equal_ideals(local, &ai, &zi, b)?;
                    for (ad, zd) in a
                        .history()
                        .ledger()
                        .divisors()
                        .iter()
                        .zip(z.history().ledger().divisors())
                    {
                        if ad.id != zd.id {
                            return Err(Error::Invalid("ambient overlap global boundary ID"));
                        }
                        let av = common.parent_map.pull(&ad.equation, b)?;
                        let zv = common.child_map.pull(&zd.equation, b)?;
                        if !local.zero(&(&av - &zv), b)? {
                            return Err(Error::Invalid("ambient overlap boundary equation"));
                        }
                    }
                    OverlapGeometry::Nonempty(common)
                }
            },
            _ => OverlapGeometry::EmptyAmbient,
        };
        Ok(Arc::new(Self {
            left,
            right,
            geometry,
        }))
    }
    pub fn left(&self) -> &Arc<LocalCenter> {
        &self.left
    }
    pub fn right(&self) -> &Arc<LocalCenter> {
        &self.right
    }
    pub fn indices(&self) -> (usize, usize) {
        (self.left.index(), self.right.index())
    }
    pub fn frame(&self) -> Option<&Arc<EtaleFrame>> {
        match &self.geometry {
            OverlapGeometry::Nonempty(g) => Some(&g.frame),
            _ => None,
        }
    }
    pub fn empty_intersection(&self) -> Option<(&Ideal, &[Poly])> {
        match &self.geometry {
            OverlapGeometry::EmptyIntersection {
                equations,
                relations,
            } => Some((equations, relations)),
            _ => None,
        }
    }
    pub fn includes_empty_ambient(&self) -> bool {
        matches!(self.geometry, OverlapGeometry::EmptyAmbient)
    }
}
