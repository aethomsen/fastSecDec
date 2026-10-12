use super as retry;
use super::super::tests::{analytic, compile, projective_tests};
use super::*;
use std::fs;
use symbolica::symbol;
const CAP: u64 = 64 * 1024 * 1024;
fn snapshots(root: &Path, checkpoint: &PreparationCheckpoint) -> Vec<(String, Vec<u8>)> {
    records::references(checkpoint)
        .unwrap()
        .iter()
        .map(|r| (r.path.clone(), fs::read(r.resolve(root).unwrap()).unwrap()))
        .collect()
}
fn unchanged(root: &Path, before: &[(String, Vec<u8>)]) {
    for (path, bytes) in before {
        assert_eq!(&fs::read(root.join(path)).unwrap(), bytes);
    }
}
fn interrupted(root: &Path) -> PreparationCheckpoint {
    let (input, options) = analytic();
    let mut saved = None;
    let result = prepare(input, options, root, |p| {
        if let Some(checkpoint) = p.checkpoint() {
            saved = Some(checkpoint);
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    });
    assert!(matches!(result, Err(Error::Cancelled)));
    saved.unwrap()
}
#[test]
fn retry_legacy_incomplete_and_completed_leave_original_immutable() {
    let base = tempfile::tempdir().unwrap();
    let old = base.path().join("old");
    fs::create_dir(&old).unwrap();
    let checkpoint = interrupted(&old);
    let before = snapshots(&old, &checkpoint);
    let mut overlay = EndpointPreparationResources {
        max_charts: Some(777),
        ..Default::default()
    };
    overlay.geometry.max_degree = Some(48);
    let target = base.path().join("retry");
    let mut stages = Vec::new();
    let result = retry_endpoint_evidence(
        &old,
        &checkpoint,
        &checkpoint.evidence.request.source_identity,
        &overlay,
        &target,
        CAP,
        |p| {
            stages.push(p.preparation.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    let (options, route) =
        configuration::read(&target, &result.checkpoint.preparation.configuration, CAP).unwrap();
    assert_eq!(route, regular::Route::LegacyFiber);
    assert!(options.regular_family.is_none());
    assert_eq!(options.regularization.max_charts, 777);
    assert_eq!(options.regularization.max_degree, 48);
    unchanged(&old, &before);
    let receipt = result.prepared.receipt;
    let completed_checkpoint = receipt.checkpoint().unwrap();
    let completed_before = snapshots(&target, &completed_checkpoint);
    let new = base.path().join("completed-retry");
    let second = retry_endpoint_evidence(
        &target,
        &completed_checkpoint,
        &receipt.publication.source_identity,
        &overlay,
        &new,
        CAP,
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    unchanged(&target, &completed_before);
    let mut first = compile(&target, &receipt);
    let mut later = compile(&new, &second.prepared.receipt);
    assert_eq!(first.content_id(), later.content_id());
    for point in [0.13, 0.51, 0.89] {
        for (a, b) in first.sectors_mut().iter_mut().zip(later.sectors_mut()) {
            let mut x = vec![0.; a.output_count()];
            let mut y = vec![0.; b.output_count()];
            a.evaluate(&[point], &mut x).unwrap();
            b.evaluate(&[point], &mut y).unwrap();
            assert_eq!(x, y);
        }
    }
    assert!(
        resume_endpoint_retry(
            &new,
            &second.checkpoint,
            "wrong",
            &receipt.publication.source_identity,
            Some("wrong-prepared"),
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
}
#[test]
fn retry_checkpoint_precedes_verify_and_cold_resume_keeps_effective_policy() {
    let base = tempfile::tempdir().unwrap();
    let old = base.path().join("old");
    fs::create_dir(&old).unwrap();
    let checkpoint = interrupted(&old);
    let target = base.path().join("retry");
    let overlay = EndpointPreparationResources {
        max_charts: Some(555),
        ..Default::default()
    };
    let result = retry_endpoint_evidence(
        &old,
        &checkpoint,
        &checkpoint.evidence.request.source_identity,
        &overlay,
        &target,
        CAP,
        |p| {
            if p.checkpoint.is_some() {
                assert_eq!(p.preparation.stage, Stage::Verify);
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        },
    );
    assert!(matches!(result, Err(Error::Cancelled)));
    assert!(target.join("endpoint-retry-checkpoint.json").is_file());
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "threshold::generation::retry::tests::retry_cold_child",
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("FSD_ENDPOINT_RETRY_CHILD", &target)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
}
#[test]
#[ignore = "invoked by retry parent with retained native records"]
fn retry_cold_child() {
    let Some(path) = std::env::var_os("FSD_ENDPOINT_RETRY_CHILD") else {
        return;
    };
    let root = std::path::PathBuf::from(path);
    for n in 0..31 {
        let _ = symbol!(format!("retry_cold_noise::s{n}"));
    }
    let checkpoint: EndpointRetryCheckpoint =
        serde_json::from_slice(&fs::read(root.join("endpoint-retry-checkpoint.json")).unwrap())
            .unwrap();
    let mut stages = Vec::new();
    let result = resume_endpoint_retry(
        &root,
        &checkpoint,
        "cold",
        &checkpoint.preparation.evidence.request.source_identity,
        None,
        CAP,
        |p| {
            stages.push(p.preparation.stage);
            ControlFlow::Continue(())
        },
    )
    .unwrap();
    assert!(!stages.contains(&Stage::Solve));
    assert!(stages.contains(&Stage::Verify));
    let (options, _) =
        configuration::read(&root, &result.prepared.receipt.configuration, CAP).unwrap();
    assert_eq!(options.regularization.max_charts, 555);
    assert!(
        compile(&root, &result.prepared.receipt)
            .threshold_metadata()
            .is_some()
    );
}
#[test]
fn retry_refuses_changed_identity_limits_legacy_policy_and_paths() {
    let base = tempfile::tempdir().unwrap();
    let old = base.path().join("old");
    fs::create_dir(&old).unwrap();
    let checkpoint = interrupted(&old);
    let before = snapshots(&old, &checkpoint);
    let target = base.path().join("new");
    let resources = EndpointPreparationResources::default();
    for (source, cap) in [
        ("wrong-source", CAP),
        (checkpoint.evidence.request.source_identity.as_str(), 1),
    ] {
        assert!(
            retry_endpoint_evidence(&old, &checkpoint, source, &resources, &target, cap, |_| {
                ControlFlow::Continue(())
            })
            .is_err()
        );
        assert!(!target.exists());
    }
    let mut bad = resources.clone();
    bad.discovery.wall_time_secs = Some(100.);
    assert!(matches!(
        retry_endpoint_evidence(
            &old,
            &checkpoint,
            &checkpoint.evidence.request.source_identity,
            &bad,
            &target,
            CAP,
            |_| ControlFlow::Continue(())
        ),
        Err(Error::Unsupported(_))
    ));
    assert!(!target.exists());
    for destination in [&old, &old.join("nested"), base.path()] {
        assert!(
            retry_endpoint_evidence(
                &old,
                &checkpoint,
                &checkpoint.evidence.request.source_identity,
                &resources,
                destination,
                CAP,
                |_| ControlFlow::Continue(())
            )
            .is_err()
        );
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&old, &target).unwrap();
        assert!(
            retry_endpoint_evidence(
                &old,
                &checkpoint,
                &checkpoint.evidence.request.source_identity,
                &resources,
                &target,
                CAP,
                |_| ControlFlow::Continue(())
            )
            .is_err()
        );
        fs::remove_file(&target).unwrap();
    }
    unchanged(&old, &before);
    let raw = checkpoint.evidence.record.resolve(&old).unwrap();
    let mut bytes = fs::read(&raw).unwrap();
    bytes[0] ^= 1;
    fs::write(&raw, bytes).unwrap();
    assert!(
        retry_endpoint_evidence(
            &old,
            &checkpoint,
            &checkpoint.evidence.request.source_identity,
            &resources,
            &target,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    assert!(!target.exists());
}
#[test]
fn retry_projective_preserves_full_triangle_and_refuses_forged_lineage() {
    let base = tempfile::tempdir().unwrap();
    let old = base.path().join("old");
    fs::create_dir(&old).unwrap();
    let (input, options) = projective_tests::triangle();
    let first = prepare(input, options, &old, |_| ControlFlow::Continue(()))
        .unwrap()
        .receipt;
    let checkpoint = first.checkpoint().unwrap();
    let before = snapshots(&old, &checkpoint);
    let mut resources = EndpointPreparationResources::default();
    resources.geometry.wall_time_secs = Some(45.);
    resources.discovery.wall_time_secs = Some(45.);
    let target = base.path().join("retry");
    let result = retry_endpoint_evidence(
        &old,
        &checkpoint,
        &first.publication.source_identity,
        &resources,
        &target,
        CAP,
        |_| ControlFlow::Continue(()),
    )
    .unwrap();
    assert_eq!(result.prepared.receipt.cells, 4);
    assert_eq!(result.prepared.receipt.endpoint_charts, 18);
    let (old_options, old_route) = configuration::read(&old, &first.configuration, CAP).unwrap();
    let (new_options, new_route) =
        configuration::read(&target, &result.prepared.receipt.configuration, CAP).unwrap();
    assert_eq!(old_route, new_route);
    let old_regular = old_options.regular_family.unwrap();
    let new_regular = new_options.regular_family.unwrap();
    assert_eq!(old_regular.coordinates, new_regular.coordinates);
    assert_eq!(
        old_regular.limits.unit_margin,
        new_regular.limits.unit_margin
    );
    assert_eq!(
        new_regular.limits.geometry.wall_time,
        std::time::Duration::from_secs(45)
    );
    unchanged(&old, &before);
    let mut a = compile(&old, &first);
    let mut b = compile(&target, &result.prepared.receipt);
    for point in [[0.23, 0.37], [0.71, 0.61]] {
        for (x, y) in a.sectors_mut().iter_mut().zip(b.sectors_mut()) {
            let mut xv = vec![0.; x.output_count()];
            let mut yv = vec![0.; y.output_count()];
            x.evaluate(&point, &mut xv).unwrap();
            y.evaluate(&point, &mut yv).unwrap();
            assert_eq!(xv, yv);
        }
    }
    let mut bad = result.checkpoint.clone();
    bad.lineage.blake3 = "00".repeat(32);
    assert!(
        resume_endpoint_retry(
            &target,
            &bad,
            "bad",
            &first.publication.source_identity,
            None,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    // Even a freshly issued envelope cannot make a non-resource generation change valid.
    let (mut changed, route) =
        configuration::read(&target, &result.prepared.receipt.configuration, CAP).unwrap();
    changed.generation.max_order += 1;
    let wrong = configuration::write(&target, &changed, route).unwrap();
    let (mut lineage, _, _): (retry::EndpointRetryLineage, _, _) =
        retry::codec::read(&target, &result.checkpoint.lineage, retry::KIND).unwrap();
    lineage.effective_configuration = wrong.clone();
    bad.preparation.configuration = wrong;
    bad.lineage = retry::codec::write(
        &target,
        "forged-lineage",
        retry::KIND,
        &lineage,
        retry::Atoms::default(),
        vec![],
    )
    .unwrap();
    assert!(
        resume_endpoint_retry(
            &target,
            &bad,
            "bad-policy",
            &first.publication.source_identity,
            None,
            CAP,
            |_| ControlFlow::Continue(())
        )
        .is_err()
    );
    // Native configuration equality also authenticates exact Atom values and
    // ordered symbol roles, independently of the serialized policy DTO.
    for mutation in ["unit", "coordinate", "margin"] {
        let (mut changed, route) =
            configuration::read(&target, &result.prepared.receipt.configuration, CAP).unwrap();
        match mutation {
            "unit" => changed.unit = symbol!("retry_wrong_unit"),
            "coordinate" => {
                changed
                    .regular_family
                    .as_mut()
                    .unwrap()
                    .coordinates
                    .as_mut()
                    .unwrap()[0] = symbol!("retry_wrong_coordinate");
            }
            "margin" => {
                changed.regular_family.as_mut().unwrap().limits.unit_margin =
                    symbolica::domains::rational::Rational::from((1, 3));
            }
            _ => unreachable!(),
        }
        let wrong = configuration::write(&target, &changed, route).unwrap();
        let (mut lineage, _, _): (retry::EndpointRetryLineage, _, _) =
            retry::codec::read(&target, &result.checkpoint.lineage, retry::KIND).unwrap();
        let mut forged = result.checkpoint.clone();
        lineage.effective_configuration = wrong.clone();
        forged.preparation.configuration = wrong;
        forged.lineage = retry::codec::write(
            &target,
            "forged-exact-configuration",
            retry::KIND,
            &lineage,
            retry::Atoms::default(),
            vec![],
        )
        .unwrap();
        assert!(forged.lineage(&target, CAP).is_err(), "{mutation}");
    }
}
