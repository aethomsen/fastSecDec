use super::{AliasRole, CellMap, GcadError, Result};
use std::{collections::BTreeMap, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    id::{Pattern, Replacement},
    prelude::Rational,
};
use symgcad::{algebra::Poly, output::RootIndexDomain};

pub(super) mod root;

#[derive(Debug)]
pub(super) struct LinearRoot {
    pub polynomial: usize,
    pub selector: RootIndexDomain,
    pub numerator: Poly,
    pub denominator: Poly,
    pub expression: Atom,
}
#[derive(Clone, Debug)]
pub struct LinearAxis {
    pub(super) lower: Option<Arc<LinearRoot>>,
    pub(super) upper: Option<Arc<LinearRoot>>,
}

/// Exact rational-function triangular map, with its general cell description.
/// The construction admits only finite linear integration bounds. Parameter
/// axes may be unbounded, but every finite parameter bound must also be linear.
#[derive(Debug)]
pub struct LinearCellMap {
    pub(super) source: Arc<CellMap>,
    pub(super) axes: Vec<LinearAxis>,
    parameter_count: usize,
    prefix: Arc<super::parameters::ParameterPrefix>,
}
impl LinearAxis {
    /// Finite root expression in preceding source coordinates and parameters.
    /// None denotes an unbounded parameter side, never a sampled enclosure.
    pub fn lower(&self) -> Option<&Atom> {
        self.lower.as_ref().map(|r| &r.expression)
    }
    pub fn upper(&self) -> Option<&Atom> {
        self.upper.as_ref().map(|r| &r.expression)
    }
}
impl LinearCellMap {
    pub(super) fn new(source: Arc<CellMap>) -> Result<Arc<Self>> {
        let prefix = super::parameters::ParameterPrefix::prepare(source.clone())?;
        let parameter_count = prefix.axes().len();
        let mut axes = prefix.axes().to_vec();
        axes.extend(root::lower_axes(
            &source,
            parameter_count..source.axes.len(),
        )?);
        Ok(Arc::new(Self {
            source,
            axes,
            parameter_count,
            prefix,
        }))
    }
    pub fn axes(&self) -> &[LinearAxis] {
        &self.axes
    }
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    /// Bind an exact rational point in the actual parameter chamber. This is
    /// not a general symbolic/algebraic-fiber or exceptional-stratum admission.
    pub fn admit_parameters(
        self: &Arc<Self>,
        values: &BTreeMap<Symbol, Rational>,
    ) -> Result<ParameterAdmission> {
        let admitted = self.prefix.admit(values)?;
        Ok(ParameterAdmission {
            map: self.clone(),
            values: values.clone(),
            ordered: admitted.raw_values().to_vec(),
        })
    }
}
impl LinearRoot {
    pub(super) fn evaluate(&self, prefix: &[Rational]) -> Result<Rational> {
        let leading = self.denominator.replace_all(prefix);
        if leading.is_zero() {
            return Err(GcadError::Invalid(format!(
                "linear root polynomial {} changes degree at this point",
                self.polynomial
            )));
        }
        let root = self.numerator.replace_all(prefix) / leading;
        if self.selector == RootIndexDomain::Positive && root <= Rational::zero() {
            return Err(GcadError::Invalid(
                "positive-root selector has no strictly positive linear root".into(),
            ));
        }
        Ok(root)
    }
}

/// A parameter point tied to the immutable map/proof that admitted it.
#[derive(Clone, Debug)]
pub struct ParameterAdmission {
    pub(super) map: Arc<LinearCellMap>,
    pub(super) values: BTreeMap<Symbol, Rational>,
    ordered: Vec<Rational>,
}
#[derive(Debug)]
pub struct RationalMappedPoint {
    pub prepared_coordinates: Vec<Rational>,
    pub original_coordinates: Vec<Rational>,
    pub measure: Rational,
}
impl ParameterAdmission {
    pub fn map(&self) -> &Arc<LinearCellMap> {
        &self.map
    }
    pub fn values(&self) -> &BTreeMap<Symbol, Rational> {
        &self.values
    }
    /// Exact interior control using native polynomial evaluation. This checks
    /// actual leading coefficients, selector domains and positive widths.
    pub fn map_rational(&self, unit: &[Rational]) -> Result<RationalMappedPoint> {
        if unit.len() != self.map.source.unit_coordinates.len()
            || unit
                .iter()
                .any(|v| v <= &Rational::zero() || v >= &Rational::one())
        {
            return Err(GcadError::Invalid(
                "cell map requires a point in its open unit cube".into(),
            ));
        }
        let mut point = self.ordered.clone();
        point.resize(self.map.source.axes.len(), Rational::zero());
        let mut measure = Rational::one();
        for index in self.map.parameter_count..self.map.axes.len() {
            let axis = &self.map.axes[index];
            let lower = axis
                .lower
                .as_ref()
                .expect("finite linear lowering")
                .evaluate(&point)?;
            let upper = axis
                .upper
                .as_ref()
                .expect("finite linear lowering")
                .evaluate(&point)?;
            let width = upper - &lower;
            if width <= Rational::zero() {
                return Err(GcadError::Invalid(
                    "cell map has a nonpositive interval width".into(),
                ));
            }
            point[index] = lower + &width * &unit[index - self.map.parameter_count];
            measure *= width;
        }
        let mapped = self
            .map
            .source
            .axes
            .iter()
            .zip(&point)
            .map(|(axis, value)| (axis.symbol, value.clone()))
            .collect::<BTreeMap<_, _>>();
        let request = self.map.source.owner.request();
        let prepared_coordinates = request
            .domain()
            .coordinates()
            .iter()
            .map(|s| mapped[s].clone())
            .collect::<Vec<_>>();
        let original_coordinates = if let Some(preparation) = request.projective_preparation() {
            preparation
                .images()
                .iter()
                .map(|image| {
                    let value = image.replace_multiple(preparation.coordinates().iter().map(|s| {
                        Replacement::new(
                            Pattern::Literal(Atom::var(*s)),
                            Pattern::Literal(Atom::num(mapped[s].clone())),
                        )
                    }));
                    Rational::try_from(value.as_view())
                        .map_err(|e| GcadError::Invalid(e.to_string()))
                })
                .collect::<Result<Vec<_>>>()?
        } else {
            prepared_coordinates.clone()
        };
        Ok(RationalMappedPoint {
            prepared_coordinates,
            original_coordinates,
            measure,
        })
    }
}
