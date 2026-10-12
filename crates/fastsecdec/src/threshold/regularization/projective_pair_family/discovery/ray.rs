use super::*;

pub(super) fn rays(
    context: Context<'_>,
    bound_polys: [Option<Atom>; 2],
    options: &DiscoveryOptions,
    w: &Work<'_>,
) -> Result<Vec<CellProposal>> {
    let Context {
        cell,
        units,
        lo,
        hi,
        factors,
        mut field,
        ..
    } = context;
    let v = Atom::var(units[1]);
    // Whole positive rays: straight maps first; endpoint centre folds are
    // proposed only where a native quadratic discriminant actually vanishes.
    if bound_polys[1].is_some() {
        return Err(unsupported(
            "automatic constant bounded fibre not yet enumerated",
        ));
    }
    let lower = bound_polys[0]
        .as_ref()
        .ok_or_else(|| unsupported("positive ray lower boundary"))?;
    let c = coeffs(lower, &mut field, w)?;
    if c.len() != 2 || !zero(&c[0], &mut field, w)? {
        return Err(unsupported(
            "automatic plain fibre currently starts at zero",
        ));
    }
    let full = [Rational::zero(), Rational::one()];
    let mut result = vec![CellProposal {
        cell,
        constants: Vec::new(),
        pieces: vec![basic(
            full,
            if hi.is_some() {
                PrefixShape::Power {
                    exponent: 1,
                    reversed: false,
                }
            } else {
                PrefixShape::Ray
            },
        )],
    }];
    let Some(hi) = hi else { return Ok(result) };
    for (_, _, p) in &factors {
        let pp = field.polynomial(p, w)?;
        if pp.degree(1) != 2 {
            continue;
        }
        let c = coeffs(p, &mut field, w)?;
        let centre = -&c[1] / (Atom::num(2) * &c[2]);
        let disc = discriminant(&pp, &mut field, w)?;
        let mut collision = [false; 2];
        for (i, x) in [&lo, &hi].into_iter().enumerate() {
            collision[i] = zero(&at(&disc, units[0], x, &mut field, w)?, &mut field, w)?
                && geometry::constant_sign(&at(&centre, units[0], x, &mut field, w)?, &field, w)?
                    == Ordering::Greater;
        }
        if !collision.into_iter().any(|x| x) {
            continue;
        }
        for level in 1..=options.max_subdivision_depth {
            let count = 1u32
                .checked_shl(level)
                .ok_or_else(|| Error::ResourceIncomplete("dyadic partition overflow".into()))?;
            if count as usize > w.limits.max_charts / 4 {
                return Err(Error::ResourceIncomplete(
                    "dyadic complete-half allowance".into(),
                ));
            }
            let mut pieces = Vec::new();
            for i in 0..count {
                let fractions = [Rational::from((i, count)), Rational::from((i + 1, count))];
                let reversed = i + 1 == count && collision[1];
                let fold = (i == 0 && collision[0]) || reversed;
                let mut piece = basic(
                    fractions,
                    PrefixShape::Power {
                        exponent: if fold { 2 } else { 1 },
                        reversed,
                    },
                );
                if fold {
                    let (x, _, _) =
                        geometry::prefix_image(&lo, Some(&hi), &piece, units[0], &field, w)?;
                    let centre = at(&centre, units[0], &x, &mut field, w)?;
                    let aa = at(&c[2], units[0], &x, &mut field, w)?;
                    let rem = canonical(
                        &(-at(&disc, units[0], &x, &mut field, w)? / (Atom::num(4) * &aa)),
                        &mut field,
                        w,
                    )?;
                    for fibre in 0..2 {
                        let offset = if fibre == 0 {
                            -&centre * &v
                        } else {
                            &v / (Atom::one() - &v)
                        };
                        let y = &centre + &offset;
                        let body = &aa * offset.pow(2) + &rem;
                        piece.rewrites.extend(rewrite_rows(
                            &pp,
                            &factors,
                            RewriteTarget {
                                image: &[x.clone(), y],
                                body: &body,
                                fibre,
                            },
                            units,
                            &mut field,
                            w,
                        )?);
                    }
                    piece.fibre = FibreProposal::PositiveCentre { centre };
                }
                pieces.push(piece);
            }
            result.push(CellProposal {
                cell,
                constants: Vec::new(),
                pieces,
            });
        }
    }
    Ok(result)
}
