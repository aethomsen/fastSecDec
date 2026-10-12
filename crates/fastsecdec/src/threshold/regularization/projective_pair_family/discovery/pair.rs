use super::*;

pub(super) fn moving(context: Context<'_>, p: Atom, w: &Work<'_>) -> Result<Vec<CellProposal>> {
    let Context {
        owner,
        cell,
        units,
        root,
        lo,
        hi,
        factors,
        mut field,
    } = context;
    let [u, v] = [Atom::var(units[0]), Atom::var(units[1])];
    let z = Atom::var(root);

    let Some(hi) = hi else {
        return Err(unsupported(
            "moving pair on unbounded prefix needs another normalization",
        ));
    };
    let pp = field.polynomial(&p, w)?;
    let c = coeffs(&p, &mut field, w)?;
    let disc = discriminant(&pp, &mut field, w)?;
    if !zero(&at(&disc, units[0], &lo, &mut field, w)?, &mut field, w)?
        || !zero(&at(&disc, units[0], &hi, &mut field, w)?, &mut field, w)?
    {
        return Err(unsupported(
            "automatic pair requires both retained prefix endpoints to be discriminant sections",
        ));
    }
    let width = &hi - &lo;
    let d = u.pow(2) + (Atom::one() - &u).pow(2);
    let x = &lo + &width * u.pow(2) / &d;
    let aa = at(&c[2], units[0], &x, &mut field, w)?;
    let bb = at(&c[1], units[0], &x, &mut field, w)?;
    let cc = at(&c[0], units[0], &x, &mut field, w)?;
    let leading_at = at(
        &c[2],
        units[0],
        &((&lo + &hi) / Atom::num(2)),
        &mut field,
        w,
    )?;
    if geometry::constant_sign(&leading_at, &field, w)? != Ordering::Greater {
        return Err(unsupported(
            "positive-leading quadratic automatic pair class",
        ));
    }
    let scale = &width * &u * (Atom::one() - &u) / &d;
    let h = canonical(
        &(at(&disc, units[0], &x, &mut field, w)? / scale.pow(2)),
        &mut field,
        w,
    )?;
    let (lower, upper) = bracket(&h, units[0], &mut field, w)?;
    let equation = z.pow(2) - h;
    let ratio = field.ratio(&equation, w)?;
    let native = ratio.numerator.to_univariate(2);
    let content = native.content();
    let prim = native.make_primitive().flatten();
    if &content * &prim != ratio.numerator {
        return Err(invalid("native primitive coefficient reconstruction"));
    }
    let equation = field.expression(&prim)?;
    let centre = -bb / (Atom::num(2) * &aa);
    let plus = &centre + &scale * &z / (Atom::num(2) * &aa);
    let minus = cc / (&aa * &plus);
    let gap = &scale * &z / &aa;
    let mut endpoints = Vec::new();
    for side in [SectionSide::Lower, SectionSide::Upper] {
        let bound = geometry::bound(owner, cell, 1, side)?;
        let endpoint = if let Some(b) = bound {
            let original = geometry::native_polynomial(owner, b, &[u.clone(), v.clone()])?;
            let p = field.polynomial(&original, w)?;
            if p.degree(0) == 0 {
                let c = coeffs(&original, &mut field, w)?;
                if c.len() != 2 {
                    return Err(unsupported(
                        "constant fibre boundary automatic linear class",
                    ));
                }
                Some(canonical(&(-&c[0] / &c[1]), &mut field, w)?)
            } else {
                match b.index {
                    0 => Some(minus.clone()),
                    1 => Some(plus.clone()),
                    _ => return Err(unsupported("quadratic selected real-root index")),
                }
            }
        } else {
            None
        };
        endpoints.push(endpoint);
    }
    let low = endpoints[0]
        .clone()
        .ok_or_else(|| unsupported("positive fibre lower boundary required"))?;
    let high = endpoints[1].clone();
    let low_zero = zero(&low, &mut field, w)?;
    let reversed = low_zero && high.is_some();
    let (y, body) = if reversed {
        let r = high.as_ref().unwrap();
        (r * (Atom::one() - &v), &aa * r * &v * (&gap + r * &v))
    } else if let Some(high) = &high {
        (
            &low + (high - &low) * &v,
            -&aa * gap.pow(2) * &v * (Atom::one() - &v),
        )
    } else {
        (
            &low + &v / (Atom::one() - &v),
            &aa * &v * ((Atom::one() - &v) * &gap + &v) / (Atom::one() - &v).pow(2),
        )
    };
    let rewrites = rewrite_rows(
        &pp,
        &factors,
        RewriteTarget {
            image: &[x, y],
            body: &body,
            fibre: 0,
        },
        units,
        &mut field,
        w,
    )?;
    let piece = PrefixPiece {
        fractions: [Rational::zero(), Rational::one()],
        shape: PrefixShape::TwoEnded { exponent: 2 },
        roots: vec![RootProposal {
            role: root,
            equation,
            bracket: BracketProposal {
                lower,
                upper,
                derivative_margin: Rational::from((1, 1i64 << 32)),
            },
            witness: Rational::from((1, 2)),
        }],
        fibre: FibreProposal::Native {
            lower: low,
            upper: high,
            reversed,
        },
        rewrites,
    };
    Ok(vec![CellProposal {
        cell,
        constants: Vec::new(),
        pieces: vec![piece],
    }])
}
