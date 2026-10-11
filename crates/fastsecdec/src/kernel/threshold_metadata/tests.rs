use super::*;

fn digest(n: u8) -> Digest {
    Digest(format!("{n:064x}"))
}
fn checked(m: &LineageManifestV1) -> StructureCheckedLineage<'_> {
    m.validate_structure(&tables(), Limits::default()).unwrap()
}
fn tables() -> NativeTables {
    NativeTables {
        expression_records: std::collections::BTreeMap::from([(digest(20), 16)]),
        symbols: 8,
    }
}
fn map(coordinates: &[usize], images: &[usize], reversed: bool) -> MapDescriptor {
    MapDescriptor::RationalV1 {
        geometry: MapGeometry {
            expressions: digest(20),
            coordinates: coordinates.iter().map(|v| NativeSymbolId(*v)).collect(),
            images: images.iter().map(|v| NativeAtomId(*v)).collect(),
            positive_measure: NativeAtomId(2),
            orientation: if reversed {
                Orientation::Reversed
            } else {
                Orientation::Forward
            },
            certificate: digest(3),
        },
    }
}
fn fixture() -> LineageManifestV1 {
    let selection = serde_json::from_value(serde_json::json!({
        "original_source_count": 22, "source_sectors": [2, 5]
    }))
    .unwrap();
    LineageManifestV1 {
        version: 1,
        preparation: Preparation {
            runtime_chamber: None,
            source_identity: digest(1),
            extent: SourceExtent::Selected {
                selection,
                ordinary_geometry: digest(2),
            },
            strategy: ResolvedStrategy::SectorFirst,
            actual_options: digest(4),
            expressions: digest(20),
            partition_certificate: digest(17),
            original_coordinates: vec![NativeSymbolId(0), NativeSymbolId(1)],
            regulator: NativeSymbolId(2),
            fiber: FixedRationalFiber {
                physical_parameters: vec![NativeSymbolId(3)],
                bindings: vec![RationalBinding {
                    symbol: NativeSymbolId(3),
                    value: NativeAtomId(3),
                }],
                admission: digest(5),
            },
        },
        patches: vec![
            Patch {
                id: PatchId(1),
                source: SourceOrigin::OrdinarySector(2),
                map: map(&[0, 1], &[0, 1], false),
                coverage: digest(6),
            },
            Patch {
                id: PatchId(2),
                source: SourceOrigin::OrdinarySector(5),
                map: map(&[0, 1], &[0, 1], false),
                coverage: digest(6),
            },
        ],
        cells: vec![
            Cell {
                id: CellId(10),
                patch: PatchId(1),
                native: NativeCellLocator {
                    request_record: digest(7),
                    raw_evidence: digest(8),
                    raw_cell_index: 0,
                },
                coordinate_order: vec![NativeSymbolId(1), NativeSymbolId(0)],
                coverage: digest(9),
            },
            Cell {
                id: CellId(20),
                patch: PatchId(2),
                native: NativeCellLocator {
                    request_record: digest(10),
                    raw_evidence: digest(11),
                    raw_cell_index: 0,
                },
                coordinate_order: vec![NativeSymbolId(0), NativeSymbolId(1)],
                coverage: digest(9),
            },
        ],
        endpoint_charts: vec![
            EndpointChart {
                id: EndpointChartId(100),
                origin: EndpointOrigin::Cell(CellId(10)),
                path: vec![0],
                map: map(&[4, 5], &[4, 5], false),
                endpoint_certificate: digest(12),
                causal_phase_certificate: digest(18),
            },
            EndpointChart {
                id: EndpointChartId(101),
                origin: EndpointOrigin::Cell(CellId(10)),
                path: vec![1],
                map: map(&[4, 5], &[6, 7], true),
                endpoint_certificate: digest(12),
                causal_phase_certificate: digest(18),
            },
            EndpointChart {
                id: EndpointChartId(200),
                origin: EndpointOrigin::Cell(CellId(20)),
                path: vec![0],
                map: map(&[4, 5], &[4, 5], false),
                endpoint_certificate: digest(12),
                causal_phase_certificate: digest(18),
            },
        ],
        continuation_groups: vec![ContinuationGroup {
            id: GroupId(1),
            charts: vec![
                EndpointChartId(100),
                EndpointChartId(101),
                EndpointChartId(200),
            ],
            continuation: Continuation::EpsilonStripV1 {
                certificate: digest(13),
            },
        }],
        contributions: vec![
            Contribution {
                id: ContributionId(1000),
                chart: EndpointChartId(100),
                group: GroupId(1),
                kind: ContributionKind::Stochastic {
                    coordinates: vec![NativeSymbolId(4), NativeSymbolId(5)],
                },
            },
            Contribution {
                id: ContributionId(1001),
                chart: EndpointChartId(100),
                group: GroupId(1),
                kind: ContributionKind::Exact,
            },
            Contribution {
                id: ContributionId(1002),
                chart: EndpointChartId(101),
                group: GroupId(1),
                kind: ContributionKind::Stochastic {
                    coordinates: vec![NativeSymbolId(4)],
                },
            },
            Contribution {
                id: ContributionId(1003),
                chart: EndpointChartId(200),
                group: GroupId(1),
                kind: ContributionKind::CertifiedZero {
                    certificate: digest(14),
                },
            },
            Contribution {
                id: ContributionId(1004),
                chart: EndpointChartId(101),
                group: GroupId(1),
                kind: ContributionKind::Exact,
            },
        ],
    }
}
fn records(m: &LineageManifestV1) -> Vec<RecordLineageV1> {
    let manifest = m.descriptor_digest().unwrap();
    vec![
        RecordLineageV1 {
            manifest: manifest.clone(),
            contributions: vec![ContributionId(1000)],
            kind: RecordKind::Stochastic {
                coordinates: vec![NativeSymbolId(4), NativeSymbolId(5)],
            },
        },
        RecordLineageV1 {
            manifest: manifest.clone(),
            contributions: vec![ContributionId(1002)],
            kind: RecordKind::Stochastic {
                coordinates: vec![NativeSymbolId(4)],
            },
        },
        RecordLineageV1 {
            manifest,
            contributions: vec![ContributionId(1001), ContributionId(1004)],
            kind: RecordKind::Exact,
        },
    ]
}

#[test]
fn one_to_many_source_extent_and_complete_contribution_inventory() {
    let m = fixture();
    m.validate_structure(&tables(), Limits::default()).unwrap();
    m.validate_initial_schema_kinds().unwrap();
    checked(&m)
        .validate_complete_records(&m.descriptor_digest().unwrap(), &records(&m))
        .unwrap();
    let restored: LineageManifestV1 =
        serde_json::from_slice(&serde_json::to_vec(&m).unwrap()).unwrap();
    assert_eq!(restored, m);
    assert_eq!(
        restored.descriptor_digest().unwrap(),
        m.descriptor_digest().unwrap()
    );
    // Source 5 remains declared even though its only final leaf is zero.
    let SourceExtent::Selected { selection, .. } = &restored.preparation.extent else {
        panic!()
    };
    assert_eq!(selection.source_sectors(), [2, 5]);
}

#[test]
fn partial_records_never_satisfy_complete_coverage_and_zero_is_not_a_record() {
    let m = fixture();
    let hash = m.descriptor_digest().unwrap();
    let mut r = records(&m);
    checked(&m).validate_record(&hash, &r[0]).unwrap();
    assert!(
        checked(&m)
            .validate_complete_records(&hash, &r[..1])
            .is_err()
    );
    r.push(r[0].clone());
    assert!(checked(&m).validate_complete_records(&hash, &r).is_err());
    r = records(&m);
    r[2].contributions.push(ContributionId(1003));
    r[2].contributions.sort();
    assert!(checked(&m).validate_complete_records(&hash, &r).is_err());
    r = records(&m);
    r[0].manifest = digest(255);
    assert!(checked(&m).validate_complete_records(&hash, &r).is_err());
    assert!(checked(&m).validate_record(&digest(255), &r[0]).is_err());
    r = records(&m);
    if let RecordKind::Stochastic { coordinates } = &mut r[0].kind {
        coordinates.reverse();
    }
    assert!(checked(&m).validate_complete_records(&hash, &r).is_err());
}

#[test]
fn exact_only_and_zero_only_keep_complete_original_extent() {
    let mut m = fixture();
    for c in &mut m.contributions {
        c.kind = ContributionKind::Exact;
    }
    m.validate_structure(&tables(), Limits::default()).unwrap();
    let hash = m.descriptor_digest().unwrap();
    checked(&m)
        .validate_complete_records(
            &hash,
            &[RecordLineageV1 {
                manifest: hash.clone(),
                contributions: m.contributions.iter().map(|c| c.id).collect(),
                kind: RecordKind::Exact,
            }],
        )
        .unwrap();
    for c in &mut m.contributions {
        c.kind = ContributionKind::CertifiedZero {
            certificate: digest(14),
        };
    }
    m.validate_structure(&tables(), Limits::default()).unwrap();
    checked(&m)
        .validate_complete_records(&m.descriptor_digest().unwrap(), &[])
        .unwrap();
    m.contributions.pop(); // chart 101 still has its other zero leaf.
    m.validate_structure(&tables(), Limits::default()).unwrap();
    m.contributions.retain(|c| c.chart != EndpointChartId(200));
    assert!(m.validate_structure(&tables(), Limits::default()).is_err());
}

#[test]
fn rejects_missing_ancestry_continuation_and_dimension_or_order_tampering() {
    let changes: &[fn(&mut LineageManifestV1)] = &[
        |m| m.patches[1].source = SourceOrigin::OrdinarySector(6),
        |m| m.patches[1].source = SourceOrigin::OrdinarySector(2),
        |m| m.cells[0].patch = PatchId(50),
        |m| m.cells[0].coordinate_order.pop().map(|_| ()).unwrap(),
        |m| m.endpoint_charts[1].path = vec![0],
        |m| m.endpoint_charts[0].origin = EndpointOrigin::Cell(CellId(50)),
        |m| m.continuation_groups[0].charts.pop().map(|_| ()).unwrap(),
        |m| m.continuation_groups[0].charts.push(EndpointChartId(200)),
        |m| m.contributions[0].group = GroupId(2),
        |m| {
            m.contributions[0].kind = ContributionKind::Stochastic {
                coordinates: vec![],
            }
        },
        |m| {
            m.contributions[0].kind = ContributionKind::Stochastic {
                coordinates: vec![NativeSymbolId(0)],
            }
        },
        |m| m.contributions[1].id = m.contributions[0].id,
        |m| m.endpoint_charts[0].map = map(&[4], &[4], false),
        |m| m.preparation.actual_options = Digest("invalid".into()),
    ];
    for (i, change) in changes.iter().enumerate() {
        let mut m = fixture();
        change(&mut m);
        assert!(
            m.validate_structure(&tables(), Limits::default()).is_err(),
            "mutation {i}"
        );
    }
}

#[test]
fn fixed_binding_roles_table_references_and_descriptor_identity_are_fenced() {
    let m = fixture();
    let digest_before = m.descriptor_digest().unwrap();
    let mut changed = m.clone();
    changed.preparation.fiber.bindings[0].value = NativeAtomId(4);
    changed
        .validate_structure(&tables(), Limits::default())
        .unwrap();
    assert_ne!(digest_before, changed.descriptor_digest().unwrap());
    let mut other_extent = m.clone();
    let SourceExtent::Selected { selection, .. } = &mut other_extent.preparation.extent else {
        panic!()
    };
    *selection = serde_json::from_value(serde_json::json!({
        "original_source_count": 22, "source_sectors": [2, 6]
    }))
    .unwrap();
    other_extent.patches[1].source = SourceOrigin::OrdinarySector(6);
    other_extent
        .validate_structure(&tables(), Limits::default())
        .unwrap();
    assert_ne!(digest_before, other_extent.descriptor_digest().unwrap());
    changed.preparation.fiber.bindings.clear();
    assert!(
        changed
            .validate_structure(&tables(), Limits::default())
            .is_err()
    );
    changed = m.clone();
    changed.preparation.fiber.physical_parameters[0] = changed.preparation.regulator;
    changed.preparation.fiber.bindings[0].symbol = changed.preparation.regulator;
    assert!(
        changed
            .validate_structure(&tables(), Limits::default())
            .is_err()
    );
    changed = m.clone();
    changed.preparation.fiber.bindings[0].value = NativeAtomId(16);
    assert!(
        changed
            .validate_structure(&tables(), Limits::default())
            .is_err()
    );
    changed = m;
    changed.preparation.regulator = NativeSymbolId(8);
    assert!(
        changed
            .validate_structure(&tables(), Limits::default())
            .is_err()
    );
}

#[test]
fn zero_dimensional_projective_preparation_needs_no_stochastic_sector() {
    let mut m = fixture();
    m.preparation.extent = SourceExtent::Full {
        ordinary_geometry: None,
    };
    m.preparation.strategy = ResolvedStrategy::GcadFirst;
    m.preparation.original_coordinates = vec![NativeSymbolId(0)];
    m.patches.truncate(1);
    m.patches[0].source = SourceOrigin::OriginalInput;
    m.patches[0].map = map(&[], &[0], false);
    m.cells.clear();
    m.endpoint_charts.truncate(1);
    m.endpoint_charts[0].origin = EndpointOrigin::ExactPatch(PatchId(1));
    m.endpoint_charts[0].map = map(&[], &[], false);
    m.continuation_groups[0].charts.truncate(1);
    m.contributions.truncate(1);
    m.contributions[0].kind = ContributionKind::Exact;
    m.validate_structure(&tables(), Limits::default()).unwrap();
    let hash = m.descriptor_digest().unwrap();
    checked(&m)
        .validate_complete_records(
            &hash,
            &[RecordLineageV1 {
                manifest: hash.clone(),
                contributions: vec![ContributionId(1000)],
                kind: RecordKind::Exact,
            }],
        )
        .unwrap();
    let mut fake_cell = fixture().cells[0].clone();
    fake_cell.coordinate_order.clear();
    m.cells.push(fake_cell);
    m.endpoint_charts[0].origin = EndpointOrigin::Cell(CellId(10));
    assert!(m.validate_structure(&tables(), Limits::default()).is_err());
    m.cells.clear();
    m.endpoint_charts[0].origin = EndpointOrigin::ExactPatch(PatchId(1));
    m.contributions[0].kind = ContributionKind::Stochastic {
        coordinates: vec![],
    };
    assert!(m.validate_structure(&tables(), Limits::default()).is_err());
}

#[test]
fn full_compact_gauge_and_full_sector_first_are_distinct_valid_strategies() {
    let mut m = fixture();
    m.preparation.extent = SourceExtent::Full {
        ordinary_geometry: None,
    };
    m.preparation.strategy = ResolvedStrategy::GcadFirst;
    for p in &mut m.patches {
        p.source = SourceOrigin::OriginalInput;
    }
    m.validate_structure(&tables(), Limits::default()).unwrap();
    let gauge_hash = m.descriptor_digest().unwrap();
    m.preparation.strategy = ResolvedStrategy::SectorFirst;
    assert!(m.validate_structure(&tables(), Limits::default()).is_err());
    m.preparation.extent = SourceExtent::Full {
        ordinary_geometry: Some(OrdinaryGeometry {
            identity: digest(2),
            original_source_count: 2,
        }),
    };
    m.patches[0].source = SourceOrigin::OrdinarySector(0);
    m.patches[1].source = SourceOrigin::OrdinarySector(1);
    m.validate_structure(&tables(), Limits::default()).unwrap();
    assert_ne!(gauge_hash, m.descriptor_digest().unwrap());
}

#[test]
fn nonlinear_and_auxiliary_descriptors_are_inspectable_but_not_executable() {
    let mut m = fixture();
    let geometry = m.endpoint_charts[0].map.geometry().clone();
    m.endpoint_charts[0].map = MapDescriptor::AlgebraicSectionsV1 {
        geometry,
        sections: vec![RootSection {
            polynomial: NativeAtomId(8),
            root_variable: NativeSymbolId(6),
            preceding_coordinates: vec![NativeSymbolId(4)],
            domain: RootDomain::RealDescending,
            ordinal: 1,
            branch_certificate: digest(15),
        }],
    };
    m.validate_structure(&tables(), Limits::default()).unwrap();
    assert!(matches!(
        m.validate_initial_schema_kinds(),
        Err(Error::Unsupported(_))
    ));
    if let MapDescriptor::AlgebraicSectionsV1 { sections, .. } = &mut m.endpoint_charts[0].map {
        sections[0].preceding_coordinates.push(NativeSymbolId(6));
    }
    assert!(m.validate_structure(&tables(), Limits::default()).is_err());
    m = fixture();
    m.continuation_groups[0].continuation = Continuation::AuxiliaryIntegralCancellationV1 {
        certificate: digest(16),
    };
    m.validate_structure(&tables(), Limits::default()).unwrap();
    assert!(matches!(
        m.validate_initial_schema_kinds(),
        Err(Error::Unsupported(_))
    ));
}

#[test]
fn structural_budgets_and_unknown_wire_fields_are_rejected() {
    let m = fixture();
    assert_eq!(
        m.validate_structure(
            &tables(),
            Limits {
                maximum_nodes: 1,
                maximum_references: usize::MAX
            }
        ),
        Err(Error::Limit)
    );
    assert_eq!(
        m.validate_structure(
            &tables(),
            Limits {
                maximum_nodes: usize::MAX,
                maximum_references: 1
            }
        ),
        Err(Error::Limit)
    );
    let mut json = serde_json::to_value(m).unwrap();
    json["verified"] = serde_json::json!(true);
    assert!(serde_json::from_value::<LineageManifestV1>(json).is_err());
}

#[test]
fn full_original_input_partial_and_exact_only_residents_never_claim_full_integral() {
    use crate::results::ExactContributionPolicy;
    let mut m = fixture();
    m.preparation.extent = SourceExtent::Full {
        ordinary_geometry: None,
    };
    m.preparation.strategy = ResolvedStrategy::GcadFirst;
    for p in &mut m.patches {
        p.source = SourceOrigin::OriginalInput;
    }
    m.validate_structure(&tables(), Limits::default()).unwrap();
    let manifest = m.descriptor_digest().unwrap();
    let records = records(&m);
    let mut resident = ResidentLineageV1 {
        manifest: manifest.clone(),
        selection: ResidentSelection::Complete,
    };
    assert!(
        checked(&m)
            .validate_resident_records(&manifest, &resident, &records)
            .unwrap()
    );
    assert!(
        checked(&m)
            .validate_resident_records(&manifest, &resident, &records[..1])
            .is_err()
    );
    resident.selection = ResidentSelection::Selected {
        stochastic_contributions: vec![ContributionId(1000)],
        exact_policy: ExactContributionPolicy::ExcludeAll,
    };
    assert!(
        !checked(&m)
            .validate_resident_records(&manifest, &resident, &records[..1])
            .unwrap()
    );
    resident.selection = ResidentSelection::Selected {
        stochastic_contributions: vec![],
        exact_policy: ExactContributionPolicy::IncludeAll,
    };
    assert!(
        !checked(&m)
            .validate_resident_records(&manifest, &resident, &records[2..])
            .unwrap()
    );
    assert!(
        checked(&m)
            .validate_resident_records(&manifest, &resident, &[])
            .is_err()
    );
    resident.selection = ResidentSelection::Selected {
        stochastic_contributions: vec![ContributionId(1000), ContributionId(1002)],
        exact_policy: ExactContributionPolicy::IncludeAll,
    };
    assert!(
        !checked(&m)
            .validate_resident_records(&manifest, &resident, &records)
            .unwrap()
    );
    resident.selection = ResidentSelection::Selected {
        stochastic_contributions: vec![],
        exact_policy: ExactContributionPolicy::ExcludeAll,
    };
    assert!(
        !checked(&m)
            .validate_resident_records(&manifest, &resident, &[])
            .unwrap()
    );
    let restored: ResidentLineageV1 =
        serde_json::from_slice(&serde_json::to_vec(&resident).unwrap()).unwrap();
    assert_eq!(restored, resident);
}

#[test]
fn native_record_references_need_only_independent_schema_bounds() {
    let mut m = fixture();
    let geometry = match &mut m.endpoint_charts[0].map {
        MapDescriptor::RationalV1 { geometry } => geometry,
        _ => unreachable!(),
    };
    geometry.expressions = digest(21);
    assert!(m.validate_structure(&tables(), Limits::default()).is_err());
    let mut schema = tables();
    schema.expression_records.insert(digest(21), 6);
    m.validate_structure(&schema, Limits::default()).unwrap();
    schema.expression_records.insert(digest(21), 5);
    assert!(m.validate_structure(&schema, Limits::default()).is_err());
    // These are lengths and native record IDs only: this schema test does not
    // import any expression record, decode its Atoms, or restore global proof.
}

#[test]
fn every_tagged_variant_rejects_unknown_wire_fields() {
    fn rejected<T: serde::Serialize + serde::de::DeserializeOwned>(
        name: &str,
        value: T,
    ) -> Option<String> {
        let mut value = serde_json::to_value(value).unwrap();
        value
            .as_object_mut()
            .unwrap()
            .insert("unexpected_math_metadata".into(), serde_json::json!(17));
        serde_json::from_value::<T>(value)
            .is_ok()
            .then(|| name.to_owned())
    }
    let accepted = [
        rejected("SourceOrigin::OriginalInput", SourceOrigin::OriginalInput),
        rejected(
            "SourceOrigin::OrdinarySector",
            SourceOrigin::OrdinarySector(2),
        ),
        rejected(
            "SourceExtent::Full",
            SourceExtent::Full {
                ordinary_geometry: None,
            },
        ),
        rejected("SourceExtent::Selected", fixture().preparation.extent),
        rejected("EndpointOrigin::Cell", EndpointOrigin::Cell(CellId(1))),
        rejected(
            "EndpointOrigin::ExactPatch",
            EndpointOrigin::ExactPatch(PatchId(1)),
        ),
        rejected("ContributionKind::Exact", ContributionKind::Exact),
        rejected(
            "ContributionKind::Stochastic",
            ContributionKind::Stochastic {
                coordinates: vec![NativeSymbolId(4)],
            },
        ),
        rejected(
            "ContributionKind::CertifiedZero",
            ContributionKind::CertifiedZero {
                certificate: digest(1),
            },
        ),
        rejected("RecordKind::Exact", RecordKind::Exact),
        rejected(
            "RecordKind::Stochastic",
            RecordKind::Stochastic {
                coordinates: vec![NativeSymbolId(4)],
            },
        ),
        rejected("ResidentSelection::Complete", ResidentSelection::Complete),
        rejected(
            "ResidentSelection::Selected",
            ResidentSelection::Selected {
                stochastic_contributions: vec![],
                exact_policy: crate::results::ExactContributionPolicy::ExcludeAll,
            },
        ),
        rejected(
            "Continuation::EpsilonStripV1",
            Continuation::EpsilonStripV1 {
                certificate: digest(1),
            },
        ),
        rejected(
            "Continuation::AuxiliaryIntegralCancellationV1",
            Continuation::AuxiliaryIntegralCancellationV1 {
                certificate: digest(1),
            },
        ),
        rejected("MapDescriptor::RationalV1", map(&[4], &[0], false)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    assert!(
        accepted.is_empty(),
        "unknown metadata accepted by {accepted:?}"
    );
}

#[test]
fn uniform_runtime_capability_has_strict_roles_and_explicit_optional_wire() {
    let fixed = fixture();
    assert!(
        serde_json::to_value(&fixed).unwrap()["preparation"]
            .get("runtime_chamber")
            .is_none()
    );
    let mut manifest = fixed;
    manifest.preparation.runtime_chamber = Some(UniformParameterChamberV1 {
        version: 1,
        runtime_parameters: vec![NativeSymbolId(6)],
        prefix_to_public: vec![0],
        integration_axes: vec![1, 2],
        predicates: vec![ParameterPredicateV1 {
            role: ParameterPredicateRole::LowerBound,
            index: 0,
            positive_selector: false,
            predicate: digest(31),
        }],
        guard_semantic: digest(32),
        family_certificate: digest(33),
    });
    manifest
        .validate_structure(&tables(), Limits::default())
        .unwrap();
    assert_eq!(manifest.patches.len(), 2);
    for patch in 0..manifest.patches.len() {
        let mut bad = manifest.clone();
        if let MapDescriptor::RationalV1 { geometry } = &mut bad.patches[patch].map {
            geometry.coordinates.pop();
        }
        assert_eq!(
            bad.validate_structure(&tables(), Limits::default())
                .unwrap_err(),
            Error::Invalid("uniform runtime prepared-patch dimension".into()),
        );
    }
    let mut empty = manifest.clone();
    empty.patches.clear();
    assert_eq!(
        empty
            .validate_structure(&tables(), Limits::default())
            .unwrap_err(),
        Error::Invalid("uniform runtime prepared-patch dimension".into()),
    );
    for mutation in 0..6 {
        let mut bad = manifest.clone();
        let chamber = bad.preparation.runtime_chamber.as_mut().unwrap();
        match mutation {
            0 => chamber.version = 2,
            1 => chamber.runtime_parameters[0] = NativeSymbolId(0),
            2 => chamber.runtime_parameters[0] = NativeSymbolId(4),
            3 => chamber.prefix_to_public[0] = 1,
            4 => chamber.integration_axes = vec![0, 1],
            _ => chamber.predicates.push(chamber.predicates[0].clone()),
        }
        assert!(
            bad.validate_structure(&tables(), Limits::default())
                .is_err()
        );
    }
    let mut json = serde_json::to_value(&manifest).unwrap();
    json["preparation"]["runtime_chamber"]["claimed_geometry_proof"] = true.into();
    assert!(serde_json::from_value::<LineageManifestV1>(json).is_err());
}
