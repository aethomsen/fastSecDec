use super::{ActiveFactor, Error, Result};
use crate::threshold::maps::{
    endpoint_transport::{FaceAxis, FaceSide, cover::IssuedEndpointChart},
    normalized::algebra::Ring,
    regular::Budget,
};
use symbolica::{
    atom::{Atom, Symbol},
    domains::rational::Rational,
};
use symgcad::algebra::Poly;

#[derive(Clone, Debug)]
pub struct RelativeFace {
    code: usize,
    axes: Vec<FaceAxis>,
}
impl RelativeFace {
    pub fn code(&self) -> usize {
        self.code
    }
    pub fn axes(&self) -> &[FaceAxis] {
        &self.axes
    }
    pub(super) fn new(units: &[Symbol], code: usize) -> Result<Self> {
        let mut n = code;
        let mut axes = Vec::new();
        for unit in units {
            match n % 3 {
                1 => axes.push(FaceAxis {
                    unit: *unit,
                    side: FaceSide::Lower,
                }),
                2 => axes.push(FaceAxis {
                    unit: *unit,
                    side: FaceSide::Upper,
                }),
                _ => {}
            };
            n /= 3;
        }
        if n != 0 || axes.is_empty() {
            return Err(Error::Invalid(
                "nonempty compatible named relative face required".into(),
            ));
        }
        Ok(Self { code, axes })
    }
}
pub(super) fn face_count(n: usize, max: usize) -> Result<usize> {
    let mut count = 1usize;
    for _ in 0..n {
        count = count
            .checked_mul(3)
            .ok_or_else(|| Error::ResourceIncomplete("relative face inventory overflow".into()))?;
        if count - 1 > max {
            return Err(Error::ResourceIncomplete(
                "relative face inventory allowance".into(),
            ));
        }
    }
    Ok(count - 1)
}
#[derive(Debug)]
pub struct RestrictedFactor {
    pub(super) numerator: Atom,
    pub(super) denominator: Atom,
}
impl RestrictedFactor {
    pub fn numerator(&self) -> &Atom {
        &self.numerator
    }
    pub fn denominator(&self) -> &Atom {
        &self.denominator
    }
}
pub(super) struct Source {
    ring: Ring,
    units: Vec<Symbol>,
    polynomials: Vec<(Poly, Poly)>,
}
impl Source {
    pub(super) fn new(
        chart: &IssuedEndpointChart,
        active: &[ActiveFactor],
        b: &Budget,
    ) -> Result<Self> {
        let units = chart.target_units().to_vec();
        let ring = Ring::new(
            units
                .iter()
                .copied()
                .chain(
                    chart
                        .transport()
                        .root_relations()
                        .iter()
                        .map(|r| r.section().root_symbol()),
                )
                .collect(),
        )?;
        let polynomials = active
            .iter()
            .map(|a| {
                let f = &chart.chart().factors()[a.identity];
                Ok((
                    ring.polynomial(f.numerator(), b)?,
                    ring.polynomial(f.denominator(), b)?,
                ))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            ring,
            units,
            polynomials,
        })
    }
    pub(super) fn restrict(
        &self,
        index: usize,
        face: &RelativeFace,
        b: &Budget,
    ) -> Result<RestrictedFactor> {
        b.check()?;
        let (mut n, mut d) = self
            .polynomials
            .get(index)
            .ok_or_else(|| Error::Invalid("factor ordinal".into()))?
            .clone();
        for a in face.axes() {
            let i = self
                .units
                .iter()
                .position(|u| u == &a.unit)
                .ok_or_else(|| Error::Invalid("foreign relative-face axis".into()))?;
            let v = Rational::from(if a.side == FaceSide::Lower { 0 } else { 1 });
            n = n.replace(i, &v);
            d = d.replace(i, &v);
            b.check()?;
        }
        Ok(RestrictedFactor {
            numerator: self.ring.expression(&n),
            denominator: self.ring.expression(&d),
        })
    }
}
