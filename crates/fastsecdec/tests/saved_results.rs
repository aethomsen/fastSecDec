//! Result-only contracts: native numerical sessions, no symbolic runtime.
use fastsecdec::{
    integration::{
        IntegrationProblem, Periodization, QmcSession, QmcSettings, RuleSource, SectorSpec,
        Tolerance,
    },
    reference::{
        CoefficientKey, ComparisonContext, Compatibility, Independence, ReferenceCoefficient,
        ReferenceProvenance, ReferenceResult, ReferenceUncertainty, ReferenceValidation,
    },
    results::*,
    status::{
        CoefficientComponent::{Imag, Real},
        EvaluationDiagnostics, StoppingReason, UncertaintyStatus,
    },
};

fn fixture(selected: bool) -> SavedIntegrationResult {
    fixture_with_diagnostics(selected, false)
}

fn fixture_with_diagnostics(selected: bool, diagnostics: bool) -> SavedIntegrationResult {
    let manifest = KernelResultManifest {
        threshold_scope: None,
        source_selection: None,
        kernel_content_id: "inner-kernel-id".into(),
        orders: vec![-1, 0, 0],
        components: vec![Real, Real, Imag],
        sectors: vec![
            SectorSpec {
                id: 10,
                dimension: 1,
            },
            SectorSpec {
                id: 20,
                dimension: 2,
            },
        ],
        exact_coefficients: vec![2.0, 3.0, -4.0],
    };
    let sectors = if selected {
        vec![manifest.sectors[0].clone()]
    } else {
        manifest.sectors.clone()
    };
    let problem = IntegrationProblem::new_with_components(
        "outer-checkpoint-identity".into(),
        manifest.orders.clone(),
        manifest.components.clone(),
        sectors,
        manifest.exact_coefficients.clone(),
    )
    .unwrap();
    let mut session = QmcSession::democratic(
        problem,
        QmcSettings {
            points: 4,
            shifts: 4,
            package_points: 4,
            periodization: Periodization::None,
            rule: RuleSource::Supplied(vec![1, 3]),
            ..QmcSettings::default()
        },
    )
    .unwrap();
    while let Some(task) = session.next_work().unwrap() {
        let value = (task.work().start() / 4) as f64 * task.sector_id() as f64 / 10.0;
        let returned = session
            .worker_context(task.sector_id())
            .unwrap()
            .evaluate(task, |_, out| {
                out.copy_from_slice(&[value, 2.0 * value, -value]);
                Ok::<_, String>(())
            })
            .unwrap();
        session.submit(returned).unwrap();
    }
    let mut target = ReferenceResult::new(
        vec![ReferenceCoefficient {
            key: CoefficientKey {
                order: 0,
                component: Real,
            },
            value: 101.0,
            uncertainty: ReferenceUncertainty::Unknown,
        }],
        ReferenceProvenance::new("independent fixture", "native measure"),
    );
    target.validation = ReferenceValidation::Checked {
        evidence: "independent analytic source".into(),
    };
    target.kernel_content_id = Some(manifest.kernel_content_id.clone());
    SavedIntegrationResult {
        scope: if selected {
            ResultScope::SelectedSectors {
                sector_ids: vec![10],
                exact_policy: ExactContributionPolicy::IncludeAll,
            }
        } else {
            ResultScope::FullIntegral
        },
        contributions: session.contributions().unwrap(),
        weight_diagnostics: diagnostics.then(|| session.weight_diagnostics().unwrap()),
        stopping_reason: StoppingReason::PlannedWorkComplete,
        requested_tolerance: None,
        requested_accuracy_target: Default::default(),
        evaluation_diagnostics: Some(EvaluationDiagnostics {
            evaluations: 32,
            rescues: 1,
            ..EvaluationDiagnostics::default()
        }),
        qmc_design: Some(session.design()),
        provenance: ReferenceProvenance::new("saved numeric fixture", "native measure"),
        validation: ReferenceValidation::Unverified,
        stored_reference: Some(StoredReference {
            reference: target,
            context: ComparisonContext {
                kernel_content_id: manifest.kernel_content_id.clone(),
                normalization: Compatibility::Confirmed {
                    basis: "same convention".into(),
                },
                kinematics: Compatibility::Unknown,
                independence: Independence::Independent {
                    basis: "different provider".into(),
                },
            },
        }),
        timings: ResultTimings {
            elapsed_seconds: Some(0.2),
            artifact_load_seconds: None,
        },
        manifest,
    }
}

#[test]
fn accepted_weight_payload_round_trips_and_rejects_inconsistent_derived_fields() {
    let result = fixture_with_diagnostics(false, true);
    let loaded = read_result(&encode_result(&result).unwrap()).unwrap();
    assert_eq!(loaded, result);
    let text = loaded.to_string();
    assert!(text.contains("Accepted weight diagnostics: Available"));
    assert!(text.contains("norm relative SE"));
    let mut bad = loaded.clone();
    bad.weight_diagnostics.as_mut().unwrap().accepted_points += 1;
    assert!(bad.validate().is_err());
    let mut bad = loaded.clone();
    bad.weight_diagnostics.as_mut().unwrap().orders[0].maximum_ratio = Some(999.0);
    assert!(bad.validate().is_err());
    let mut bad = loaded;
    bad.weight_diagnostics.as_mut().unwrap().orders[0].total_mean_magnitude = None;
    assert!(bad.validate().is_err());
}

#[test]
fn native_round_trip_preserves_covariance_evidence_scope_and_distinct_identities() {
    let result = fixture(false);
    let bytes = encode_result(&result).unwrap();
    // Version-1 historical documents omit this optional diagnostic payload.
    assert!(
        !serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["result"]
            .as_object()
            .unwrap()
            .contains_key("weight_diagnostics")
    );
    let loaded = read_result(&bytes).unwrap();
    assert!(loaded.weight_diagnostics.is_none());
    assert_eq!(loaded, result);
    let covariance = &loaded
        .contributions
        .total
        .as_ref()
        .unwrap()
        .covariance_of_mean;
    assert_ne!(covariance[1], 0.0);
    assert_ne!(covariance[2], 0.0);
    let estimate = loaded
        .reference(ResultReferenceSelection::Estimate)
        .unwrap();
    assert_eq!(estimate.validation, ReferenceValidation::Unverified);
    assert_eq!(
        estimate.kernel_content_id.as_deref(),
        Some("inner-kernel-id")
    );
    let stored = loaded
        .reference(ResultReferenceSelection::StoredReference)
        .unwrap();
    assert_eq!(stored.coefficients[0].value, 101.0);
    assert_eq!(
        stored.coefficients[0].uncertainty,
        ReferenceUncertainty::Unknown
    );
    assert_ne!(estimate.coefficients[1].value, 101.0);
    assert!(matches!(
        loaded.comparison().unwrap(),
        ResultComparison::Compared(_)
    ));
}

#[test]
fn internally_complete_subset_never_becomes_full_integral_reference() {
    let result = fixture(true);
    assert!(result.production_complete());
    assert!(
        result
            .contributions
            .total
            .as_ref()
            .unwrap()
            .production_complete
    );
    assert_eq!(
        read_result(&encode_result(&result).unwrap()).unwrap(),
        result
    );
    assert!(matches!(
        result.reference(ResultReferenceSelection::Estimate),
        Err(ResultError::SelectedScope)
    ));
    assert!(matches!(
        result.comparison().unwrap(),
        ResultComparison::Unavailable(ResultComparisonUnavailable::SelectedScope)
    ));
    assert!(result.to_string().contains("selected stochastic sectors"));
    let mut false_full = result.clone();
    false_full.scope = ResultScope::FullIntegral;
    assert!(false_full.validate().is_err());
    let mut wrong_exact = result;
    wrong_exact.scope = ResultScope::SelectedSectors {
        sector_ids: vec![10],
        exact_policy: ExactContributionPolicy::ExcludeAll,
    };
    assert!(wrong_exact.validate().is_err());
}

#[test]
fn failed_and_cancelled_records_keep_original_target_without_promoting_estimates() {
    for stop in [
        StoppingReason::Cancelled,
        StoppingReason::NumericalFailure("failed next precision check".into()),
    ] {
        let mut result = fixture(false);
        result.stopping_reason = stop;
        let loaded = read_result(&encode_result(&result).unwrap()).unwrap();
        assert!(
            loaded
                .reference(ResultReferenceSelection::Estimate)
                .is_err()
        );
        assert_eq!(
            loaded
                .reference(ResultReferenceSelection::StoredReference)
                .unwrap()
                .coefficients[0]
                .value,
            101.0
        );
        assert!(matches!(
            loaded.comparison().unwrap(),
            ResultComparison::Unavailable(_)
        ));
    }
    let mut absent = fixture(false);
    absent.stored_reference = None;
    assert!(matches!(
        absent.reference(ResultReferenceSelection::StoredReference),
        Err(ResultError::MissingReferenceSource)
    ));
}

#[test]
fn exact_only_floating_values_export_zero_standard_error_without_exactness_upgrade() {
    let mut result = fixture(false);
    result.manifest.sectors.clear();
    let session = QmcSession::democratic(
        IntegrationProblem::new_with_components(
            "exact".into(),
            result.manifest.orders.clone(),
            result.manifest.components.clone(),
            vec![],
            result.manifest.exact_coefficients.clone(),
        )
        .unwrap(),
        QmcSettings::default(),
    )
    .unwrap();
    result.contributions = session.contributions().unwrap();
    result.qmc_design = Some(session.design());
    let exported = result
        .reference(ResultReferenceSelection::Estimate)
        .unwrap();
    assert!(
        exported
            .coefficients
            .iter()
            .all(|c| c.uncertainty == ReferenceUncertainty::StandardError(0.0))
    );
    assert_eq!(exported.validation, ReferenceValidation::Unverified);
    result.requested_tolerance = Some(Tolerance::new(0.0, 0.0).unwrap());
    result.stopping_reason = StoppingReason::TargetReached;
    result.validate().unwrap();
}

#[test]
fn sorting_uses_explicit_component_and_keeps_unavailable_rows_last() {
    let mut result = fixture(false);
    let sort = ResultSectorSort::Magnitude(CoefficientKey {
        order: 0,
        component: Imag,
    });
    assert_eq!(result.sector_order(sort).unwrap(), vec![20, 10]);
    result.contributions.sectors[1].estimate = None;
    result.contributions.sectors[1].uncertainty = UncertaintyStatus::StatisticalFailure {
        reason: "marginal second moment outside range".into(),
    };
    result.validate().unwrap();
    assert_eq!(result.sector_order(sort).unwrap(), vec![10, 20]);
    // The authoritative total remains usable when only a marginal is unavailable.
    assert!(result.reference(ResultReferenceSelection::Estimate).is_ok());
    assert!(matches!(
        result.sector_order(ResultSectorSort::Magnitude(CoefficientKey {
            order: 17,
            component: Real
        })),
        Err(ResultError::UnknownCoefficient(_))
    ));
}

#[test]
fn malformed_coverage_and_claims_reject_without_inventing_missing_data() {
    let result = fixture(false);
    let mut absent = result.clone();
    absent.contributions.total = None;
    absent.contributions.uncertainty = UncertaintyStatus::WaitingForCoverage;
    assert!(absent.validate().is_err());
    absent.contributions.uncertainty = UncertaintyStatus::StatisticalFailure {
        reason: "total overflow".into(),
    };
    absent.stopping_reason = StoppingReason::NumericalFailure("total overflow".into());
    absent.validate().unwrap();
    let mut impossible = result.clone();
    let row = &mut impossible.contributions.sectors[0];
    row.progress.complete_replicas = 0;
    row.used_replicas = 0;
    row.used_points = 0;
    row.estimate = None;
    row.uncertainty = UncertaintyStatus::WaitingForCoverage;
    row.progress.completed_points = 15;
    assert!(impossible.validate().is_err());
    let mut missing_tolerance = result.clone();
    missing_tolerance.stopping_reason = StoppingReason::TargetReached;
    assert!(missing_tolerance.validate().is_err());
    let mut wrong_covariance = result;
    wrong_covariance
        .contributions
        .total
        .as_mut()
        .unwrap()
        .covariance_of_mean
        .pop();
    assert!(wrong_covariance.validate().is_err());
    assert!(wrong_covariance.to_string().contains("Invalid saved"));
}

#[test]
fn reading_large_unstarted_design_does_not_generate_billions_of_random_shifts() {
    let mut result = fixture(true);
    result.stopping_reason = StoppingReason::WorkLimit;
    let design = result.qmc_design.as_mut().unwrap();
    design.settings.points = 2;
    design.settings.shifts = u32::MAX;
    design.allocations[0].points = 2;
    design.allocations[0].shifts = u32::MAX;
    let row = &mut result.contributions.sectors[0];
    row.progress.planned_points = Some(2 * u32::MAX as u64);
    row.progress.planned_replicas = u32::MAX as usize;
    row.progress.completed_points = 0;
    row.progress.complete_replicas = 0;
    row.used_replicas = 0;
    row.used_points = 0;
    row.estimate = None;
    row.uncertainty = UncertaintyStatus::WaitingForCoverage;
    result.contributions.total = None;
    result.contributions.uncertainty = UncertaintyStatus::WaitingForCoverage;
    assert_eq!(
        read_result(&encode_result(&result).unwrap()).unwrap(),
        result
    );
}

#[test]
fn versioned_transport_rejects_legacy_and_unknown_owned_fields() {
    let bytes = encode_result(&fixture(false)).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(read_result(&serde_json::to_vec(&value["result"]).unwrap()).is_err());
    let mut version = value.clone();
    version["version"] = 2.into();
    assert!(read_result(&serde_json::to_vec(&version).unwrap()).is_err());
    let mut extra = value;
    extra["result"]["invented_complete"] = true.into();
    assert!(read_result(&serde_json::to_vec(&extra).unwrap()).is_err());
}

#[test]
fn genuine_pilot_and_partial_native_observations_remain_readable_but_unexportable() {
    for pilot in [false, true] {
        let mut result = fixture(false);
        let problem = IntegrationProblem::new_with_components(
            "partial".into(),
            result.manifest.orders.clone(),
            result.manifest.components.clone(),
            result.manifest.sectors.clone(),
            result.manifest.exact_coefficients.clone(),
        )
        .unwrap();
        let settings = result.qmc_design.as_ref().unwrap().settings.clone();
        let mut session = if pilot {
            QmcSession::adaptive(problem, settings).unwrap()
        } else {
            QmcSession::democratic(problem, settings).unwrap()
        };
        let task = session.next_work().unwrap().unwrap();
        let returned = session
            .worker_context(task.sector_id())
            .unwrap()
            .evaluate(task, |_, out| {
                out.fill(2.0);
                Ok::<_, String>(())
            })
            .unwrap();
        session.submit(returned).unwrap();
        result.contributions = session.diagnostic_observation().unwrap().contributions;
        result.qmc_design = Some(session.design());
        result.stopping_reason = StoppingReason::TimeLimit;
        let loaded = read_result(&encode_result(&result).unwrap()).unwrap();
        assert_eq!(
            loaded
                .contributions
                .sectors
                .iter()
                .map(|r| r.progress.completed_points)
                .sum::<u64>(),
            4
        );
        assert!(!loaded.production_complete());
        assert!(
            loaded
                .reference(ResultReferenceSelection::Estimate)
                .is_err()
        );
        assert!(loaded.contributions.total.is_none());
    }
}

#[test]
fn selected_scope_can_explicitly_exclude_the_entire_folded_exact_vector() {
    let mut result = fixture(true);
    let problem = IntegrationProblem::new_with_components(
        "without-exact".into(),
        result.manifest.orders.clone(),
        result.manifest.components.clone(),
        vec![result.manifest.sectors[0].clone()],
        vec![0.0; 3],
    )
    .unwrap();
    let mut session = QmcSession::democratic(
        problem,
        result.qmc_design.as_ref().unwrap().settings.clone(),
    )
    .unwrap();
    while let Some(task) = session.next_work().unwrap() {
        let returned = session
            .worker_context(task.sector_id())
            .unwrap()
            .evaluate(task, |_, out| {
                out.copy_from_slice(&[1.0, 2.0, 3.0]);
                Ok::<_, String>(())
            })
            .unwrap();
        session.submit(returned).unwrap();
    }
    result.contributions = session.contributions().unwrap();
    result.scope = ResultScope::SelectedSectors {
        sector_ids: vec![10],
        exact_policy: ExactContributionPolicy::ExcludeAll,
    };
    assert_eq!(
        read_result(&encode_result(&result).unwrap())
            .unwrap()
            .contributions
            .total
            .unwrap()
            .mean,
        vec![1.0, 2.0, 3.0]
    );
    assert!(matches!(
        result.reference(ResultReferenceSelection::Estimate),
        Err(ResultError::SelectedScope)
    ));
}

#[test]
fn finite_large_original_values_remain_viewable_when_comparison_exceeds_range() {
    let mut result = fixture(false);
    result.contributions.total.as_mut().unwrap().mean[0] = 1e308;
    let target = &mut result
        .stored_reference
        .as_mut()
        .unwrap()
        .reference
        .coefficients[0];
    target.key = CoefficientKey {
        order: -1,
        component: Real,
    };
    target.value = -1e308;
    let loaded = read_result(&encode_result(&result).unwrap()).unwrap();
    assert!(
        matches!(loaded.comparison().unwrap(),ResultComparison::Unavailable(ResultComparisonUnavailable::NumericRange{quantity,..}) if quantity=="difference")
    );
    assert!(loaded.to_string().contains("exceeds numeric range"));
    assert_eq!(
        loaded
            .reference(ResultReferenceSelection::StoredReference)
            .unwrap()
            .coefficients[0]
            .value,
        -1e308
    );
}
