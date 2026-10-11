//! Accepted diagnostics consume a snapshot, never a running session.
use super::{Cached, number};

fn ratio(value: Option<f64>) -> String {
    value.filter(|v| v.is_finite()).map_or_else(
        || "—".into(),
        |v| {
            if v == 0.0 || (0.001..10000.0).contains(&v) {
                format!("{v:.3}")
            } else {
                format!("{v:.3e}")
            }
        },
    )
}

pub(super) fn line(data: &Cached, order: i32) -> String {
    let weights = data
        .observation
        .weight_diagnostics
        .as_ref()
        .and_then(|report| report.orders.iter().find(|row| row.order == order));
    let norm = data
        .observation
        .contributions
        .total
        .as_ref()
        .map(|estimate| estimate.complex_norm_uncertainty(order));
    let relative = norm
        .as_ref()
        .and_then(|n| n.radial_relative_error)
        .map(number::relative_percent)
        .unwrap_or_else(|| "—".into());
    let warning = if norm.as_ref().is_some_and(|n| n.linearization_warning) {
        "†"
    } else {
        ""
    };
    format!(
        "Accepted ε{} · C {} · max/|I| {} · norm rel {relative}{warning}",
        number::superscript(order),
        ratio(weights.and_then(|w| w.cancellation_ratio)),
        ratio(weights.and_then(|w| w.maximum_ratio)),
    )
}
