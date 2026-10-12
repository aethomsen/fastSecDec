use super::super::{BracketProposal, CoefficientChart, Limits};
use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions, VerifiedDecomposition},
        projective::AffineProjectivePreparation,
    },
};
use std::sync::{OnceLock, atomic::AtomicBool};
use symbolica::symbol;

fn source() -> Arc<VerifiedDecomposition> {
    static OWNER: OnceLock<Arc<VerifiedDecomposition>> = OnceLock::new();
    OWNER
        .get_or_init(|| {
            let (x, y, a, eps) = symbol!(
                "affine_normalized_test::x",
                "affine_normalized_test::y",
                "affine_normalized_test::a",
                "affine_normalized_test::eps"
            );
            let x = Atom::var(x);
            let y = Atom::var(y);
            let sum = &x + &y + Atom::var(a);
            let shifted = Atom::num(8) * &y - &sum;
            // Homogeneous source, then the native delta gauge sets sum=1.
            // The selected finite chain is 0<x<1/sqrt(128),
            // 1/8<y<1/8+x*Z(x), with Z^3+Z-1-x=0.
            let factors = [
                Atom::num(128) * x.pow(2) - sum.pow(2),
                shifted.clone(),
                &sum * shifted.pow(3) + Atom::num(64) * &sum * x.pow(2) * &shifted
                    - Atom::num(512) * x.pow(3) * (&sum + &x),
            ];
            let input = ParametricIntegrand::new(
                vec![
                    x.as_view().as_var_view().unwrap().get_symbol(),
                    y.as_view().as_var_view().unwrap().get_symbol(),
                    a,
                ],
                eps,
                ParametricDomain::ProjectiveSimplex,
                vec![ParametricTerm::new(
                    Atom::one(),
                    vec![Atom::zero(); 3],
                    factors
                        .into_iter()
                        .map(|f| {
                            PolynomialFactor::new(f, Atom::num((-3, 7)), FactorRole::Singularity)
                                .with_semantics(FactorSemantics::Causal)
                        })
                        .collect(),
                )],
            )
            .unwrap();
            let mut limits = GcadRequest::default_limits();
            limits.wall_time_secs = 30.;
            limits.memory_mib = 1024;
            limits.workers = 1;
            let request = GcadRequest::projective(
                AffineProjectivePreparation::last_coordinate(&input).unwrap(),
                GcadKinematics::default(),
                SolverOptions {
                    order: vec!["v0".into(), "v1".into()],
                    ..Default::default()
                },
                limits,
            )
            .unwrap();
            Arc::new(request.solve_verified().unwrap())
        })
        .clone()
}

/// A genuine original affine cell; never a synthetic raw decomposition.
pub(super) fn fixture() -> (Arc<AffineNormalizedPrefix>, SectionProposal) {
    let owner = source();
    let (u, v, p, z) = symbol!(
        "affine_normalized_test::u",
        "affine_normalized_test::v",
        "affine_normalized_test::p",
        "affine_normalized_test::Z"
    );
    for i in 0..owner.cells().len() {
        let mut budget = Budget::new(Limits::default());
        let prefix = AffineNormalizedPrefix::new(
            CellMap::new(owner.clone(), i, vec![u, v]).unwrap(),
            &budget,
        )
        .unwrap();
        let Ok(first) = prefix.constant_interval(2, &mut budget) else {
            continue;
        };
        let Some(AffinePrefixStep::ConstantInterval { lower, upper, .. }) = first.step() else {
            unreachable!()
        };
        if lower.rational() != Some(&Rational::zero()) || upper.section().is_none() {
            continue;
        }
        let proposal = SectionProposal {
            chart: CoefficientChart {
                parameters: vec![p],
                bounds: vec![[Rational::zero(), Rational::from((1, 8))]],
                source_prefix: vec![Atom::var(p)],
                parameter_images: vec![first.images()[0].clone()],
                witness: vec![Rational::from((1, 32))],
            },
            root_symbol: z,
            scale: Atom::var(p),
            removed_coefficient: Rational::one(),
            bracket: BracketProposal {
                lower: Rational::from((1, 2)),
                upper: Rational::one(),
                derivative_margin: Rational::one(),
            },
        };
        if first
            .normalized_upper(proposal.clone(), &mut budget)
            .is_ok()
        {
            return (first, proposal);
        }
    }
    panic!("native affine inventory has no expected shifted cubic cell");
}

#[test]
fn affine_normalized_actual_constant_shifted_cubic_and_prefix_lineage() {
    let (first, proposal) = fixture();
    let mut budget = Budget::new(Limits::default());
    let second = first.normalized_upper(proposal, &mut budget).unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 2);
    assert!(Arc::ptr_eq(second.previous().unwrap(), &first));
    assert!(Arc::ptr_eq(second.source(), first.source()));
    let Some(AffinePrefixStep::NormalizedUpper { section, .. }) = second.step() else {
        unreachable!()
    };
    assert_eq!(section.lower(), &Rational::from((1, 8)));
    assert_eq!(section.coefficients().len(), 4);
    assert!(
        (section.total_pullback() - section.removed_presentation_monomial() * section.equation())
            .expand()
            .is_zero()
    );
    let p = Atom::var(section.chart().parameters[0]);
    let z = Atom::var(section.root_symbol());
    let ring =
        algebra::Ring::new(vec![section.chart().parameters[0], section.root_symbol()]).unwrap();
    let q = ring.polynomial(section.equation(), &budget).unwrap();
    let expected = ring
        .polynomial(&(z.pow(3) + &z - Atom::one() - &p), &budget)
        .unwrap();
    // Native projection may normalize a rational content or orientation.
    assert!(q.quot_rem(&expected, false).1.is_zero());
    assert_eq!(section.derivative_certificate().sign, 1);
    assert!(section.endpoint_certificates().iter().all(Option::is_some));
    assert_eq!(section.exponent_determinant(), &Rational::one());
    assert_eq!(
        section.original_bound(),
        second.source().native_axes()[1].upper.as_ref().unwrap()
    );
    assert_eq!(section.original_polynomial().degree(1), 3);
    let Some(AffinePrefixStep::ConstantInterval {
        lower,
        upper,
        power,
        ..
    }) = first.step()
    else {
        unreachable!()
    };
    assert_eq!(*power, 2);
    assert_eq!(lower.rational(), Some(&Rational::zero()));
    let constant = upper.section().unwrap();
    assert_eq!(constant.coefficients().len(), 3);
    assert!(Arc::ptr_eq(constant.source(), first.source()));
    assert!(upper.enclosure()[0] > Rational::zero());
    assert!(upper.enclosure()[1] <= (1, 8));
}

#[test]
fn affine_normalized_refuses_incomplete_roles_wrong_branch_and_resource_cancel() {
    let (first, proposal) = fixture();
    let mut b = Budget::new(Limits::default());
    assert!(matches!(
        first.previous().unwrap().constant_interval(0, &mut b),
        Err(Error::ResourceIncomplete(_))
    ));
    let mut bad = proposal.clone();
    bad.chart.parameter_images[0] = Atom::one();
    assert!(first.normalized_upper(bad, &mut b).is_err());
    let mut bad = proposal.clone();
    bad.root_symbol = first.source().unit_coordinates()[0];
    assert!(matches!(
        first.normalized_upper(bad, &mut b),
        Err(Error::Invalid(_))
    ));
    let mut bad = proposal.clone();
    bad.chart.witness[0] = Rational::from((1, 9));
    // Inside the proposed box, outside the actual original first interval.
    assert!(matches!(
        first.normalized_upper(bad, &mut b),
        Err(Error::Invalid(_))
    ));
    let mut bad = proposal.clone();
    bad.bracket.lower = Rational::from(2);
    bad.bracket.upper = Rational::from(3);
    assert!(first.normalized_upper(bad, &mut b).is_err());
    let mut cancelled =
        Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)));
    assert!(matches!(
        first.normalized_upper(proposal.clone(), &mut cancelled),
        Err(Error::Cancelled)
    ));
    let limited = Limits {
        max_degree: 1,
        ..Limits::default()
    };
    assert!(matches!(
        first.normalized_upper(proposal, &mut Budget::new(limited)),
        Err(Error::ResourceIncomplete(_))
    ));
    assert_eq!(first.len(), 1);
}

#[test]
fn affine_monomial_class_explicitly_refuses_a_shifted_prior_interval() {
    let (_, mut proposal) = fixture();
    let owner = source();
    let (u, v) = symbol!("affine_normalized_test::u", "affine_normalized_test::v");
    for i in 0..owner.cells().len() {
        let mut budget = Budget::new(Limits::default());
        let empty = AffineNormalizedPrefix::new(
            CellMap::new(owner.clone(), i, vec![u, v]).unwrap(),
            &budget,
        )
        .unwrap();
        let Ok(prefix) = empty.constant_interval(2, &mut budget) else {
            continue;
        };
        let AffinePrefixStep::ConstantInterval { lower, .. } = prefix.step().unwrap() else {
            unreachable!()
        };
        if lower.rational() == Some(&Rational::zero()) {
            continue;
        }
        proposal.chart.bounds[0] = [Rational::zero(), Rational::from(2)];
        proposal.chart.parameter_images[0] = prefix.images()[0].clone();
        proposal.chart.witness[0] = Rational::from((1, 2));
        assert!(
            matches!(prefix.normalized_upper(proposal,&mut budget),Err(Error::Unsupported(message)) if message=="positive monomial chart required")
        );
        assert_eq!(prefix.len(), 1);
        return;
    }
    panic!(
        "genuine source must retain a nonzero-lower first interval for the sufficient-class refusal control"
    );
}
