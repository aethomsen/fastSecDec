use super::*;
use crate::threshold::maps::{
    normalized::{Limits, affine::cover::AffinePrefixCover},
    regular::SectionSide,
};
use fastsecdec_sectors::DecompositionOptions;
use std::sync::{Arc, atomic::AtomicBool};
use symbolica::domains::float::{ErrorPropagatingFloat as Epf, Float, Real};

fn fixture() -> Arc<AffinePrefixCover> {
    let (first, _) = crate::threshold::maps::normalized::affine::tests::fixture();
    AffinePrefixCover::prepare_next_bound(
        first,
        SectionSide::Upper,
        &DecompositionOptions::default(),
        &mut Budget::new(Limits::default()),
    )
    .unwrap()
}
fn compile(leaf: AffinePrefixLeaf) -> CompiledAffineRestriction {
    CompiledAffineRestriction::prepare(
        leaf,
        OptimizationSettings::default().cores(1),
        &Budget::new(Limits::default()),
    )
    .unwrap()
}

#[test]
fn restriction_native_all_leaves_codec_constant_slots_and_exact_measure_once() {
    let cover = fixture();
    assert_eq!(cover.half_counts().len(), 2);
    assert_eq!(cover.dependent_symbols().len(), 1);
    let mut orientations = std::collections::BTreeSet::new();
    for i in 0..cover.leaves().len() {
        let compiled = compile(cover.leaf(i).unwrap());
        assert_eq!(compiled.exact().get_output_len(), 3);
        assert_eq!(compiled.checked.get_output_len(), 6);
        assert_eq!(
            compiled.inputs(),
            &cover.parent().source().unit_coordinates()[..1]
        );
        assert_eq!(
            compiled.coordinates(),
            &[cover.parent().source().axes()[0].symbol()]
        );
        assert_eq!(compiled.dependent_symbols(), cover.dependent_symbols());
        // The constant stays a native fixed callback, evaluated at map precision.
        assert_eq!(
            compiled
                .checked
                .export_instructions()
                .constant_functions
                .len(),
            1
        );
        let restored = compiled.native_roundtrip().unwrap();
        assert!(Arc::ptr_eq(restored.leaf().cover(), &cover));
        orientations.insert(compiled.leaf().restriction().orientation());
        let restriction = compiled.leaf().restriction();
        let unit = compiled.inputs()[0];
        // Differentiate the issued unit map for an independent chain-rule oracle.
        let reference = Atom::evaluator_multiple(
            &[
                restriction.unit_images()[0].clone(),
                restriction.unit_images()[0].derivative(unit),
            ],
            &[Atom::var(unit)],
        )
        .optimization_settings(OptimizationSettings::default().cores(1))
        .build()
        .unwrap();
        for bits in [128, 256] {
            let a = Float::rational_at(&Rational::from((1, 128)), bits).sqrt();
            let mut expected_map = reference
                .clone()
                .map_coeff_with_prec(&|q| Float::rational_at(&q.re, bits), bits);
            let mut numeric = restored.map::<Float>(bits);
            let mut tracked = restored.map::<Epf<Float>>(bits);
            let tolerance = Rational::from((1, 2)).pow(u64::from(bits - 12));
            for q in [
                Rational::from((1, 4)),
                Rational::from((3, 4)),
                Rational::from((1, 4)),
            ] {
                let point = [Float::rational_at(&q, bits)];
                let mut map = vec![Float::exact_zero_at(bits); 2];
                expected_map.try_evaluate(&point, &mut map).unwrap();
                let mu = if restriction.orientation() == 1 {
                    map[1].clone()
                } else {
                    -map[1].clone()
                };
                let expected = [
                    a.clone() * &map[0] * &map[0],
                    Float::rational_at(&Rational::from(2), bits) * &a * &map[0] * &mu,
                    a.clone(),
                ];
                let values = numeric.evaluate(&point).unwrap();
                for (value, expected) in values.iter().zip(&expected) {
                    assert!((value.to_rational() - expected.to_rational()).abs() < tolerance);
                }
                // A second measure factor must not accidentally disappear in the oracle.
                let wrong = expected[1].clone() * &mu;
                assert!((values[1].to_rational() - wrong.to_rational()).abs() > (1, 10000));
                let values = tracked.evaluate(&point.map(|x| Epf::new(x, 20.))).unwrap();
                assert!(
                    values
                        .iter()
                        .all(|x| x.valid() && x.get_absolute_error() > 0.)
                );
            }
        }
    }
    assert_eq!(
        orientations,
        [-1, 1]
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>()
    );
}

#[test]
fn restriction_runtime_owner_open_inputs_cancellation_and_individual_guards() {
    let cover = fixture();
    let leaf = cover.leaf(0).unwrap();
    assert!(matches!(
        CompiledAffineRestriction::prepare(
            leaf.clone(),
            OptimizationSettings::default(),
            &Budget::new(Limits::default()).with_cancellation(Arc::new(AtomicBool::new(true)))
        ),
        Err(Error::Cancelled)
    ));
    let compiled = compile(leaf);
    let weak = Arc::downgrade(&cover);
    let mut numeric = compiled.map::<f64>(53);
    drop(cover);
    drop(compiled);
    assert!(weak.upgrade().is_some());
    assert_eq!(numeric.evaluate(&[0.5]).unwrap().len(), 3);
    for point in [vec![], vec![0.], vec![1.], vec![f64::NAN], vec![0.5, 0.5]] {
        assert!(numeric.evaluate(&point).is_err());
    }
    // Mutate only private execution scratch: positive visible J cannot mask a
    // negative parent diagonal and refinement measure, or mapped boundary input.
    let input = Atom::var(numeric.leaf().cover().parent().source().unit_coordinates()[0]);
    for values in [[0.1, 1., 0.2, -1., -1., 0.5], [0.1, 1., 0.2, 1., 1., 1.]] {
        numeric.program = Atom::evaluator_multiple(
            &values.map(|x| Atom::num(Rational::try_from(x).unwrap())),
            std::slice::from_ref(&input),
        )
        .build()
        .unwrap()
        .map_coeff(&|q| q.re.to_f64());
        assert!(matches!(numeric.evaluate(&[0.5]), Err(Error::Native(_))));
    }
    drop(numeric);
    assert!(weak.upgrade().is_none());
}
