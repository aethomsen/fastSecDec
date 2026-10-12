//! Native Q(i)[coordinates,epsilon] linear-resultant denominator transport.
//! Geometry retains its real selected field; it is never extended by i/epsilon.
use super::projective_family::algebra::{Field, Polynomial, pull};
use super::*;
use symbolica::{
    atom::{AtomCore, AtomView},
    domains::{
        algebraic::{AlgebraicContext, AlgebraicExtension},
        float::Complex,
        rational::Q,
    },
    poly::PolyVariable,
};

pub(super) struct Template {
    pub term: usize,
    pub indices: Vec<usize>,
    pub source: Atom,
    pub roles: Vec<Symbol>,
    pub degrees: Vec<u32>,
    pub expression: Atom,
}
fn bounded(p: &Polynomial, w: &Work<'_>) -> Result<()> {
    w.check()?;
    if p.nterms() > w.limits.prefactors.terms
        || (0..p.nvars()).any(|i| p.degree(i) > u32::from(w.limits.prefactors.degree))
    {
        return Err(Error::ResourceIncomplete(
            "native numerator polynomial degree/terms".into(),
        ));
    }
    for c in &p.coefficients {
        for q in &c.poly().coefficients {
            w.budget.rational(q)?;
        }
    }
    Ok(())
}

pub(super) fn prepare(
    term: usize,
    indices: Vec<usize>,
    source: Atom,
    coordinates: &[Symbol],
    eps: Symbol,
    roles: Vec<Symbol>,
    w: &Work<'_>,
) -> Result<Template> {
    meromorphic::preflight_expression(&source, w.limits.prefactors).map_err(prefactor_error)?;
    let mut failure = None;
    source.visitor(&mut |v| {
        if failure.is_some() {
            return false;
        }
        failure = match v {
            AtomView::Num(_) if Complex::<Rational>::try_from(v).is_err() => {
                Some(unsupported("numerator coefficient must be exact Q(i)"))
            }
            AtomView::Var(v) if !coordinates.contains(&v.get_symbol()) && v.get_symbol() != eps => {
                Some(unsupported("unbound numerator coefficient symbol"))
            }
            AtomView::Fun(_) => Some(unsupported(
                "numerator must be polynomial in source coordinates and regulator",
            )),
            AtomView::Pow(p) => match Rational::try_from(p.get_exp()) {
                Ok(e) if e.is_integer() && e >= 0 => None,
                _ => Some(unsupported("numerator nonnegative integer powers required")),
            },
            _ => None,
        };
        failure.is_none()
    });
    if let Some(e) = failure {
        return Err(e);
    }
    if roles.len() != 2 * coordinates.len() {
        return Err(invalid("numerator formal-role inventory"));
    }
    let vars = Arc::new(
        coordinates
            .iter()
            .copied()
            .chain([eps])
            .chain(roles.iter().copied())
            .map(PolyVariable::from)
            .collect::<Vec<_>>(),
    );
    let mut context = AlgebraicContext::new(AlgebraicExtension::complex(Q));
    let mut p = context
        .to_polynomial::<u32>(source.as_view(), vars.clone())
        .map_err(native)?;
    if context.field() != &AlgebraicExtension::complex(Q) {
        return Err(unsupported("numerator coefficient field is not Q(i)"));
    }
    bounded(&p, w)?;
    let degrees = (0..coordinates.len())
        .map(|i| p.degree(i))
        .collect::<Vec<_>>();
    for (axis, degree) in degrees.iter().enumerate() {
        if *degree == 0 {
            continue;
        }
        w.check()?;
        let linear = Atom::var(roles[2 * axis + 1]) * Atom::var(coordinates[axis])
            - Atom::var(roles[2 * axis]);
        let linear = context
            .to_polynomial::<u32>(linear.as_view(), vars.clone())
            .map_err(native)?;
        let position = p
            .variables()
            .iter()
            .position(|v| v == &PolyVariable::from(coordinates[axis]))
            .ok_or_else(|| invalid("native resultant coordinate disappeared"))?;
        // Native public linear-first resultant uses homogeneous Horner internally:
        // Res(d*x-n,P(x)) = d^degree P(n/d), including odd degrees.
        p = linear
            .to_univariate(axis)
            .resultant(&p.to_univariate(position));
        bounded(&p, w)?;
    }
    for coordinate in coordinates {
        if let Some(axis) = p
            .variables()
            .iter()
            .position(|v| v == &PolyVariable::from(*coordinate))
            && p.degree(axis) != 0
        {
            return Err(invalid("resultant retained original coordinate"));
        }
    }
    let expression = p.to_expression_with_coeff_map(|ring, c, out| *out = ring.element_to_atom(c));
    w.atom(&expression)?;
    Ok(Template {
        term,
        indices,
        source,
        roles,
        degrees,
        expression,
    })
}

pub(super) fn compose(
    template: &Template,
    images: &[Atom],
    field: &mut Field,
    coordinates: &[Symbol],
    root_roles: &[Symbol],
    root_calls: &[Atom],
    w: &Work<'_>,
) -> Result<NumeratorTransport> {
    let mut finite = Vec::new();
    let mut powers = vec![0i64; coordinates.len()];
    let mut denominators = Vec::new();
    let mut residual_denominator = Atom::one();
    for (axis, (image, degree)) in images.iter().zip(&template.degrees).enumerate() {
        let r = field.ratio(image, w)?;
        finite.push(field.expression(&r.numerator)?);
        finite.push(field.expression(&r.denominator)?);
        if *degree == 0 {
            continue;
        }
        let one = r.denominator.one();
        let receipt = super::leaves::receipt(
            field,
            FactorOrigin::BoundDenominator(axis),
            field.expression(&r.denominator)?,
            &r.denominator,
            &one,
            w,
        )?;
        // A denominator may vanish only by its explicit endpoint monomial.
        // The residual has a native closed signed margin; integer powers retain sign.
        for (p, d) in powers.iter_mut().zip(receipt.powers()) {
            *p = p
                .checked_sub(
                    d.checked_mul(i64::from(*degree))
                        .ok_or_else(|| invalid("numerator degree transport overflow"))?,
                )
                .ok_or_else(|| invalid("numerator power sum overflow"))?;
        }
        let denominator = receipt.numerator().expression() / receipt.denominator().expression();
        residual_denominator *= denominator.pow(Atom::num(*degree));
        denominators.push(receipt);
    }
    let regular = pull(&template.expression, &template.roles, &finite) / residual_denominator;
    // Keep native composition factored. The cleared numerator may vanish; its
    // endpoint valuation is not guessed from degree bounds and no unit is claimed.
    let regular = pull(&regular, root_roles, root_calls);
    w.atom(&regular)?;
    Ok(NumeratorTransport {
        term: template.term,
        source: template.source.clone(),
        factor_indices: template.indices.clone(),
        template_roles: template.roles.clone(),
        finite_images: finite,
        degrees: template.degrees.clone(),
        template: template.expression.clone(),
        regular,
        powers,
        denominators,
    })
}
