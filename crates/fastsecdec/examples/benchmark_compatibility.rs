//! Recheck accepted benchmark observations without generating or sampling.
//! Usage: benchmark_compatibility campaign.json references.json output.json
use fastsecdec::{integration::VectorEstimate, status::CoefficientComponent};
use numerica::{
    domains::rational::{Q, Rational},
    tensors::matrix::Matrix,
};
use serde_json::{Value, json};
use std::{collections::BTreeSet, error::Error, fs};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn block(estimate: &VectorEstimate, order: i32) -> Option<([f64; 2], [f64; 4])> {
    let index = [CoefficientComponent::Real, CoefficientComponent::Imag].map(|component| {
        estimate
            .orders
            .iter()
            .zip(&estimate.components)
            .position(|(&o, &c)| o == order && c == component)
    });
    let [Some(r), Some(i)] = index else {
        return None;
    };
    let n = estimate.orders.len();
    Some((
        [estimate.mean[r], estimate.mean[i]],
        [
            estimate.covariance_of_mean[r * n + r],
            estimate.covariance_of_mean[r * n + i],
            estimate.covariance_of_mean[i * n + r],
            estimate.covariance_of_mean[i * n + i],
        ],
    ))
}

/// The matrix is the reported covariance, never an invented diagonal model.
/// Native exact-rational conversion/solve avoids a second floating-point
/// inversion error; it does not certify the estimated covariance itself.
fn diagnostic(delta: [f64; 2], covariance: [f64; 4], scale: f64) -> Result<Value> {
    let c = covariance
        .map(Rational::try_from)
        .into_iter()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let symmetric = c[1] == c[2];
    let matrix = Matrix::from_linear(c.clone(), 2, 2, Q)?;
    let determinant = matrix.det()?;
    let trace = covariance[0] + covariance[3];
    let positive = symmetric && c[0] > 0 && determinant > 0;
    // A warning scale, not a bound, a covariance regularizer, or extra SE.
    let rounding_scale = 32.0 * f64::EPSILON * scale;
    let sensitive = trace <= 0.0 || determinant.to_f64() / trace <= rounding_scale.powi(2);
    let distance = if positive {
        let rhs = Matrix::from_linear(
            delta
                .map(Rational::try_from)
                .into_iter()
                .collect::<std::result::Result<Vec<_>, _>>()?,
            2,
            1,
            Q,
        )?;
        let solved = matrix.solve(&rhs)?;
        Some(
            (&rhs[(0, 0)] * &solved[(0, 0)] + &rhs[(1, 0)] * &solved[(1, 0)])
                .to_f64()
                .sqrt(),
        )
    } else {
        None
    };
    Ok(
        json!({"difference":delta, "covariance_of_difference":covariance,
        "difference_norm":delta[0].hypot(delta[1]),
        "joint_se":(trace >= 0.0).then(|| trace.sqrt()),
        "mahalanobis_distance":distance,
        "positive_definite":positive, "symmetric":symmetric,
        "rounding_sensitive":sensitive, "rounding_warning_scale":rounding_scale,
        "interpretation":"Descriptive estimated-covariance diagnostic, not a calibrated p-value; no error inflation or numerical-bias bound"}),
    )
}

fn estimate(row: &Value) -> Result<VectorEstimate> {
    let value: VectorEstimate =
        serde_json::from_value(row["native_contributions"]["total"].clone())?;
    value.validate()?;
    let native = fastsecdec::results::read_result(&fs::read(
        row["native_result"]["path"]
            .as_str()
            .ok_or("missing native result path")?,
    )?)?;
    if native.contributions.total.as_ref() != Some(&value) {
        return Err("campaign estimate differs from saved native result".into());
    }
    if row["point_sha256"].as_str().is_none_or(str::is_empty) {
        return Err("missing point association".into());
    }
    Ok(value)
}

fn label(row: &Value) -> Value {
    json!({"topology":row["topology"],"method":row["method"],"sampler":row["sampler"],
        "arm":row["arm"],"seed":row["seed"],"native_result":row["native_result"]})
}

fn comparison_eligible(row: &Value) -> bool {
    if row["status"] == "native_complete_record_estimate" {
        return true;
    }
    let eligibility = &row["normalized_work_eligibility"];
    let actual = row["accepted_sector_evaluations"].as_u64();
    let planned = eligibility["planned_accepted_evaluations"].as_u64();
    row["status"] == "incomplete_requested_work"
        && row["arm"] == "matched_work"
        && row["sampler"] == "discrete_mc"
        && eligibility["status"] == "native_clean_cap_partial_planned_allocation"
        && eligibility["strict_campaign_status"] == row["status"]
        && eligibility["display_selected"] == true
        && eligibility["allocation_target_complete"] == false
        && eligibility["actual_accepted_evaluations"] == row["accepted_sector_evaluations"]
        && actual.zip(planned).is_some_and(|(a, p)| a > 0 && a < p)
        && row["allocation_target_complete"] == false
        && row["effective_settings"]["double_points"] == false
        && row["native_scope"] == "FullIntegral"
        && row["native_contributions"]["stage"] == "Production"
        && row["native_contributions"]["uncertainty"] == "Available"
        && (row["stopping_reason"] == "Cancelled" || row["stopping_reason"] == "WorkLimit")
}

fn main() -> Result<()> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        return Err("expected campaign.json references.json output.json".into());
    }
    let campaign: Value = serde_json::from_slice(&fs::read(&args[0])?)?;
    let references: Value = serde_json::from_slice(&fs::read(&args[1])?)?;
    let rows = campaign["integration_details"]
        .as_array()
        .ok_or("missing observations")?
        .iter()
        .filter(|r| comparison_eligible(r))
        .collect::<Vec<_>>();
    let mut comparisons = vec![];
    let mut reference_comparisons = vec![];
    for (index, row) in rows.iter().enumerate() {
        let actual = estimate(row)?;
        let orders = actual.orders.iter().copied().collect::<BTreeSet<_>>();
        let scale = actual
            .mean
            .iter()
            .copied()
            .map(f64::abs)
            .fold(0.0, f64::max);
        if let Some(reference) = references["rows"]
            .as_array()
            .ok_or("missing references")?
            .iter()
            .find(|r| r["topology"] == row["topology"])
        {
            if reference["card"]["sha256"] != row["point_sha256"] {
                return Err("reference inventory has a different point/card association".into());
            }
            for &order in &orders {
                let Some(expected) = reference["independent_reference"]["coefficients"]
                    [order.to_string()]
                .as_array() else {
                    continue;
                };
                let Some((mean, covariance)) = block(&actual, order) else {
                    continue;
                };
                let expected = [
                    expected[0].as_f64().ok_or("reference real")?,
                    expected[1].as_f64().ok_or("reference imaginary")?,
                ];
                reference_comparisons.push(json!({"run":label(row),"order":order,"reference":expected,
                    "reference_uncertainty":"Rounded reference: no certified arithmetic bound; reference roundoff is not added to stochastic covariance",
                    "diagnostic":diagnostic([mean[0]-expected[0],mean[1]-expected[1]], covariance,
                        scale.max(expected[0].abs()).max(expected[1].abs()))?}));
            }
        }
        for other in rows
            .iter()
            .skip(index + 1)
            .filter(|other| other["topology"] == row["topology"])
        {
            if row["seed"] == other["seed"] {
                return Err("same seed in compared confirmation runs".into());
            }
            if row["point_sha256"] != other["point_sha256"] {
                return Err("different input card in one topology".into());
            }
            let b = estimate(other)?;
            let other_scale = b.mean.iter().copied().map(f64::abs).fold(0.0, f64::max);
            for &order in &orders {
                let (Some((a_mean, a_cov)), Some((b_mean, b_cov))) =
                    (block(&actual, order), block(&b, order))
                else {
                    continue;
                };
                let covariance = std::array::from_fn(|i| a_cov[i] + b_cov[i]);
                comparisons.push(json!({"left":label(row),"right":label(other),"order":order,
                    "sampling_relation":"Distinct recorded seeds; covariance sum assumes independent run randomizations. No shared-point pairing or empirical cross-run covariance is claimed",
                    "diagnostic":diagnostic([a_mean[0]-b_mean[0],a_mean[1]-b_mean[1]],covariance,scale.max(other_scale))?}));
            }
        }
    }
    fs::write(
        &args[2],
        serde_json::to_vec_pretty(&json!({"schema_version":1,
        "scope":"Accepted production estimates only; all available complex Laurent blocks, including epsilon zero",
        "matrix_backend":"Numerica Rational::try_from(f64), Matrix::det and Matrix::solve; exact operations on the reported binary64 entries",
        "rounding_policy":"32 epsilon times the full-vector coefficient scale is only a warning heuristic. It changes neither estimates nor covariance; unresolved or sensitive tests are not confirmations",
        "accepted_runs":rows.len(),"comparisons":comparisons,"reference_comparisons":reference_comparisons}))?,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_work_selection_preserves_strict_status_and_requires_reviewed_full_estimate() {
        assert!(comparison_eligible(
            &json!({"status":"native_complete_record_estimate"})
        ));
        let row = json!({
            "status":"incomplete_requested_work", "arm":"matched_work", "sampler":"discrete_mc",
            "accepted_sector_evaluations":80, "allocation_target_complete":false,
            "effective_settings":{"double_points":false}, "native_scope":"FullIntegral",
            "native_contributions":{"stage":"Production","uncertainty":"Available"},
            "stopping_reason":"Cancelled",
            "normalized_work_eligibility":{
                "status":"native_clean_cap_partial_planned_allocation",
                "strict_campaign_status":"incomplete_requested_work", "display_selected":true,
                "allocation_target_complete":false,
                "planned_accepted_evaluations":100, "actual_accepted_evaluations":80
            }
        });
        assert!(comparison_eligible(&row));
        for (path, value) in [
            (
                "/normalized_work_eligibility/display_selected",
                json!(false),
            ),
            ("/normalized_work_eligibility/status", json!("unreviewed")),
            ("/status", json!("numerical_failure")),
            ("/native_contributions/uncertainty", json!("Unavailable")),
            ("/native_scope", json!("SectorSubset")),
            ("/effective_settings/double_points", json!(true)),
            ("/accepted_sector_evaluations", json!(0)),
        ] {
            let mut mutated = row.clone();
            *mutated.pointer_mut(path).unwrap() = value;
            assert!(!comparison_eligible(&mutated), "{path}");
        }
        assert_eq!(row["status"], "incomplete_requested_work");
    }

    #[test]
    fn native_covariance_solve_preserves_correlations_and_refuses_singular_inverse() {
        let correlated = diagnostic([1., 1.], [1., 0.5, 0.5, 1.], 1.).unwrap();
        assert!(
            (correlated["mahalanobis_distance"].as_f64().unwrap() - (4_f64 / 3.).sqrt()).abs()
                < 1e-14
        );
        let singular = diagnostic([1., 1.], [1., 1., 1., 1.], 1.).unwrap();
        assert!(singular["mahalanobis_distance"].is_null());
        assert_eq!(singular["rounding_sensitive"], true);
        let indefinite = diagnostic([1., 1.], [1., 2., 2., 1.], 1.).unwrap();
        assert_eq!(indefinite["positive_definite"], false);
        let tiny = diagnostic([1e-16, 0.], [1e-34, 0., 0., 1e-34], 1.).unwrap();
        assert_eq!(tiny["rounding_sensitive"], true);
    }
}
