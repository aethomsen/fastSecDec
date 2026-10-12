//! Native certified enclosures only establish inequalities. Exact rational
//! matrix inversion and unique positive powers supply the inverse identity.
use super::*;
use symbolica::{
    domains::{
        float::{FloatLike, Real, RealBall},
        integer::Integer,
        rational::Q,
    },
    tensors::matrix::Matrix,
};
use symgcad::{
    algebra::Algebra,
    output::{RootBound, RootIndexDomain},
};

pub struct InteriorWitness {
    leaf: AffinePrefixLeaf,
    source_prefix: Vec<Rational>,
    old_units: Vec<RealBall>,
    leaf_units: Vec<RealBall>,
    inverse: Matrix<symbolica::domains::rational::RationalField>,
    bits: u32,
}
impl InteriorWitness {
    pub fn leaf(&self) -> &AffinePrefixLeaf {
        &self.leaf
    }
    pub fn source_prefix(&self) -> &[Rational] {
        &self.source_prefix
    }
    pub fn old_unit_enclosures(&self) -> &[RealBall] {
        &self.old_units
    }
    pub fn unit_enclosures(&self) -> &[RealBall] {
        &self.leaf_units
    }
    pub fn inverse_exponents(&self) -> &Matrix<symbolica::domains::rational::RationalField> {
        &self.inverse
    }
    pub fn precision(&self) -> u32 {
        self.bits
    }
}

pub(super) fn selected(
    parent: &AffineNormalizedPrefix,
    axis: usize,
    bound: &RootBound,
    point: &[Rational],
    bits: u32,
    b: &Budget,
) -> Result<RealBall> {
    b.check()?;
    if bound.index_domain != RootIndexDomain::Real {
        return Err(Error::Unsupported(
            "certified witness requires distinct-real selector".into(),
        ));
    }
    let raw = parent.source().decomposition().native_result();
    let a = Algebra::new(&raw.order).map_err(native)?;
    let polynomial = super::super::super::witness::polynomial_in(
        super::super::super::witness::Source::Affine(parent.source()),
        bound,
        b,
    )?;
    let fiber = a
        .specialize_univariate(&polynomial, axis, point)
        .map_err(native)?;
    if fiber.is_zero() {
        return Err(Error::Invalid("witness source fiber vanished".into()));
    }
    let roots = fiber.isolate_real_roots();
    let root = roots
        .get(bound.index)
        .ok_or_else(|| Error::Invalid("selected original witness root absent".into()))?;
    // This public API returns DISTINCT real roots in increasing order. Its
    // internal defining-factor index is deliberately not used as CAD ordinal.
    let tolerance = Rational::from((Integer::one(), Integer::from(2).pow(u64::from(bits))));
    b.rational(&tolerance)?;
    let mut root = root.0.clone().refined(&tolerance);
    use symbolica::poly::univariate::RootLocation;
    if !matches!(
        root.classify_location(),
        RootLocation::Real | RootLocation::Zero
    ) {
        return Err(Error::Invalid(
            "native selected witness is not a certified real root".into(),
        ));
    }
    // A certified disk around a proven real root need not have a real centre.
    // Its real-axis projection still encloses that exact selected root; imposing
    // centre.im == 0 would incorrectly refuse higher-degree native isolations.
    b.check()?;
    Ok(root.enclosure().to_ball(bits).re)
}
pub(super) fn positive_power(x: &RealBall, e: &Rational, b: &Budget) -> Result<RealBall> {
    b.check()?;
    b.rational(e)?;
    if !x.is_strictly_positive() {
        return Err(Error::Unsupported(
            "positive-power witness enclosure inconclusive".into(),
        ));
    }
    let n = i64::try_from(e.numerator()).map_err(native)?;
    let d = u64::try_from(e.denominator()).map_err(native)?;
    if !d.is_power_of_two() {
        return Err(Error::Unsupported(
            "inverse witness currently supports native dyadic positive powers".into(),
        ));
    }
    if n.unsigned_abs() > b.limits.max_degree as u64 {
        return Err(Error::ResourceIncomplete(
            "inverse witness exponent allowance".into(),
        ));
    }
    let mut value = x.pow(n.unsigned_abs());
    for _ in 0..d.trailing_zeros() {
        value = value.sqrt();
    }
    Ok(if n < 0 { value.inv() } else { value })
}

pub(super) fn prepare(
    leaf: AffinePrefixLeaf,
    point: Vec<Rational>,
    b: &mut Budget,
) -> Result<InteriorWitness> {
    b.check()?;
    let parent = leaf.cover().parent();
    let n = parent.len();
    if point.len() != n {
        return Err(Error::Invalid(
            "complete original prefix witness required".into(),
        ));
    }
    for q in &point {
        b.rational(q)?;
    }
    let bits = 128;
    if b.limits.max_rational_bits < u64::from(bits) + 1 {
        return Err(Error::ResourceIncomplete(
            "witness enclosure precision allowance".into(),
        ));
    }
    let exact = |q: &Rational| RealBall::from_rational_bounds(q, q, bits);
    let one = exact(&Rational::one());
    let mut old_units = Vec::new();
    for (axis, step) in parent.steps().into_iter().enumerate() {
        let native = &parent.source().native_axes()[axis];
        let lo = native
            .lower
            .as_ref()
            .ok_or_else(|| Error::Unsupported("finite inherited lower endpoint required".into()))?;
        let hi = native
            .upper
            .as_ref()
            .ok_or_else(|| Error::Unsupported("finite inherited upper endpoint required".into()))?;
        let lower = selected(parent, axis, lo, &point[..axis], bits, b)?;
        let upper = selected(parent, axis, hi, &point[..axis], bits, b)?;
        let value = exact(&point[axis]);
        let distance = &value - &lower;
        let width = &upper - &lower;
        if !distance.is_strictly_positive()
            || !(&upper - &value).is_strictly_positive()
            || !width.is_strictly_positive()
        {
            return Err(Error::Unsupported(
                "original witness not certified inside selected interval".into(),
            ));
        }
        let power = match step {
            AffinePrefixStep::ConstantInterval { power, .. } => *power,
            AffinePrefixStep::NormalizedUpper { .. } => 1,
        };
        old_units.push(positive_power(
            &(distance / width),
            &Rational::from((1, power)),
            b,
        )?);
    }
    let part = leaf.restriction();
    let half = part.half();
    let mut half_units = Vec::new();
    for (i, u) in old_units.iter().enumerate() {
        let v = if half & (1 << i) == 0 {
            u * exact(&Rational::from(2))
        } else {
            (&one - u) * exact(&Rational::from(2))
        };
        if !v.is_strictly_positive() || !(&one - &v).is_strictly_positive() {
            return Err(Error::Unsupported(
                "rational source witness does not lie strictly in requested half".into(),
            ));
        }
        half_units.push(v);
    }
    let dim = u32::try_from(n).map_err(native)?;
    let matrix = Matrix::from_linear(
        part.native_map()
            .exponent_matrix
            .iter()
            .flatten()
            .map(|e| Rational::from(e.clone()))
            .collect(),
        dim,
        dim,
        Q,
    )
    .map_err(native)?;
    let inverse = matrix.inv().map_err(native)?;
    if &matrix * &inverse != Matrix::identity(dim, Q)
        || &inverse * &matrix != Matrix::identity(dim, Q)
    {
        return Err(Error::Invalid(
            "native exact exponent inverse identity".into(),
        ));
    }
    let mut leaf_units = Vec::new();
    for i in 0..dim {
        let mut value = one.clone();
        for j in 0..dim {
            value *= positive_power(&half_units[j as usize], &inverse[(i, j)], b)?;
        }
        if !value.is_strictly_positive() || !(&one - &value).is_strictly_positive() {
            return Err(Error::Unsupported(
                "rational source witness does not lie strictly in requested fan leaf".into(),
            ));
        }
        leaf_units.push(value);
    }
    b.check()?;
    Ok(InteriorWitness {
        leaf,
        source_prefix: point,
        old_units,
        leaf_units,
        inverse,
        bits,
    })
}

/// Certified values of the actual dependent tower at the retained source
/// witness. They are derived from the selected original fibers, never supplied
/// as independent runtime parameters or rounded to an algebraic guess.
pub(super) fn slot_values(w: &InteriorWitness, b: &Budget) -> Result<Vec<RealBall>> {
    use std::collections::BTreeMap;
    let parent = w.leaf.cover().parent();
    let units = &parent.source().unit_coordinates()[..parent.len()];
    let bits = w.bits;
    let exact = |q: &Rational| RealBall::from_rational_bounds(q, q, bits);
    let mut values = BTreeMap::<Symbol, RealBall>::new();
    for (axis, step) in parent.steps().into_iter().enumerate() {
        match step {
            AffinePrefixStep::ConstantInterval { lower, upper, .. } => {
                for endpoint in [lower, upper] {
                    if let Some(symbol) = endpoint.symbol() {
                        let value = selected(
                            parent,
                            axis,
                            endpoint.original_bound(),
                            &w.source_prefix[..axis],
                            bits,
                            b,
                        )?;
                        values.insert(symbol, value);
                    }
                }
            }
            AffinePrefixStep::NormalizedUpper { section, .. } => {
                let upper = selected(
                    parent,
                    axis,
                    section.original_bound(),
                    &w.source_prefix[..axis],
                    bits,
                    b,
                )?;
                let scale = algebra::pull(
                    section.scale(),
                    &section.chart().parameters,
                    &section.chart().parameter_images,
                );
                let roles = values.keys().copied().collect::<Vec<_>>();
                let points = values.values().cloned().collect::<Vec<_>>();
                let ring = algebra::Ring::new(roles)?;
                let scale = ring.polynomial(&scale, b)?.evaluate(exact, &points);
                if !scale.is_strictly_positive() {
                    return Err(Error::Unsupported(
                        "inherited witness scale enclosure inconclusive".into(),
                    ));
                }
                values.insert(
                    section.root_symbol(),
                    (upper - exact(section.lower())) / scale,
                );
            }
        }
        values.insert(units[axis], w.old_units[axis].clone());
    }
    let original_roles = values.keys().copied().collect::<Vec<_>>();
    let original_values = values.values().cloned().collect::<Vec<_>>();
    let ring = algebra::Ring::new(original_roles)?;
    for (image, q) in parent.images().iter().zip(&w.source_prefix) {
        let value = ring.polynomial(image, b)?.evaluate(exact, &original_values);
        if value.lower_bound().to_rational() > *q || value.upper_bound().to_rational() < *q {
            return Err(Error::Invalid(
                "certified inverse witness forward containment".into(),
            ));
        }
    }
    let mut result = w.leaf_units.clone();
    for symbol in w.leaf.cover().dependent_symbols() {
        result.push(
            values
                .get(symbol)
                .ok_or_else(|| Error::Invalid("complete inherited witness slots".into()))?
                .clone(),
        );
    }
    for (value, (_, bounds)) in result.iter().zip(w.leaf.cover().slots()) {
        if value.lower_bound().to_rational() < bounds[0]
            || value.upper_bound().to_rational() > bounds[1]
        {
            return Err(Error::Unsupported(
                "actual witness slot enclosure exceeds retained certified bounds".into(),
            ));
        }
    }
    b.check()?;
    Ok(result)
}
