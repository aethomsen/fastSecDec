use super::*;
use crate::threshold::maps::{
    endpoint_transport::clearing::pull,
    normalized::{NormalizedSection, algebra::Ring},
};
use std::collections::BTreeSet;
use symbolica::{
    domains::{
        integer::Z,
        rational::{Q, Rational},
        rational_polynomial::RationalPolynomial,
    },
    poly::PolyVariable,
};
use symgcad::{
    algebra::Poly,
    domain::{Domain, DomainCertificate},
};

/// Retained original certificates establish closed coefficient containment and
/// nonzero selected minor by polynomial continuity and actual prefix induction.
/// No new range proof is issued from the composed expression alone.
pub struct SelectedExtension {
    section: Arc<NormalizedSection>,
    parameter_images: Vec<Atom>,
}
impl SelectedExtension {
    pub fn section(&self) -> &Arc<NormalizedSection> {
        &self.section
    }
    pub fn parameter_images(&self) -> &[Atom] {
        &self.parameter_images
    }
}
/// Original small coordinate ratio, before any source-body substitution.
/// D0's proof is on remaining open units and enlarged root brackets, so every
/// actual closed selected-root value is inside that proof's root domain.
pub struct FiniteArgument {
    source_coordinate: Symbol,
    numerator: Atom,
    denominator: Atom,
    face_denominator: Atom,
    denominator_certificate: Option<DomainCertificate>,
    denominator_orientation: i8,
    constraints: Vec<Poly>,
    domain: Option<Domain>,
}
impl FiniteArgument {
    pub fn source_coordinate(&self) -> Symbol {
        self.source_coordinate
    }
    pub fn numerator(&self) -> &Atom {
        &self.numerator
    }
    pub fn denominator(&self) -> &Atom {
        &self.denominator
    }
    pub fn face_denominator(&self) -> &Atom {
        &self.face_denominator
    }
    pub fn denominator_certificate(&self) -> Option<&DomainCertificate> {
        self.denominator_certificate.as_ref()
    }
    /// The certificate proves orientation * D0 > 0; None means an exact
    /// nonzero rational constant checked without a zero-variable native box.
    pub fn denominator_orientation(&self) -> i8 {
        self.denominator_orientation
    }
    pub fn constraints(&self) -> &[Poly] {
        &self.constraints
    }
    pub fn domain(&self) -> Option<&Domain> {
        self.domain.as_ref()
    }
}
pub(in super::super) struct Admission {
    pub arguments: Vec<FiniteArgument>,
    pub sections: Vec<SelectedExtension>,
    pub images: Vec<Atom>,
    pub scope: Scope,
}
pub(super) fn prepare(
    catalogue: &SourceNumerators,
    chart: &IssuedEndpointChart,
    normal: Symbol,
    settings: OptimizationSettings,
    work: &mut Work<'_>,
) -> Result<Admission> {
    prepare_arguments(
        catalogue,
        chart,
        normal,
        catalogue.preparation().coordinates(),
        chart.chart().source_images(),
        settings,
        work,
    )
}
/// Shared admission of only the demanded finite arguments. The original
/// normal owner passes every source coordinate; mixed faces pass their exact
/// retained formal/image inventory plus explicit inverse divisor arguments.
pub(in super::super) fn prepare_arguments(
    catalogue: &SourceNumerators,
    chart: &IssuedEndpointChart,
    normal: Symbol,
    formals: &[Symbol],
    argument_images: &[Atom],
    settings: OptimizationSettings,
    work: &mut Work<'_>,
) -> Result<Admission> {
    if formals.len() != argument_images.len() {
        return Err(Error::Invalid(
            "finite argument role/image inventory".into(),
        ));
    }
    let roots = chart
        .transport()
        .root_relations()
        .iter()
        .map(|r| r.section().root_symbol())
        .collect::<Vec<_>>();
    let roles = chart
        .target_units()
        .iter()
        .chain(&roots)
        .copied()
        .collect::<Vec<_>>();
    let ring = Ring::new(roles.clone())?;
    let variables = Arc::new(
        roles
            .iter()
            .copied()
            .map(PolyVariable::from)
            .collect::<Vec<_>>(),
    );
    let mut available = chart
        .target_units()
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let mut sections = Vec::new();
    let mut calls = Vec::new();
    let mut scope = Scope::default();
    let prefix = chart.parent_prefix();
    for (i, relation) in chart.transport().root_relations().iter().enumerate() {
        work.check()?;
        let section = relation.section();
        if !Arc::ptr_eq(section.prefix().source(), prefix.source()) {
            return Err(Error::Invalid("foreign selected section source".into()));
        }
        let parameters = chart.section_parameter_images(section)?;
        for p in &parameters {
            if !p
                .get_all_symbols(true)
                .iter()
                .all(|s| available.contains(s))
            {
                return Err(Error::Invalid(
                    "selected coefficient map is not prefix triangular".into(),
                ));
            }
            // Typed issuer already proves closed-cube composition. This native
            // polynomial fence makes the no-new-pole prerequisite explicit.
            ring.polynomial(p, &work.budget)?;
        }
        let program = section.root_program(settings.clone())?;
        let tag = definitions::fresh_root_tag(catalogue, chart, i);
        let call = program.call(tag, section.coefficients())?;
        let call = pull(&call, &section.chart().parameters, &parameters);
        let call = pull(&call, &roots[..i], &calls);
        work.nodes(&call, work.limits.max_coefficient_nodes)?;
        scope.insert(tag, program)?;
        calls.push(call);
        available.insert(section.root_symbol());
        sections.push(SelectedExtension {
            section: section.clone(),
            parameter_images: parameters,
        });
    }
    let face_roles = roles
        .iter()
        .copied()
        .filter(|s| *s != normal)
        .collect::<Vec<_>>();
    let face_ring = if face_roles.is_empty() {
        None
    } else {
        Some(Ring::new(face_roles.clone())?)
    };
    let mut bounds = chart
        .target_units()
        .iter()
        .filter(|s| **s != normal)
        .map(|_| [Rational::zero(), Rational::one()])
        .collect::<Vec<_>>();
    bounds.extend(sections.iter().map(|s| {
        [
            &s.section.bracket().lower / &Rational::from(2),
            &s.section.bracket().upper * &Rational::from(2),
        ]
    }));
    let box_proof = face_ring
        .as_ref()
        .map(|r| r.box_domain(&bounds, &work.budget))
        .transpose()?;
    let mut arguments = Vec::new();
    let mut images = Vec::new();
    for (&source_coordinate, image) in formals.iter().zip(argument_images) {
        work.check()?;
        work.nodes(image, work.limits.max_coefficient_nodes)?;
        if !image
            .get_all_symbols(true)
            .is_subset(&roles.iter().copied().collect())
        {
            return Err(Error::Unsupported("foreign finite-coordinate role".into()));
        }
        use crate::threshold::regularization::meromorphic::{Limits as ML, preflight_expression};
        preflight_expression(
            image,
            ML {
                degree: u16::try_from(work.limits.algebra.max_degree).map_err(native)?,
                terms: work.limits.algebra.max_terms,
                source_nodes: work.limits.max_coefficient_nodes,
                integer_power: work.limits.algebra.max_degree as i64,
                ..Default::default()
            },
        )
        .map_err(super::super::preflight_error)?;
        let ratio: RationalPolynomial<Z, u32> = image
            .try_to_rational_polynomial(&Q, &Z, variables.clone())
            .map_err(native)?;
        let numerator = ratio.numerator.to_expression();
        let denominator = ratio.denominator.to_expression();
        ring.polynomial(&numerator, &work.budget)?;
        ring.polynomial(&denominator, &work.budget)?;
        if !(image * &denominator - &numerator).together().is_zero() {
            return Err(Error::Invalid(
                "native finite-coordinate rational reconstruction".into(),
            ));
        }
        let face_denominator = pull(&denominator, &[normal], &[Atom::zero()]);
        let (denominator_orientation, denominator_certificate, constraints, domain) =
            if let Ok(q) = Rational::try_from(face_denominator.as_view()) {
                work.budget.rational(&q)?;
                if q.is_zero() {
                    return Err(Error::Unsupported(
                        "normal-pole source argument requires separate compact clearing".into(),
                    ));
                }
                (
                    if q > Rational::zero() { 1 } else { -1 },
                    None,
                    Vec::new(),
                    None,
                )
            } else {
                let face_ring = face_ring.as_ref().ok_or_else(|| {
                    Error::Invalid("nonconstant denominator without roles".into())
                })?;
                let (constraints, domain) = box_proof.as_ref().expect("nonempty role box");
                let d0 = face_ring.polynomial(&face_denominator, &work.budget)?;
                if d0.is_zero() {
                    return Err(Error::Unsupported(
                        "normal-pole source argument requires separate compact clearing".into(),
                    ));
                }
                let (orientation, certificate) =
                    match face_ring.positive(&d0, constraints, domain, &work.budget) {
                        Ok(c) => (1, c),
                        Err(Error::Unsupported(_)) => (
                            -1,
                            face_ring.positive(
                                &(&d0 * &d0.constant(Rational::from(-1))),
                                constraints,
                                domain,
                                &work.budget,
                            )?,
                        ),
                        Err(e) => return Err(e),
                    };
                (
                    orientation,
                    Some(certificate),
                    constraints.clone(),
                    Some(domain.clone()),
                )
            };
        // Retain original D, even if cancellation produces a simple center.
        let canonical = &numerator / &denominator;
        let composed = pull(&canonical, &roots, &calls);
        work.nodes(&composed, work.limits.max_coefficient_nodes)?;
        images.push(composed);
        arguments.push(FiniteArgument {
            source_coordinate,
            numerator,
            denominator,
            face_denominator,
            denominator_certificate,
            denominator_orientation,
            constraints,
            domain,
        });
    }
    Ok(Admission {
        arguments,
        sections,
        images,
        scope,
    })
}
