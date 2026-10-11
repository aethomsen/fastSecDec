use super::*;
use crate::threshold::gcad::{AliasRole, DomainOrigin};
use regular::{BracketProposal, Budget, RegularSection, SectionSide};
use symbolica::id::{Pattern, Replacement};
use symgcad::{
    algebra::{Algebra, Poly},
    domain::{Domain, DomainCertificate, certify_nonzero, derive_domain, verify_certificate},
};

#[derive(Debug)]
pub struct WidthNormalForm {
    endpoint: Poly,
    monomial: Poly,
    powers: Vec<u32>,
    residual: Poly,
    certificate: DomainCertificate,
}
impl WidthNormalForm {
    pub fn endpoint(&self) -> &Poly {
        &self.endpoint
    }
    pub fn monomial(&self) -> &Poly {
        &self.monomial
    }
    pub fn powers(&self) -> &[u32] {
        &self.powers
    }
    pub fn residual(&self) -> &Poly {
        &self.residual
    }
    pub fn certificate(&self) -> &DomainCertificate {
        &self.certificate
    }
}
pub struct SecantGeometry {
    source: Arc<CellMap>,
    section: Arc<RegularSection>,
    whole: Arc<RegularSection>,
    lower_cell: bool,
    algebra: Algebra,
    polynomial: Poly,
    root_equation: Poly,
    secant: Poly,
    endpoint_secant: Poly,
    width: WidthNormalForm,
}
impl SecantGeometry {
    pub fn source(&self) -> &Arc<CellMap> {
        &self.source
    }
    pub fn section(&self) -> &Arc<RegularSection> {
        &self.section
    }
    pub fn whole_segment(&self) -> &Arc<RegularSection> {
        &self.whole
    }
    pub fn lower_cell(&self) -> bool {
        self.lower_cell
    }
    pub fn secant(&self) -> &Poly {
        &self.secant
    }
    pub fn root_equation(&self) -> &Poly {
        &self.root_equation
    }
    pub fn polynomial(&self) -> &Poly {
        &self.polynomial
    }
    pub fn width(&self) -> &WidthNormalForm {
        &self.width
    }
    pub fn endpoint_secant(&self) -> &Poly {
        &self.endpoint_secant
    }
    pub(super) fn atom(&self, p: &Poly, root: &Atom) -> Atom {
        let n = self.source.axes().len();
        p.to_expression()
            .replace_multiple(self.algebra.names.iter().enumerate().map(|(i, name)| {
                Replacement::new(
                    Pattern::Literal(Atom::var(Symbol::parse(name, "symgcad").unwrap())),
                    Pattern::Literal(if i == n {
                        root.clone()
                    } else {
                        Atom::var(self.source.unit_coordinates()[i])
                    }),
                )
            }))
    }
    pub(super) fn pulled_secant(&self, root: &Atom, image: &Atom) -> Atom {
        self.atom(&self.secant, root)
            .replace(Atom::var(*self.source.unit_coordinates().last().unwrap()))
            .with(image.clone())
    }
}
pub(super) fn constant_bound(
    map: &CellMap,
    axis: usize,
    side: SectionSide,
) -> Result<Option<Rational>> {
    let raw = map.decomposition().native_result();
    let algebra = Algebra::new(&raw.order).map_err(native)?;
    let bound = match side {
        SectionSide::Lower => &map.native_axes()[axis].lower,
        SectionSide::Upper => &map.native_axes()[axis].upper,
    }
    .as_ref()
    .ok_or_else(|| unsupported("unbounded cell"))?;
    let p = algebra
        .parse(&raw.polynomials[bound.polynomial])
        .map_err(native)?;
    if p.degree(axis) != 1 || (0..raw.order.len()).any(|i| i != axis && p.degree(i) > 0) {
        return Ok(None);
    }
    let c = Rational::try_from(p.replace(axis, &Rational::zero()).to_expression().as_view())
        .map_err(native)?;
    let d = Rational::try_from(p.derivative(axis).to_expression().as_view()).map_err(native)?;
    Ok(Some(-c / d))
}
pub(super) fn positive(
    algebra: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    domain: &Domain,
    limits: &Limits,
    b: &Budget,
) -> Result<DomainCertificate> {
    positive_proposal(algebra, p, constraints, domain, limits, b)
        .map_err(|error| error.into_error(unsupported))
}
pub(super) fn positive_proposal(
    algebra: &Algebra,
    p: &Poly,
    constraints: &[Poly],
    domain: &Domain,
    limits: &Limits,
    b: &Budget,
) -> ProposalResult<DomainCertificate> {
    b.check()?;
    let c = certify_nonzero(algebra, p, domain, &limits.geometry.certificates)
        .map_err(native)?
        .ok_or(regular::ProposalError::<Error>::Miss(
            "native positive closed-unit margin unavailable",
        ))?;
    verify_certificate(algebra, p, constraints, &[], &c).map_err(native)?;
    if c.sign != 1 {
        return Err(regular::ProposalError::Miss(
            "native unit margin has wrong sign",
        ));
    }
    b.check()?;
    Ok(c)
}
fn width(
    algebra: &Algebra,
    p: Poly,
    axis: usize,
    constraints: &[Poly],
    domain: &Domain,
    limits: &Limits,
    b: &Budget,
) -> ProposalResult<WidthNormalForm> {
    if p.is_zero() {
        return Err(unsupported("identically collapsed cell width").into());
    }
    if p.degree(axis) > 0 {
        return Err(invalid("endpoint width retains eliminated coordinate").into());
    }
    let powers = (0..axis).map(|i| p.degree_bounds(i).0).collect::<Vec<_>>();
    let mut monomial = p.constant(Rational::one());
    for (i, power) in powers.iter().enumerate() {
        monomial = &monomial
            * &algebra
                .parse(&algebra.names[i])
                .map_err(native)?
                .pow(*power as usize);
    }
    let (residual, remainder) = p.quot_rem(&monomial, false);
    if !remainder.is_zero() || &monomial * &residual != p {
        return Err(invalid("native width monomial recombination").into());
    }
    let margin = &residual - &residual.constant(limits.unit_margin.clone());
    let certificate = positive_proposal(algebra, &margin, constraints, domain, limits, b)?;
    Ok(WidthNormalForm {
        endpoint: p,
        monomial,
        powers,
        residual,
        certificate,
    })
}
pub(super) struct CellInterval {
    map: Arc<CellMap>,
    lower_cell: bool,
    side: SectionSide,
}
pub(super) fn topology(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
) -> Result<Vec<CellInterval>> {
    let request = owner.request();
    let n = request.domain().coordinates().len();
    if n == 0
        || !matches!(request.domain().origin(), DomainOrigin::NativeUnitCube)
        || !request.kinematics().runtime_parameters.is_empty()
    {
        return Err(unsupported(
            "first issuer requires a fixed native cube, with no free runtime parameters",
        ));
    }
    if owner.cells().len() != 2 {
        return Err(unsupported(
            "first issuer requires complete two-cell inventory; no selected-cell truncation",
        ));
    }
    let mut outputs = Vec::new();
    for cell in owner.cells() {
        let map =
            CellMap::new(owner.clone(), cell.index(), coordinates.to_vec()).map_err(native)?;
        if map
            .axes()
            .iter()
            .any(|a| a.role() != AliasRole::IntegrationCoordinate)
        {
            return Err(unsupported("unexpected nonintegration axis"));
        }
        for axis in 0..n - 1 {
            if constant_bound(&map, axis, SectionSide::Lower)? != Some(Rational::zero())
                || constant_bound(&map, axis, SectionSide::Upper)? != Some(Rational::one())
            {
                return Err(unsupported(
                    "prefix cells require an additional exact coverage partition",
                ));
            }
        }
        let l = constant_bound(&map, n - 1, SectionSide::Lower)?;
        let u = constant_bound(&map, n - 1, SectionSide::Upper)?;
        let (lower_cell, side) = match (l, u) {
            (Some(l), None) if l.is_zero() => (true, SectionSide::Upper),
            (None, Some(u)) if u == Rational::one() => (false, SectionSide::Lower),
            _ => {
                return Err(unsupported(
                    "cell is not an actual endpoint-to-section interval",
                ));
            }
        };
        outputs.push(CellInterval {
            map,
            lower_cell,
            side,
        });
    }
    outputs.sort_by_key(|cell| !cell.lower_cell);
    if !outputs[0].lower_cell
        || outputs[1].lower_cell
        || outputs[0].map.native_axes()[n - 1].upper != outputs[1].map.native_axes()[n - 1].lower
    {
        return Err(unsupported(
            "actual cells do not share one exact section and common prefix",
        ));
    }
    Ok(outputs)
}

pub(super) fn admit(
    owner: &Arc<VerifiedDecomposition>,
    coordinates: &[Symbol],
    limits: &Limits,
    budget: &mut Budget,
) -> ProposalResult<Vec<Arc<SecantGeometry>>> {
    if limits.unit_margin <= Rational::zero() {
        return Err(invalid("positive unit margin required").into());
    }
    budget.check()?;
    let n = owner.request().domain().coordinates().len();
    let mut outputs = Vec::new();
    for CellInterval {
        map,
        lower_cell,
        side,
    } in topology(owner, coordinates)?
    {
        // This is the FULL cell-range cylinder, derived from certified constant
        // neighbors0/1. A narrow root-only bracket is never used for the secant.
        let section = RegularSection::prepare_proposal(
            map.clone(),
            n - 1,
            side,
            BracketProposal {
                lower: Rational::zero(),
                upper: Rational::one(),
                derivative_margin: limits.unit_margin.clone(),
            },
            budget,
        )?;
        let whole = section.clone();
        let raw = owner.native_result();
        let mut names = raw.order.clone();
        let name = "fastsecdec_secant_root".to_string();
        if names.contains(&name) {
            return Err(invalid("fresh secant coordinate collision").into());
        }
        names.push(name);
        let algebra = Algebra::new(&names).map_err(native)?;
        let source = algebra
            .parse(&raw.polynomials[section.bound().polynomial])
            .map_err(native)?;
        let polynomial =
            &source * &source.constant(Rational::from(i64::from(section.orientation())));
        if polynomial
            .nterms()
            .checked_mul(polynomial.degree(n - 1) as usize)
            .is_none_or(|x| x > limits.geometry.max_terms)
        {
            return Err(Error::ResourceIncomplete("secant support bound".into()).into());
        }
        let y = algebra.parse(&names[n - 1]).map_err(native)?;
        let r = algebra.parse(&names[n]).map_err(native)?;
        let root_equation = polynomial.replace_with_poly(n - 1, &r);
        let (secant, remainder) = (&polynomial - &root_equation).quot_rem(&(&y - &r), false);
        if !remainder.is_zero()
            || &(&(&y - &r) * &secant) + &root_equation != polynomial
            || secant.replace_with_poly(n - 1, &r)
                != polynomial.derivative(n - 1).replace_with_poly(n - 1, &r)
        {
            return Err(invalid("native secant/diagonal identity").into());
        }
        let endpoint = if lower_cell {
            Rational::zero()
        } else {
            Rational::one()
        };
        let endpoint_secant = secant.replace(n - 1, &endpoint);
        let value = polynomial.replace(n - 1, &endpoint);
        let signed_value = if lower_cell {
            -value.clone()
        } else {
            value.clone()
        };
        let gap = if lower_cell {
            r.clone()
        } else {
            &r.constant(Rational::one()) - &r
        };
        let identity = if lower_cell {
            &value + &(&gap * &endpoint_secant)
        } else {
            &value - &(&gap * &endpoint_secant)
        };
        if identity != root_equation {
            return Err(invalid("native width identity").into());
        }
        let constraints = raw
            .normalized_constraints
            .iter()
            .map(|p| algebra.parse(p).map_err(native))
            .collect::<Result<Vec<_>>>()?;
        let domain = derive_domain(&algebra, &constraints, &[]).map_err(native)?;
        let width = width(
            &algebra,
            signed_value,
            n - 1,
            &constraints,
            &domain,
            limits,
            budget,
        )?;
        outputs.push(Arc::new(SecantGeometry {
            source: map,
            section,
            whole,
            lower_cell,
            algebra,
            polynomial,
            root_equation,
            secant,
            endpoint_secant,
            width,
        }));
    }
    outputs.sort_by_key(|g| !g.lower_cell);
    if !outputs[0].lower_cell
        || outputs[1].lower_cell
        || outputs[0].section.bound() != outputs[1].section.bound()
        || outputs[0].section.coefficients() != outputs[1].section.coefficients()
    {
        return Err(
            unsupported("actual cells do not share one exact section and common prefix").into(),
        );
    }
    // Constant0/1 outer faces, the SAME selected interior section and the full
    // two-cell inventory prove this cube partition, independently of samples.
    Ok(outputs)
}
