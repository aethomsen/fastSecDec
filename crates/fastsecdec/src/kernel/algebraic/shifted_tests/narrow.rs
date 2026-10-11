use super::*;
pub(super) fn narrow_sections() -> Vec<Arc<RegularSection>> {
    use crate::{
        parametric::{
            FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
            PolynomialFactor,
        },
        threshold::{
            gcad::{GcadKinematics, GcadRequest, SolverOptions},
            maps::{
                CellMap,
                regular::{BracketProposal, Budget, Limits, SectionSide},
            },
        },
    };
    let (x, y, e, s, t) = symbol!(
        "shifted_narrow::x",
        "shifted_narrow::y",
        "shifted_narrow::e",
        "shifted_narrow::s",
        "shifted_narrow::t"
    );
    let p = Atom::one() + Atom::var(x) - Atom::num(16) * Atom::var(y)
        + Atom::num(16) * Atom::var(y).pow(6);
    let input = ParametricIntegrand::new(
        vec![x, y],
        e,
        ParametricDomain::UnitCube,
        vec![ParametricTerm::new(
            Atom::one(),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(p, -Atom::one() - Atom::var(e), FactorRole::Singularity)
                    .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    let raw = Arc::new(
        GcadRequest::unit_cube(
            &input,
            GcadKinematics::default(),
            SolverOptions {
                order: vec!["v0".into(), "v1".into()],
                ..Default::default()
            },
            GcadRequest::default_limits(),
        )
        .unwrap()
        .solve_verified()
        .unwrap(),
    );
    assert_eq!(raw.cells().len(), 3);
    // Identify actual selected sections by their certified brackets; the saved
    // selector association is still admitted by RegularSection, not this test.
    let mut result = Vec::new();
    for (lo, hi) in [((1, 16), (17, 128)), ((31, 32), (127, 128))] {
        let mut admitted = None;
        for cell in 0..raw.cells().len() {
            let map = CellMap::new(raw.clone(), cell, vec![s, t]).unwrap();
            for side in [SectionSide::Lower, SectionSide::Upper] {
                if let Ok(section) = RegularSection::prepare(
                    map.clone(),
                    1,
                    side,
                    BracketProposal {
                        lower: Rational::from(lo),
                        upper: Rational::from(hi),
                        derivative_margin: Rational::from((1, 4)),
                    },
                    &mut Budget::new(Limits::default()),
                ) {
                    admitted = Some(section);
                    break;
                }
            }
            if admitted.is_some() {
                break;
            }
        }
        result.push(admitted.unwrap());
    }
    result
}
fn narrow_check<T: Number>(section: &RegularSection, bits: u32)
where
    Complex<T>: EvaluationDomain,
{
    let x = section.source().axes()[0].symbol();
    let (exact, scope) = evaluator(section, x);
    let mut eval = scope.enter(bits, || {
        exact.map_coeff_with_prec(
            &|q| Complex::new(T::rational_at(&q.re, bits), T::rational_at(&q.im, bits)),
            bits,
        )
    });
    // The second captured factory failure lost 17 Float precision bits
    // when rebuilding q0 from independent c_i at the 256-bit rescue tier.
    for point in [
        0.9999325137757114,
        1.9821906045953033e-6,
        0.,
        1.,
        0.125,
        0.5,
        1.9821906045953033e-6,
        0.9999325137757114,
    ] {
        let mut out = vec![Complex::new(T::exact_zero_at(bits), T::exact_zero_at(bits)); 3];
        let q = Rational::try_from(point).unwrap();
        attempt(|| {
            eval.evaluate(
                &[Complex::new(
                    T::rational_at(&q, bits),
                    T::exact_zero_at(bits),
                )],
                &mut out,
            )
        })
        .unwrap();
        assert!(out.iter().all(|v| v.re.valid() && v.im.valid()));
        assert!(
            out[0].re.to_f64() >= section.bracket().lower.to_f64()
                && out[0].re.to_f64() <= section.bracket().upper.to_f64()
        );
    }
}
#[test]
fn actual_factory_narrow_brackets_and_failed_row() {
    for section in narrow_sections() {
        narrow_check::<f64>(&section, 53);
        narrow_check::<DoubleFloat>(&section, 106);
        narrow_check::<Epf<f64>>(&section, 53);
        for bits in [128, 256, 512] {
            narrow_check::<Float>(&section, bits);
            narrow_check::<Epf<Float>>(&section, bits);
        }
        exact_fiber_check(
            &section,
            section.source().axes()[0].symbol(),
            &[
                Rational::zero(),
                Rational::try_from(0.9999325137757114).unwrap(),
                Rational::try_from(1.9821906045953033e-6).unwrap(),
                Rational::one(),
            ],
        );
    }
}
