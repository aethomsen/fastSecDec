use super::*;
use crate::{
    kernel::algebraic::RootProgram,
    threshold::maps::{normalized::PrefixStep, regular::Limits},
};
use std::sync::atomic::AtomicBool;
use symbolica::{
    domains::float::{ErrorPropagatingFloat as Epf, Float},
    evaluate::{FunctionMap, FunctionRegistrationOptions, InliningPolicy},
    function, symbol,
};

fn budget() -> Budget {
    Budget::new(Limits::default())
}
fn settings() -> OptimizationSettings {
    OptimizationSettings::default().cores(1)
}
fn compiled() -> Arc<CompiledFaceInputs> {
    let face = super::super::tests::issued_face();
    let map = Arc::new(
        CompiledOpenMap::prepare(
            face.transport().parent_prefix().clone(),
            settings(),
            &budget(),
        )
        .unwrap(),
    );
    Arc::new(CompiledFaceInputs::prepare(face, map, settings(), &budget()).unwrap())
}
fn isolate(u: &Rational, bits: u32) -> (Rational, Rational) {
    let algebra = symgcad::algebra::Algebra::new(&["u".into(), "Z".into()]).unwrap();
    let polynomial = algebra.parse("Z^2-1-u^2").unwrap();
    let fiber = algebra
        .specialize_univariate(&polynomial, 1, std::slice::from_ref(u))
        .unwrap();
    let mut roots = symgcad::roots::isolate_union(vec![(0, fiber)]).unwrap();
    for root in &mut roots {
        for _ in 0..16 {
            symgcad::roots::refine_once(root);
        }
    }
    let root = roots
        .iter_mut()
        .find(|r| r.interval.0 > Rational::zero())
        .unwrap();
    for _ in 0..bits + 64 {
        symgcad::roots::refine_once(root);
    }
    root.interval.clone()
}
fn within(value: &Float, lo: Rational, hi: Rational, bits: u32) {
    let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 16));
    let actual = value.to_rational();
    assert!(actual >= lo - &tolerance && actual <= hi + &tolerance);
}
fn coefficient_program(owner: &CompiledFaceInputs) -> ExpressionEvaluator<Complex<Rational>> {
    let face = owner.face();
    let crate::threshold::maps::endpoint_transport::numerator::NumeratorLink::Active {
        record, ..
    } = face.catalogue().links()[0][1]
    else {
        panic!()
    };
    let c = &face.coefficients()[face.record_to_coefficient()[record].unwrap()];
    let eps = face.catalogue().preparation().input().regulator();
    let mut formals = owner
        .outputs()
        .iter()
        .copied()
        .map(Atom::var)
        .collect::<Vec<_>>();
    formals.push(Atom::var(eps));
    let body = Atom::evaluator_multiple(
        &[
            c.coefficients()[0].clone(),
            c.coefficients()[1].clone(),
            Atom::var(eps),
        ],
        &formals,
    )
    .optimization_settings(settings())
    .build()
    .unwrap();
    let mut compose = EvaluatorComposer::new(owner.inputs().len() + 1);
    let directions = (0..owner.inputs().len())
        .map(Slot::Param)
        .collect::<Vec<_>>();
    let mut values = compose.append(owner.exact(), &directions).unwrap();
    values.push(Slot::Param(owner.inputs().len()));
    let outputs = compose.append(&body, &values).unwrap();
    compose.finish(&outputs, settings()).unwrap()
}

#[test]
fn finite_face_inputs_at_ray_pole_and_owned_coefficient_composition() {
    let owner = compiled();
    assert_eq!(owner.face().finite_coordinates().len(), 2);
    assert_eq!(owner.outputs()[..2], *owner.face().finite_coordinates());
    assert_eq!(owner.outputs()[2..], *owner.inputs());
    let exact = coefficient_program(&owner);
    let bytes = bincode::serde::encode_to_vec(&exact, bincode::config::standard()).unwrap();
    let (restored, used): (ExpressionEvaluator<Complex<Rational>>, _) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(used, bytes.len());
    let scope = owner.scope().native_roundtrip().unwrap();
    for bits in [128, 256] {
        let mut finite = owner.map::<Float>(bits).unwrap();
        let mut tracked = owner.map::<Epf<Float>>(bits).unwrap();
        let mut bulk = owner.map.map::<Float>(bits);
        let mut coefficients = scope
            .map_checked(&restored, bits, |q| Float::rational_at(&q.re, bits))
            .unwrap();
        // Warm input refresh includes rational and algebraic selected roots.
        for u in [
            Rational::from((3, 4)),
            Rational::from((1, 3)),
            Rational::from((3, 4)),
        ] {
            let v = Rational::from((2, 5));
            let directions = [&u, &v].map(|q| Float::rational_at(q, bits));
            let out = finite.evaluate(&directions).unwrap();
            let (zlo, zhi) = isolate(&u, bits);
            let x = &u * &u;
            let ylo = &u * &v * zlo;
            let yhi = &u * &v * zhi;
            within(&out[0], x.clone(), x.clone(), bits);
            within(&out[1], ylo.clone(), yhi.clone(), bits);
            assert_eq!(out[2].to_rational(), directions[0].to_rational());
            assert_eq!(out[3].to_rational(), directions[1].to_rational());
            // The full map correctly refuses the very ray endpoint where the
            // selected finite source inputs and coefficient program succeed.
            assert!(
                bulk.evaluate(&[
                    directions[0].clone(),
                    directions[1].clone(),
                    Float::one_at(bits)
                ])
                .is_err()
            );
            let incoming = directions.map(|x| Epf::new(x, 20.));
            let out = tracked.evaluate(&incoming).unwrap();
            assert!(out.iter().all(|x| x.valid() && x.get_absolute_error() > 0.));
            let values = [&u, &v, &Rational::from((1, 8))].map(|q| Float::rational_at(q, bits));
            let mut results = vec![Float::exact_zero_at(bits); 3];
            attempt(|| coefficients.try_evaluate(&values, &mut results))
                .unwrap()
                .unwrap();
            within(&results[0], ylo.clone(), yhi.clone(), bits);
            within(
                &results[1],
                &x - &(Rational::from(2) * &yhi),
                &x - &(Rational::from(2) * &ylo),
                bits,
            );
            assert_eq!(results[2].to_rational(), Rational::from((1, 8)));
        }
    }
}

#[test]
fn finite_face_inputs_relative_domain_source_identity_and_native_branch_refusal() {
    let owner = compiled();
    let mut numeric = owner.map::<f64>(53).unwrap();
    for directions in [vec![0., 0.5], vec![0.5, 1.], vec![f64::NAN, 0.5], vec![0.5]] {
        assert!(matches!(
            numeric.evaluate(&directions),
            Err(Error::Invalid(_))
        ));
    }
    assert!(numeric.evaluate(&[0.5, 0.5]).is_ok());
    let foreign_prefix = owner
        .map
        .owner()
        .previous()
        .unwrap()
        .open(&budget())
        .unwrap();
    let foreign =
        Arc::new(CompiledOpenMap::prepare(foreign_prefix, settings(), &budget()).unwrap());
    assert!(matches!(
        CompiledFaceInputs::prepare(owner.face().clone(), foreign, settings(), &budget()),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        CompiledFaceInputs::prepare(
            owner.face().clone(),
            owner.map.clone(),
            settings(),
            &budget().with_cancellation(Arc::new(AtomicBool::new(true)))
        ),
        Err(Error::Cancelled)
    ));
    let x = symbol!("finite_face_program_test::condition");
    let branch = function!(Symbol::IF, Atom::var(x), 1, 2)
        .evaluator(&[Atom::var(x)])
        .build()
        .unwrap();
    assert!(matches!(
        require_straight_line(&branch),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn finite_face_inputs_refuse_genuine_raw_root_coefficient_role() {
    let owner = compiled();
    let transport = owner.face().transport().clone();
    // This normal changes y = u*v*Z while the selected Q depends only on u.
    // The genuine face retains x,z as finite formals and has a coefficient
    // u*Z*z^2. Z must not become an arbitrary external numeric input.
    let face = NumeratorFace::prepare(
        owner.face().catalogue().clone(),
        transport.clone(),
        0,
        transport.target_units()[1],
        2,
        &super::super::Limits::default(),
        || std::ops::ControlFlow::Continue(()),
    )
    .unwrap();
    assert!(face.coefficients().iter().any(|coefficient| {
        coefficient.coefficients().iter().any(|body| {
            face.root_symbols()
                .iter()
                .any(|root| body.contains(Atom::var(*root).as_view()))
        })
    }));
    assert!(matches!(
        CompiledFaceInputs::prepare(face, owner.map.clone(), settings(), &budget()),
        Err(Error::Unsupported(detail)) if detail.contains("raw root role")
    ));
}

#[test]
fn finite_face_native_function_mixed_derivative_uses_normalized_coefficient_chain() {
    let owner = compiled();
    let face = owner.face();
    let section = owner
        .map
        .owner()
        .steps()
        .into_iter()
        .find_map(|step| match step {
            PrefixStep::Section { section, .. } => Some(section),
            _ => None,
        })
        .unwrap();
    let root_program = section.root_program(settings()).unwrap();
    let restored = RootProgram::from_bytes(root_program.bytes()).unwrap();
    let tag = symbol!("finite_face_program_test::root");
    let root = restored.call(tag, section.coefficients()).unwrap();
    let root = super::super::super::clearing::pull(
        &root,
        &section.chart().parameters,
        &section.chart().parameter_images,
    );
    // The transport target roles are distinct symbols from the original map
    // inputs. Follow the actual selected chart before taking face derivatives.
    let root = super::super::super::clearing::pull(
        &root,
        owner.map.inputs(),
        face.transport().charts()[face.chart_index()].unit_images(),
    );
    let root = super::super::super::clearing::pull(&root, &[face.normal()], &[Atom::zero()]);
    let images = face
        .finite_images()
        .iter()
        .map(|a| {
            super::super::super::clearing::pull(
                a,
                &[section.root_symbol()],
                std::slice::from_ref(&root),
            )
        })
        .collect::<Vec<_>>();
    let crate::threshold::maps::endpoint_transport::numerator::NumeratorLink::Active {
        record, ..
    } = face.catalogue().links()[0][1]
    else {
        panic!()
    };
    let c = &face.coefficients()[face.record_to_coefficient()[record].unwrap()].coefficients()[1];
    let head = symbol!("finite_face_program_test::coefficient");
    let formals = face.finite_coordinates().to_vec();
    let mut functions = FunctionMap::new();
    functions
        .add_tagged_function_with_options(
            head,
            vec![Atom::num(0)],
            formals.clone(),
            c.clone(),
            FunctionRegistrationOptions::new().inlining(InliningPolicy::Always),
        )
        .unwrap();
    // Existing native DERIVATIVE convention: order per tag/formal, then head
    // and tag value. No hand-written implicit derivative or new AD is used.
    for i in 0..=2 {
        for j in 0..=2 - i {
            if i + j == 0 {
                continue;
            }
            let mut body = c.clone();
            for _ in 0..i {
                body = body.derivative(formals[0]);
            }
            for _ in 0..j {
                body = body.derivative(formals[1]);
            }
            functions
                .add_tagged_function_with_options(
                    Symbol::DERIVATIVE,
                    vec![
                        Atom::num(0),
                        Atom::num(i),
                        Atom::num(j),
                        Atom::var(head),
                        Atom::num(0),
                    ],
                    formals.clone(),
                    body,
                    FunctionRegistrationOptions::new().inlining(InliningPolicy::Always),
                )
                .unwrap();
        }
    }
    let call = function!(head, 0, images[0].clone(), images[1].clone());
    let u = owner.inputs()[0];
    let v = owner.inputs()[1];
    let exact = Atom::evaluator_multiple(
        &[call.clone(), call.derivative(u).derivative(v)],
        &owner
            .inputs()
            .iter()
            .copied()
            .map(Atom::var)
            .collect::<Vec<_>>(),
    )
    .function_map(functions)
    .optimization_settings(settings())
    .build()
    .unwrap();
    let mut scope = Scope::default();
    scope.insert(tag, restored).unwrap();
    for bits in [128, 256] {
        let mut eval = scope
            .map_checked(&exact, bits, |q| Float::rational_at(&q.re, bits))
            .unwrap();
        let mut composed = owner
            .scope()
            .map_checked(&coefficient_program(&owner), bits, |q| {
                Float::rational_at(&q.re, bits)
            })
            .unwrap();
        let input =
            [Rational::from((3, 4)), Rational::from((2, 5))].map(|q| Float::rational_at(&q, bits));
        let mut out = vec![Float::exact_zero_at(bits); 2];
        attempt(|| eval.try_evaluate(&input, &mut out))
            .unwrap()
            .unwrap();
        // Z=5/4 at u=3/4: C1=9/16−3/4=−3/16;
        // mixed derivative −2(Z+u²/Z)=−17/5 is nonzero.
        within(
            &out[0],
            Rational::from((-3, 16)),
            Rational::from((-3, 16)),
            bits,
        );
        within(
            &out[1],
            Rational::from((-17, 5)),
            Rational::from((-17, 5)),
            bits,
        );
        let mut values = input.to_vec();
        values.push(Float::rational_at(&Rational::from((1, 8)), bits));
        let mut reference = vec![Float::exact_zero_at(bits); 3];
        attempt(|| composed.try_evaluate(&values, &mut reference))
            .unwrap()
            .unwrap();
        within(
            &reference[1],
            out[0].to_rational(),
            out[0].to_rational(),
            bits,
        );
    }
}
