"""Display-only transformations of already reviewed native epsilon-zero results.

No estimates, covariance, allocations or native receipts are changed. The square
root rescalings are MC-equivalent heuristics, not a QMC convergence model.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path


def pin(path):
    path = Path(path)
    return {"path": str(path.resolve()), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}


def radial(detail):
    """Delta-method norm uncertainty from the full native covariance of mean."""
    total = detail["native_contributions"]["total"]
    indices = [i for i, order in enumerate(total["orders"]) if order == 0]
    real = [i for i in indices if total["components"][i] == "Real"]
    imag = [i for i in indices if total["components"][i] == "Imag"]
    assert len(real) == len(imag) == 1
    r, i = real[0], imag[0]
    n = len(total["mean"])
    re, im = total["mean"][r], total["mean"][i]
    assert [re, im] == detail["mean_epsilon0"]
    covariance = total["covariance_of_mean"]
    rr, ri, ir, ii = (covariance[r*n+r], covariance[r*n+i],
                      covariance[i*n+r], covariance[i*n+i])
    assert all(math.isfinite(x) for x in [re, im, rr, ri, ir, ii])
    assert rr >= 0 and ii >= 0 and ri == ir
    magnitude = math.hypot(re, im)
    result = {"mean_magnitude": magnitude, "covariance_block": [[rr, ri], [ir, ii]],
              "absolute_se": None, "relative_se": None,
              "method": "delta_method_radial_full_complex_covariance"}
    if magnitude == 0:
        result["status"] = "undefined_zero_mean"
        return result
    a, b = re / magnitude, im / magnitude
    variance = math.fsum([a*a*rr, 2*a*b*ri, b*b*ii])
    if variance < 0:
        result["status"] = "unavailable_negative_projected_variance"
        return result
    result.update(absolute_se=math.sqrt(variance),
                  relative_se=math.sqrt(variance)/magnitude,
                  status="available")
    result["linearization"] = (
        "large_uncertainty_relative_joint_se_ge_0.1" if
        detail["joint_se_epsilon0"] / magnitude >= 0.1 else
        "first_order_delta_method")
    return result


def apply(data, targets):
    accepted = [d for d in data["integration_details"]
                if d["status"] == "native_complete_record_estimate"
                and d.get("table_eligible", True)]
    sampler_map = {(t["id"], m["id"], s["id"]): s
                   for t in data["topologies"] for m in t["methods"]
                   for s in m["samplers"]}
    for sampler in sampler_map.values():
        sampler["normalized_metrics"] = dict.fromkeys(
            ["work_radial_relative_se", "work_joint_se", "time_radial_relative_se",
             "time_reported_sector_calls_million"])
        sampler["weight_metrics"] = {
            "status": "not_measured_by_campaign_executable",
            "absolute_weight_integral_over_norm": None,
            "maximum_observed_weight_over_norm": None}
    time_windows = {}
    for topology in data["topologies"]:
        rows = [d for d in accepted if d["topology"] == topology["id"]
                and d["arm"] == "fixed_time"]
        times = [d["actual_seconds"] for d in rows]
        assert all(math.isfinite(t) and t > 0 for t in times)
        ratio = max(times) / min(times) if times else None
        near_target = bool(times) and all(480 <= t <= 750 for t in times)
        time_windows[topology["id"]] = {
            "accepted_rows": len(rows), "minimum_seconds": min(times) if times else None,
            "maximum_seconds": max(times) if times else None, "maximum_minimum_ratio": ratio,
            "all_seconds_within_480_to_750": near_target,
            "eligible": near_target and ratio <= targets["max_min_native_time_ratio"]}
        target = targets["targets"].get(topology["id"])
        topology["display_work_target_average_per_sector"] = (
            target["target_average_sector_evaluations"] if target else None)
    for detail in accepted:
        key = (detail["topology"], detail["method"], detail["sampler"])
        sampler = sampler_map[key]
        radial_result = radial(detail)
        detail["radial_uncertainty_epsilon0"] = radial_result
        sectors = len(detail["manifest"]["sectors"])
        assert sectors > 0
        ids = {s["id"] for s in detail["manifest"]["sectors"]}
        assert len(ids) == sectors
        native_ids = {s["progress"]["id"] for s in detail["native_contributions"]["sectors"]}
        assert ids == native_ids
        normalized = {"scope": "MC_equivalent_heuristic_not_QMC_convergence_law",
                      "stochastic_sectors": sectors,
                      "actual_average_accepted_evaluations_per_sector":
                          detail["accepted_sector_evaluations"] / sectors,
                      "factor": None, "radial_relative_se": None, "joint_se": None}
        if detail["arm"] == "fixed_time":
            normalized["target_seconds"] = targets["time_target_seconds"]
            eligible = time_windows[detail["topology"]]["eligible"]
            normalized["status"] = "available" if eligible else "time_comparison_window_refused"
            factor = math.sqrt(detail["actual_seconds"] / targets["time_target_seconds"])
            metric_key = "time_radial_relative_se"
            operational = detail["native_operational_timings"]
            calls = operational["evaluations"]
            assert calls == operational["diagnostics"]["evaluations"]
            assert calls == sum(s["evaluations"] for s in operational["sectors"])
            assert calls >= detail["accepted_sector_evaluations"]
            detail["logical_sector_calls"] = {
                "reported": calls,
                "coverage": "observed_completed_callbacks_including_pilots_and_discarded_work; unpublished_tail_possible",
                "accepted_statistical_evaluations": detail["accepted_sector_evaluations"],
                "backend_calls": {tier: operational["diagnostics"][tier+"_timing"]["calls"]
                                  for tier in ["f64", "double_float", "arbitrary"]}}
            sampler["normalized_metrics"]["time_reported_sector_calls_million"] = f"≥ {calls/1e6:.3g}"
        else:
            target = targets["targets"].get(detail["topology"])
            eligible = target is not None
            normalized["status"] = "available" if eligible else "target_pending"
            factor = None
            if target:
                average = target["target_average_sector_evaluations"]
                normalized["target_average_evaluations_per_sector"] = average
                normalized["target_total_evaluations"] = sectors * average
                factor = math.sqrt(detail["accepted_sector_evaluations"] / (sectors * average))
            metric_key = "work_radial_relative_se"
        if eligible:
            normalized["factor"] = factor
            normalized["joint_se"] = detail["joint_se_epsilon0"] * factor
            if radial_result["status"] == "available":
                normalized["radial_relative_se"] = radial_result["relative_se"] * factor
                warning = "†" if radial_result["linearization"].startswith("large_uncertainty") else ""
                sampler["normalized_metrics"][metric_key] = f'{normalized["radial_relative_se"]:.3g}{warning}'
            else:
                normalized["status"] = radial_result["status"]
            if detail["arm"] == "matched_work":
                sampler["normalized_metrics"]["work_joint_se"] = f'{normalized["joint_se"]:.3g}'
        detail["display_normalization"] = normalized
    data["display_normalization"] = {
        "scope": "MC-equivalent display heuristic only; not a QMC convergence law",
        "relative_error": "delta-method sigma(norm(complex mean))/norm(complex mean), full native covariance",
        "raw_estimates_and_covariance_unchanged": True,
        "targets": targets, "time_windows": time_windows,
        "large_uncertainty_flag_threshold_relative_joint_se": 0.1}
    return data


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input")
    parser.add_argument("targets")
    parser.add_argument("output")
    args = parser.parse_args()
    data = apply(json.loads(Path(args.input).read_text()), json.loads(Path(args.targets).read_text()))
    data["display_normalization"]["source"] = pin(__file__)
    data["display_normalization"]["input"] = pin(args.input)
    data["display_normalization"]["target_receipt"] = pin(args.targets)
    Path(args.output).write_text(json.dumps(data, indent=2, allow_nan=False)+"\n")
    print(json.dumps(pin(args.output)))
