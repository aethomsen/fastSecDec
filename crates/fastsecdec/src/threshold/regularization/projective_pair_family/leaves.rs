//! Complete half-cubes and native support fans, with exact source ledgers.
use super::projective_family::algebra::{Field, Polynomial, pull};
use super::*;
use symbolica::{domains::rational::Q, tensors::matrix::Matrix};

pub(super) fn receipt(
    field: &Field,
    origin: FactorOrigin,
    source: Atom,
    n: &Polynomial,
    d: &Polynomial,
    w: &Work<'_>,
) -> Result<FactorReceipt> {
    let (n, np) = field.extract_in(n, 2, w)?;
    let (d, dp) = field.extract_in(d, 2, w)?;
    let numerator = field.unit_with_margins(&n, &[8, 16, 32, 64], w)?;
    let denominator = field.unit_with_margins(&d, &[8, 16, 32, 64], w)?;
    Ok(FactorReceipt {
        origin,
        source,
        powers: np
            .iter()
            .zip(dp)
            .map(|(n, d)| i64::from(*n) - i64::from(d))
            .collect(),
        numerator,
        denominator,
    })
}

pub(super) fn build(
    owner: &Arc<PositiveProjectiveQuotient>,
    coordinates: &[Symbol],
    mut base: geometry::Base,
    inputs: &[(FactorOrigin, Atom, Atom)],
    templates: &[numerator::Template],
    w: &mut Work<'_>,
) -> Result<Vec<ResolvedPairChart>> {
    let prepared = owner.preparation().coordinates();
    let source_roles = base
        .source
        .axes()
        .iter()
        .map(|a| a.symbol())
        .collect::<Vec<_>>();
    let order = prepared
        .iter()
        .map(|s| {
            source_roles
                .iter()
                .position(|a| a == s)
                .ok_or_else(|| invalid("prepared coordinate absent from native CAD axes"))
        })
        .collect::<Result<Vec<_>>>()?;
    if order.len() != 2 || source_roles.len() != 2 {
        return Err(invalid("two-dimensional source order"));
    }
    let permutation = if order[0] < order[1] { 1i8 } else { -1 };
    let images = order
        .iter()
        .map(|i| base.images[*i].clone())
        .collect::<Vec<_>>();
    let mut factors = inputs
        .iter()
        .map(|(o, source, canonical)| (*o, source.clone(), pull(canonical, prepared, &images)))
        .collect::<Vec<_>>();
    let mut rewrites = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let equations = base
        .sections
        .iter()
        .map(|s| base.field.polynomial(s.equation(), w))
        .collect::<Result<Vec<_>>>()?;
    for proposed in &base.rewrites {
        if !seen.insert((proposed.term, proposed.factor)) {
            return Err(invalid("duplicate source factor rewrite"));
        }
        let origin = FactorOrigin::Factor {
            term: proposed.term,
            factor: proposed.factor,
        };
        let row = factors
            .iter_mut()
            .find(|r| r.0 == origin)
            .ok_or_else(|| invalid("rewrite does not name an active singular source factor"))?;
        let old = base.field.ratio(&row.2, w)?;
        let new = base.field.ratio(&proposed.expression, w)?;
        // Retain the cleared relation explicitly. The later denominator rows
        // separately certify both map images and the proposed rational form.
        let cross = &old.numerator * &new.denominator - &new.numerator * &old.denominator;
        let mut remainder = cross.clone();
        for equation in &equations {
            remainder = remainder.quot_rem(equation, false).1;
        }
        if !remainder.is_zero() {
            return Err(unsupported(
                "proposed factor equality not established by native normalized-ideal reduction",
            ));
        }
        rewrites.push(FactorRewriteReceipt {
            origin,
            original: row.2.clone(),
            proposed: proposed.expression.clone(),
            crossproduct: base.field.expression(&cross)?,
        });
        row.2 = proposed.expression.clone();
    }
    // All source-image denominators enter the complete fan even when their
    // original monomial powers and numerator degrees happen to be zero.
    for (axis, image) in images.iter().enumerate() {
        let r = base.field.ratio(image, w)?;
        let d = base.field.expression(&r.denominator)?;
        factors.push((FactorOrigin::BoundDenominator(axis), d.clone(), d));
    }
    factors.push((
        FactorOrigin::Measure,
        base.measure.clone(),
        base.measure.clone(),
    ));
    let mut result = Vec::new();
    let rewrites = Arc::new(rewrites);
    for half in 0..4usize {
        w.emit(Progress::Half {
            cell: base.cell,
            half,
        })?;
        let half_images = coordinates
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let a = Atom::var(*s) / Atom::num(2);
                if half & (1 << i) == 0 {
                    a
                } else {
                    Atom::one() - a
                }
            })
            .collect::<Vec<_>>();
        let ratios = factors
            .iter()
            .map(|(_, _, a)| base.field.ratio(&pull(a, coordinates, &half_images), w))
            .collect::<Result<Vec<_>>>()?;
        let supports = ratios
            .iter()
            .flat_map(|r| [&r.numerator, &r.denominator])
            .map(|p| Field::support_in(p, 2))
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
            if result.len() >= w.limits.max_charts {
                return Err(Error::ResourceIncomplete(
                    "complete pair-family leaf allowance".into(),
                ));
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
            for (i, ((origin, source, _), r)) in factors.iter().zip(&ratios).enumerate() {
                let n = base.field.polynomial(
                    &pull(
                        &base.field.expression(&r.numerator)?,
                        coordinates,
                        fan_images,
                    ),
                    w,
                )?;
                let d = base.field.polynomial(
                    &pull(
                        &base.field.expression(&r.denominator)?,
                        coordinates,
                        fan_images,
                    ),
                    w,
                )?;
                let (_, np) = base.field.extract_in(&n, 2, w)?;
                let (_, dp) = base.field.extract_in(&d, 2, w)?;
                for axis in 0..2 {
                    if map.factor_valuations[2 * i][axis].to_i64() != Some(i64::from(np[axis]))
                        || map.factor_valuations[2 * i + 1][axis].to_i64()
                            != Some(i64::from(dp[axis]))
                    {
                        return Err(invalid(
                            "native support valuation differs from exact coefficient-field pullback",
                        ));
                    }
                }
                receipts.push(receipt(&base.field, *origin, source.clone(), &n, &d, w)?);
            }
            let composed = half_images
                .iter()
                .map(|a| pull(a, coordinates, fan_images))
                .collect::<Vec<_>>();
            let formal_images = images
                .iter()
                .map(|a| pull(a, coordinates, &composed))
                .collect::<Vec<_>>();
            let formal_measure = pull(&base.measure, coordinates, &composed)
                * native_map.measure_jacobian()
                / Atom::num(4);
            let r = base.field.ratio(&formal_measure, w)?;
            let mr = receipt(
                &base.field,
                FactorOrigin::Measure,
                base.measure.clone(),
                &r.numerator,
                &r.denominator,
                w,
            )?;
            if mr.sign() != 1 {
                return Err(invalid("positive triangular/half/fan measure"));
            }
            *receipts
                .iter_mut()
                .find(|r| r.origin() == FactorOrigin::Measure)
                .expect("measure row retained") = mr;
            let mut scope = Scope::default();
            let mut root_inputs = Vec::new();
            let mut calls = Vec::new();
            for section in &base.sections {
                let coefficients = section
                    .coefficients()
                    .iter()
                    .map(|a| pull(a, coordinates, &composed))
                    .collect::<Vec<_>>();
                let call = section
                    .root_program()
                    .call(section.tag(), &coefficients)
                    .map_err(native)?;
                scope
                    .insert(section.tag(), section.root_program().clone())
                    .map_err(native)?;
                root_inputs.push((section.tag(), coefficients));
                calls.push(call);
            }
            let root_roles = base
                .sections
                .iter()
                .map(|s| s.root_symbol())
                .collect::<Vec<_>>();
            let numerators = templates
                .iter()
                .map(|t| {
                    numerator::compose(
                        t,
                        &formal_images,
                        &mut base.field,
                        coordinates,
                        &root_roles,
                        &calls,
                        w,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let images = formal_images
                .iter()
                .map(|a| pull(a, &root_roles, &calls))
                .collect();
            let measure = pull(&formal_measure, &root_roles, &calls);
            let signed = Matrix::from_linear(
                map.exponent_matrix
                    .iter()
                    .flatten()
                    .cloned()
                    .map(Rational::from)
                    .collect(),
                2,
                2,
                Q,
            )
            .map_err(native)?
            .det()
            .map_err(native)?;
            let reflection = if half.count_ones() % 2 == 0 { 1 } else { -1 };
            let orientation =
                base.orientation * permutation * reflection * if signed < 0 { -1 } else { 1 };
            result.push(ResolvedPairChart {
                cell: base.cell,
                path: [base.ordinal, half, sector],
                coordinate_order: source_roles.clone(),
                source_axes: base.source.native_axes().to_vec(),
                sections: base.sections.clone(),
                branches: base.branches.clone(),
                coverage: base.coverage.clone(),
                rewrites: rewrites.clone(),
                root_inputs,
                map,
                orientation,
                images,
                measure,
                receipts,
                numerators,
                terms: Vec::new(),
                roots: scope,
            });
        }
    }
    Ok(result)
}
