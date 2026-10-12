//! Native normalized-root certificates and original selected-fibre association.
use super::projective_family::algebra::{Field, Polynomial, pull};
use super::*;
use std::cmp::Ordering;
use symbolica::{
    atom::AtomCore,
    domains::{RealEmbedding, algebraic::AlgebraicContext, rational::Q},
};
use symgcad::output::RootIndexDomain;

fn context(error: Error, label: &str) -> Error {
    match error {
        Error::Unsupported(s) => unsupported(format!("{label}: {s}")),
        Error::Invalid(s) => invalid(format!("{label}: {s}")),
        Error::Native(s) => native(format!("{label}: {s}")),
        other => other,
    }
}

pub(super) struct Pending {
    role: Symbol,
    equation: Atom,
    polynomial: Polynomial,
    denominator: ClosedUnit,
    coefficients: Vec<Atom>,
    bracket: BracketProposal,
    orientation: i8,
    margin: ClosedUnit,
    endpoints: [ClosedUnit; 2],
    witness: Rational,
    anchor: Atom,
    branches: Vec<BranchReceipt>,
}

fn sign(c: &mut AlgebraicContext, a: &Atom, w: &Work<'_>) -> Result<Ordering> {
    w.atom(a)?;
    c.extend(a.as_view()).map_err(native)?;
    if c.field().poly().degree(0) as usize > w.limits.max_field_degree {
        return Err(Error::ResourceIncomplete(
            "selected fibre compositum degree".into(),
        ));
    }
    let x = c.convert_atom(a.as_view()).map_err(native)?;
    c.field().try_sign(&x).map_err(native)
}
fn real_roots(p: &Polynomial, axis: usize, w: &Work<'_>) -> Result<Vec<Atom>> {
    if (0..p.nvars()).any(|i| i != axis && p.degree(i) != 0)
        || p.coefficients.iter().any(|c| !c.poly().is_constant())
    {
        return Err(unsupported(
            "initial selected-fibre anchoring requires rational specialized coefficients",
        ));
    }
    let qq = p.map_coeff(|c| c.poly().get_constant(), Q);
    let mut r = qq.to_univariate_from_univariate(axis).isolate_real_roots();
    r.sort_by(|a, b| {
        a.0.enclosure()
            .center()
            .re
            .cmp(&b.0.enclosure().center().re)
    });
    w.check()?;
    Ok(r.into_iter()
        .map(|(r, _)| {
            r.refined(&Rational::from((1, 1_000_000_000_000i64)))
                .to_atom()
        })
        .collect())
}

pub(super) fn prepare(
    owner: &Arc<PositiveProjectiveQuotient>,
    cell: usize,
    x: &Atom,
    roles: &[Symbol],
    piece: &PrefixPiece,
    field: &mut Field,
    w: &Work<'_>,
) -> Result<Vec<Pending>> {
    let mut result = Vec::new();
    for (ri, r) in piece.roots.iter().enumerate() {
        w.check()?;
        if r.witness <= 0
            || r.witness >= 1
            || r.bracket.lower >= r.bracket.upper
            || r.bracket.derivative_margin <= 0
        {
            return Err(invalid(
                "interior normalized-root witness and positive bracket margin required",
            ));
        }
        for q in [
            &r.witness,
            &r.bracket.lower,
            &r.bracket.upper,
            &r.bracket.derivative_margin,
        ] {
            w.budget.rational(q)?;
        }
        let q = field.ratio(&r.equation, w)?;
        let axis = 2 + ri;
        if q.numerator.degree(axis) == 0
            || q.denominator.degree(axis) != 0
            || (1..roles.len())
                .any(|i| i != axis && (q.numerator.degree(i) != 0 || q.denominator.degree(i) != 0))
        {
            return Err(unsupported(
                "normalized equation depends on one prefix role and its own root only",
            ));
        }
        let denominator = field
            .unit_with_margins(&q.denominator, &[8, 16, 32, 64], w)
            .map_err(|e| context(e, "normalized equation denominator"))?;
        let equation = field.expression(&q.numerator)?;
        let derivative = q.numerator.derivative(axis);
        let unit = field
            .unit_with_margins(&derivative, &[8, 16, 32, 64], w)
            .map_err(|e| context(e, "normalized derivative sign"))?;
        let orientation = unit.sign();
        let target = field.polynomial(
            &(Atom::num(orientation) * field.expression(&derivative)?
                - Atom::num(r.bracket.derivative_margin.clone())),
            w,
        )?;
        let margin = field
            .unit_with_margins(&target, &[8, 16, 32, 64], w)
            .map_err(|e| context(e, "normalized cleared-polynomial derivative margin"))?;
        if margin.sign() != 1 {
            return Err(invalid("normalized derivative margin sign"));
        }
        let mut endpoints = Vec::new();
        for (endpoint, expected) in [
            (&r.bracket.lower, -orientation),
            (&r.bracket.upper, orientation),
        ] {
            let p = field.polynomial(
                &pull(&equation, &[r.role], &[Atom::num(endpoint.clone())]),
                w,
            )?;
            let proof = field
                .unit_with_margins(&p, &[8, 16, 32, 64], w)
                .map_err(|e| context(e, "normalized bracket endpoint"))?;
            if proof.sign() != expected {
                return Err(invalid(
                    "normalized endpoint signs do not isolate the requested branch",
                ));
            }
            endpoints.push(proof);
        }
        let univariate = q.numerator.to_univariate(axis);
        let coefficients = univariate
            .coefficients()
            .iter()
            .map(|p| field.expression(p))
            .collect::<Result<Vec<_>>>()?;
        let specialized = field.polynomial(
            &pull(&equation, &roles[..1], &[Atom::num(r.witness.clone())]),
            w,
        )?;
        let mut selected = Vec::new();
        for root in real_roots(&specialized, axis, w)? {
            let mut ctx = AlgebraicContext::new(field.context().field().clone());
            if sign(&mut ctx, &(&root - Atom::num(r.bracket.lower.clone())), w)?
                == Ordering::Greater
                && sign(&mut ctx, &(Atom::num(r.bracket.upper.clone()) - &root), w)?
                    == Ordering::Greater
            {
                selected.push(root);
            }
        }
        if selected.len() != 1 {
            return Err(invalid(
                "normalized anchor is not the unique bracketed native root",
            ));
        }
        let source_x = pull(x, &roles[..1], &[Atom::num(r.witness.clone())]);
        let px = field.polynomial(&source_x, w)?;
        if !px.is_constant() || !px.get_constant().poly().is_constant() {
            return Err(unsupported(
                "initial original-fibre anchor must be rational",
            ));
        }
        let _ = (owner, cell);
        result.push(Pending {
            role: r.role,
            equation,
            polynomial: q.numerator,
            denominator,
            coefficients,
            bracket: r.bracket.clone(),
            orientation,
            margin,
            endpoints: endpoints
                .try_into()
                .map_err(|_| invalid("root endpoint inventory"))?,
            witness: r.witness.clone(),
            anchor: selected.remove(0),
            branches: Vec::new(),
        });
    }
    Ok(result)
}

// Keep the original cell/side, proposed images and native proof context explicit
// at this private certificate boundary rather than hiding source association.
#[allow(clippy::too_many_arguments)]
pub(super) fn verify_bound(
    owner: &Arc<PositiveProjectiveQuotient>,
    cell: usize,
    side: SectionSide,
    x: &Atom,
    image: &Atom,
    roles: &[Symbol],
    roots: &mut [Pending],
    field: &mut Field,
    w: &Work<'_>,
) -> Result<BranchReceipt> {
    let b = geometry::bound(owner, cell, 1, side)?
        .ok_or_else(|| invalid("missing original finite fibre bound"))?;
    if b.index_domain != RootIndexDomain::Real {
        return Err(unsupported(
            "initial moving-bound issuer requires real-root selectors",
        ));
    }
    let original = geometry::native_polynomial(owner, b, &[x.clone(), image.clone()])?;
    let ratio = field.ratio(&original, w)?;
    let mut remainder = ratio.numerator;
    for root in roots.iter() {
        remainder = remainder.quot_rem(&root.polynomial, false).1;
    }
    if !remainder.is_zero() {
        return Err(invalid(
            "original selected polynomial pullback does not vanish modulo admitted normalized equations",
        ));
    }
    let image_ratio = field.ratio(image, w)?;
    let denominator = field.unit_with_margins(&image_ratio.denominator, &[8, 16, 32, 64], w)?;
    let _ = denominator;
    let used = roots
        .iter()
        .enumerate()
        .filter_map(|(i, r)| image.contains(Atom::var(r.role).as_view()).then_some(i))
        .collect::<Vec<_>>();
    let source_images = owner
        .native_result()
        .order
        .iter()
        .map(|name| {
            owner
                .aliases()
                .iter()
                .find(|a| &a.name == name)
                .map(|a| Atom::var(a.symbol))
                .ok_or_else(|| invalid("original named coordinate alias"))
        })
        .collect::<Result<Vec<_>>>()?;
    let original_body = geometry::native_polynomial(owner, b, &source_images)?;
    let native_body =
        geometry::native_polynomial(owner, b, &[Atom::var(roles[0]), Atom::var(roles[1])])?;
    if used.is_empty()
        && !native_body.contains(Atom::var(roles[0]).as_view())
        && !image.contains(Atom::var(roles[0]).as_view())
    {
        let source = field.polynomial(&native_body, w)?;
        let values = real_roots(&source, 1, w)?;
        let expected = values
            .get(b.index)
            .ok_or_else(|| invalid("constant fibre root ordinal"))?
            .clone();
        let mut context = AlgebraicContext::new(field.context().field().clone());
        if sign(&mut context, &(image - &expected), w)? != Ordering::Equal {
            return Err(invalid("different constant fibre root"));
        }
        return Ok(BranchReceipt {
            side,
            bound: b.clone(),
            original: original_body,
            image: image.clone(),
            source_fibre: None,
            selected_anchor: expected,
        });
    }
    let witness = used
        .first()
        .map(|i| roots[*i].witness.clone())
        .unwrap_or_else(|| Rational::from((1, 2)));
    if used.iter().any(|i| roots[*i].witness != witness) {
        return Err(unsupported("one common exact source-fibre anchor required"));
    }
    let sx = pull(x, &roles[..1], &[Atom::num(witness.clone())]);
    let sx = field.polynomial(&sx, w)?;
    if !sx.is_constant() || !sx.get_constant().poly().is_constant() {
        return Err(unsupported("rational original anchor required"));
    }
    let sx = sx.get_constant().poly().get_constant();
    let source_poly =
        geometry::native_polynomial(owner, b, &[Atom::num(sx.clone()), Atom::var(roles[1])])?;
    let source_poly = field.polynomial(&source_poly, w)?;
    let source_roots = real_roots(&source_poly, 1, w)?;
    let expected = source_roots
        .get(b.index)
        .ok_or_else(|| invalid("original real-root ordinal at source anchor"))?
        .clone();
    let mut from = vec![roles[0]];
    let mut to = vec![Atom::num(witness.clone())];
    for i in &used {
        from.push(roots[*i].role);
        to.push(roots[*i].anchor.clone());
    }
    let image_at = pull(image, &from, &to);
    let mut context = AlgebraicContext::new(field.context().field().clone());
    if sign(&mut context, &(&image_at - &expected), w)? != Ordering::Equal {
        return Err(invalid(
            "normalized image selects a different original source root",
        ));
    }
    let receipt = BranchReceipt {
        side,
        bound: b.clone(),
        original: original_body,
        image: image.clone(),
        source_fibre: Some(sx),
        selected_anchor: expected,
    };
    for i in used {
        roots[i].branches.push(receipt.clone());
    }
    Ok(receipt)
}

pub(super) fn seal(
    owner: &Arc<PositiveProjectiveQuotient>,
    cell: usize,
    x: &Atom,
    roles: &[Symbol],
    roots: Vec<Pending>,
    settings: &OptimizationSettings,
    w: &Work<'_>,
) -> Result<Vec<Arc<NormalizedRootSection>>> {
    use crate::generation::identity::CanonicalAtom;
    let mut result = Vec::new();
    for (i, r) in roots.into_iter().enumerate() {
        if r.branches.is_empty() {
            return Err(invalid(
                "normalized root has no retained original selected-bound association",
            ));
        }
        let mut digest = blake3::Hasher::new();
        serde_json::to_writer(
            &mut digest,
            &crate::generation::source_identity(owner.preparation().input(), &[], &[])
                .map_err(native)?,
        )
        .map_err(native)?;
        let canonical_roles = (0..roles.len())
            .map(|j| {
                Symbol::parse(format!("role_{j}"), "fastsecdec_pair_contract")
                    .map(Atom::var)
                    .map_err(native)
            })
            .collect::<Result<Vec<_>>>()?;
        let canonical_x = pull(x, roles, &canonical_roles);
        let canonical_q = pull(&r.equation, roles, &canonical_roles);
        let root_axis = roles
            .iter()
            .position(|s| *s == r.role)
            .ok_or_else(|| invalid("root contract role inventory"))?;
        serde_json::to_writer(
            &mut digest,
            &(
                "fastsecdec-positive-projective-normalized-root-v1",
                &owner.native_result().order,
                cell,
                CanonicalAtom(&canonical_x),
                root_axis,
                CanonicalAtom(&canonical_q),
                r.bracket.lower.to_string(),
                r.bracket.upper.to_string(),
                r.orientation,
            ),
        )
        .map_err(native)?;
        // Mathematical cell boundaries use their named polynomial and selector;
        // native isolating enclosures and proof/work receipts remain provenance.
        let source_roles = owner
            .native_result()
            .order
            .iter()
            .map(|name| {
                owner
                    .aliases()
                    .iter()
                    .find(|a| &a.name == name)
                    .map(|a| Atom::var(a.symbol))
                    .ok_or_else(|| invalid("source role inventory"))
            })
            .collect::<Result<Vec<_>>>()?;
        for axis in 0..source_roles.len() {
            for side in [SectionSide::Lower, SectionSide::Upper] {
                if let Some(bound) = geometry::bound(owner, cell, axis, side)? {
                    let polynomial = geometry::native_polynomial(owner, bound, &source_roles)?;
                    serde_json::to_writer(
                        &mut digest,
                        &(
                            axis,
                            format!("{side:?}"),
                            Some((CanonicalAtom(&polynomial), &bound.index_domain, bound.index)),
                        ),
                    )
                    .map_err(native)?;
                } else {
                    serde_json::to_writer(&mut digest, &(axis, format!("{side:?}"), None::<()>))
                        .map_err(native)?;
                }
            }
        }
        for b in &r.branches {
            let image = pull(&b.image, roles, &canonical_roles);
            serde_json::to_writer(
                &mut digest,
                &(
                    (&b.bound.index_domain, b.bound.index),
                    CanonicalAtom(&b.original),
                    CanonicalAtom(&image),
                    b.source_fibre.as_ref().map(ToString::to_string),
                    CanonicalAtom(&b.selected_anchor),
                ),
            )
            .map_err(native)?;
        }
        let contract = serde_json::to_vec(&(
            "positive-projective-pair-v1",
            digest.finalize().to_hex().to_string(),
        ))
        .map_err(native)?;
        let program = RootProgram::prepare_shifted_from_certificate(
            r.coefficients.len() - 1,
            r.bracket.lower.clone(),
            r.bracket.upper.clone(),
            contract,
            settings.clone(),
        )
        .map_err(native)?;
        let tag = Symbol::parse(
            format!("pair_{cell}_{i}_{}", digest.finalize().to_hex()),
            "fastsecdec_projective_pair",
        )
        .map_err(native)?;
        if owner.occupied_symbols().contains(&tag) || roles.contains(&tag) || !tag.is_exportable() {
            return Err(invalid("normalized root callback tag collision"));
        }
        w.check()?;
        result.push(Arc::new(NormalizedRootSection {
            owner: owner.clone(),
            cell,
            prefix: x.clone(),
            parameter: roles[0],
            role: r.role,
            equation: r.equation,
            equation_denominator: r.denominator,
            coefficients: r.coefficients,
            bracket: r.bracket,
            orientation: r.orientation,
            margin: r.margin,
            endpoints: r.endpoints,
            branches: r.branches,
            program,
            tag,
        }));
    }
    Ok(result)
}
