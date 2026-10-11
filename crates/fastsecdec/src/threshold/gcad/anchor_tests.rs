use super::*;
use crate::{
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{maps::PositiveProjectiveCellMap, projective::PositiveProjectivePreparation},
};
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore},
    symbol,
};

fn input(positive_sign: i64, causal_shift: i64) -> Arc<PositiveProjectivePreparation> {
    let (x, y, eps) = symbol!("anchor_test::x", "anchor_test::y", "anchor_test::eps");
    let source = ParametricIntegrand::new(
        vec![x, y],
        eps,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    Atom::num(positive_sign) * (Atom::var(x) + Atom::var(y)),
                    Atom::one(),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(
                    Atom::var(x) - Atom::num(causal_shift) * Atom::var(y),
                    Atom::num(-3),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    Arc::new(PositiveProjectivePreparation::anchor(Arc::new(source), 0).unwrap())
}
fn problem() -> Problem {
    Problem {
        variables: vec!["x0".into(), "x1".into()],
        constraints: vec!["x0>0".into(), "x1>0".into()],
        split: vec!["x0-x1".into()],
        solver: SolverOptions {
            homogeneous_anchor: Some("x0".into()),
            homogeneous_block: vec!["x0".into(), "x1".into()],
            ordering: "given".into(),
            ..Default::default()
        },
        limits: Limits {
            workers: 1,
            wall_time_secs: 10.0,
            memory_mib: 1024,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn positive_anchor_actual_quotient_preserves_roles_and_original_ordinals() {
    let raw = Arc::new(symgcad::solver::solve(&problem()).unwrap());
    let owner = Arc::new(PositiveProjectiveQuotient::verify(input(1, 1), raw.clone()).unwrap());
    assert!(Arc::ptr_eq(owner.original_result(), &raw));
    assert!(std::ptr::eq(
        owner.native_result(),
        raw.homogeneous.as_ref().unwrap().reduced.as_ref()
    ));
    assert_eq!(owner.cell_count(), 2);
    assert_eq!(owner.cell_count(), raw.cells.len());
    assert_eq!(owner.native_result().order, vec!["x1"]);
    assert!(matches!(
        owner.factor_authorities()[0][0],
        ProjectiveFactorAuthority::Positive { .. }
    ));
    assert!(matches!(
        owner.factor_authorities()[0][1],
        ProjectiveFactorAuthority::Split { index: 0, .. }
    ));
    let mut finite = 0;
    let mut unbounded = 0;
    for index in 0..owner.cell_count() {
        let map = PositiveProjectiveCellMap::new(
            owner.clone(),
            index,
            vec![symbol!("anchor_test::unit")],
        )
        .unwrap();
        assert_eq!(map.cell_index(), index);
        assert_eq!(map.axes()[0].symbol(), owner.preparation().coordinates()[0]);
        match map.linear_axes() {
            Ok(axes) => {
                assert_eq!(axes.len(), 1);
                finite += 1;
            }
            Err(GcadError::Unsupported(reason)) => {
                assert!(reason.contains("unbounded"));
                unbounded += 1;
            }
            Err(e) => panic!("{e}"),
        }
    }
    assert_eq!((finite, unbounded), (1, 1));
    assert!(
        PositiveProjectiveCellMap::new(owner.clone(), 2, vec![symbol!("anchor_test::unit")])
            .is_err()
    );
    assert!(
        PositiveProjectiveCellMap::new(
            owner.clone(),
            0,
            vec![owner.preparation().input().regulator()]
        )
        .is_err()
    );
    assert!(PositiveProjectiveCellMap::new(owner, 0, vec![symbol!("anchor_test::x")]).is_err());
}

#[test]
fn positive_anchor_source_domain_sign_and_native_evidence_mutations_refuse() {
    let raw = symgcad::solver::solve(&problem()).unwrap();
    assert!(matches!(
        PositiveProjectiveQuotient::verify(input(-1, 1), Arc::new(raw.clone())),
        Err(GcadError::Unsupported(_))
    ));
    assert!(matches!(
        PositiveProjectiveQuotient::verify(input(1, 2), Arc::new(raw.clone())),
        Err(GcadError::RequestMismatch)
    ));
    let mut missing = raw.clone();
    missing.problem.constraints.pop();
    assert!(matches!(
        PositiveProjectiveQuotient::verify(input(1, 1), Arc::new(missing)),
        Err(GcadError::RequestMismatch)
    ));
    let mut narrowed = raw.clone();
    narrowed.problem.constraints.push("x0>x1".into());
    assert!(matches!(
        PositiveProjectiveQuotient::verify(input(1, 1), Arc::new(narrowed)),
        Err(GcadError::Unsupported(_))
    ));
    let mut changed = raw.clone();
    changed.homogeneous.as_mut().unwrap().reduced.cells.pop();
    assert!(matches!(
        PositiveProjectiveQuotient::verify(input(1, 1), Arc::new(changed)),
        Err(GcadError::Verification(_))
    ));
    let mut changed = raw;
    changed.problem.solver.monotone_negative_target = true;
    assert!(matches!(
        PositiveProjectiveQuotient::verify(input(1, 1), Arc::new(changed)),
        Err(GcadError::Unsupported(_))
    ));
}

#[test]
fn positive_anchor_full_density_and_normalized_map_jacobian() {
    let p = input(1, 1);
    let original = p.input();
    let xs = original.parameters();
    let normalized = p.normalized_images();
    assert!(
        (normalized.iter().sum::<Atom>() - Atom::one())
            .together()
            .is_zero()
    );
    let derivative = normalized[1].derivative(xs[1]);
    assert!((derivative - p.simplex_jacobian()).together().is_zero());
    let source = original
        .density()
        .replace_multiple(xs.iter().zip(normalized).map(|(s, a)| {
            symbolica::id::Replacement::new(
                symbolica::id::Pattern::Literal(Atom::var(*s)),
                symbolica::id::Pattern::Literal(a.clone()),
            )
        }));
    let anchored = p.terms()[0]
        .factors()
        .iter()
        .map(|f| f.polynomial().pow(f.exponent()))
        .product::<Atom>();
    assert!(
        (source * p.simplex_jacobian() - anchored)
            .together()
            .is_zero()
    );
    assert_eq!(p.factor_degrees(), &[vec![Some(1), Some(1)]]);
}

#[test]
fn positive_anchor_foreign_native_alias_coefficient_cannot_be_captured() {
    let (x, y, eps) = symbol!("anchor_test::x", "anchor_test::y", "anchor_test::eps");
    let coefficient = Atom::var(symbol!("symgcad::x0"));
    let source = ParametricIntegrand::new(
        vec![x, y],
        eps,
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    Atom::var(x) + Atom::var(y),
                    Atom::one(),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Positive),
                PolynomialFactor::new(
                    coefficient * (Atom::var(x) - Atom::var(y)),
                    Atom::num(-3),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let prep = Arc::new(PositiveProjectivePreparation::anchor(Arc::new(source), 0).unwrap());
    let mut native = problem();
    native.split = vec!["x0*(x0-x1)".into()];
    let raw = Arc::new(symgcad::solver::solve(&native).unwrap());
    let error = PositiveProjectiveQuotient::verify(prep, raw).unwrap_err();
    assert!(
        matches!(error,GcadError::Unsupported(ref reason) if reason.contains("unbound coefficient"))
    );
}
