use super::*;
use crate::parametric::{
    FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
    PolynomialFactor,
};
use symbolica::{atom::Atom, parse, symbol};

const MAXIMUM: u64 = 16 * 1024 * 1024;

fn source() -> Arc<PositiveProjectiveRequest> {
    let input = ParametricIntegrand::new(
        vec![
            symbol!("positive_staging::x"),
            symbol!("positive_staging::y"),
        ],
        symbol!("positive_staging::eps"),
        ParametricDomain::ProjectiveSimplex,
        vec![ParametricTerm::new(
            parse!("1+I"),
            vec![Atom::zero(); 2],
            vec![
                PolynomialFactor::new(
                    parse!("positive_staging::x-positive_staging::y"),
                    Atom::num(-2),
                    FactorRole::Singularity,
                )
                .with_semantics(FactorSemantics::Causal),
            ],
        )],
    )
    .unwrap();
    Arc::new(
        PositiveProjectiveRequest::exact(Arc::new(input), 1, Default::default(), limits()).unwrap(),
    )
}

fn limits() -> symgcad::Limits {
    symgcad::Limits {
        workers: 1,
        wall_time_secs: 10.0,
        memory_mib: 1024,
        ..Default::default()
    }
}

#[test]
fn exact_source_and_raw_evidence_survive_owner_release_and_fresh_replay() {
    let dir = tempfile::tempdir().unwrap();
    let request = source();
    let weak = Arc::downgrade(&request);
    let raw = request.solve().unwrap();
    let original = serde_json::to_vec(&raw).unwrap();
    let staged = StagedPositiveRequest::write(dir.path(), request.clone()).unwrap();
    assert!(Arc::ptr_eq(staged.request_owner(), &request));
    let repeated = StagedPositiveRequest::write(dir.path(), request.clone()).unwrap();
    assert_eq!(staged.receipt(), repeated.receipt());
    let evidence = staged.write_evidence(dir.path(), &raw).unwrap();
    assert_eq!(evidence.cells, 2);
    let receipt = staged.receipt().clone();
    drop((staged, repeated, request, raw));
    assert!(weak.upgrade().is_none());
    let restored = StagedPositiveRequest::read(dir.path(), &receipt, MAXIMUM).unwrap();
    let raw = restored
        .read_evidence(dir.path(), &evidence, MAXIMUM)
        .unwrap();
    assert_eq!(serde_json::to_vec(&*raw).unwrap(), original);
    assert_eq!(
        restored.request().preparation().input().terms()[0].prefactor(),
        &parse!("1+I")
    );
    let result = restored
        .verify_evidence(dir.path(), &evidence, MAXIMUM)
        .unwrap();
    assert_eq!(result.result.unwrap().cell_count(), 2);
    assert!(matches!(
        result.observation.read(dir.path(), MAXIMUM).unwrap(),
        VerificationObservation::Verified { cells: 2, .. }
    ));
}

#[test]
fn incomplete_and_tampered_proofs_stay_untrusted_and_keep_raw_records() {
    let dir = tempfile::tempdir().unwrap();
    let request = source();
    let mut raw = request.solve().unwrap();
    let staged = StagedPositiveRequest::write(dir.path(), request).unwrap();
    raw.status = Status::Incomplete;
    let record = staged.write_evidence(dir.path(), &raw).unwrap();
    let result = staged
        .verify_evidence(dir.path(), &record, MAXIMUM)
        .unwrap();
    assert!(matches!(
        result.result,
        Err(GcadError::Incomplete(Status::Incomplete))
    ));
    assert!(matches!(
        result.observation.read(dir.path(), MAXIMUM).unwrap(),
        VerificationObservation::Incomplete {
            status: Status::Incomplete
        }
    ));
    record.record.verify(dir.path()).unwrap();
    raw.status = Status::CompleteGeneric;
    raw.homogeneous.as_mut().unwrap().reduced.cells.pop();
    let record = staged.write_evidence(dir.path(), &raw).unwrap();
    let result = staged
        .verify_evidence(dir.path(), &record, MAXIMUM)
        .unwrap();
    assert!(result.result.is_err());
    assert!(matches!(
        result.observation.read(dir.path(), MAXIMUM).unwrap(),
        VerificationObservation::Rejected { .. }
    ));
    record.record.verify(dir.path()).unwrap();
}

#[test]
fn association_and_transport_caps_precede_import_and_verification() {
    let dir = tempfile::tempdir().unwrap();
    let request = source();
    let mut raw = request.solve().unwrap();
    let staged = StagedPositiveRequest::write(dir.path(), request).unwrap();
    let receipt = staged.write_evidence(dir.path(), &raw).unwrap();
    assert!(matches!(
        StagedPositiveRequest::read(dir.path(), staged.receipt(), 0),
        Err(Error::RecordLimit)
    ));
    assert!(matches!(
        staged.read_evidence(dir.path(), &receipt, staged.receipt().record.bytes),
        Err(Error::RecordLimit)
    ));
    let mut wrong = receipt.clone();
    wrong.cells += 1;
    assert!(matches!(
        staged.read_evidence(dir.path(), &wrong, MAXIMUM),
        Err(Error::Association)
    ));
    wrong = receipt.clone();
    wrong.request.source_identity.push('x');
    assert!(matches!(
        staged.read_evidence(dir.path(), &wrong, MAXIMUM),
        Err(Error::Association)
    ));
    raw.problem.limits.wall_time_secs = 9.0;
    assert!(matches!(
        staged.write_evidence(dir.path(), &raw),
        Err(Error::Association)
    ));
    // The original artifact remains usable after each rejected association.
    staged
        .verify_evidence(dir.path(), &receipt, MAXIMUM)
        .unwrap()
        .result
        .unwrap();
    let path = receipt.record.resolve(dir.path()).unwrap();
    std::fs::write(path, b"truncated native record").unwrap();
    assert!(staged.read_evidence(dir.path(), &receipt, MAXIMUM).is_err());
}

fn graph_point(measure: f64) -> Arc<GraphPoint> {
    use crate::{FeynmanDiagram, Kinematics, Model};
    use std::collections::BTreeMap;
    let model = Arc::new(
        Model::from_json(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/models/scalar.json"
        )))
        .unwrap(),
    );
    let diagram = Arc::new(
        FeynmanDiagram::from_dot(
            model,
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../examples/graphs/bubble.dot"
            )),
        )
        .unwrap(),
    );
    let p = feynkit_graph::symbols::external_momentum().call(1);
    Arc::new(GraphPoint {
        diagram,
        kinematics: Arc::new(
            Kinematics::in_dimension(&parse!("D"))
                .unwrap()
                .with_mass_squared(&p, Atom::num(12.0))
                .unwrap(),
        ),
        scalar_values: BTreeMap::from([(symbol!("UFO::mt"), Atom::num(1.5))]),
        auxiliary_momenta: vec![],
        powers: BTreeMap::new(),
        measure_multiplier: Atom::num(measure),
        coordinates: vec![
            symbol!("positive_staging::gx"),
            symbol!("positive_staging::gy"),
        ],
        regulator: symbol!("positive_staging::geps"),
        dimension: parse!("4-2*positive_staging::geps"),
    })
}

#[test]
fn cold_graph_replay_requires_actual_point_and_checks_representation_policy() {
    let dir = tempfile::tempdir().unwrap();
    let point = graph_point(0.5);
    let graph = ExactRepresentedGraphInput::prepare(
        point.clone(),
        represented::NumericalMeaning::RepresentedValues,
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let request = Arc::new(
        PositiveProjectiveRequest::graph(Arc::new(graph), 1, Default::default(), limits()).unwrap(),
    );
    let raw = request.solve().unwrap();
    let staged = StagedPositiveRequest::write(dir.path(), request).unwrap();
    let receipt = staged.receipt().clone();
    let evidence = staged.write_evidence(dir.path(), &raw).unwrap();
    drop((staged, raw));
    assert!(matches!(
        StagedPositiveRequest::read(dir.path(), &receipt, MAXIMUM),
        Err(Error::OriginalGraphRequired)
    ));
    assert!(
        StagedPositiveRequest::read_with_original_graph(
            dir.path(),
            &receipt,
            MAXIMUM,
            graph_point(0.75),
            Default::default(),
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    let small_cap = represented::Limits {
        nodes: 1,
        ..Default::default()
    };
    assert!(
        StagedPositiveRequest::read_with_original_graph(
            dir.path(),
            &receipt,
            MAXIMUM,
            point.clone(),
            small_cap,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    let restored = StagedPositiveRequest::read_with_original_graph(
        dir.path(),
        &receipt,
        MAXIMUM,
        point.clone(),
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    let SourceProvenance::PreparametricGraph(rebuilt) = restored.request().provenance() else {
        panic!("graph owner lost");
    };
    assert!(Arc::ptr_eq(rebuilt.original(), &point));
    assert_eq!(
        restored
            .verify_evidence(dir.path(), &evidence, MAXIMUM)
            .unwrap()
            .result
            .unwrap()
            .cell_count(),
        3
    );
    std::fs::write(
        dir.path().join("receipts.json"),
        serde_json::to_vec(&(receipt, evidence)).unwrap(),
    )
    .unwrap();
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "threshold::gcad::staging::positive::tests::fresh_process_graph_child",
            "--ignored",
            "--test-threads=1",
        ])
        .env("FSD_POSITIVE_STAGING_CHILD", dir.path())
        .status()
        .unwrap();
    assert!(status.success());
}

#[test]
#[ignore = "invoked by the cold graph parent with its saved record directory"]
fn fresh_process_graph_child() {
    let dir = std::path::PathBuf::from(std::env::var_os("FSD_POSITIVE_STAGING_CHILD").unwrap());
    let (receipt, evidence): (RequestRecord, EvidenceRecord) =
        serde_json::from_slice(&std::fs::read(dir.join("receipts.json")).unwrap()).unwrap();
    let staged = StagedPositiveRequest::read_with_original_graph(
        &dir,
        &receipt,
        MAXIMUM,
        graph_point(0.5),
        Default::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(
        staged
            .verify_evidence(&dir, &evidence, MAXIMUM)
            .unwrap()
            .result
            .unwrap()
            .cell_count(),
        3
    );
}
