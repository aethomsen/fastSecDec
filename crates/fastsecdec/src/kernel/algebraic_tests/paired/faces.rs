use super::*;

#[test]
fn native_face_polynomial_remainder_probe() {
    use symbolica::domains::atom::AtomField;
    let f = family(2, true);
    let input = f.owner().request().input();
    let y = Atom::var(input.parameters()[1]);
    let e = Atom::var(input.regulator());
    let p = input.terms()[0].factors()[0].polynomial();
    let numerator = input.terms()[0].factors()[1].polynomial();
    let field = AtomField {
        statistical_zero_test: false,
        ..Default::default()
    };
    let divisor = p.to_polynomial_in_vars_with_field::<u32>(std::slice::from_ref(&y), &field);
    for order in [0, 1] {
        let body = if order == 0 {
            numerator.replace(e.clone()).with(Atom::zero())
        } else {
            numerator.derivative(input.regulator())
        };
        let polynomial =
            body.to_polynomial_in_vars_with_field::<u32>(std::slice::from_ref(&y), &field);
        let (quotient, remainder) = polynomial.quot_rem(&divisor, false);
        assert!(
            (&body
                - quotient.to_expression_with_coeff_map(|_, a, out| *out = a.clone()) * p
                - remainder.to_expression_with_coeff_map(|_, a, out| *out = a.clone()))
            .expand()
            .is_zero()
        );
        if order == 0 {
            assert!(remainder.is_zero());
        } else {
            let expected = Atom::num(-5) * &y
                + Atom::num((3, 8)) * (Atom::one() + Atom::var(input.parameters()[0]));
            assert!(
                (remainder.to_expression_with_coeff_map(|_, a, out| *out = a.clone()) - expected)
                    .expand()
                    .is_zero()
            );
        }
        println!(
            "face-order={order} quotient={} remainder={}",
            quotient.to_expression_with_coeff_map(|_, a, out| *out = a.clone()),
            remainder.to_expression_with_coeff_map(|_, a, out| *out = a.clone())
        );
    }
    let continued = f.continue_symbolically(&options(2)).unwrap();
    let vector = continued.expand_vector(2).unwrap();
    let pole = vector
        .get(&-1)
        .map(|a| a.clone().into_inner())
        .unwrap_or_else(Atom::zero);
    assert!(pole.is_zero());
    println!("unreduced symbolic pole={}", pole.to_canonical_string());
}

#[test]
fn paired_face_reduction_uses_ambient_derivatives() {
    let f = family(3, true);
    let p = f.owner().request().input().terms()[0].factors()[0].polynomial();
    let derivative = p.derivative(f.owner().request().input().parameters()[1]);
    let c = f.continue_symbolically(&options(1)).unwrap();
    assert!(
        c.face_reductions()
            .iter()
            .any(|r| (&r.body - &derivative).expand().is_zero() && !r.remainder.is_zero())
    );
    for receipt in c.face_reductions() {
        assert!(
            (&receipt.body - &receipt.quotient * &receipt.equation - &receipt.remainder)
                .expand()
                .is_zero()
        );
        assert!(receipt.role < 2 && receipt.chart < 4);
    }
    let f = family(2, true);
    let c = f.continue_symbolically(&options(2)).unwrap();
    assert!(c.face_reductions().iter().any(|r| r.remainder.is_zero()));
    for chart in c.charts() {
        let vector = crate::generation::threshold_expand_vector(
            chart,
            f.coordinates(),
            f.regulators()[0],
            2,
        )
        .unwrap();
        assert!(
            vector
                .get(&-1)
                .is_none_or(|a| a.clone().into_inner().is_zero())
        );
    }
}
