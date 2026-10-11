use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{Limits, PositiveProjectiveQuotient, Problem, SolverOptions},
        projective::PositiveProjectivePreparation,
    },
};
use symbolica::{
    atom::{Atom, AtomCore},
    domains::atom::AtomField,
    id::{Pattern, Replacement},
    symbol,
    tensors::matrix::Matrix,
};

fn owner(kind: u8) -> Arc<PositiveProjectiveQuotient> {
    let (a, x, y, eps) = symbol!("open_map::a", "open_map::x", "open_map::y", "open_map::eps");
    let f = if kind == 1 {
        Atom::var(y).pow(2) - Atom::var(x) * Atom::var(a)
    } else if kind == 2 {
        Atom::var(x) * Atom::var(y) - Atom::var(a).pow(2)
    } else {
        Atom::var(y) - Atom::var(x) - Atom::var(a)
    };
    let input = Arc::new(
        ParametricIntegrand::new(
            vec![a, x, y],
            eps,
            ParametricDomain::ProjectiveSimplex,
            vec![ParametricTerm::new(
                Atom::one(),
                vec![Atom::zero(); 3],
                vec![
                    PolynomialFactor::new(
                        f,
                        if kind != 0 {
                            Atom::num((-3, 2))
                        } else {
                            Atom::num(-3)
                        },
                        FactorRole::Singularity,
                    )
                    .with_semantics(FactorSemantics::Causal),
                ],
            )],
        )
        .unwrap(),
    );
    let prep = Arc::new(PositiveProjectivePreparation::anchor(input, 0).unwrap());
    let p = Problem {
        variables: vec!["x0".into(), "x1".into(), "x2".into()],
        constraints: vec!["x0>0".into(), "x1>0".into(), "x2>0".into()],
        split: vec![if kind == 1 {
            "x2^2-x0*x1".into()
        } else if kind == 2 {
            "x1*x2-x0^2".into()
        } else {
            "x2-x1-x0".into()
        }],
        solver: SolverOptions {
            homogeneous_anchor: Some("x0".into()),
            homogeneous_block: vec!["x0".into(), "x1".into(), "x2".into()],
            ordering: "given".into(),
            ..Default::default()
        },
        limits: Limits {
            workers: 1,
            wall_time_secs: 10.,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    };
    Arc::new(
        PositiveProjectiveQuotient::verify(prep, Arc::new(symgcad::solver::solve(&p).unwrap()))
            .unwrap(),
    )
}
fn same(a: &Atom, b: &Atom) {
    assert!((a - b).together().is_zero());
}
fn pull(a: &Atom, s: Symbol, b: &Atom) -> Atom {
    a.replace_multiple([Replacement::new(
        Pattern::Literal(Atom::var(s)),
        Pattern::Literal(b.clone()),
    )])
}

#[test]
fn open_projective_axes_retain_actual_bounds_faces_and_exact_triangular_measure() {
    let owner = owner(0);
    assert_eq!(owner.cell_count(), 2);
    let (u, v) = symbol!("open_map::u", "open_map::v");
    let mut finite = 0;
    let mut ray = 0;
    for cell in 0..owner.cell_count() {
        let map = PositiveProjectiveCellMap::new(owner.clone(), cell, vec![u, v]).unwrap();
        assert!(matches!(map.linear_axes(), Err(GcadError::Unsupported(_))));
        assert!(map.open_linear_axis(2).is_err());
        let first = map.open_linear_axis(0).unwrap();
        let second = map.open_linear_axis(1).unwrap();
        assert!(Arc::ptr_eq(first.source(), &map));
        assert!(Arc::ptr_eq(second.source(), &map));
        assert_eq!(first.source().cell_index(), cell);
        assert_eq!(second.axis_index(), 1);
        assert_eq!(first.faces()[0].origin(), OpenFaceOrigin::FiniteLower);
        assert_eq!(first.faces()[1].origin(), OpenFaceOrigin::PositiveInfinity);
        assert_eq!(first.faces()[1].interval_jacobian_power(), -2);
        same(first.faces()[1].divisor(), &(Atom::one() - Atom::var(u)));
        for (axis, receipt) in [(0, &first), (1, &second)] {
            same(
                &receipt.image().derivative(receipt.unit_coordinate()),
                receipt.diagonal_jacobian(),
            );
            for side in [IntervalSide::Lower, IntervalSide::Upper] {
                if let Some(bound) = receipt.finite_bound(side) {
                    let original = match side {
                        IntervalSide::Lower => map.native_axes()[axis].lower.as_ref(),
                        IntervalSide::Upper => map.native_axes()[axis].upper.as_ref(),
                    }
                    .unwrap();
                    assert!(std::ptr::eq(bound.native_bound(), original));
                    assert!(Arc::ptr_eq(bound.source(), &map));
                    assert_eq!(bound.axis_index(), axis);
                    assert!(!bound.leading_coefficient().is_zero());
                }
            }
        }
        let original_x = map.axes()[0].symbol();
        let images = [
            first.image().clone(),
            pull(second.image(), original_x, first.image()),
        ];
        let matrix = Matrix::from_linear(
            images
                .iter()
                .flat_map(|a| [a.derivative(u), a.derivative(v)])
                .collect(),
            2,
            2,
            AtomField {
                statistical_zero_test: false,
                ..Default::default()
            },
        )
        .unwrap();
        same(
            &matrix.det().unwrap(),
            &(first.diagonal_jacobian()
                * pull(second.diagonal_jacobian(), original_x, first.image())),
        );
        if second.finite_bound(IntervalSide::Upper).is_some() {
            finite += 1;
            assert_eq!(second.faces()[1].origin(), OpenFaceOrigin::FiniteUpper);
        } else {
            ray += 1;
            assert_eq!(second.faces()[1].origin(), OpenFaceOrigin::PositiveInfinity);
            same(
                second
                    .finite_bound(IntervalSide::Lower)
                    .unwrap()
                    .expression(),
                &(Atom::one() + Atom::var(original_x)),
            );
        }
    }
    assert_eq!((finite, ray), (1, 1));
}

#[test]
fn open_projective_partial_axis_does_not_issue_nonlinear_prefix_or_closed_map() {
    let owner = owner(1);
    let (u, v) = symbol!("open_map::nonlinear_u", "open_map::nonlinear_v");
    for cell in 0..owner.cell_count() {
        let map = PositiveProjectiveCellMap::new(owner.clone(), cell, vec![u, v]).unwrap();
        assert!(map.open_linear_axis(0).is_ok());
        assert!(matches!(
            map.open_linear_axis(1),
            Err(GcadError::Unsupported(_))
        ));
    }
}

#[test]
fn open_projective_retains_leading_coefficient_at_excluded_prefix_face() {
    let owner = owner(2);
    let (u, v) = symbol!("open_map::denominator_u", "open_map::denominator_v");
    let mut checked = false;
    for cell in 0..owner.cell_count() {
        let map = PositiveProjectiveCellMap::new(owner.clone(), cell, vec![u, v]).unwrap();
        let axis = map.open_linear_axis(1).unwrap();
        if axis.finite_bound(IntervalSide::Upper).is_none() {
            let bound = axis.finite_bound(IntervalSide::Lower).unwrap();
            assert!(bound.leading_coefficient().degree(0) > 0);
            let zeros = vec![symbolica::domains::rational::Rational::zero(); 2];
            assert!(bound.leading_coefficient().replace_all(&zeros).is_zero());
            same(
                bound.expression(),
                &Atom::var(map.axes()[0].symbol()).pow(-1),
            );
            assert_eq!(axis.faces()[1].origin(), OpenFaceOrigin::PositiveInfinity);
            checked = true;
        }
    }
    assert!(checked);
}

#[test]
fn private_open_interval_arithmetic_preserves_orientation_and_refuses_whole_line() {
    let (a, u, x) = symbol!(
        "open_arithmetic::a",
        "open_arithmetic::u",
        "open_arithmetic::x"
    );
    let bound = Atom::var(a);
    assert!(open_interval::pullback(None, None, u).is_err());
    let lower = open_interval::pullback(Some(&bound), None, u).unwrap();
    let upper = open_interval::pullback(None, Some(&bound), u).unwrap();
    same(&lower.image.derivative(u), &lower.diagonal);
    same(&upper.image.derivative(u), &upper.diagonal);
    let lower_inverse = (Atom::var(x) - &bound) / (Atom::one() + Atom::var(x) - &bound);
    let upper_inverse = (Atom::one() + &bound - Atom::var(x)).pow(-1);
    same(&pull(&lower_inverse, x, &lower.image), &Atom::var(u));
    same(&pull(&upper_inverse, x, &upper.image), &Atom::var(u));
}
