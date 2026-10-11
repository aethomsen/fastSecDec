use super::*;

#[test]
fn paired_local_coefficient_zero_has_gap_and_inverse_identity() {
    use crate::threshold::regularization::secant::zeros;
    let f = family(2, true);
    let c = f.continue_symbolically(&options(2)).unwrap();
    let vector = crate::generation::threshold_expand_vector(
        &c.charts()[0],
        f.coordinates(),
        f.regulators()[0],
        2,
    )
    .unwrap();
    let mut receipt = zeros::prove(&c, 0, 0, &vector[&0])
        .unwrap()
        .expect("issued localized exact zero");
    assert!(receipt.gap_power > 0);
    assert_eq!(
        zeros::validate(&receipt).unwrap().as_ref(),
        Some(&receipt.digest)
    );
    // Changing native symbol registration/names cannot change role-index math.
    let new = receipt
        .variables
        .iter()
        .enumerate()
        .map(|(i, _)| Symbol::parse(format!("paired_zero_remap::v{i}"), "").unwrap())
        .collect::<Vec<_>>();
    for atom in &mut receipt.atoms {
        *atom = atom.replace_multiple(receipt.variables.iter().zip(&new).map(|(a, b)| {
            symbolica::id::Replacement::new(
                symbolica::id::Pattern::Literal(Atom::var(*a)),
                symbolica::id::Pattern::Literal(Atom::var(*b)),
            )
        }));
    }
    receipt.variables = new;
    assert_eq!(
        zeros::validate(&receipt).unwrap().as_ref(),
        Some(&receipt.digest)
    );
    let inverse = receipt.inverse_units[0];
    receipt.inverse_units[0].1 = u32::MAX;
    assert!(zeros::validate(&receipt).unwrap().is_none());
    receipt.inverse_units[0] = inverse;
    receipt.atoms[7] += Atom::one();
    assert!(zeros::validate(&receipt).unwrap().is_none());
    let shifted = symbolica::atom::AliasedAtom::from(vector[&0].clone().into_inner() + Atom::one());
    assert!(zeros::prove(&c, 0, 0, &shifted).unwrap().is_none());
    let inverse = symbolica::atom::AliasedAtom::from(
        vector[&0].clone().into_inner() / (Atom::var(f.coordinates()[0]) - Atom::num((1, 2))),
    );
    assert!(zeros::prove(&c, 0, 0, &inverse).unwrap().is_none());
}

#[test]
fn paired_optional_zero_discovery_inventory() {
    use crate::threshold::regularization::secant::zeros;
    let f = family(2, true);
    let c = f.continue_symbolically(&options(2)).unwrap();
    for (chart, expression) in c.charts().iter().enumerate() {
        let vector = crate::generation::threshold_expand_vector(
            expression,
            f.coordinates(),
            f.regulators()[0],
            2,
        )
        .unwrap();
        for (order, value) in vector {
            let result = zeros::discover(&c, chart, order, &value);
            println!(
                "DISCOVERY chart={chart} order={order} status={}",
                match &result {
                    Ok(Some(_)) => "Proved".to_string(),
                    Ok(None) => "Unproved".to_string(),
                    Err(error) => format!("{error:?}"),
                }
            );
            if chart == 0 && order == 0 {
                assert!(result.unwrap().is_some());
            } else {
                assert!(zeros::prove(&c, chart, order, &value).unwrap().is_none());
            }
        }
    }
}
