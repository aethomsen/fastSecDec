//! Native graph -> issued projective uniform pair -> one compiled template.
use super::*;
use crate::{
    generation::{GenerationMode, GenerationOptions},
    input::GraphIntegral,
    parametric::ParametricIntegrand,
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions},
        maps::regular::BracketProposal,
        projective::AffineProjectivePreparation,
        regularization::secant::{Limits, NormalizedFamily},
    },
};
use std::{collections::BTreeMap, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore},
    domains::rational::Rational,
    symbol,
};

fn family() -> (NormalizedFamily, Symbol) {
    let (s, x, y, e, u) = symbol!(
        "compiled_projective::s",
        "compiled_projective::x",
        "compiled_projective::y",
        "compiled_projective::eps",
        "compiled_projective::u"
    );
    let model = Arc::new(
        crate::Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let kin = crate::Kinematics::in_dimension(&symbolica::parse!("D"))
        .unwrap()
        .with_mass_squared(
            &feynkit_graph::symbols::external_momentum().call(1),
            Atom::var(s),
        )
        .unwrap();
    let graph=GraphIntegral::from_dot(model,include_str!(concat!(env!("CARGO_MANIFEST_DIR"),"/../../examples/graphs/bubble.dot")),&kin).unwrap()
        .with_scalar_values(&BTreeMap::from([(symbol!("UFO::mt"),Atom::num(3).pow(Atom::num((1,2))))])).unwrap()
        .with_measure_multiplier(symbolica::parse!("gamma(1-2*compiled_projective::eps)/(gamma(1+compiled_projective::eps)*gamma(1-compiled_projective::eps)^2)"));
    let input = ParametricIntegrand::from_graph(
        &graph,
        vec![x, y],
        e,
        Atom::num(4) - Atom::num(2) * Atom::var(e),
    )
    .unwrap();
    let preparation = AffineProjectivePreparation::last_coordinate(&input).unwrap();
    assert_eq!(
        preparation.images(),
        [Atom::var(x), Atom::one() - Atom::var(x)]
    );
    assert_eq!(preparation.measure_jacobian(), &Atom::one());
    let owner = Arc::new(
        GcadRequest::projective(
            Arc::new(preparation),
            GcadKinematics {
                runtime_parameters: vec![s],
                strict_positive: vec![Atom::var(s) - Atom::num(15), Atom::num(18) - Atom::var(s)],
                ..Default::default()
            },
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
    let family = NormalizedFamily::admit_parameter_pair_affine_shifted(
        owner,
        vec![u],
        [
            BracketProposal {
                lower: Rational::from((1, 8)),
                upper: Rational::from((3, 8)),
                derivative_margin: Rational::one(),
            },
            BracketProposal {
                lower: Rational::from((5, 8)),
                upper: Rational::from((7, 8)),
                derivative_margin: Rational::one(),
            },
        ],
        Limits {
            unit_margin: Rational::from((1, 8)),
            ..Default::default()
        },
    )
    .unwrap();
    (family, s)
}
fn options() -> GenerationOptions {
    GenerationOptions {
        mode: GenerationMode::Symbolic,
        max_order: 1,
        ..Default::default()
    }
}
fn point(set: &mut KernelSet) -> Vec<f64> {
    let mut result = set.exact_coefficients().to_vec();
    for sector in set.sectors_mut() {
        let mut row = vec![0.; result.len()];
        let report = sector.replay_scaled(&[0.37], &mut row, 1., 128).unwrap();
        assert!(report.checked && report.bits >= 256);
        for (a, b) in result.iter_mut().zip(row) {
            *a += b;
        }
    }
    assert!(result.iter().all(|v| v.is_finite()));
    result
}
fn programs(set: &KernelSet) -> Vec<Vec<u8>> {
    set.sectors()
        .iter()
        .map(|s| program::encode(s.exact_program()).unwrap())
        .collect()
}
#[test]
fn original_prepared_raw_and_sample_roles_survive_native_compile_and_bind() {
    let (family, s) = family();
    let source = family.owner().request();
    assert_eq!(source.input().parameters().len(), 2);
    assert_eq!(source.domain().coordinates().len(), 1);
    let original = source.input().parameters().to_vec();
    let native_patch = source.projective_preparation().unwrap().clone();
    let continued = family.continue_symbolically(&options()).unwrap();
    let staging = tempfile::tempdir().unwrap();
    let mut set = KernelSet::compile_threshold_family(
        &continued,
        staging.path(),
        1,
        PrecisionPolicy::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap();
    let metadata = set.threshold_metadata().unwrap();
    let lineage = metadata.lineage();
    assert_eq!(lineage.preparation.original_coordinates.len(), 2);
    assert_eq!(
        lineage
            .preparation
            .runtime_chamber
            .as_ref()
            .unwrap()
            .integration_axes,
        [1]
    );
    assert_eq!(lineage.cells.len(), 3);
    assert_eq!(lineage.endpoint_charts.len(), 4);
    let patch = lineage.patches[0].map.geometry();
    assert_eq!(patch.coordinates.len(), 1);
    assert_eq!(patch.images.len(), 2);
    assert_eq!(
        metadata.native_symbol(patch.coordinates[0]).unwrap(),
        original[0]
    );
    for (id, expected) in patch.images.iter().zip(native_patch.images()) {
        assert_eq!(
            metadata.native_atom(&patch.expressions, *id).unwrap(),
            expected
        );
    }
    assert_eq!(
        metadata
            .native_atom(&patch.expressions, patch.positive_measure)
            .unwrap(),
        native_patch.measure_jacobian()
    );
    assert!(
        lineage
            .cells
            .iter()
            .all(|c| c.coordinate_order == patch.coordinates)
    );
    assert!(
        lineage
            .endpoint_charts
            .iter()
            .all(|c| c.map.geometry().coordinates.len() == 1)
    );
    assert!(metadata.full_original_scope().unwrap());
    check_manifest_mutations(metadata);
    drop(continued);
    drop(family);
    exercise(&mut set, s);
}
fn exercise(set: &mut KernelSet, s: Symbol) {
    let template = set.template_content_id().to_owned();
    let ir = programs(set);
    let mut first = None;
    for value in [16., 17., 16.] {
        set.bind_parameters(&BTreeMap::from([(s, value)])).unwrap();
        let result = point(set);
        assert_eq!(set.orders(), [-1, -1, 0, 0, 1, 1]);
        assert!((result[0] - 1.).abs() < 1e-11 && result[1].abs() < 1e-11);
        assert!((result[3] - std::f64::consts::PI * (1. - 12. / value).sqrt()).abs() < 1e-11);
        assert_eq!(set.template_content_id(), template);
        assert_eq!(programs(set), ir);
        if value == 16. {
            if let Some((id, row)) = &first {
                assert_eq!(set.content_id(), id);
                assert_eq!(&result, row);
            } else {
                first = Some((set.content_id().to_owned(), result.clone()));
            }
        }
        for invalid in [
            BTreeMap::new(),
            BTreeMap::from([(s, 12.)]),
            BTreeMap::from([(s, 15.)]),
            BTreeMap::from([(s, 18.)]),
            BTreeMap::from([(s, f64::NAN)]),
        ] {
            let id = set.content_id().to_owned();
            assert!(set.bind_parameters(&invalid).is_err());
            assert_eq!(set.content_id(), id);
            assert_eq!(point(set), result);
            assert_eq!(programs(set), ir);
        }
    }
}

fn check_manifest_mutations(metadata: &ThresholdMetadata) {
    use super::threshold_metadata as m;
    let tables = metadata.tables();
    for mutation in 0..8 {
        let mut lineage = metadata.lineage().clone();
        match mutation {
            0 => {
                lineage.preparation.original_coordinates.pop();
            }
            1 => {
                if let m::MapDescriptor::RationalV1 { geometry } = &mut lineage.patches[0].map {
                    geometry
                        .coordinates
                        .push(lineage.preparation.original_coordinates[1]);
                }
            }
            2 => {
                lineage.cells[0].coordinate_order[0] = lineage.preparation.original_coordinates[1];
            }
            3 => {
                lineage
                    .preparation
                    .runtime_chamber
                    .as_mut()
                    .unwrap()
                    .integration_axes = vec![0];
            }
            4 => {
                if let m::MapDescriptor::RationalV1 { geometry } = &mut lineage.patches[0].map {
                    geometry.images.pop();
                }
            }
            5 => {
                if let m::MapDescriptor::RationalV1 { geometry } = &mut lineage.patches[0].map {
                    geometry.positive_measure = m::NativeAtomId(usize::MAX);
                }
            }
            6 => {
                if let m::MapDescriptor::RationalV1 { geometry } = &mut lineage.patches[0].map {
                    geometry.expressions = m::Digest("0".repeat(64));
                }
            }
            _ => lineage.patches.push(lineage.patches[0].clone()),
        }
        assert!(
            lineage
                .validate_structure(&tables, m::Limits::default())
                .is_err(),
            "projective mutation {mutation}"
        );
    }
}
mod artifacts;
mod science;
mod transport;

mod fibers;

mod catalogue_point;
