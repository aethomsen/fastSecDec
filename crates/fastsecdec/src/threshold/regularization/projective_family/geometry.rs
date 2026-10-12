use super::algebra::{Field, pull};
use super::*;
use symbolica::{
    domains::rational::Q,
    poly::{PolyVariable, univariate::RootLocation},
};
use symgcad::{algebra::Algebra, output::RootIndexDomain};

fn constant(
    source: &Arc<CellMap>,
    side: regular::SectionSide,
    w: &mut Work<'_>,
) -> Result<(Atom, Option<Arc<regular::RegularSection>>)> {
    let raw = source.decomposition().native_result();
    let axis = &source.native_axes()[0];
    let bound = match side {
        regular::SectionSide::Lower => &axis.lower,
        regular::SectionSide::Upper => &axis.upper,
    }
    .as_ref()
    .ok_or_else(|| unsupported("finite prefix bounds required"))?;
    if bound.index_domain != RootIndexDomain::Real {
        return Err(unsupported(
            "first constant-bound issuer requires real-root selectors",
        ));
    }
    let native_ring = Algebra::new(&raw.order).map_err(native)?;
    let p = native_ring
        .parse(&raw.polynomials[bound.polynomial])
        .map_err(native)?;
    if p.degree(0) == 0
        || (1..raw.order.len()).any(|i| p.degree(i) != 0)
        || p.degree(0) as usize > w.limits.max_field_degree
    {
        return Err(unsupported("constant first-axis algebraic bound required"));
    }
    let mut roots = p.to_univariate_from_univariate(0).isolate_real_roots();
    roots.sort_by(|a, b| {
        a.0.enclosure()
            .center()
            .re
            .cmp(&b.0.enclosure().center().re)
    });
    let mut selected = roots
        .get(bound.index)
        .ok_or_else(|| invalid("native real-root ordinal"))?
        .0
        .clone();
    if !matches!(
        selected.classify_location(),
        RootLocation::Real | RootLocation::Zero
    ) {
        return Err(invalid("selected constant is not real"));
    }
    selected = selected.refined(&Rational::from((1, 1_000_000)));
    let value = selected.to_atom();
    if p.degree(0) == 1 {
        return Ok((value, None));
    }
    // The native selector determines the root. Its certified rational enclosure
    // proposes a closed regular-section bracket; GCAD sample intervals do not
    // become map bounds or branch authority.
    let c = &selected.enclosure().center().re;
    let radius = selected.enclosure().radius();
    let padding = radius + &Rational::from((1, 1_000_000));
    let bracket = regular::BracketProposal {
        lower: c - &padding,
        upper: c + &padding,
        derivative_margin: Rational::from((1, 1_000_000)),
    };
    let section =
        regular::RegularSection::prepare(source.clone(), 0, side, bracket, &mut w.budget)?;
    Ok((value, Some(section)))
}

pub(super) fn build_cell(
    owner: &Arc<VerifiedDecomposition>,
    cell: usize,
    coordinates: &[Symbol],
    primitive: Symbol,
    inputs: &[(FactorOrigin, Atom, Atom)],
    w: &mut Work<'_>,
) -> Result<Vec<ResolvedChart>> {
    w.emit(Progress::Cell(cell))?;
    let source = CellMap::new(owner.clone(), cell, coordinates.to_vec()).map_err(native)?;
    let mut constants = Vec::new();
    let mut sections = Vec::new();
    for side in [regular::SectionSide::Lower, regular::SectionSide::Upper] {
        let (a, s) = constant(&source, side, w)?;
        constants.push(a);
        sections.extend(s);
    }
    let mut field = Field::new(&constants, coordinates, primitive, w)?;
    let dimensions = coordinates.len();
    let native_dimensions = u32::try_from(dimensions)
        .map_err(|_| Error::ResourceIncomplete("half-cube dimension allowance".into()))?;
    let half_count = 1usize
        .checked_shl(native_dimensions)
        .ok_or_else(|| Error::ResourceIncomplete("half-cube count overflow".into()))?;
    if half_count > w.limits.max_charts {
        return Err(Error::ResourceIncomplete(
            "resolved half-cube allowance".into(),
        ));
    }
    let source_roles = source.axes().iter().map(|a| a.symbol()).collect::<Vec<_>>();
    let prepared = owner.request().domain().coordinates();
    if dimensions == 0 || source_roles.len() != dimensions || prepared.len() != dimensions {
        return Err(invalid("triangular source dimension association"));
    }
    let source_order = prepared
        .iter()
        .map(|s| {
            source_roles
                .iter()
                .position(|a| a == s)
                .ok_or_else(|| invalid("prepared coordinate missing from CAD order"))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut permutation = 1i8;
    for (i, a) in source_order.iter().enumerate() {
        for b in &source_order[i + 1..] {
            if a > b {
                permutation = -permutation;
            }
        }
    }
    let first_width = &constants[1] - &constants[0];
    let mut cad_images = vec![&constants[0] + &first_width * Atom::var(coordinates[0])];
    let mut measure = first_width;
    // Each native linear root is a function of the complete original prefix.
    // Substitute all already-issued images, retaining triangular dependence.
    for (axis_index, axis) in source
        .linear_axes(1..dimensions)
        .map_err(native)?
        .iter()
        .enumerate()
    {
        let axis_index = axis_index + 1;
        let lower = pull(
            axis.lower()
                .ok_or_else(|| unsupported("finite later lower bound"))?,
            &source_roles[..axis_index],
            &cad_images,
        );
        let upper = pull(
            axis.upper()
                .ok_or_else(|| unsupported("finite later upper bound"))?,
            &source_roles[..axis_index],
            &cad_images,
        );
        let width = &upper - &lower;
        cad_images.push(&lower + &width * Atom::var(coordinates[axis_index]));
        measure *= width;
        w.atom(cad_images.last().expect("new triangular image"))?;
        w.atom(&measure)?;
    }
    let images = source_order
        .iter()
        .map(|i| cad_images[*i].clone())
        .collect::<Vec<_>>();
    let mut base_inputs = inputs
        .iter()
        .map(|(o, original, canonical)| (*o, original.clone(), pull(canonical, prepared, &images)))
        .collect::<Vec<_>>();
    // Preserve original leading coefficients even if native rational arithmetic
    // canceled them in the extracted root expression.
    let raw = owner.native_result();
    let ring = Algebra::new(&raw.order).map_err(native)?;
    let aliases = ring
        .variables
        .iter()
        .map(|v| {
            v.to_atom()
                .as_view()
                .as_var_view()
                .expect("native variable")
                .get_symbol()
        })
        .collect::<Vec<_>>();
    let source_symbols = source
        .axes()
        .iter()
        .map(|a| Atom::var(a.symbol()))
        .collect::<Vec<_>>();
    for axis in 1..dimensions {
        for (side, bound) in [
            &source.native_axes()[axis].lower,
            &source.native_axes()[axis].upper,
        ]
        .into_iter()
        .enumerate()
        {
            let p = ring
                .parse(&raw.polynomials[bound.as_ref().expect("finite lowered bound").polynomial])
                .map_err(native)?;
            let cs = p.to_univariate_polynomial_list(axis);
            let leading = &cs
                .iter()
                .find(|(_, d)| *d == 1)
                .ok_or_else(|| invalid("linear leading coefficient"))?
                .0;
            let original = pull(&leading.to_expression(), &aliases, &source_symbols);
            base_inputs.push((
                FactorOrigin::BoundDenominator(2 * (axis - 1) + side),
                original.clone(),
                pull(&original, prepared, &images),
            ));
        }
    }
    base_inputs.push((FactorOrigin::Measure, measure.clone(), measure.clone()));
    // Source-coordinate images must extend smoothly, even when that coordinate
    // has zero monomial power. Their denominators are independently certified.
    let source_ratios = images
        .iter()
        .map(|a| field.ratio(a, w))
        .collect::<Result<Vec<_>>>()?;
    for r in &source_ratios {
        let (_, p) = field.extract(&r.denominator, w)?;
        if p.iter().any(|p| *p != 0) {
            return Err(unsupported("source image has a closed-face pole"));
        }
        field.unit(&r.denominator, w)?;
    }
    let mut pending = Vec::new();
    for half in 0..half_count {
        w.emit(Progress::Half { cell, half })?;
        let half_images = coordinates
            .iter()
            .enumerate()
            .map(|(i, s)| {
                if half & (1 << i) == 0 {
                    Atom::var(*s) / Atom::num(2)
                } else {
                    Atom::one() - Atom::var(*s) / Atom::num(2)
                }
            })
            .collect::<Vec<_>>();
        let ratios = base_inputs
            .iter()
            .map(|(_, _, a)| field.ratio(&pull(a, coordinates, &half_images), w))
            .collect::<Result<Vec<_>>>()?;
        let supports = ratios
            .iter()
            .flat_map(|r| [&r.numerator, &r.denominator])
            .map(Field::support)
            .collect::<Result<Vec<_>>>()?;
        let fan = fastsecdec_sectors::decompose(
            fastsecdec_sectors::ParametricDomain::UnitCube,
            &supports,
            &w.limits.sectors,
            |_| {
                if w.check().is_ok() {
                    ControlFlow::Continue(())
                } else {
                    ControlFlow::Break(())
                }
            },
        )
        .map_err(native)?;
        w.check()?;
        for (sector, map) in fan.sectors.into_iter().enumerate() {
            if pending.len() >= w.limits.max_charts {
                return Err(Error::ResourceIncomplete("resolved chart allowance".into()));
            }
            map.validate(fastsecdec_sectors::ParametricDomain::UnitCube)
                .map_err(native)?;
            let native_map = crate::generation::coordinates_from_parts(
                coordinates,
                fastsecdec_sectors::ParametricDomain::UnitCube,
                &map,
                coordinates,
            );
            let fan_images = native_map.images();
            let mut receipts = Vec::new();
            for (i, ((origin, original, _), r)) in base_inputs.iter().zip(&ratios).enumerate() {
                let n = field.polynomial(
                    &pull(&field.expression(&r.numerator)?, coordinates, fan_images),
                    w,
                )?;
                let d = field.polynomial(
                    &pull(&field.expression(&r.denominator)?, coordinates, fan_images),
                    w,
                )?;
                let (_, np) = field.extract(&n, w)?;
                let (_, dp) = field.extract(&d, w)?;
                for (axis, power) in np.iter().enumerate() {
                    if map.factor_valuations[2 * i][axis].to_i64() != Some(i64::from(*power))
                        || map.factor_valuations[2 * i + 1][axis].to_i64()
                            != Some(i64::from(dp[axis]))
                    {
                        return Err(invalid(
                            "native support valuation differs from exact field pullback",
                        ));
                    }
                }
                receipts.push(field.receipt(*origin, original.clone(), &n, &d, w)?);
            }
            let composed_half = half_images
                .iter()
                .map(|a| pull(a, coordinates, fan_images))
                .collect::<Vec<_>>();
            let final_images = images
                .iter()
                .map(|a| pull(a, coordinates, &composed_half))
                .collect::<Vec<_>>();
            let final_measure = pull(&measure, coordinates, &composed_half)
                * native_map.measure_jacobian()
                / Atom::num(half_count as u64);
            // The measure ledger includes fan determinant/powers and the half
            // Jacobian exactly once, rather than folding them into factor rows.
            let ratio = field.ratio(&final_measure, w)?;
            let receipt = field.receipt(
                FactorOrigin::Measure,
                measure.clone(),
                &ratio.numerator,
                &ratio.denominator,
                w,
            )?;
            if receipt.sign() != 1 {
                return Err(invalid("positive map measure"));
            }
            let slot = receipts
                .iter()
                .position(|r| r.origin == FactorOrigin::Measure)
                .expect("measure inventory");
            receipts[slot] = receipt;
            for r in &receipts {
                if matches!(r.origin, FactorOrigin::BoundDenominator(_))
                    && r.powers.iter().any(|p| *p != 0)
                {
                    return Err(unsupported(
                        "linear leading coefficient vanishes on chart closure",
                    ));
                }
            }
            let signed = symbolica::tensors::matrix::Matrix::from_linear(
                map.exponent_matrix
                    .iter()
                    .flatten()
                    .cloned()
                    .map(Rational::from)
                    .collect(),
                native_dimensions,
                native_dimensions,
                Q,
            )
            .map_err(native)?
            .det()
            .map_err(native)?;
            let reflection = if half.count_ones() % 2 == 0 {
                1i8
            } else {
                -1i8
            };
            let orientation =
                permutation * reflection * if signed < Rational::zero() { -1 } else { 1 };
            pending.push((
                half,
                sector,
                map,
                orientation,
                final_images,
                final_measure,
                receipts,
            ));
        }
    }
    let geometry = Arc::new(CellGeometry {
        source,
        sections,
        selected_constants: constants,
        algebra: field,
    });
    Ok(pending
        .into_iter()
        .map(
            |(half, sector, map, orientation, images, measure, receipts)| ResolvedChart {
                geometry: geometry.clone(),
                half,
                sector,
                map,
                orientation,
                images,
                measure,
                receipts,
                terms: Vec::new(),
            },
        )
        .collect())
}

pub(super) fn rational_source(a: &Atom, coordinates: &[Symbol], w: &Work<'_>) -> Result<Atom> {
    w.check()?;
    meromorphic::preflight_expression(a, w.limits.prefactors).map_err(prefactor_error)?;
    let variables = Arc::new(
        coordinates
            .iter()
            .copied()
            .map(PolyVariable::from)
            .collect::<Vec<_>>(),
    );
    let p: symgcad::algebra::Poly = a.try_to_polynomial(&Q, variables.clone()).map_err(native)?;
    if p.variables() != &variables {
        return Err(unsupported("fixed rational singular geometry required"));
    }
    w.check()?;
    if p.nterms() > w.limits.algebra.max_terms
        || (0..p.nvars()).any(|i| p.degree(i) as usize > w.limits.algebra.max_degree)
    {
        return Err(Error::ResourceIncomplete(
            "source geometry polynomial degree/terms".into(),
        ));
    }
    for coefficient in &p.coefficients {
        w.budget.rational(coefficient)?;
    }
    // Canonicalize only the admitted singular geometry before repeated chart
    // substitution. Receipts retain the original source expression separately.
    // Numerator bodies never pass through this conversion.
    let canonical = p.to_expression();
    w.atom(&canonical)?;
    Ok(canonical)
}
