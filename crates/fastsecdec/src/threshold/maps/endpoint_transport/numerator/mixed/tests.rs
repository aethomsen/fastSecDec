use super::program::CompiledMixedNumeratorFace;
use super::*;
use crate::{
    kernel::algebraic::{Number, attempt},
    threshold::maps::endpoint_transport::numerator::{
        SourceNumerators, normal::NormalNumeratorJets,
    },
};
use symbolica::domains::SelfRing;
use symbolica::{
    domains::{
        float::{Complex, Float},
        rational::Rational,
    },
    evaluate::{ExpressionEvaluator, FunctionMap},
    id::{Pattern, Replacement},
    symbol,
};
fn settings() -> OptimizationSettings {
    OptimizationSettings::default().cores(1)
}
fn issued() -> Arc<MixedNumeratorFace> {
    let t = super::super::tests::fixture();
    let c = SourceNumerators::prepare(
        t.parent_prefix()
            .source()
            .decomposition()
            .preparation()
            .clone(),
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let face = NumeratorFace::prepare(
        c,
        t.clone(),
        0,
        t.target_units()[2],
        3,
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    MixedNumeratorFace::prepare(
        face,
        t.target_units()[0],
        4,
        settings(),
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap()
}
fn pull(a: &Atom, s: &[Symbol], v: &[Atom]) -> Atom {
    a.replace_multiple(s.iter().zip(v).map(|(s, v)| {
        Replacement::new(Pattern::Literal(Atom::var(*s)), Pattern::Literal(v.clone()))
    }))
}
fn equal(a: &Atom, b: &Atom) {
    assert!((a - b).together().is_zero(), "{a} != {b}");
}
fn close(a: &Float, b: &Rational, bits: u32) {
    let d = a.to_rational() - b;
    let t = Rational::from((1, 2)).pow(u64::from(bits - 16));
    assert!(d >= -t.clone() && d <= t, "{d}");
}
#[test]
fn mixed_joint_root_chain_exact_clearing_and_native_codec() {
    let owner = issued();
    let face = owner.face();
    let prep = face.catalogue().preparation();
    let n = prep.terms()[0].factors()[1].polynomial();
    let record = face
        .catalogue()
        .records()
        .iter()
        .position(|r| r.body() == n)
        .unwrap();
    let row = face
        .coefficients()
        .iter()
        .find(|r| r.source_record() == record)
        .unwrap();
    assert_eq!(row.normal_order().to_string(), "-2");
    assert_eq!(row.cleared_order().to_string(), "0");
    equal(row.direction_divisor(), &Atom::one());
    equal(row.clearing(), &Atom::var(face.normal()).pow(2));
    let coefficient = row
        .coefficients()
        .iter()
        .map(|x| pull(x, face.finite_coordinates(), face.finite_images()))
        .collect::<Vec<_>>();
    let h = coefficient
        .iter()
        .enumerate()
        .map(|(i, x)| x * Atom::var(face.normal()).pow(i as u32))
        .sum::<Atom>();
    let source = pull(n, prep.coordinates(), owner.chart().chart().source_images());
    equal(&h, &(&source * row.clearing()));
    assert!(
        !(&h - &source * Atom::var(face.normal()))
            .together()
            .is_zero()
    );
    assert!(Arc::ptr_eq(
        owner.selected_extensions()[0].section(),
        face.transport().root_relations()[0].section()
    ));
    assert!(!owner.closed_density_certified());
    let eps = prep.input().regulator();
    let compiled = Arc::new(
        CompiledMixedNumeratorFace::prepare(
            owner.clone(),
            &[eps],
            FunctionMap::new(),
            settings(),
            &Budget::new(Limits::default().algebra),
        )
        .unwrap(),
    );
    let bytes =
        bincode::serde::encode_to_vec(compiled.exact(), bincode::config::standard()).unwrap();
    let (restored, used): (ExpressionEvaluator<Complex<Rational>>, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(used, bytes.len());
    let mut issued_derivatives = Vec::new();
    for bits in [128, 256] {
        let v = Rational::from((2, 5));
        let point = [Float::rational_at(&v, bits)];
        let eps = Complex::new(
            Float::rational_at(&Rational::from((1, 7)), bits),
            Float::rational_at(&Rational::from((1, 9)), bits),
        );
        let values = compiled
            .map::<Float>(bits)
            .unwrap()
            .evaluate(&point, std::slice::from_ref(&eps))
            .unwrap();
        let expected = [
            [
                Rational::zero(),
                v.clone(),
                Rational::zero(),
                &v / &Rational::from(2),
            ],
            [
                Rational::zero(),
                -v.clone() * &Rational::from(2),
                Rational::one(),
                -v.clone(),
            ],
            [
                Rational::one(),
                v.clone(),
                Rational::from(-1),
                &v / &Rational::from(2),
            ],
        ];
        for (j, want) in expected.iter().enumerate() {
            let i = owner
                .coefficients()
                .iter()
                .position(|r| r.source_record() == record && r.clearing_order() == j)
                .unwrap();
            for (a, b) in values[i].iter().zip(want) {
                close(&a.re, b, bits);
                assert!(a.im.is_zero());
            }
        }
        let mixed_row = owner
            .coefficients()
            .iter()
            .position(|r| r.source_record() == record && r.clearing_order() == 1)
            .unwrap();
        issued_derivatives.push((
            bits,
            [1_i64, 2, 6]
                .into_iter()
                .enumerate()
                .map(|(i, factorial)| {
                    values[mixed_row][i + 1].re.to_rational() * Rational::from(factorial)
                })
                .collect::<Vec<_>>(),
        ));
        let mut decoded = compiled
            .scope()
            .map_checked(&restored, bits, |q| {
                Complex::new(
                    Float::rational_at(&q.re, bits),
                    Float::rational_at(&q.im, bits),
                )
            })
            .unwrap();
        let input = [
            Complex::new(point[0].clone(), Float::exact_zero_at(bits)),
            eps,
        ];
        let mut out = vec![
            Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits));
            restored.get_output_len()
        ];
        attempt(|| decoded.try_evaluate(&input, &mut out))
            .unwrap()
            .unwrap();
        let flat = values.iter().flatten().cloned().collect::<Vec<_>>();
        assert_eq!(&out[..flat.len()], flat.as_slice());
    }
    // Independent native Etale derivatives, including actual mixed factorials.
    use crate::threshold::resolution::{
        Budget as RB, EtaleCertificate, Ideal, Limits as RL, LocalizedAlgebra, Ring,
    };
    let u = owner.lower_normal();
    let v = owner.direction_units()[0];
    let c = face.normal();
    let z = owner.selected_extensions()[0].section().root_symbol();
    let inv = symbol!("mixed_test::inverse");
    let ring = Arc::new(Ring::new(vec![u, v, c, z, inv], vec![]).unwrap());
    let mut budget = RB::new(RL::default());
    let equation = Atom::one() + Atom::var(u).pow(2) - Atom::var(z).pow(2);
    let ideal = Ideal::new(
        ring.clone(),
        vec![ring.atom(&equation).unwrap()],
        &mut budget,
    )
    .unwrap();
    let local = LocalizedAlgebra::new(ideal, vec![0, 1, 2, 3], vec![], &mut budget).unwrap();
    let frame = EtaleCertificate {
        source: local,
        equations: vec![0],
        dependent_axes: vec![3],
        free_axes: vec![0, 1, 2],
        determinant_inverse_axis: Some(4),
    }
    .verify(&mut budget)
    .unwrap();
    let point = [
        Rational::zero(),
        Rational::from((2, 5)),
        Rational::zero(),
        Rational::one(),
        Rational::from((-1, 2)),
    ];
    assert!(frame.local().check_real_point(&point).unwrap());
    let p = ring.atom(&h).unwrap();
    let mut a = p.clone();
    let mut b = frame.derivative(2, &p, &mut budget).unwrap();
    for (order, want) in [
        Rational::from((-4, 5)),
        Rational::from(2),
        Rational::from((-12, 5)),
    ]
    .into_iter()
    .enumerate()
    {
        for (bits, actual) in &issued_derivatives {
            let error = &actual[order] - &want;
            let tolerance = Rational::from((1, 2)).pow(u64::from(*bits - 16));
            assert!(error >= -tolerance.clone() && error <= tolerance);
        }
        a = frame.derivative(0, &a, &mut budget).unwrap();
        b = frame.derivative(0, &b, &mut budget).unwrap();
        assert_eq!(b.replace_all(&point), want);
        assert_eq!(
            frame
                .derivative(2, &a, &mut budget)
                .unwrap()
                .replace_all(&point),
            want
        );
    }
    assert!(
        pull(&coefficient[1], &[z], &[Atom::one()])
            .derivative(u)
            .derivative(u)
            .derivative(u)
            .is_zero()
    );
}
#[test]
fn mixed_joint_refuses_foreign_normals_depth_and_unremoved_ray() {
    let o = issued();
    let f = o.face().clone();
    let limits = Limits::default();
    for normal in [f.normal(), symbol!("mixed_test::foreign")] {
        assert!(matches!(
            MixedNumeratorFace::prepare(f.clone(), normal, 4, settings(), &limits, || {
                ControlFlow::Continue(())
            }),
            Err(Error::Invalid(_))
        ));
    }
    assert!(matches!(
        MixedNumeratorFace::prepare(f.clone(), o.lower_normal(), 0, settings(), &limits, || {
            ControlFlow::Continue(())
        }),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        MixedNumeratorFace::prepare(f.clone(), o.lower_normal(), 4, settings(), &limits, || {
            ControlFlow::Break(())
        }),
        Err(Error::Cancelled)
    ));
    let t = f.transport();
    let c = t.target_units()[2];
    let v = t.target_units()[1];
    let other = NumeratorFace::prepare(f.catalogue().clone(), t.clone(), 0, v, 3, &limits, || {
        ControlFlow::Continue(())
    })
    .unwrap();
    assert!(matches!(
        MixedNumeratorFace::prepare(other, c, 4, settings(), &limits, || ControlFlow::Continue(
            ()
        )),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        NormalNumeratorJets::prepare(
            f.catalogue().clone(),
            o.chart().clone(),
            c,
            4,
            settings(),
            &limits,
            || ControlFlow::Continue(())
        ),
        Err(Error::Unsupported(_))
    ));
}
#[test]
fn mixed_coefficient_aliases_cannot_rebind_geometry_or_source_formals() {
    let o = issued();
    let eps = o.face().catalogue().preparation().input().regulator();
    let budget = Budget::new(Limits::default().algebra);
    for role in o
        .formals
        .iter()
        .copied()
        .chain([o.lower_normal(), o.face().normal()])
    {
        assert!(matches!(
            CompiledMixedNumeratorFace::prepare(
                o.clone(),
                &[role],
                FunctionMap::new(),
                settings(),
                &budget
            ),
            Err(Error::Invalid(_))
        ));
    }
    let mut aliases = FunctionMap::new();
    aliases
        .add_aliases(o.formals.iter().map(|s| (Atom::var(*s), Atom::zero())))
        .unwrap();
    let clean = Arc::new(
        CompiledMixedNumeratorFace::prepare(
            o.clone(),
            &[eps],
            FunctionMap::new(),
            settings(),
            &budget,
        )
        .unwrap(),
    );
    let protected = Arc::new(
        CompiledMixedNumeratorFace::prepare(o.clone(), &[eps], aliases, settings(), &budget)
            .unwrap(),
    );
    let bits = 128;
    let p = [Float::rational_at(&Rational::from((2, 5)), bits)];
    let e = [Complex::new(
        Float::rational_at(&Rational::from((1, 7)), bits),
        Float::rational_at(&Rational::from((1, 9)), bits),
    )];
    assert_eq!(
        clean.map::<Float>(bits).unwrap().evaluate(&p, &e).unwrap(),
        protected
            .map::<Float>(bits)
            .unwrap()
            .evaluate(&p, &e)
            .unwrap()
    );
    let mut bad = FunctionMap::new();
    bad.add_aliases([(Atom::var(eps), Atom::var(o.lower_normal()))])
        .unwrap();
    assert!(CompiledMixedNumeratorFace::prepare(o, &[], bad, settings(), &budget).is_err());
}
#[test]
fn mixed_nontrivial_direction_divisor_is_certified_and_joint_pole_refuses() {
    let transport = super::test_fixture::fixture();
    let limits = Limits::default();
    let catalogue = SourceNumerators::prepare(
        transport
            .parent_prefix()
            .source()
            .decomposition()
            .preparation()
            .clone(),
        &limits,
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let mut checked = false;
    for chart in 0..transport.charts().len() {
        for normal in &transport.target_units()[2..] {
            let face = NumeratorFace::prepare(
                catalogue.clone(),
                transport.clone(),
                chart,
                *normal,
                3,
                &limits,
                || ControlFlow::Continue(()),
            )
            .unwrap();
            let Some(row) = face
                .coefficients()
                .iter()
                .find(|r| !r.direction_divisor().is_one())
            else {
                continue;
            };
            let divisor = row.direction_divisor().clone();
            let source_record = row.source_record();
            let owner = MixedNumeratorFace::prepare(
                face.clone(),
                transport.target_units()[0],
                4,
                settings(),
                &limits,
                || ControlFlow::Continue(()),
            )
            .unwrap();
            assert!(Arc::ptr_eq(owner.face(), &face));
            assert!(
                owner
                    .coefficients()
                    .iter()
                    .any(|r| r.source_record() == source_record)
            );
            let extra = &owner.finite_arguments()[owner.formals.len()..];
            assert!(extra.iter().any(|a| a.denominator_certificate().is_some()));
            let bad = *transport
                .target_units()
                .iter()
                .find(|s| **s != *normal && divisor.contains(Atom::var(**s).as_view()))
                .unwrap();
            assert!(matches!(
                MixedNumeratorFace::prepare(face, bad, 4, settings(), &limits, || {
                    ControlFlow::Continue(())
                }),
                Err(Error::Unsupported(_))
            ));
            checked = true;
            break;
        }
        if checked {
            break;
        }
    }
    assert!(
        checked,
        "genuine native fan must exhibit a direction clearing divisor"
    );
}

#[test]
fn mixed_nonzero_formal_valuation_keeps_absolute_lower_coefficients() {
    let transport = super::test_fixture::fixture();
    let limits = Limits::default();
    let catalogue = SourceNumerators::prepare(
        transport
            .parent_prefix()
            .source()
            .decomposition()
            .preparation()
            .clone(),
        &limits,
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let y = Atom::var(catalogue.preparation().coordinates()[2]);
    let record = catalogue
        .records()
        .iter()
        .position(|r| r.body() == &y)
        .unwrap();
    let face = NumeratorFace::prepare(
        catalogue.clone(),
        transport.clone(),
        0,
        transport.target_units()[1],
        3,
        &limits,
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let row = face
        .coefficients()
        .iter()
        .find(|r| r.source_record() == record)
        .unwrap();
    assert_eq!(row.cleared_order().to_string(), "1");
    assert_eq!(row.normal_order().to_string(), "1");
    equal(row.clearing(), &Atom::one());
    equal(row.direction_divisor(), &Atom::one());
    let owner = MixedNumeratorFace::prepare(
        face,
        transport.target_units()[0],
        4,
        settings(),
        &limits,
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let position = owner
        .coefficients()
        .iter()
        .position(|r| r.source_record() == record && r.clearing_order() == 0)
        .unwrap();
    let eps = catalogue.preparation().input().regulator();
    let program = Arc::new(
        CompiledMixedNumeratorFace::prepare(
            owner,
            &[eps],
            FunctionMap::new(),
            settings(),
            &Budget::new(limits.algebra),
        )
        .unwrap(),
    );
    for bits in [128, 256] {
        let point =
            [Rational::from((1, 3)), Rational::from((2, 5))].map(|q| Float::rational_at(&q, bits));
        let epsilon = [Complex::new(
            Float::rational_at(&Rational::from((1, 7)), bits),
            Float::exact_zero_at(bits),
        )];
        let values = program
            .map::<Float>(bits)
            .unwrap()
            .evaluate(&point, &epsilon)
            .unwrap();
        for (a, b) in values[position].iter().zip([
            Rational::zero(),
            Rational::one(),
            Rational::zero(),
            Rational::from((1, 2)),
        ]) {
            close(&a.re, &b, bits);
        }
        // N=v G and G=u sqrt(1+u²): ∂u³∂v N = 3! * [u³]G.
        let actual = values[position][3].re.to_rational() * Rational::from(6);
        let error = actual - Rational::from(3);
        let tol = Rational::from((1, 2)).pow(u64::from(bits - 16));
        assert!(error >= -tol.clone() && error <= tol);
    }
}
