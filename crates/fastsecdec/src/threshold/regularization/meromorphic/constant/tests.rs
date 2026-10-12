use super::*;
use symbolica::symbol;

fn parse(s: &str) -> Atom {
    Atom::parse(s, "constant_prefactor_test", Default::default()).unwrap()
}
#[test]
fn exact_complex_epsilon_polynomial_retains_native_constant_receipts() {
    let eps = symbol!("constant_prefactor_test::eps");
    let a = Atom::num(Complex::new(Rational::one(), Rational::from(2)))
        * (Atom::one() + Atom::var(Symbol::PI).pow(Atom::num((1, 2))));
    let source = &a * Atom::var(eps).pow(2) + a.pow(-1) * Atom::var(eps) + Atom::one();
    let owner = ConstantPolynomialPrefactor::admit(
        source.clone(),
        eps,
        Limits::default(),
        Default::default(),
    )
    .unwrap();
    assert_eq!(owner.original(), &source);
    assert_eq!(owner.degree(), 2);
    assert_eq!(owner.coefficients().len(), 3);
    assert!(!owner.source_constants().is_empty());
    for (_, c) in owner.coefficients() {
        let mut occupied = HashSet::new();
        occupied.insert(eps);
        let high = ball::certify(
            c.original().clone(),
            &occupied,
            Limits::default(),
            ConstantCoefficientLimits {
                precision_bits: 512,
            },
        )
        .unwrap();
        assert!(c.enclosure().contains_ball(high.enclosure()));
    }
}
#[test]
fn coefficient_branch_and_denominator_refusals_preserve_uncertainty() {
    let eps = symbol!("constant_prefactor_test::eps");
    for source in [
        "(pi+eps)^(1/2)",
        "(pi+1)^(1/3)",
        "a+eps",
        "log(pi)+eps",
        "(-pi)^(1/2)+eps",
        "1/(1+eps)",
    ] {
        assert!(
            matches!(
                ConstantPolynomialPrefactor::admit(
                    parse(source),
                    eps,
                    Limits::default(),
                    Default::default()
                ),
                Err(Error::Unsupported(_))
            ),
            "{source}"
        );
    }
    let source = parse("eps/(pi-355/113)");
    assert!(matches!(
        ConstantPolynomialPrefactor::admit(
            source.clone(),
            eps,
            Limits::default(),
            ConstantCoefficientLimits { precision_bits: 8 }
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(
        ConstantPolynomialPrefactor::admit(source, eps, Limits::default(), Default::default())
            .is_ok()
    );
    assert!(matches!(
        ConstantPolynomialPrefactor::admit(
            Atom::num(0.25) * Atom::var(eps),
            eps,
            Limits::default(),
            Default::default()
        ),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn original_constant_branches_are_checked_before_coefficient_cancellation() {
    let eps = symbol!("constant_prefactor_test::eps");
    // Epsilon-polynomial expansion cancels the bad cross coefficient. The
    // original square-root occurrence must still be refused before conversion.
    let r = (-Atom::var(Symbol::PI)).pow(Atom::num((1, 2)));
    let source = (&r + Atom::var(eps)).pow(2) - Atom::num(2) * &r * Atom::var(eps);
    assert!(matches!(
        ConstantPolynomialPrefactor::admit(source, eps, Limits::default(), Default::default()),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn constant_resources_and_regulator_role_are_checked() {
    let eps = symbol!("fastsecdec_prefactor_constant::slot_0");
    let source = Atom::var(Symbol::PI).pow(Atom::num((1, 2))) * Atom::var(eps);
    assert!(
        ConstantPolynomialPrefactor::admit(
            source.clone(),
            eps,
            Limits::default(),
            Default::default()
        )
        .is_ok()
    );
    assert!(matches!(
        ConstantPolynomialPrefactor::admit(
            source.clone(),
            eps,
            Limits {
                source_nodes: 1,
                ..Default::default()
            },
            Default::default()
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        ConstantPolynomialPrefactor::admit(
            source,
            eps,
            Limits::default(),
            ConstantCoefficientLimits { precision_bits: 1 }
        ),
        Err(Error::Invalid(_))
    ));
    assert!(
        ConstantPolynomialPrefactor::admit(
            Atom::zero(),
            eps,
            Limits::default(),
            Default::default()
        )
        .is_ok()
    );
}

#[test]
fn meromorphic_mixed_residuals_preserve_gamma_and_shared_witness() {
    use super::super::{
        MeromorphicPrefactor, MeromorphicWitness, PrefactorResidual, SharedWitness,
    };
    use std::ops::ControlFlow;
    let eps = symbol!("constant_prefactor_test::eps");
    let old = parse("1/eps");
    let old_owner = MeromorphicPrefactor::admit(old.clone(), eps, Limits::default()).unwrap();
    assert!(matches!(
        old_owner.residual(),
        PrefactorResidual::Rational(_)
    ));
    let rational = old_owner.rational().unwrap().clone();
    let existing = SharedWitness::construct(&[rational], None, None, Limits::default(), |_| {
        ControlFlow::Continue(())
    })
    .unwrap();
    let unchanged = MeromorphicWitness::construct(
        vec![old_owner.clone()],
        None,
        None,
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(existing.epsilon(), unchanged.rational_witness().epsilon());
    assert_eq!(
        existing.denominator_values(),
        unchanged.rational_witness().denominator_values()
    );
    assert_eq!(unchanged.rational_term_indices(), &[0]);
    let source = parse("(1+eps*pi^(1/2))*gamma(-1+2*eps)");
    let new = MeromorphicPrefactor::admit(source.clone(), eps, Limits::default()).unwrap();
    assert_eq!(new.original(), &source);
    assert!(new.rational().is_none());
    assert_eq!(new.gamma_factors().len(), 1);
    let mixed = MeromorphicWitness::construct(
        vec![new.clone(), old_owner],
        Some(&Rational::zero()),
        Some(&Rational::one()),
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(mixed.rational_term_indices(), &[1]);
    assert_eq!(mixed.rational_witness().prefactors().len(), 1);
    assert_ne!(mixed.rational_witness().epsilon(), &Rational::from((1, 2)));
    assert_eq!(mixed.gamma_witnesses()[0].term(), 0);
    assert!(mixed.gamma_witnesses()[0].pole_distance().unwrap() > &Rational::zero());
    let all_new = MeromorphicWitness::construct(
        vec![new.clone()],
        Some(&Rational::zero()),
        Some(&Rational::one()),
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(all_new.rational_term_indices().is_empty());
    assert!(all_new.rational_witness().prefactors().is_empty());
    assert_ne!(
        all_new.rational_witness().epsilon(),
        &Rational::from((1, 2))
    );
    let other_eps = symbol!("constant_prefactor_test::eta");
    let foreign =
        MeromorphicPrefactor::admit(Atom::var(Symbol::PI), other_eps, Limits::default()).unwrap();
    assert!(matches!(
        MeromorphicWitness::construct(
            vec![new.clone(), foreign],
            None,
            None,
            Limits::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
    assert_eq!(
        MeromorphicWitness::construct(vec![new], None, None, Limits::default(), |_| {
            ControlFlow::Break(())
        })
        .unwrap_err(),
        Error::Cancelled
    );
}

#[test]
fn meromorphic_fallback_preserves_resource_refusals_and_native_precision() {
    use super::super::MeromorphicPrefactor;
    let eps = symbol!("constant_prefactor_test::eps");
    let source = parse("eps/(pi-355/113)*gamma(1+eps)");
    assert!(matches!(
        MeromorphicPrefactor::admit_with_constants(
            source.clone(),
            eps,
            Limits::default(),
            ConstantCoefficientLimits { precision_bits: 8 }
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(MeromorphicPrefactor::admit(source, eps, Limits::default()).is_ok());
    for source in [parse("(eps+1)^100"), parse("pi*(eps+1)^100")] {
        assert!(matches!(
            MeromorphicPrefactor::admit(
                source,
                eps,
                Limits {
                    terms: 10,
                    ..Limits::default()
                }
            ),
            Err(Error::ResourceIncomplete(_))
        ));
    }
    for source in [
        parse("pi/(1+eps)"),
        parse("gamma(eps^2)*pi"),
        parse("pi^eps"),
        parse("log(pi)*eps"),
    ] {
        assert!(matches!(
            MeromorphicPrefactor::admit(source, eps, Limits::default()),
            Err(Error::Unsupported(_))
        ));
    }
}
