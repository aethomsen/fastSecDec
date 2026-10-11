use super::*;
use crate::{
    kernel::algebraic::Number,
    threshold::maps::endpoint_transport::{Limits as TransportLimits, cover::EndpointChartCover},
};
use program::CompiledNormalNumeratorJets;
use symbolica::domains::SelfRing;
use symbolica::{
    domains::{
        float::{Complex, ErrorPropagatingFloat, Float},
        rational::Rational,
    },
    evaluate::{ExpressionEvaluator, FunctionMap},
    symbol,
};
fn settings() -> OptimizationSettings {
    OptimizationSettings::default().cores(1)
}
fn source() -> (
    Arc<SourceNumerators>,
    Arc<crate::threshold::maps::endpoint_transport::EndpointTransport>,
) {
    let transport = super::super::tests::fixture();
    let catalogue = SourceNumerators::prepare(
        transport
            .parent_prefix()
            .source()
            .decomposition()
            .preparation()
            .clone(),
        &Limits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    (catalogue, transport)
}
fn prepare(
    c: Arc<SourceNumerators>,
    v: IssuedEndpointChart,
    n: Symbol,
) -> Arc<NormalNumeratorJets> {
    NormalNumeratorJets::prepare(c, v, n, 4, settings(), &Limits::default(), || {
        ControlFlow::Continue(())
    })
    .unwrap()
}
fn close(actual: &Float, expected: &Rational, bits: u32) {
    let error = actual.to_rational() - expected;
    let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 16));
    assert!(error >= -tolerance.clone() && error <= tolerance, "{error}");
}
#[test]
fn compact_normal_jets_root_chain_original_body_and_native_codec() {
    let (catalogue, transport) = source();
    let owner = prepare(
        catalogue.clone(),
        transport.issued_chart(0).unwrap(),
        transport.target_units()[0],
    );
    assert!(!owner.closed_density_certified());
    assert_eq!(owner.selected_extensions().len(), 1);
    assert!(Arc::ptr_eq(
        owner.selected_extensions()[0].section(),
        transport.root_relations()[0].section()
    ));
    // Every generated body stays in original preparation formals, never Q slots.
    for d in &owner.definitions {
        assert!(
            !d.body
                .contains(Atom::var(owner.sections[0].section().root_symbol()).as_view())
        );
    }
    let eps = catalogue.preparation().input().regulator();
    let budget = Budget::new(Limits::default().algebra);
    let compiled = CompiledNormalNumeratorJets::prepare(
        owner.clone(),
        &[eps],
        FunctionMap::new(),
        settings(),
        &budget,
    )
    .unwrap();
    let bytes =
        bincode::serde::encode_to_vec(compiled.exact(), bincode::config::standard()).unwrap();
    eprintln!(
        "normal-jet code: records={}, depth={}, source_definitions={}, instructions={}, coefficient_stage_outputs={}, native_ir_bytes={}",
        owner.coefficients().len(),
        owner.depth(),
        owner.source_definition_count(),
        compiled.exact().export_instructions().instructions.len(),
        compiled.coefficient_stage().get_output_len(),
        bytes.len()
    );
    let (restored, used): (ExpressionEvaluator<Complex<Rational>>, usize) =
        bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
    assert_eq!(used, bytes.len());
    let compiled = Arc::new(compiled);
    for bits in [128, 256] {
        let mut numeric = compiled.map::<Float>(bits).unwrap();
        let point =
            [Rational::from((2, 5)), Rational::from((1, 3))].map(|q| Float::rational_at(&q, bits));
        let value = numeric
            .evaluate(
                &point,
                &[Complex::new(
                    Float::rational_at(&Rational::from((1, 7)), bits),
                    Float::rational_at(&Rational::from((1, 9)), bits),
                )],
            )
            .unwrap();
        let mut restored_numeric = compiled
            .scope()
            .map_checked(&restored, bits, |q| {
                Complex::new(
                    Float::rational_at(&q.re, bits),
                    Float::rational_at(&q.im, bits),
                )
            })
            .unwrap();
        let mut input = point
            .iter()
            .map(|x| Complex::new(x.clone(), Float::exact_zero_at(bits)))
            .collect::<Vec<_>>();
        input.push(Complex::new(
            Float::rational_at(&Rational::from((1, 7)), bits),
            Float::rational_at(&Rational::from((1, 9)), bits),
        ));
        let mut output = vec![
            Complex::new(Float::exact_zero_at(bits), Float::exact_zero_at(bits));
            restored.get_output_len()
        ];
        crate::kernel::algebraic::attempt(|| restored_numeric.try_evaluate(&input, &mut output))
            .unwrap()
            .unwrap();
        assert_eq!(
            &output[..8],
            value.iter().flatten().cloned().collect::<Vec<_>>()
        );
        let expected = [
            Rational::one(),
            Rational::from((8, 5)),
            Rational::from(2),
            Rational::from((4, 5)),
        ];
        for (a, e) in value[0].iter().zip(&expected) {
            close(&a.re, e, bits);
            assert!(a.im.is_zero());
        }
        close(&value[1][0].re, &Rational::from((9, 7)), bits);
        close(&value[1][0].im, &Rational::from((2, 9)), bits);
        assert!(
            value[1][1..]
                .iter()
                .all(|x| x.re.is_zero() && x.im.is_zero())
        );
    }
}
#[test]
fn compact_normal_jets_aliases_are_source_local_and_tracking_survives() {
    let (catalogue, transport) = source();
    let owner = prepare(
        catalogue.clone(),
        transport.issued_chart(0).unwrap(),
        transport.target_units()[0],
    );
    let eps = catalogue.preparation().input().regulator();
    let mut aliases = FunctionMap::new();
    aliases
        .add_aliases(
            transport
                .target_units()
                .iter()
                .chain(transport.parent_prefix().source().unit_coordinates())
                .map(|s| (Atom::var(*s), Atom::zero()))
                .chain([(
                    Atom::var(owner.sections[0].section().root_symbol()),
                    Atom::zero(),
                )]),
        )
        .unwrap();
    let budget = Budget::new(Limits::default().algebra);
    let compiled = Arc::new(
        CompiledNormalNumeratorJets::prepare(owner.clone(), &[eps], aliases, settings(), &budget)
            .unwrap(),
    );
    let bits = 128;
    let mut numeric = compiled.map::<Float>(bits).unwrap();
    let point =
        [Rational::from((2, 5)), Rational::from((1, 3))].map(|q| Float::rational_at(&q, bits));
    let value = numeric
        .evaluate(
            &point,
            &[Complex::new(
                Float::rational_at(&Rational::from((1, 7)), bits),
                Float::exact_zero_at(bits),
            )],
        )
        .unwrap();
    close(&value[0][3].re, &Rational::from((4, 5)), bits);
    let mut tracked = compiled.map::<ErrorPropagatingFloat<Float>>(bits).unwrap();
    let point = point.map(|x| ErrorPropagatingFloat::new(x, 20.));
    let zero = ErrorPropagatingFloat::<Float>::exact_zero_at(bits);
    let result = tracked
        .evaluate(&point, &[Complex::new(zero.clone(), zero)])
        .unwrap();
    assert!(result[0][1].re.get_absolute_error() > 0.);
    assert!(result[0][3].re.get_absolute_error() > 0.);
    // The actual coefficient stage, not a discarded validation, determines
    // coefficient aliases. A coordinate cannot become an untracked argument.
    let x = catalogue.preparation().coordinates()[0];
    let mut aliases = FunctionMap::new();
    aliases
        .add_aliases([(Atom::var(eps), Atom::var(x))])
        .unwrap();
    assert!(
        CompiledNormalNumeratorJets::prepare(
            owner.clone(),
            &[],
            aliases.clone(),
            settings(),
            &budget
        )
        .is_err()
    );
    aliases.add_aliases([(Atom::var(x), Atom::zero())]).unwrap();
    let changed = Arc::new(
        CompiledNormalNumeratorJets::prepare(owner, &[], aliases, settings(), &budget).unwrap(),
    );
    let mut numeric = changed.map::<Float>(bits).unwrap();
    let inputs =
        [Rational::from((2, 5)), Rational::from((1, 3))].map(|q| Float::rational_at(&q, bits));
    let result = numeric.evaluate(&inputs, &[]).unwrap();
    close(&result[0][3].re, &Rational::from((4, 5)), bits);
    close(&result[1][0].re, &Rational::one(), bits);
}
#[test]
fn compact_normal_jets_refined_finite_denominator_and_typed_refusals() {
    let (catalogue, transport) = source();
    let axes = transport.target_units();
    let cover = EndpointChartCover::prepare(
        transport.clone(),
        0,
        &[axes[0], axes[2]],
        &[],
        &TransportLimits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    // Lower u, upper c: the ray denominator is 1-c_child/2, not a monomial.
    let leaf = cover.halves()[2].leaf_range().start;
    let chart = cover.issued_chart(leaf).unwrap();
    let normal = cover.target_units()[0];
    let owner = prepare(catalogue.clone(), chart, normal);
    assert!(owner.chart().cover().is_some());
    let eps = catalogue.preparation().input().regulator();
    let compiled = Arc::new(
        CompiledNormalNumeratorJets::prepare(
            owner,
            &[eps],
            FunctionMap::new(),
            settings(),
            &Budget::new(Limits::default().algebra),
        )
        .unwrap(),
    );
    for bits in [128, 256] {
        let point =
            [Rational::from((2, 5)), Rational::from((1, 3))].map(|q| Float::rational_at(&q, bits));
        let value = compiled
            .map::<Float>(bits)
            .unwrap()
            .evaluate(
                &point,
                &[Complex::new(
                    Float::exact_zero_at(bits),
                    Float::exact_zero_at(bits),
                )],
            )
            .unwrap();
        // u_parent=u/2, c_parent=1-c/2=5/6, ray=1/5.
        for (a, e) in value[0].iter().zip([
            Rational::one(),
            Rational::from((1, 125)),
            Rational::from((1, 20)),
            Rational::from((1, 1000)),
        ]) {
            close(&a.re, &e, bits);
        }
    }
    let original = transport.issued_chart(0).unwrap();
    assert!(matches!(
        NormalNumeratorJets::prepare(
            catalogue.clone(),
            original.clone(),
            axes[2],
            4,
            settings(),
            &Limits::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        NormalNumeratorJets::prepare(
            catalogue.clone(),
            original.clone(),
            symbol!("normal_jet_test::foreign"),
            4,
            settings(),
            &Limits::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        NormalNumeratorJets::prepare(
            catalogue.clone(),
            original.clone(),
            axes[0],
            0,
            settings(),
            &Limits::default(),
            || ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        NormalNumeratorJets::prepare(
            catalogue,
            original,
            axes[0],
            4,
            settings(),
            &Limits::default(),
            || ControlFlow::Break(())
        ),
        Err(Error::Cancelled)
    ));
}

#[test]
fn compact_normal_jets_zero_source_coordinate_keeps_integer_powers() {
    let (catalogue, transport) = source();
    let axes = transport.target_units();
    let cover = EndpointChartCover::prepare(
        transport.clone(),
        0,
        &[axes[0], axes[2]],
        &[],
        &TransportLimits::default(),
        || ControlFlow::Continue(()),
    )
    .unwrap();
    let chart = cover
        .issued_chart(cover.halves()[2].leaf_range().start)
        .unwrap();
    let owner = prepare(catalogue.clone(), chart, cover.target_units()[2]);
    let eps = catalogue.preparation().input().regulator();
    let compiled = Arc::new(
        CompiledNormalNumeratorJets::prepare(
            owner,
            &[eps],
            FunctionMap::new(),
            settings(),
            &Budget::new(Limits::default().algebra),
        )
        .unwrap(),
    );
    for bits in [128, 256] {
        let inputs =
            [Rational::from((1, 2)), Rational::from((2, 5))].map(|q| Float::rational_at(&q, bits));
        let output = compiled
            .map::<Float>(bits)
            .unwrap()
            .evaluate(
                &inputs,
                &[Complex::new(
                    Float::exact_zero_at(bits),
                    Float::exact_zero_at(bits),
                )],
            )
            .unwrap();
        // Parent u=1/4, ray z=c/(2-c)=0 on this face. The source
        // y*z^2+x*z+1 has exact coefficients C0=1, C1=x/2=1/32.
        close(&output[0][0].re, &Rational::one(), bits);
        close(&output[0][1].re, &Rational::from((1, 32)), bits);
        assert!(
            output
                .iter()
                .flatten()
                .all(|x| x.re.valid() && x.im.valid())
        );
    }
}
