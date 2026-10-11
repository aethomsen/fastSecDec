//! Exact one-axis open-interval pullback, retaining the actual native cell.
use super::{
    GcadError, PositiveProjectiveCellMap, Result,
    linear::{LinearAxis, LinearRoot, root},
    open_interval,
    source::GeometryView,
};
use std::sync::Arc;
use symbolica::atom::{Atom, Symbol};
use symgcad::{algebra::Poly, output::RootBound};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntervalSide {
    Lower,
    Upper,
}

/// Origin of an excluded unit face. This is a divisor inventory, not a
/// regularity or nonvanishing certificate on that face.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OpenFaceOrigin {
    FiniteLower,
    FiniteUpper,
    NegativeInfinity,
    PositiveInfinity,
}

#[derive(Clone, Debug)]
pub struct OpenFace {
    side: IntervalSide,
    origin: OpenFaceOrigin,
    divisor: Atom,
    jacobian_power: i32,
}
impl OpenFace {
    pub fn side(&self) -> IntervalSide {
        self.side
    }
    pub fn origin(&self) -> OpenFaceOrigin {
        self.origin
    }
    pub fn divisor(&self) -> &Atom {
        &self.divisor
    }
    /// Contribution of this interval operation only. Pulled root/width/source
    /// factors can contribute further orders at the same face.
    pub fn interval_jacobian_power(&self) -> i32 {
        self.jacobian_power
    }
}

/// A borrowed finite bound attached to its exact cell, axis and side.
pub struct OpenFiniteBound<'a> {
    axis: &'a OpenProjectiveAxis,
    side: IntervalSide,
    root: &'a LinearRoot,
}
impl OpenFiniteBound<'_> {
    pub fn source(&self) -> &Arc<PositiveProjectiveCellMap> {
        &self.axis.source
    }
    pub fn axis_index(&self) -> usize {
        self.axis.axis_index
    }
    pub fn side(&self) -> IntervalSide {
        self.side
    }
    pub fn native_bound(&self) -> &RootBound {
        let native = &self.axis.source.native_axes()[self.axis.axis_index];
        match self.side {
            IntervalSide::Lower => native.lower.as_ref(),
            IntervalSide::Upper => native.upper.as_ref(),
        }
        .expect("finite bound retained by the same immutable cell")
    }
    pub fn expression(&self) -> &Atom {
        &self.root.expression
    }
    /// Original native polynomial ring/order. Kept even if expression reduction
    /// cancels it; it must not be silently omitted from later divisor work.
    pub fn leading_coefficient(&self) -> &Poly {
        &self.root.denominator
    }
}

/// One genuine triangular interval with exact rational finite bounds.
/// Preceding source coordinates stay symbolic. This receipt does not assert
/// that those preceding coordinates have an executable map, and it supplies no
/// closed-face, infinity, root-margin or density-continuation certificate.
#[derive(Debug)]
pub struct OpenProjectiveAxis {
    source: Arc<PositiveProjectiveCellMap>,
    axis_index: usize,
    bounds: LinearAxis,
    image: Atom,
    diagonal: Atom,
    faces: [OpenFace; 2],
}
impl PositiveProjectiveCellMap {
    /// Admit one actual axis independently of whether earlier nonlinear axes
    /// can yet be lowered. All finite bounds must pass existing linear-root
    /// extraction; a missing side is explicitly mapped to infinity.
    pub fn open_linear_axis(
        self: &Arc<Self>,
        axis_index: usize,
    ) -> Result<Arc<OpenProjectiveAxis>> {
        let axis = self
            .axes()
            .get(axis_index)
            .ok_or_else(|| GcadError::Invalid("open-map axis is outside the actual cell".into()))?;
        let unit_index = axis.unit_index().ok_or_else(|| {
            GcadError::Unsupported("open interval requires an integration axis".into())
        })?;
        let unit = self.unit_coordinates()[unit_index];
        let mut extracted = root::lower_open_geometry(
            &GeometryView::projective(self.decomposition()),
            self.cell_index(),
            self.axes(),
            axis_index..axis_index + 1,
        )?;
        let bounds = extracted.pop().expect("one exact requested axis");
        let pullback = open_interval::pullback(bounds.lower(), bounds.upper(), unit)?;
        let faces = [
            OpenFace {
                side: IntervalSide::Lower,
                origin: if bounds.lower.is_some() {
                    OpenFaceOrigin::FiniteLower
                } else {
                    OpenFaceOrigin::NegativeInfinity
                },
                divisor: Atom::var(unit),
                jacobian_power: if bounds.lower.is_some() { 0 } else { -2 },
            },
            OpenFace {
                side: IntervalSide::Upper,
                origin: if bounds.upper.is_some() {
                    OpenFaceOrigin::FiniteUpper
                } else {
                    OpenFaceOrigin::PositiveInfinity
                },
                divisor: Atom::one() - Atom::var(unit),
                jacobian_power: if bounds.upper.is_some() { 0 } else { -2 },
            },
        ];
        Ok(Arc::new(OpenProjectiveAxis {
            source: self.clone(),
            axis_index,
            bounds,
            image: pullback.image,
            diagonal: pullback.diagonal,
            faces,
        }))
    }
}
impl OpenProjectiveAxis {
    pub fn source(&self) -> &Arc<PositiveProjectiveCellMap> {
        &self.source
    }
    pub fn axis_index(&self) -> usize {
        self.axis_index
    }
    pub fn unit_coordinate(&self) -> Symbol {
        self.source.unit_coordinates()[self.source.axes()[self.axis_index]
            .unit_index()
            .expect("integration axis")]
    }
    /// In preceding actual source coordinates and this axis's unit variable.
    pub fn image(&self) -> &Atom {
        &self.image
    }
    /// Positive on the actual open cell with 0<unit<1. This is the diagonal
    /// factor, not a claim of a full composed-map determinant or boundary bound.
    pub fn diagonal_jacobian(&self) -> &Atom {
        &self.diagonal
    }
    pub fn faces(&self) -> &[OpenFace; 2] {
        &self.faces
    }
    pub fn finite_bound(&self, side: IntervalSide) -> Option<OpenFiniteBound<'_>> {
        let root = match side {
            IntervalSide::Lower => self.bounds.lower.as_deref(),
            IntervalSide::Upper => self.bounds.upper.as_deref(),
        }?;
        Some(OpenFiniteBound {
            axis: self,
            side,
            root,
        })
    }
}
