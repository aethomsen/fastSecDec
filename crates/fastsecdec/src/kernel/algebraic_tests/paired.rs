use crate as fastsecdec;
use fastsecdec::{
    generation::{GenerationMode, GenerationOptions},
    kernel::{
        CompilationSettings, EvaluatorBackend, KernelSet, ProgramRecipe, ThresholdCompilationPlan,
    },
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions, VerifiedDecomposition},
        maps::regular::{self, BracketProposal},
        regularization::secant::{self, NormalizedFamily},
    },
};
use std::{
    io::Cursor,
    sync::{Arc, atomic::AtomicBool},
};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::OptimizationSettings,
    symbol,
};
fn proposals() -> [BracketProposal; 2] {
    [
        BracketProposal {
            lower: Rational::from((1, 32)),
            upper: Rational::from((1, 4)),
            derivative_margin: Rational::from((1, 4)),
        },
        BracketProposal {
            lower: Rational::from((3, 4)),
            upper: Rational::from((127, 128)),
            derivative_margin: Rational::from((1, 4)),
        },
    ]
}
fn limits() -> secant::Limits {
    secant::Limits {
        unit_margin: Rational::from((1, 4)),
        ..Default::default()
    }
}
fn source(kind: u8) -> (Arc<VerifiedDecomposition>, Vec<Symbol>) {
    source_config(kind, false, None)
}
fn source_config(
    kind: u8,
    reverse: bool,
    collision: Option<(Symbol, bool)>,
) -> (Arc<VerifiedDecomposition>, Vec<Symbol>) {
    let (x, y, e, s, t) = symbol!(
        "three_cell::x",
        "three_cell::y",
        "three_cell::e",
        "three_cell::s",
        "three_cell::t"
    );
    let p = Atom::var(y).pow(6) - Atom::var(y) + (Atom::one() + Atom::var(x)) / Atom::num(16);
    let numerator = match kind {
        3 => p.clone(),
        1 => p.derivative(y),
        2 => &p - Atom::var(e) * Atom::var(y) * p.derivative(y),
        _ => Atom::one(),
    };
    let mut kinematics = GcadKinematics::default();
    if let Some((symbol, _)) = collision {
        kinematics.exact_values.insert(symbol, Rational::one());
    }
    let input = ParametricIntegrand::new(
        if reverse { vec![y, x] } else { vec![x, y] },
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            if let Some((symbol, true)) = collision {
                Atom::var(symbol)
            } else {
                Atom::one()
            },
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    p,
                    -Atom::num(if kind == 3 { 2 } else { 1 }) - Atom::var(e),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
                PolynomialFactor::new(numerator, Atom::one(), FactorRole::Polynomial),
            ],
        )],
    )
    .unwrap();
    let mut l = GcadRequest::default_limits();
    l.wall_time_secs = 30.;
    l.memory_mib = 1024;
    let owner = Arc::new(
        GcadRequest::unit_cube(
            &input,
            kinematics,
            SolverOptions {
                order: if reverse {
                    vec!["v1".into(), "v0".into()]
                } else {
                    vec!["v0".into(), "v1".into()]
                },
                ..Default::default()
            },
            l,
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    (owner, vec![s, t])
}
fn family(kind: u8, affine: bool) -> NormalizedFamily {
    let (owner, u) = source(kind);
    let admit = if affine {
        NormalizedFamily::admit_pair_affine
    } else {
        NormalizedFamily::admit_pair
    };
    admit(owner, u, proposals(), limits()).unwrap()
}
fn options(max_order: i32) -> GenerationOptions {
    GenerationOptions {
        mode: GenerationMode::Symbolic,
        max_order,
        ..Default::default()
    }
}
fn archived(kind: u8, affine: bool) -> Vec<u8> {
    let tmp = tempfile::tempdir().unwrap();
    let f = family(kind, affine);
    let weak = Arc::downgrade(f.owner());
    let c = f.continue_symbolically(&options(2)).unwrap();
    let plan = ThresholdCompilationPlan::prepare_family(
        &c,
        tmp.path(),
        2,
        Default::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plan.job_count(), 5);
    drop(c);
    drop(f);
    assert!(weak.upgrade().is_none());
    let mut writer = plan.archive_writer(Cursor::new(Vec::new())).unwrap();
    for i in (0..plan.job_count()).rev() {
        let complete = plan.job(i).unwrap().run().unwrap();
        plan.validate_completion(&complete).unwrap();
        let mut bytes = Vec::new();
        let receipt = complete.write_record(&mut bytes).unwrap();
        writer
            .append_record(ProgramRecipe::ThresholdV1, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    writer.finish().unwrap().0.into_inner()
}

mod artifacts;
mod faces;
mod geometry;
mod science;
mod zeros;
