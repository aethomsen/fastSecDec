use super::*;

fn selected_fixture() -> LineageManifestV1 {
    let mut m = fixture();
    m.preparation.strategy = ResolvedStrategy::GcadFirst;
    m.patches.truncate(1);
    m.patches[0].source = SourceOrigin::OriginalInput;
    m.patches[0].coverage = m.preparation.partition_certificate.clone();
    for (cell, raw) in m.cells.iter_mut().zip([2, 5]) {
        cell.patch = m.patches[0].id;
        cell.native = NativeCellLocator {
            request_record: digest(7),
            raw_evidence: digest(8),
            raw_cell_index: raw,
        };
    }
    m.preparation.extent = SourceExtent::SelectedOriginalCellsV1 {
        selection: OriginalCellSelectionV1 {
            source: m.preparation.source_identity.clone(),
            original_request_record: digest(30),
            original_raw_evidence: digest(31),
            quotient_request_record: digest(7),
            quotient_raw_evidence: digest(8),
            quotient: m.patches[0].map.geometry().certificate.clone(),
            original_cell_count: 7,
            cells: vec![2, 5],
            coverage: m.preparation.partition_certificate.clone(),
        },
    };
    m
}

fn selection(m: &mut LineageManifestV1) -> &mut OriginalCellSelectionV1 {
    let SourceExtent::SelectedOriginalCellsV1 { selection } = &mut m.preparation.extent else {
        panic!("test original-cell extent")
    };
    selection
}

#[test]
fn selected_original_cells_survive_codec_and_remain_partial() {
    let m = selected_fixture();
    let restored: LineageManifestV1 =
        serde_json::from_slice(&serde_json::to_vec(&m).unwrap()).unwrap();
    assert_eq!(restored, m);
    let checked = checked(&restored);
    let digest = restored.descriptor_digest().unwrap();
    assert!(
        !checked
            .validate_resident_records(
                &digest,
                &ResidentLineageV1 {
                    manifest: digest.clone(),
                    selection: ResidentSelection::Complete,
                },
                &records(&restored)
            )
            .unwrap()
    );
    assert!(
        !crate::kernel::ThresholdResultScope::complete_catalogue(restored.preparation.extent)
            .unwrap()
            .is_full_original()
    );
}

#[test]
fn original_cell_scope_rejects_owner_coverage_and_inventory_tampering() {
    let changes: &[fn(&mut LineageManifestV1)] = &[
        |m| selection(m).source = digest(40),
        |m| selection(m).quotient = digest(40),
        |m| selection(m).coverage = digest(40),
        |m| selection(m).quotient_request_record = digest(40),
        |m| selection(m).quotient_raw_evidence = digest(40),
        |m| selection(m).original_raw_evidence = Digest("bad".into()),
        |m| selection(m).original_request_record = Digest("bad".into()),
        |m| selection(m).cells = vec![5, 2],
        |m| selection(m).cells = vec![2, 2],
        |m| selection(m).cells = vec![2],
        |m| selection(m).cells = vec![2, 6],
        |m| selection(m).cells.clear(),
        |m| selection(m).original_cell_count = 5,
        |m| selection(m).original_cell_count = 0,
        |m| m.cells[1].native.raw_cell_index = 2,
        |m| m.patches[0].coverage = digest(40),
        |m| m.preparation.strategy = ResolvedStrategy::SectorFirst,
    ];
    for (index, change) in changes.iter().enumerate() {
        let mut m = selected_fixture();
        change(&mut m);
        assert!(
            m.validate_structure(&tables(), Limits::default()).is_err(),
            "mutation {index}"
        );
    }
}

#[test]
fn selected_all_cells_never_becomes_full_original_result_scope() {
    let mut m = selected_fixture();
    selection(&mut m).original_cell_count = 2;
    selection(&mut m).cells = vec![0, 1];
    for (i, c) in m.cells.iter_mut().enumerate() {
        c.native.raw_cell_index = i;
    }
    checked(&m);
    let scope =
        crate::kernel::ThresholdResultScope::complete_catalogue(m.preparation.extent).unwrap();
    assert!(!scope.is_full_original());
    let manifest = crate::results::KernelResultManifest {
        threshold_scope: Some(scope),
        source_selection: None,
        kernel_content_id: "selected-original-cells".into(),
        orders: vec![0],
        components: vec![crate::status::CoefficientComponent::Real],
        sectors: vec![crate::integration::SectorSpec {
            id: 0,
            dimension: 2,
        }],
        exact_coefficients: vec![0.0],
    };
    let restored: crate::results::KernelResultManifest =
        serde_json::from_slice(&serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(matches!(
        restored
            .canonical_scope(&crate::results::ResultScope::FullIntegral)
            .unwrap(),
        crate::results::ResultScope::SelectedSectors { .. }
    ));
}

#[test]
fn selected_cell_tag_is_strict_and_previous_extent_wire_is_unchanged() {
    let mut value = serde_json::to_value(selected_fixture().preparation.extent).unwrap();
    assert_eq!(value["kind"], "selected_original_cells_v1");
    value["unexpected"] = serde_json::json!(1);
    assert!(serde_json::from_value::<SourceExtent>(value.clone()).is_err());
    value.as_object_mut().unwrap().remove("unexpected");
    value["selection"]["unexpected"] = serde_json::json!(1);
    assert!(serde_json::from_value::<SourceExtent>(value).is_err());
    assert_eq!(
        serde_json::to_value(SourceExtent::Full {
            ordinary_geometry: None
        })
        .unwrap(),
        serde_json::json!({"kind":"full", "ordinary_geometry": null})
    );
    assert_eq!(
        serde_json::to_value(fixture().preparation.extent).unwrap(),
        serde_json::json!({
            "kind":"selected", "selection":{"original_source_count":22,"source_sectors":[2,5]},
            "ordinary_geometry": digest(2),
        })
    );
}
