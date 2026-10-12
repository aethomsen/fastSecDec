//! Source-selected constants and complete triangular interval/fibre covers.
use super::projective_family::algebra::{Field, pull};
use super::*;
use crate::threshold::maps::PositiveProjectiveCellMap;
use symbolica::{
    atom::AtomCore,
    domains::{RealEmbedding, algebraic::AlgebraicContext},
    poly::univariate::RootLocation,
};
use symgcad::{algebra::Algebra, output::RootIndexDomain};

pub(super) fn bound(
    owner: &PositiveProjectiveQuotient,
    cell: usize,
    axis: usize,
    side: SectionSide,
) -> Result<Option<&RootBound>> {
    let a = owner
        .native_result()
        .cells
        .get(cell)
        .and_then(|c| c.axes.get(axis))
        .ok_or_else(|| invalid("original cell/axis ordinal"))?;
    Ok(match side {
        SectionSide::Lower => a.lower.as_ref(),
        SectionSide::Upper => a.upper.as_ref(),
    })
}
pub(super) fn selected_constant(
    owner: &PositiveProjectiveQuotient,
    r: ConstantReference,
    limits: &Limits,
) -> Result<Atom> {
    let b = bound(owner, r.cell, 0, r.side)?
        .ok_or_else(|| unsupported("finite selected constant required"))?;
    if b.index_domain != RootIndexDomain::Real {
        return Err(unsupported(
            "constant issuer currently requires real-root ordinal",
        ));
    }
    let raw = owner.native_result();
    let a = Algebra::new(&raw.order).map_err(native)?;
    let text = raw
        .polynomials
        .get(b.polynomial)
        .ok_or_else(|| invalid("selected constant polynomial ordinal"))?;
    let p = a.parse(text).map_err(native)?;
    if p.degree(0) == 0
        || p.degree(0) as usize > limits.max_field_degree
        || (1..p.nvars()).any(|i| p.degree(i) != 0)
    {
        return Err(unsupported(
            "bounded-degree constant first-axis section required",
        ));
    }
    let mut roots = p.to_univariate_from_univariate(0).isolate_real_roots();
    roots.sort_by(|a, b| {
        a.0.enclosure()
            .center()
            .re
            .cmp(&b.0.enclosure().center().re)
    });
    let mut r = roots
        .get(b.index)
        .ok_or_else(|| invalid("selected constant real-root ordinal"))?
        .0
        .clone();
    if !matches!(
        r.classify_location(),
        RootLocation::Real | RootLocation::Zero
    ) {
        return Err(invalid("nonreal selected constant"));
    }
    Ok(r.refined(&Rational::from((1, 1_000_000_000_000i64)))
        .to_atom())
}
pub(super) fn constant_sign(a: &Atom, field: &Field, w: &Work<'_>) -> Result<std::cmp::Ordering> {
    w.atom(a)?;
    let mut c = AlgebraicContext::new(field.context().field().clone());
    let value = c.convert_atom(a.as_view()).map_err(native)?;
    if c.field() != field.context().field() {
        return Err(invalid("constant outside retained selected field"));
    }
    c.field().try_sign(&value).map_err(native)
}
pub(super) fn zero(field: &mut Field, a: &Atom, w: &Work<'_>) -> Result<()> {
    if !field.ratio(a, w)?.numerator.is_zero() {
        return Err(invalid("native source identity has nonzero numerator"));
    }
    Ok(())
}
pub(super) fn native_polynomial(
    owner: &PositiveProjectiveQuotient,
    b: &RootBound,
    images: &[Atom],
) -> Result<Atom> {
    let raw = owner.native_result();
    let a = Algebra::new(&raw.order).map_err(native)?;
    let p = a
        .parse(
            raw.polynomials
                .get(b.polynomial)
                .ok_or_else(|| invalid("source polynomial ordinal"))?,
        )
        .map_err(native)?;
    let roles = a
        .variables
        .iter()
        .map(|v| match v.to_atom().as_view() {
            symbolica::atom::AtomView::Var(v) => Ok(v.get_symbol()),
            _ => Err(invalid("native coordinate role")),
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(pull(&p.to_expression(), &roles, images))
}

/// Positive on the entire open prefix, with only explicit endpoint monomials
/// allowed to vanish. Every half is retained, including reflection orientation.
pub(super) fn positive_prefix(
    field: &mut Field,
    a: &Atom,
    coordinates: &[Symbol],
    allow_poles: bool,
    w: &Work<'_>,
) -> Result<Vec<FactorReceipt>> {
    let mut receipts = Vec::new();
    for upper in [false, true] {
        let u = Atom::var(coordinates[0]);
        let image = if upper {
            Atom::one() - &u / Atom::num(2)
        } else {
            &u / Atom::num(2)
        };
        let pulled = pull(a, &coordinates[..1], &[image]);
        let r = field.ratio(&pulled, w)?;
        let (n, np) = field.extract_in(&r.numerator, 2, w)?;
        let (d, dp) = field.extract_in(&r.denominator, 2, w)?;
        if !allow_poles && np.iter().zip(&dp).any(|(n, d)| n < d) {
            return Err(unsupported(
                "finite selected-width/centre has an endpoint pole",
            ));
        }
        let numerator = field.unit_with_margins(&n, &[8, 16, 32, 64], w)?;
        let denominator = field.unit_with_margins(&d, &[8, 16, 32, 64], w)?;
        if numerator.sign() * denominator.sign() != 1 {
            return Err(invalid("selected width/centre is not positive"));
        }
        receipts.push(FactorReceipt {
            origin: FactorOrigin::BoundDenominator(usize::from(upper)),
            source: a.clone(),
            powers: np
                .iter()
                .zip(dp)
                .map(|(n, d)| i64::from(*n) - i64::from(d))
                .collect(),
            numerator,
            denominator,
        });
    }
    Ok(receipts)
}

pub(super) struct Base {
    pub cell: usize,
    pub ordinal: usize,
    pub source: Arc<PositiveProjectiveCellMap>,
    pub field: Field,
    pub images: Vec<Atom>,
    pub measure: Atom,
    pub orientation: i8,
    pub sections: Vec<Arc<NormalizedRootSection>>,
    pub coverage: Arc<Vec<FactorReceipt>>,
    pub branches: Arc<Vec<BranchReceipt>>,
    pub rewrites: Vec<FactorRewrite>,
}

pub(super) fn prefix_image(
    lower: &Atom,
    upper: Option<&Atom>,
    piece: &PrefixPiece,
    u: Symbol,
    field: &Field,
    w: &Work<'_>,
) -> Result<(Atom, Atom, i8)> {
    let u = Atom::var(u);
    let [lo, hi] = &piece.fractions;
    for q in [lo, hi] {
        w.budget.rational(q)?;
    }
    if *lo < 0 || *hi > 1 || lo >= hi {
        return Err(invalid(
            "ordered prefix fractions in the closed unit interval",
        ));
    }
    if constant_sign(lower, field, w)? == std::cmp::Ordering::Less {
        return Err(invalid("negative positive-projective prefix endpoint"));
    }
    match (piece.shape, upper) {
        (PrefixShape::Ray, None) => {
            if *lo != 0 || *hi != 1 {
                return Err(invalid(
                    "complete ray requires the sole full fraction interval",
                ));
            }
            Ok((
                lower + &u / (Atom::one() - &u),
                (Atom::one() - u).pow(-2),
                1,
            ))
        }
        (PrefixShape::Ray, Some(_)) | (_, None) => Err(unsupported(
            "finite/ray prefix proposal differs from native endpoints",
        )),
        (shape, Some(upper)) => {
            let width = upper - lower;
            if constant_sign(&width, field, w)? != std::cmp::Ordering::Greater {
                return Err(invalid("source prefix endpoints are not ordered"));
            }
            let left = lower + &width * Atom::num(lo.clone());
            let width = width * Atom::num(hi - lo);
            let (image, orientation) = match shape {
                PrefixShape::Power { exponent, reversed } => {
                    if exponent == 0 || exponent as usize > w.limits.algebra.max_degree {
                        return Err(unsupported(
                            "checked positive prefix integer power required",
                        ));
                    }
                    let t = u.pow(Atom::num(exponent));
                    (
                        if reversed {
                            &left + &width * (Atom::one() - t)
                        } else {
                            &left + &width * t
                        },
                        if reversed { -1 } else { 1 },
                    )
                }
                PrefixShape::TwoEnded { exponent } => {
                    if exponent == 0 || exponent as usize > w.limits.algebra.max_degree {
                        return Err(unsupported(
                            "checked positive two-ended integer power required",
                        ));
                    }
                    let up = u.pow(Atom::num(exponent));
                    let down = (Atom::one() - &u).pow(Atom::num(exponent));
                    (&left + &width * &up / (up + down), 1)
                }
                PrefixShape::Ray => unreachable!(),
            };
            let symbol = match u.as_view() {
                symbolica::atom::AtomView::Var(v) => v.get_symbol(),
                _ => unreachable!(),
            };
            let measure = Atom::num(orientation) * image.derivative(symbol);
            Ok((image, measure, orientation))
        }
    }
}

pub(super) fn bases(
    owner: &Arc<PositiveProjectiveQuotient>,
    coordinates: &[Symbol],
    proposal: &CellProposal,
    primitive: Symbol,
    settings: &OptimizationSettings,
    w: &mut Work<'_>,
) -> Result<Vec<Base>> {
    let source = PositiveProjectiveCellMap::new(owner.clone(), proposal.cell, coordinates.to_vec())
        .map_err(native)?;
    if source.axes().len() != 2 {
        return Err(unsupported(
            "two-dimensional triangular pair family required",
        ));
    }
    let lo = selected_constant(
        owner,
        ConstantReference {
            cell: proposal.cell,
            side: SectionSide::Lower,
        },
        w.limits,
    )?;
    let hi = if bound(owner, proposal.cell, 0, SectionSide::Upper)?.is_some() {
        Some(selected_constant(
            owner,
            ConstantReference {
                cell: proposal.cell,
                side: SectionSide::Upper,
            },
            w.limits,
        )?)
    } else {
        None
    };
    let mut constants = vec![lo.clone()];
    constants.extend(hi.iter().cloned());
    for r in &proposal.constants {
        constants.push(selected_constant(owner, *r, w.limits)?);
    }
    if proposal.pieces.is_empty() {
        return Err(invalid("missing source prefix subdivision"));
    }
    let mut end = Rational::zero();
    for p in &proposal.pieces {
        if p.fractions[0] != end {
            return Err(invalid("missing or overlapping prefix sibling"));
        }
        end = p.fractions[1].clone();
    }
    if end != 1 {
        return Err(invalid("incomplete source prefix subdivision"));
    }
    if hi.is_none() && proposal.pieces.len() != 1 {
        return Err(invalid(
            "ray subdivision not admitted by this initial grammar",
        ));
    }
    let mut result = Vec::new();
    for (piece_index, piece) in proposal.pieces.iter().enumerate() {
        w.check()?;
        let roles = coordinates
            .iter()
            .copied()
            .chain(piece.roots.iter().map(|r| r.role))
            .collect::<Vec<_>>();
        let bounds = std::iter::repeat_n([Rational::zero(), Rational::one()], 2)
            .chain(
                piece
                    .roots
                    .iter()
                    .map(|r| [r.bracket.lower.clone(), r.bracket.upper.clone()]),
            )
            .collect::<Vec<_>>();
        let mut field = Field::new_with_bounds(
            &constants,
            &roles,
            primitive,
            &bounds,
            &Rational::from((1, 1_000_000_000_000i64)),
            w,
        )?;
        let (x, dx, orientation) =
            prefix_image(&lo, hi.as_ref(), piece, coordinates[0], &field, w)?;
        let mut coverage = positive_prefix(&mut field, &dx, coordinates, true, w)?;
        let mut sections = pair::prepare(owner, proposal.cell, &x, &roles, piece, &mut field, w)?;
        let v = Atom::var(coordinates[1]);
        let mut branches = Vec::new();
        let fibres = match &piece.fibre {
            FibreProposal::Native {
                lower,
                upper,
                reversed,
            } => {
                branches.push(pair::verify_bound(
                    owner,
                    proposal.cell,
                    SectionSide::Lower,
                    &x,
                    lower,
                    &roles,
                    &mut sections,
                    &mut field,
                    w,
                )?);
                match (upper, bound(owner, proposal.cell, 1, SectionSide::Upper)?) {
                    (Some(upper), Some(_)) => {
                        branches.push(pair::verify_bound(
                            owner,
                            proposal.cell,
                            SectionSide::Upper,
                            &x,
                            upper,
                            &roles,
                            &mut sections,
                            &mut field,
                            w,
                        )?);
                        let width = upper - lower;
                        coverage.extend(positive_prefix(
                            &mut field,
                            &width,
                            coordinates,
                            false,
                            w,
                        )?);
                        let y = if *reversed {
                            upper - &width * &v
                        } else {
                            lower + &width * &v
                        };
                        vec![(y, width, if *reversed { -1 } else { 1 })]
                    }
                    (None, None) => {
                        if *reversed {
                            return Err(unsupported("reversed ray fibre proposal"));
                        }
                        vec![(
                            lower + &v / (Atom::one() - &v),
                            (Atom::one() - &v).pow(-2),
                            1,
                        )]
                    }
                    _ => {
                        return Err(invalid(
                            "native finite/ray fibre endpoint inventory differs",
                        ));
                    }
                }
            }
            FibreProposal::PositiveCentre { centre } => {
                if bound(owner, proposal.cell, 1, SectionSide::Upper)?.is_some()
                    || !sections.is_empty()
                {
                    return Err(unsupported(
                        "positive-centre split requires an unsplit positive ray fibre",
                    ));
                }
                branches.push(pair::verify_bound(
                    owner,
                    proposal.cell,
                    SectionSide::Lower,
                    &x,
                    &Atom::zero(),
                    &roles,
                    &mut sections,
                    &mut field,
                    w,
                )?);
                coverage.extend(positive_prefix(&mut field, centre, coordinates, false, w)?);
                vec![
                    (centre * (Atom::one() - &v), centre.clone(), -1),
                    (
                        centre + &v / (Atom::one() - &v),
                        (Atom::one() - &v).pow(-2),
                        1,
                    ),
                ]
            }
        };
        let sealed = pair::seal(owner, proposal.cell, &x, &roles, sections, settings, w)?;
        let coverage = Arc::new(coverage);
        let branches = Arc::new(branches);
        if piece.rewrites.iter().any(|r| r.fibre >= fibres.len()) {
            return Err(invalid("factor rewrite fibre ordinal"));
        }
        for (fibre, (y, dy, ysign)) in fibres.into_iter().enumerate() {
            let j = &dx * &dy;
            let determinant = x.derivative(coordinates[0]) * y.derivative(coordinates[1]);
            zero(
                &mut field,
                &(determinant - Atom::num(orientation * ysign) * &j),
                w,
            )?;
            // Reconstructing the same selected field is native and bounded. It
            // keeps each base owner independent without cloning hidden state.
            let local = Field::new_with_bounds(
                &constants,
                &roles,
                primitive,
                &bounds,
                &Rational::from((1, 1_000_000_000_000i64)),
                w,
            )?;
            result.push(Base {
                cell: proposal.cell,
                ordinal: piece_index
                    .checked_mul(2)
                    .and_then(|x| x.checked_add(fibre))
                    .ok_or_else(|| invalid("base path overflow"))?,
                source: source.clone(),
                field: local,
                images: vec![x.clone(), y],
                measure: j,
                orientation: orientation * ysign,
                sections: sealed.clone(),
                coverage: coverage.clone(),
                branches: branches.clone(),
                rewrites: piece
                    .rewrites
                    .iter()
                    .filter(|r| r.fibre == fibre)
                    .cloned()
                    .collect(),
            });
        }
    }
    Ok(result)
}
