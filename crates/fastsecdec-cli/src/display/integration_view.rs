//! Cached views of native observations. Sorting changes presentation only.
use super::{IntegrationWorkerActivity, memory, number};
use crate::terminal_policy::ColorPolicy;
use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use fastsecdec::{
    integration::{
        IntegrationObservation, LiveObservation, LiveSource, LiveStatus, OperationalMetrics,
        SectorOperationalMetrics,
    },
    status::{CoefficientComponent, EvaluationDiagnostics},
};
use ratatui::{
    layout::{Position, Rect},
    text::Line,
    widgets::TableState,
};

mod accepted;
pub(super) use accepted::CompletedAllocation;
mod metrics;
mod render;
mod sectors;
#[cfg(test)]
mod tests;
mod weights;
pub(super) use render::render;
use std::{cmp::Ordering, collections::BTreeSet};

#[derive(Clone)]
pub(super) struct Cached {
    pub observation: IntegrationObservation,
    pub previous_completed: Option<CompletedAllocation>,
    pub live: Option<LiveObservation>,
    pub operational: OperationalMetrics,
    pub stability_mode: fastsecdec::kernel::StabilityMode,
    pub memory: memory::Snapshot,
    pub elapsed: f64,
    pub scope: fastsecdec::results::ResultScope,
    pub workers: Vec<IntegrationWorkerActivity>,
    pub serial: Option<crate::driver::serial::SerialRunSnapshot>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sort {
    Id,
    Points,
    Real,
    Imag,
    RealError,
    ImagError,
    Time,
    RelativeError,
    F64Mean,
    Maximum,
    F64,
    Double,
    Arb,
    Unstable,
}
impl Sort {
    const ALL: [Self; 14] = [
        Self::Id,
        Self::Points,
        Self::Real,
        Self::Imag,
        Self::RealError,
        Self::ImagError,
        Self::Time,
        Self::RelativeError,
        Self::F64Mean,
        Self::Maximum,
        Self::F64,
        Self::Double,
        Self::Arb,
        Self::Unstable,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Id => "sector",
            Self::Points => "points",
            Self::Real => "Re",
            Self::Imag => "Im",
            Self::RealError => "Re error",
            Self::ImagError => "Im error",
            Self::Time => "worker time",
            Self::RelativeError => "relative error",
            Self::F64Mean => "f64 mean time",
            Self::Maximum => "Max |wgt|",
            Self::F64 => "f64 fraction",
            Self::Double => "DoubleFloat fraction",
            Self::Arb => "Arb fraction",
            Self::Unstable => "unstable fraction",
        }
    }
}

pub(super) struct View {
    pub order: Option<i32>,
    selected_id: Option<u64>,
    sort: Sort,
    descending: bool,
    table: TableState,
    hits: HitRegions,
}
impl Default for View {
    fn default() -> Self {
        Self {
            order: None,
            selected_id: None,
            sort: Sort::Id,
            descending: false,
            table: TableState::default(),
            hits: HitRegions::default(),
        }
    }
}
#[derive(Default)]
struct HitRegions {
    headers: Vec<(Rect, Sort)>,
    rows: Vec<(Rect, u64)>,
    body: Rect,
}

impl View {
    fn orders(data: &Cached) -> Vec<i32> {
        data.observation
            .contributions
            .orders
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }
    fn order(&self, data: &Cached) -> i32 {
        let orders = Self::orders(data);
        self.order
            .filter(|order| orders.contains(order))
            .or_else(|| orders.last().copied())
            .unwrap_or(0)
    }
    pub fn key(&mut self, key: KeyCode, data: &Cached) -> bool {
        let ids = self.ids(data);
        let position = self
            .selected_id
            .and_then(|id| ids.iter().position(|x| *x == id))
            .unwrap_or(0);
        let selected = match key {
            KeyCode::Up => Some(position.saturating_sub(1)),
            KeyCode::Down => Some(position.saturating_add(1).min(ids.len().saturating_sub(1))),
            KeyCode::PageUp => Some(position.saturating_sub(10)),
            KeyCode::PageDown => Some(position.saturating_add(10).min(ids.len().saturating_sub(1))),
            KeyCode::Home => Some(0),
            KeyCode::End => Some(ids.len().saturating_sub(1)),
            _ => None,
        };
        if let Some(position) = selected {
            self.selected_id = ids.get(position).copied();
            return true;
        }
        match key {
            KeyCode::Left | KeyCode::Right => {
                let orders = Self::orders(data);
                let index = orders
                    .iter()
                    .position(|order| *order == self.order(data))
                    .unwrap_or(0);
                let next = if key == KeyCode::Left {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(orders.len().saturating_sub(1))
                };
                self.order = orders.get(next).copied();
            }
            KeyCode::Tab | KeyCode::Char('s') | KeyCode::BackTab => {
                let index = Sort::ALL
                    .iter()
                    .position(|sort| *sort == self.sort)
                    .unwrap();
                let next = if key == KeyCode::BackTab {
                    (index + Sort::ALL.len() - 1) % Sort::ALL.len()
                } else {
                    (index + 1) % Sort::ALL.len()
                };
                self.sort = Sort::ALL[next];
                self.descending = self.sort != Sort::Id;
            }
            KeyCode::Char('r') => self.descending = !self.descending,
            _ => return false,
        }
        true
    }
    pub(super) fn mouse(&mut self, event: MouseEvent, data: &Cached) -> bool {
        let point = Position::new(event.column, event.row);
        match event.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                if let Some((_, sort)) = self
                    .hits
                    .headers
                    .iter()
                    .find(|(area, _)| area.contains(point))
                {
                    if self.sort == *sort {
                        self.descending = !self.descending;
                    } else {
                        self.sort = *sort;
                        self.descending = self.sort != Sort::Id;
                    }
                    return true;
                }
                if let Some((_, id)) = self.hits.rows.iter().find(|(area, _)| area.contains(point))
                {
                    self.selected_id = Some(*id);
                    return true;
                }
                false
            }
            MouseEventKind::ScrollUp if self.hits.body.contains(point) => {
                self.key(KeyCode::Up, data)
            }
            MouseEventKind::ScrollDown if self.hits.body.contains(point) => {
                self.key(KeyCode::Down, data)
            }
            _ => false,
        }
    }
    fn ids(&self, data: &Cached) -> Vec<u64> {
        let mut ids = data
            .observation
            .contributions
            .sectors
            .iter()
            .map(|row| row.progress.id)
            .collect::<Vec<_>>();
        let indices = indices(data, self.order(data));
        ids.sort_by(|&a, &b| {
            let compare = if self.sort == Sort::Id {
                a.cmp(&b)
            } else if self.sort == Sort::Points {
                estimate(data, Some(a)).2.cmp(&estimate(data, Some(b)).2)
            } else {
                let metric = |id| {
                    let (mean, error, _) = estimate(data, Some(id));
                    let op = operation(data, id);
                    let component = if matches!(self.sort, Sort::Imag | Sort::ImagError) {
                        indices.1
                    } else {
                        indices.0
                    };
                    match self.sort {
                        Sort::Real | Sort::Imag => {
                            component.and_then(|i| mean.and_then(|v| v.get(i))).copied()
                        }
                        Sort::RealError | Sort::ImagError => component
                            .and_then(|i| error.and_then(|v| v.get(i)))
                            .copied(),
                        Sort::Time => op.map(|o| o.worker_seconds),
                        Sort::RelativeError => relative_error(data, Some(id), indices),
                        Sort::F64Mean => {
                            op.and_then(|o| super::diagnostic_summary::f64_mean(&o.diagnostics))
                        }
                        Sort::Maximum => op.and_then(|o| {
                            o.maximum_weighted_contribution
                                .get(&self.order(data))
                                .copied()
                        }),
                        Sort::F64 | Sort::Double | Sort::Arb | Sort::Unstable => op.and_then(|o| {
                            let n = o.diagnostics.evaluations;
                            let counts = counts(&o.diagnostics);
                            let c = match self.sort {
                                Sort::F64 => counts[0],
                                Sort::Double => counts[1],
                                Sort::Arb => counts[2],
                                _ => counts[3],
                            };
                            (n > 0).then_some(c as f64 / n as f64)
                        }),
                        Sort::Id | Sort::Points => unreachable!(),
                    }
                    .filter(|v| v.is_finite())
                };
                match (metric(a), metric(b)) {
                    (Some(a), Some(b)) => a.total_cmp(&b),
                    (Some(_), None) => return Ordering::Less,
                    (None, Some(_)) => return Ordering::Greater,
                    (None, None) => Ordering::Equal,
                }
            };
            (if self.descending {
                compare.reverse()
            } else {
                compare
            })
            .then_with(|| a.cmp(&b))
        });
        ids
    }
}

fn indices(data: &Cached, order: i32) -> (Option<usize>, Option<usize>) {
    let report = &data.observation.contributions;
    let find = |component| {
        report
            .orders
            .iter()
            .zip(&report.components)
            .position(|(&o, &c)| o == order && c == component)
    };
    (
        find(CoefficientComponent::Real),
        find(CoefficientComponent::Imag),
    )
}
fn operation(data: &Cached, id: u64) -> Option<&SectorOperationalMetrics> {
    data.operational.sectors.iter().find(|row| row.id == id)
}
type Estimate<'a> = (Option<&'a [f64]>, Option<&'a [f64]>, u64);
fn estimate(data: &Cached, id: Option<u64>) -> Estimate<'_> {
    if let Some(live) = &data.live {
        let row = if let Some(id) = id {
            live.sectors
                .iter()
                .find(|row| row.id == id)
                .map(|row| &row.estimate)
        } else {
            Some(&live.total)
        };
        return row.map_or((None, None, 0), |e| {
            (e.mean.as_deref(), e.standard_error.as_deref(), e.points)
        });
    }
    let report = &data.observation.contributions;
    let (value, points) = if let Some(id) = id {
        report
            .sectors
            .iter()
            .find(|row| row.progress.id == id)
            .map_or((None, 0), |row| (row.estimate.as_ref(), row.used_points))
    } else {
        (
            report.total.as_ref(),
            report.sectors.iter().map(|row| row.used_points).sum(),
        )
    };
    (
        value.map(|v| v.mean.as_slice()),
        value.map(|v| v.standard_error.as_slice()),
        points,
    )
}
fn is_qmc(data: &Cached) -> bool {
    matches!(
        data.observation.snapshot.method,
        fastsecdec::status::IntegrationMethod::DemocraticQmc
            | fastsecdec::status::IntegrationMethod::AdaptiveQmc
    )
}
fn shifts(data: &Cached, id: Option<u64>) -> usize {
    if let Some(live) = &data.live {
        id.map_or(Some(&live.total), |id| {
            live.sectors
                .iter()
                .find(|s| s.id == id)
                .map(|s| &s.estimate)
        })
        .map_or(0, |e| e.replicas)
    } else if let Some(id) = id {
        data.observation
            .contributions
            .sectors
            .iter()
            .find(|s| s.progress.id == id)
            .map_or(0, |s| s.used_replicas)
    } else {
        data.observation
            .contributions
            .sectors
            .iter()
            .map(|s| s.used_replicas)
            .min()
            .unwrap_or(0)
    }
}
fn used_points(data: &Cached, id: Option<u64>) -> String {
    let points = estimate(data, id).2;
    if is_qmc(data) {
        format!(
            "{} / {}",
            number::compact_count(points),
            number::compact_count(shifts(data, id) as u64)
        )
    } else {
        number::compact_count(points)
    }
}
fn value(data: &Cached, id: Option<u64>, index: Option<usize>) -> String {
    let Some(index) = index else {
        return "—".into();
    };
    let (mean, error, _) = estimate(data, id);
    mean.and_then(|v| v.get(index)).map_or_else(
        || {
            let status = data
                .live
                .as_ref()
                .and_then(|live| {
                    id.map_or(Some(&live.total), |id| {
                        live.sectors
                            .iter()
                            .find(|s| s.id == id)
                            .map(|s| &s.estimate)
                    })
                })
                .map(|e| e.status);
            match status {
                Some(LiveStatus::NumericalRange) => "numerical range unavailable",
                Some(LiveStatus::NotSampled) => "not sampled",
                _ => "unavailable",
            }
            .into()
        },
        |&value| number::uncertainty(value, error.and_then(|v| v.get(index)).copied()),
    )
}
fn counts(d: &EvaluationDiagnostics) -> [u64; 4] {
    [
        d.f64_points,
        d.double_float_points,
        d.arbitrary_points,
        d.unstable_points,
    ]
}
fn fractions(d: Option<&EvaluationDiagnostics>) -> [String; 4] {
    d.map_or_else(
        || std::array::from_fn(|_| "—".into()),
        super::diagnostic_summary::fractions,
    )
}
fn relative_error(
    data: &Cached,
    id: Option<u64>,
    (real, imag): (Option<usize>, Option<usize>),
) -> Option<f64> {
    let (means, errors, _) = estimate(data, id);
    let means = means?;
    let errors = errors?;
    let component =
        |values: &[f64], index: Option<usize>| index.map_or(Some(0.0), |i| values.get(i).copied());
    let mean = component(means, real)?.hypot(component(means, imag)?);
    let error = component(errors, real)?.hypot(component(errors, imag)?);
    (mean > 0.0 && mean.is_finite() && error.is_finite())
        .then(|| error / mean)
        .filter(|v| v.is_finite())
}
fn relative(data: &Cached, id: Option<u64>, indices: (Option<usize>, Option<usize>)) -> String {
    relative_error(data, id, indices).map_or_else(|| "—".into(), number::relative_percent)
}
fn f64_time(op: Option<&SectorOperationalMetrics>) -> String {
    op.and_then(|o| super::diagnostic_summary::f64_mean(&o.diagnostics))
        .map(number::sample_duration)
        .unwrap_or_else(|| "—".into())
}
fn maximum(op: Option<&SectorOperationalMetrics>, order: i32) -> String {
    op.and_then(|o| o.maximum_weighted_contribution.get(&order))
        .map(|&v| number::scientific(v))
        .unwrap_or_else(|| "—".into())
}
fn source(data: &Cached) -> &'static str {
    match data.live.as_ref().map(|live| live.source) {
        Some(LiveSource::CurrentIteration) => "Current iteration · provisional",
        Some(LiveSource::SinceResume) => "Since resume · provisional",
        Some(LiveSource::CompleteLattices) => "Complete lattices · provisional",
        None => "Accepted contributions",
    }
}
fn timing(data: &Cached) -> Vec<String> {
    let mut rows = vec![format!(
        "This invocation · wall {} · process CPU {}",
        number::sample_duration(data.elapsed),
        data.memory
            .process_cpu_seconds
            .map(number::sample_duration)
            .unwrap_or_else(|| "unavailable".into())
    )];
    rows.extend(
        super::diagnostic_summary::diagnostic_summary_with_duration(
            &data.operational,
            data.stability_mode,
            number::sample_duration,
        )
        .into_iter()
        .map(|[name, value]| format!("{name}: {value}")),
    );
    rows
}

// Reuse tabled's Unicode-aware wrapping for multiline cells, avoiding byte or
// character-count approximations of terminal widths.
fn wrapped(text: &str, width: u16) -> Vec<Line<'static>> {
    use tabled::{
        builder::Builder,
        settings::{Padding, Style, Width},
    };
    let mut builder = Builder::new();
    builder.push_record([text]);
    let mut table = builder.build();
    table
        .with(Style::empty())
        .with(Padding::zero())
        .with(Width::wrap(usize::from(width.max(1))).keep_words(true));
    table
        .to_string()
        .lines()
        .map(|s| Line::from(s.trim_end().to_owned()))
        .collect()
}

pub(super) fn plain(data: &Cached, view: &View) -> String {
    use tabled::{builder::Builder, settings::Style};
    let order = view.order(data);
    let (real, imag) = indices(data, order);
    let mut builder = Builder::new();
    builder.push_record([
        "Sector",
        if is_qmc(data) {
            "Used points / complete shifts"
        } else {
            "Points"
        },
        "Real",
        "Imag",
        "Rel error",
        "f64 mean",
        "Max |wgt|",
        "f64",
        "DoubleFloat",
        super::diagnostic_summary::arbitrary_label(data.stability_mode),
        "Unstable",
    ]);
    for id in view.ids(data) {
        let op = operation(data, id);
        let fs = fractions(op.map(|o| &o.diagnostics));
        builder.push_record([
            id.to_string(),
            used_points(data, Some(id)),
            value(data, Some(id), real),
            value(data, Some(id), imag),
            relative(data, Some(id), (real, imag)),
            f64_time(op),
            maximum(op, order),
            fs[0].clone(),
            fs[1].clone(),
            fs[2].clone(),
            fs[3].clone(),
        ]);
    }
    let mut table = builder.build();
    table.with(Style::modern());
    format!(
        "{} · {:?} {:?} · {}\nε{} sum Re {} · Im {}\n{}: Re {} · Im {}\n{}\nCoverage: {}\n{}\n{}{}",
        data.scope,
        data.observation.snapshot.method,
        data.observation.snapshot.stage,
        source(data),
        number::superscript(order),
        value(data, None, real),
        value(data, None, imag),
        accepted::label(data),
        accepted::value(data, real),
        accepted::value(data, imag),
        accepted::explanation(data),
        if is_qmc(data) {
            format!(
                "{} used points / {} complete shifts",
                number::compact_count(estimate(data, None).2),
                number::compact_count(shifts(data, None) as u64)
            )
        } else {
            format!(
                "{} sampled points",
                number::compact_count(estimate(data, None).2)
            )
        },
        table,
        timing(data).join("\n"),
        data.serial
            .as_ref()
            .map_or_else(String::new, |status| format!(
                "\n{}",
                super::serial_view::plain(status)
            ))
    )
}
