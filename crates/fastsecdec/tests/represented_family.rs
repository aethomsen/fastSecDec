#![cfg(feature = "threshold-decomposition")]
use fastsecdec::{
    Atom, AtomCore, IntegralFamily, Kinematics,
    input::prepare_family_input,
    parametric::ParametricIntegrand,
    threshold::{
        gcad::{
            GcadRequest, SourceProvenance,
            staging::{self, EvidenceRecord, StagedRequest},
        },
        represented::{
            Error, Limits, NumericalMeaning,
            family::{ExactRepresentedFamilyInput, FamilyLocation, FamilyPoint, Stage},
        },
    },
};
use std::{collections::BTreeMap, ops::ControlFlow, sync::Arc};
use symbolica::{domains::float::Float, parse, symbol};

fn point(bits: u32) -> Arc<FamilyPoint> {
    let (k, p, u) = (
        parse!("family_owner::k"),
        parse!("family_owner::p"),
        parse!("family_owner::unused"),
    );
    let (s, m) = symbol!("family_owner::s", "family_owner::m2");
    let base = Kinematics::in_dimension(&parse!("family_owner::D"))
        .unwrap()
        .with_momenta([k.clone(), p.clone(), u.clone()])
        .unwrap()
        .with_scalar_product(&p, &p, Atom::var(s))
        .unwrap()
        .with_scalar_product(&u, &u, Atom::num(7))
        .unwrap();
    let family = Arc::new(
        IntegralFamily::new(
            vec![k.clone()],
            vec![p.clone()],
            vec![
                base.scalar_product(&k, &k).unwrap() - Atom::var(m),
                base.scalar_product(&(&k + &p), &(&k + &p)).unwrap() - Atom::var(m),
            ],
            &base,
        )
        .unwrap(),
    );
    let kin = base
        .with_scalar_product(&p, &p, Atom::num(Float::with_val(bits, 16)))
        .unwrap()
        .with_scalar_product(&u, &u, Atom::num(Float::with_val(bits, 7)))
        .unwrap();
    let (x, y, eps) = symbol!("family_owner::x", "family_owner::y", "family_owner::eps");
    Arc::new(FamilyPoint {
        family,
        powers: vec![1, 1],
        numerator: Atom::num(Float::with_val(bits, 0.5)),
        measure_multiplier: Atom::num(Float::with_val(bits, 2)),
        kinematics: Some(Arc::new(kin)),
        scalar_values: BTreeMap::from([
            (s, Atom::num(Float::with_val(bits, 16))),
            (m, Atom::num(Float::with_val(bits, 3))),
        ]),
        auxiliary_momenta: vec![],
        coordinates: vec![x, y],
        regulator: eps,
        dimension: Atom::num(4) - Atom::num(2) * Atom::var(eps),
    })
}
fn prepare(point: Arc<FamilyPoint>) -> ExactRepresentedFamilyInput {
    ExactRepresentedFamilyInput::prepare(
        point,
        NumericalMeaning::RepresentedValues,
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap()
}
fn request(point: Arc<FamilyPoint>) -> GcadRequest {
    GcadRequest::preparametric_family_projective(
        Arc::new(prepare(point)),
        1,
        Default::default(),
        GcadRequest::default_limits(),
    )
    .unwrap()
}

#[test]
fn separate_scalar_slots_convert_before_native_family_arithmetic() {
    let original = point(192);
    let owner = prepare(original.clone());
    assert!(Arc::ptr_eq(owner.original(), &original));
    assert_eq!(owner.exact_numerator(), &Atom::num((1, 2)));
    assert_eq!(owner.exact_measure(), &Atom::num(2));
    assert!(
        owner
            .conversions()
            .iter()
            .any(|r| matches!(r.location, FamilyLocation::Numerator {}))
    );
    assert!(
        owner
            .conversions()
            .iter()
            .any(|r| matches!(r.location, FamilyLocation::Measure {}))
    );
    for row in owner.conversions() {
        assert_eq!(row.precision_bits[0], 192);
    }
    let s = symbol!("family_owner::s");
    let m = symbol!("family_owner::m2");
    let values = BTreeMap::from([(s, Atom::num(16)), (m, Atom::num(3))]);
    let (family, powers, numerator) = prepare_family_input(
        &original.family,
        &original.powers,
        Atom::one(),
        None,
        &values,
        &[],
    )
    .unwrap();
    let reference = ParametricIntegrand::from_family(
        &family,
        &powers,
        numerator,
        original.coordinates.clone(),
        original.regulator,
        original.dimension.clone(),
    )
    .unwrap();
    assert!(
        (owner.exact().density() - reference.density())
            .expand()
            .together()
            .cancel()
            .is_zero()
    );
    let p = parse!("family_owner::p");
    assert_eq!(
        original.family.kinematics().scalar_product(&p, &p).unwrap(),
        Atom::var(s)
    );
    assert_ne!(
        original
            .kinematics
            .as_ref()
            .unwrap()
            .scalar_product(&p, &p)
            .unwrap(),
        Atom::num(16)
    );
}

#[test]
fn full_original_context_and_precision_change_identity_without_changing_density() {
    let a = prepare(point(53));
    let b = prepare(point(192));
    assert_eq!(a.exact(), b.exact());
    assert_ne!(a.source_identity(), b.source_identity());
    let mut changed = (*point(53)).clone();
    let u = parse!("family_owner::unused");
    changed.kinematics = Some(Arc::new(
        changed
            .kinematics
            .as_ref()
            .unwrap()
            .as_ref()
            .clone()
            .with_scalar_product(&u, &u, Atom::num(9.0))
            .unwrap(),
    ));
    let c = prepare(Arc::new(changed));
    assert_eq!(a.exact(), c.exact());
    assert_ne!(a.source_identity(), c.source_identity());
    assert!(
        a.conversions()
            .iter()
            .filter_map(
                |r| if let FamilyLocation::KinematicValue { keys } = &r.location {
                    Some(keys)
                } else {
                    None
                }
            )
            .all(|keys| keys.len() == 2)
    );
}

#[test]
fn refusals_preserve_original_native_owner_and_control_bounds() {
    let original = point(53);
    assert!(matches!(
        ExactRepresentedFamilyInput::prepare(
            original.clone(),
            NumericalMeaning::UncertaintyBounds,
            Limits::default(),
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(matches!(
        ExactRepresentedFamilyInput::prepare(
            original.clone(),
            NumericalMeaning::RepresentedValues,
            Limits {
                converted_literals: 0,
                ..Limits::default()
            },
            |_| ControlFlow::Continue(())
        ),
        Err(Error::ResourceIncomplete(_))
    ));
    assert!(matches!(
        ExactRepresentedFamilyInput::prepare(
            original.clone(),
            NumericalMeaning::RepresentedValues,
            Limits::default(),
            |p| if p.stage == Stage::Numerator {
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        ),
        Err(Error::Cancelled)
    ));
    let mut bad = (*original).clone();
    let p = parse!("family_owner::p");
    bad.kinematics = Some(Arc::new(
        bad.kinematics
            .as_ref()
            .unwrap()
            .as_ref()
            .clone()
            .with_scalar_product(&p, &p, Atom::num(17.0))
            .unwrap(),
    ));
    assert!(
        matches!(ExactRepresentedFamilyInput::prepare(Arc::new(bad),NumericalMeaning::RepresentedValues,Limits::default(), |_|ControlFlow::Continue(())),Err(Error::Invalid(message)) if message.contains("cannot change existing family external products"))
    );
    assert_eq!(prepare(original).exact_scalar_values().len(), 2);
}

#[test]
fn native_original_family_replay_reverifies_without_solving() {
    let dir = tempfile::tempdir().unwrap();
    let source = point(53);
    let request = request(source.clone());
    assert!(matches!(
        request.source_provenance(),
        SourceProvenance::PreparametricFamily(_)
    ));
    assert!(request.preparametric_graph_input().is_none() && request.represented_input().is_none());
    let raw = request.solve().unwrap();
    let staged = StagedRequest::write(dir.path(), Arc::new(request)).unwrap();
    let evidence = staged.write_evidence(dir.path(), &raw).unwrap();
    assert!(staged.request().verify(raw).is_ok());
    assert!(matches!(
        StagedRequest::read(dir.path(), staged.receipt(), 10_000_000),
        Err(staging::Error::OriginalFamilyRequired)
    ));
    let restored = StagedRequest::read_with_original_family(
        dir.path(),
        staged.receipt(),
        10_000_000,
        source,
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(restored.request().identity(), staged.request().identity());
    assert!(
        restored
            .verify_evidence(dir.path(), &evidence, 10_000_000)
            .unwrap()
            .result
            .is_ok()
    );
    assert!(
        StagedRequest::read_with_original_family(
            dir.path(),
            staged.receipt(),
            10_000_000,
            point(192),
            Limits::default(),
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(
        StagedRequest::read_with_original_family(
            dir.path(),
            staged.receipt(),
            10_000_000,
            point(53),
            Limits {
                converted_literals: 0,
                ..Limits::default()
            },
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    std::fs::write(
        dir.path().join("evidence.json"),
        serde_json::to_vec(&evidence).unwrap(),
    )
    .unwrap();
    drop(restored);
    drop(staged);
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "cold_family_replay_child",
            "--ignored",
            "--nocapture",
        ])
        .env("FASTSECDEC_FAMILY_REPLAY_TEST", dir.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dir.path().join("child-verified").exists());
}

#[test]
#[ignore = "invoked as a fresh-process native original-family replay child"]
fn cold_family_replay_child() {
    let dir = std::path::PathBuf::from(std::env::var_os("FASTSECDEC_FAMILY_REPLAY_TEST").unwrap());
    let evidence: EvidenceRecord =
        serde_json::from_slice(&std::fs::read(dir.join("evidence.json")).unwrap()).unwrap();
    let staged = StagedRequest::read_with_original_family(
        &dir,
        &evidence.request,
        10_000_000,
        point(53),
        Limits::default(),
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    // No solve call: original owner and recorded native evidence are mandatory.
    let verified = staged
        .verify_evidence(&dir, &evidence, 10_000_000)
        .unwrap()
        .result
        .unwrap();
    assert!(verified.cells().len() > 0);
    std::fs::write(dir.join("child-verified"), b"verified without solve").unwrap();
}

fn options() -> fastsecdec::threshold::generation::PreparationOptions {
    let mut o =
        fastsecdec::threshold::generation::PreparationOptions::new(symbol!("family_owner::unit"));
    o.represented = Some((NumericalMeaning::RepresentedValues, Limits::default()));
    o
}
fn compile(
    root: &std::path::Path,
    receipt: &fastsecdec::threshold::generation::PreparationReceipt,
) -> fastsecdec::kernel::KernelSet {
    use fastsecdec::kernel::{KernelLoadOptions, KernelSet, ThresholdPublicationPlan};
    let work_root = root.join(&receipt.work_directory);
    let plan = ThresholdPublicationPlan::from_trusted_preparer(
        &work_root,
        receipt.publication.clone(),
        &receipt.publication.source_identity,
        &receipt.publication.prepared_identity,
        32 * 1024 * 1024,
    )
    .unwrap();
    let mut archive = plan
        .archive_writer(std::io::Cursor::new(Vec::new()))
        .unwrap();
    for index in (0..plan.job_count()).rev() {
        let mut bytes = Vec::new();
        let receipt = plan
            .work(index)
            .unwrap()
            .compile_record(&work_root, 32 * 1024 * 1024, &mut bytes)
            .unwrap();
        plan.append_work_record(&mut archive, &mut bytes.as_slice(), receipt)
            .unwrap();
    }
    let (bytes, _) = archive.finish().unwrap();
    KernelSet::from_bytes_with_options(&bytes.into_inner(), KernelLoadOptions { validate: true })
        .unwrap()
}
#[test]
fn public_family_preparation_publication_and_resume_keep_complete_vectors() {
    use fastsecdec::threshold::generation as g;
    let dir = tempfile::tempdir().unwrap();
    let original = point(53);
    let mut details = Vec::new();
    let first = g::prepare_family(original.clone(), options(), dir.path(), |p| {
        if let Some(detail) = p.family_input {
            details.push(detail.stage);
        }
        assert!(p.graph_input.is_none());
        ControlFlow::Continue(())
    })
    .unwrap();
    assert!(details.contains(&Stage::Parameterization));
    let source = &first.receipt.publication.source_identity;
    let identity = &first.receipt.publication.prepared_identity;
    let mut stages = Vec::new();
    let resumed = g::resume_family(
        original,
        Limits::default(),
        dir.path(),
        &first.receipt,
        "replay",
        source,
        identity,
        32 * 1024 * 1024,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&g::Stage::Solve));
    assert!(stages.contains(&g::Stage::Verify));
    assert_eq!(identity, &resumed.receipt.publication.prepared_identity);
    let mut a = compile(dir.path(), &first.receipt);
    let mut b = compile(dir.path(), &resumed.receipt);
    assert_eq!(a.content_id(), b.content_id());
    assert_eq!(a.exact_coefficients(), b.exact_coefficients());
    assert_eq!(a.sectors().len(), b.sectors().len());
    for x in [0.125, 0.43, 0.875] {
        for (a, b) in a.sectors_mut().iter_mut().zip(b.sectors_mut()) {
            let mut av = vec![0.; a.output_count()];
            let mut bv = vec![0.; b.output_count()];
            a.evaluate(&[x], &mut av).unwrap();
            b.evaluate(&[x], &mut bv).unwrap();
            assert_eq!(av, bv);
        }
    }
    assert!(matches!(
        g::resume(
            dir.path(),
            &first.receipt,
            "no-original",
            source,
            identity,
            32 * 1024 * 1024,
            |_| ControlFlow::Continue(())
        ),
        Err(g::Error::Evidence(staging::Error::OriginalFamilyRequired))
    ));
}
#[test]
fn family_raw_checkpoint_cancellation_reverifies_and_refuses_foreign_policy() {
    use fastsecdec::threshold::generation as g;
    let dir = tempfile::tempdir().unwrap();
    let mut checkpoint = None;
    assert!(matches!(
        g::prepare_family(point(53), options(), dir.path(), |p| {
            if let Some(c) = p.checkpoint() {
                checkpoint = Some(c);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }),
        Err(g::Error::Cancelled)
    ));
    let checkpoint = checkpoint.unwrap();
    let mut stages = Vec::new();
    g::resume_family_evidence(
        point(53),
        Limits::default(),
        dir.path(),
        &checkpoint,
        "recovered",
        &checkpoint.evidence.request.source_identity,
        32 * 1024 * 1024,
        |p| {
            stages.push(p.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&g::Stage::Solve));
    assert!(stages.contains(&g::Stage::Verify));
    let other = tempfile::tempdir().unwrap();
    let mut changed = options();
    changed.represented.as_mut().unwrap().1.converted_literals += 1;
    let mut other_checkpoint = None;
    assert!(matches!(
        g::prepare_family(point(53), changed, other.path(), |p| {
            if let Some(c) = p.checkpoint() {
                other_checkpoint = Some(c);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        }),
        Err(g::Error::Cancelled)
    ));
    let other_configuration = other_checkpoint.unwrap().configuration;
    let configuration_path = other_configuration.resolve(other.path()).unwrap();
    let destination = other_configuration.resolve(dir.path()).unwrap();
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::copy(configuration_path, destination).unwrap();
    let wrong = g::PreparationCheckpoint {
        configuration: other_configuration,
        ..checkpoint
    };
    assert!(matches!(
        g::resume_family_evidence(
            point(53),
            Limits::default(),
            dir.path(),
            &wrong,
            "foreign",
            &wrong.evidence.request.source_identity,
            32 * 1024 * 1024,
            |_| ControlFlow::Continue(())
        ),
        Err(g::Error::Association(
            "family conversion policy/configuration"
        ))
    ));
}

#[test]
fn native_signed_projection_and_consumed_context_fences_remain_explicit() {
    let original = point(53);
    for powers in [vec![0, 1], vec![1, -1]] {
        let mut input = (*original).clone();
        input.powers = powers.clone();
        input.coordinates.truncate(1);
        let represented = prepare(Arc::new(input.clone()));
        let (family, positive, numerator) = prepare_family_input(
            &input.family,
            &powers,
            represented.exact_numerator() * represented.exact_measure(),
            Some(represented.exact_kinematics()),
            represented.exact_scalar_values(),
            &[],
        )
        .unwrap();
        let exact = ParametricIntegrand::from_family(
            &family,
            &positive,
            numerator,
            input.coordinates,
            input.regulator,
            input.dimension,
        )
        .unwrap();
        assert_eq!(represented.exact().as_ref(), &exact);
    }
    let mut input = (*original).clone();
    let u = parse!("family_owner::unused");
    let kin = input
        .family
        .kinematics()
        .clone()
        .with_scalar_product(&u, &u, Atom::num(7.0))
        .unwrap();
    input.family = Arc::new(
        IntegralFamily::new(
            input.family.loop_momenta().to_vec(),
            input.family.external_momenta().to_vec(),
            input.family.denominators().to_vec(),
            &kin,
        )
        .unwrap(),
    );
    assert!(
        matches!(ExactRepresentedFamilyInput::prepare(Arc::new(input),NumericalMeaning::RepresentedValues,Limits::default(), |_|ControlFlow::Continue(())),Err(Error::Unsupported(message)) if message.contains("stored family"))
    );
}
