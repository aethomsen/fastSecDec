//! Native Ratatui layout of one immutable dashboard observation.
use super::*;
use crate::display::{GOLD, TEAL};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Gauge, Paragraph, Row, Table},
};

pub(super) const BLUE: Color = Color::Rgb(115, 171, 255);
pub(super) const PURPLE: Color = Color::Rgb(194, 151, 255);
pub(super) const RED: Color = Color::Rgb(255, 132, 139);
pub(super) fn card(
    title: impl Into<Line<'static>>,
    color: Color,
    colors: ColorPolicy,
) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(colors.foreground(color))
        .title(title)
}
pub(super) fn bold(colors: ColorPolicy, color: Color) -> Style {
    colors.foreground(color).add_modifier(Modifier::BOLD)
}
pub(super) fn right(text: impl Into<String>) -> ratatui::widgets::Cell<'static> {
    ratatui::widgets::Cell::from(Line::from(text.into()).alignment(Alignment::Right))
}
pub(super) fn ram(value: Option<u64>) -> String {
    memory::bytes(value)
}

pub(in crate::display) fn render(
    frame: &mut Frame<'_>,
    data: &Cached,
    view: &mut View,
    colors: ColorPolicy,
    cancelling: bool,
) {
    view.hits = HitRegions::default();
    let area = frame.area();
    let order = view.order(data);
    view.order = Some(order);
    let ids = view.ids(data);
    let selected = view
        .selected_id
        .and_then(|id| ids.iter().position(|&x| x == id))
        .or((!ids.is_empty()).then_some(0));
    view.selected_id = selected.and_then(|i| ids.get(i).copied());
    view.table.select(selected);
    if area.width < 65 || area.height < 24 {
        compact(frame, area, data, view, colors, cancelling);
        return;
    }
    let short = area.height < 28;
    let header_height = if short { 4 } else { 5 };
    let serial_height = if data.serial.is_some() {
        if short { 4 } else { 6 }
    } else {
        0
    };
    let regions = Layout::vertical([
        Constraint::Length(header_height),
        Constraint::Length(1),
        Constraint::Length(3),
        Constraint::Length(serial_height),
        Constraint::Min(5),
        Constraint::Length(if short { 6 } else { 7 }),
        Constraint::Length(if short { 5 } else { 6 }),
        Constraint::Length(1),
    ])
    .split(area);
    header(frame, regions[0], data, order, colors);
    frame.render_widget(
        Paragraph::new(super::weights::line(data, order)).style(colors.foreground(GOLD)),
        regions[1],
    );
    progress(frame, regions[2], data, colors, cancelling);
    if let Some(serial) = &data.serial {
        super::super::serial_view::render(frame, regions[3], serial, colors);
    }
    super::sectors::render(frame, regions[4], data, view, colors);
    super::metrics::selected(frame, regions[5], data, view, colors);
    super::metrics::global(frame, regions[6], data, colors);
    help(frame, regions[7], colors, cancelling);
}

fn header(frame: &mut Frame<'_>, area: Rect, data: &Cached, order: i32, colors: ColorPolicy) {
    let wide = area.width >= 110;
    let (memory_label, memory_capacity) = data.memory.free_or_available();
    let chunks = Layout::horizontal(if wide {
        vec![Constraint::Percentage(70), Constraint::Percentage(30)]
    } else {
        vec![Constraint::Percentage(100)]
    })
    .split(area);
    let snapshot = &data.observation.snapshot;
    let method = match snapshot.method {
        fastsecdec::status::IntegrationMethod::DemocraticQmc => "QMC",
        fastsecdec::status::IntegrationMethod::AdaptiveQmc => "Adaptive QMC",
        fastsecdec::status::IntegrationMethod::HavanaMc => "Havana MC",
        fastsecdec::status::IntegrationMethod::HavanaDiscreteMc => "Havana discrete MC",
    };
    let title = format!(
        " FastSecDec · {method} · {:?} · ε{} ",
        snapshot.stage,
        number::superscript(order)
    );
    let title = if area.height < 5 {
        format!(
            " {} · ε{} · joint rel {} · {} / {} ",
            if data.scope.is_full_integral() {
                "Full sum"
            } else {
                "Selected sum"
            },
            number::superscript(order),
            relative(data, None, indices(data, order)),
            if data.live.is_some() {
                "preview"
            } else {
                "current"
            },
            if super::accepted::previous(data).is_some() {
                "previous allocation"
            } else {
                "accepted"
            }
        )
    } else {
        title
    };
    let panel = card(title, TEAL, colors);
    let panel = if !wide {
        panel.title_bottom(Line::from(format!(
            " RSS {} · peak {} · RAM {}/{} · {} {} ",
            ram(data.memory.resident_bytes()),
            ram(data.memory.resident_peak_bytes()),
            ram(data.memory.system_used_bytes),
            ram(data.memory.system_total_bytes),
            memory_label,
            ram(memory_capacity)
        )))
    } else {
        panel
    };
    let inner = panel.inner(chunks[0]);
    frame.render_widget(panel, chunks[0]);
    let (real, imag) = indices(data, order);
    let sum = if data.scope.is_full_integral() {
        "Full sum"
    } else {
        "Selected sum"
    };
    let preview = data.live.is_some();
    let live_label = format!(
        "{} · joint rel {}",
        if preview {
            if data.scope.is_full_integral() {
                "Full preview"
            } else {
                "Selected preview"
            }
        } else {
            sum
        },
        relative(data, None, (real, imag))
    );
    let summary_area = Rect {
        height: inner.height.min(3),
        ..inner
    };
    let values = [("Re", real, TEAL), ("Im", imag, PURPLE)].map(|(label, index, color)| {
        (
            label,
            super::sectors::split(value(data, None, index)),
            super::sectors::split(super::accepted::value(data, index)),
            color,
        )
    });
    let preview_width = values
        .iter()
        .map(|(_, (_, suffix), _, _)| Line::from(suffix.as_str()).width())
        .max()
        .unwrap_or(0) as u16;
    let accepted_width = values
        .iter()
        .map(|(_, _, (_, suffix), _)| Line::from(suffix.as_str()).width())
        .max()
        .unwrap_or(0) as u16;
    let widths = [
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(preview_width),
        Constraint::Fill(1),
        Constraint::Length(accepted_width),
    ];
    let columns = Layout::horizontal(widths).spacing(1).split(summary_area);
    let rows = values.map(
        |(label, (value, exponent), (accepted, accepted_exp), color)| {
            Row::new(vec![
                right(label),
                right(super::sectors::fitted(value, columns[1].width)),
                ratatui::widgets::Cell::from(super::sectors::fitted(exponent, columns[2].width)),
                right(super::sectors::fitted(accepted, columns[3].width)),
                ratatui::widgets::Cell::from(super::sectors::fitted(
                    accepted_exp,
                    columns[4].width,
                )),
            ])
            .style(colors.foreground(color))
        },
    );
    let table = Table::new(rows, widths).column_spacing(1);
    let table = if inner.height >= 3 {
        table.header(
            Row::new(vec![
                right(""),
                right(live_label),
                right(""),
                right(super::accepted::label(data)),
                right(""),
            ])
            .style(bold(colors, GOLD)),
        )
    } else {
        table
    };
    frame.render_widget(table, summary_area);
    if wide {
        let panel = card(" Memory · process / machine ", BLUE, colors);
        let inner = panel.inner(chunks[1]);
        frame.render_widget(panel, chunks[1]);
        let rows = [
            (
                "RSS / peak",
                format!(
                    "{} / {}",
                    ram(data.memory.resident_bytes()),
                    ram(data.memory.resident_peak_bytes())
                ),
            ),
            (
                "RAM used",
                format!(
                    "{} / {}",
                    ram(data.memory.system_used_bytes),
                    ram(data.memory.system_total_bytes)
                ),
            ),
            (memory_label, ram(memory_capacity)),
        ];
        frame.render_widget(
            Table::new(
                rows.into_iter()
                    .map(|(k, v)| Row::new(vec![right(k), right(v)])),
                [Constraint::Length(10), Constraint::Fill(1)],
            )
            .column_spacing(1)
            .style(colors.foreground(BLUE)),
            inner,
        );
    }
}

fn progress(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &Cached,
    colors: ColorPolicy,
    cancelling: bool,
) {
    let s = &data.observation.snapshot;
    let title = if cancelling {
        " Cancelling · draining active calls "
    } else if is_qmc(data) {
        " Lattice allocation · accepted work "
    } else {
        " Iteration progress · accepted work "
    };
    let title = if data.observation.contributions.total.is_none() && !cancelling {
        format!("{title}· {} ", super::accepted::waiting(data))
    } else {
        title.to_owned()
    };
    let panel = card(title, if cancelling { RED } else { BLUE }, colors);
    let panel = if is_qmc(data) {
        panel.title_bottom(Line::from(format!(
            " {} sum coverage: {} points / {} complete shifts{} ",
            if data.scope.is_full_integral() {
                "Full"
            } else {
                "Selected"
            },
            number::compact_count(estimate(data, None).2),
            number::compact_count(shifts(data, None) as u64),
            if data.live.is_some() {
                " · provisional"
            } else {
                ""
            }
        )))
    } else {
        panel
    };
    let panel = if !is_qmc(data) && data.observation.contributions.total.is_none() {
        panel.title_bottom(Line::from(format!(
            " {} ",
            super::accepted::explanation(data)
        )))
    } else {
        panel
    };
    let inner = panel.inner(area);
    frame.render_widget(panel, area);
    let chunks = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
        .spacing(2)
        .split(inner);
    let ratio = |done: u64, planned: u64| {
        if planned > 0 {
            (done as f64 / planned as f64).clamp(0.0, 1.0)
        } else {
            0.0
        }
    };
    frame.render_widget(
        Gauge::default()
            .ratio(ratio(s.completed_points, s.planned_points))
            .gauge_style(colors.foreground(BLUE))
            .label(format!(
                "{} / {} accepted",
                number::compact_count(s.completed_points),
                number::compact_count(s.planned_points)
            )),
        chunks[0],
    );
    let complete: u64 = data.workers.iter().map(|w| w.completed_points).sum();
    let planned: u64 = data.workers.iter().map(|w| w.planned_points).sum();
    let preparing = data.workers.iter().filter(|w| w.preparing_context).count();
    let label = if preparing > 0 {
        format!(
            "{} prep · {}/{} in flight",
            number::compact_count(preparing as u64),
            number::compact_count(complete),
            number::compact_count(planned)
        )
    } else if planned > 0 {
        format!(
            "{} / {} in flight",
            number::compact_count(complete),
            number::compact_count(planned)
        )
    } else {
        "No work in flight".into()
    };
    // In-flight reservations have their own denominator; they are never added to
    // accepted counts, including the interval before a completed wave is admitted.
    frame.render_widget(
        Gauge::default()
            .ratio(ratio(complete, planned))
            .gauge_style(colors.foreground(PURPLE))
            .label(label),
        chunks[1],
    );
}

fn help(frame: &mut Frame<'_>, area: Rect, colors: ColorPolicy, cancelling: bool) {
    let text = if cancelling {
        "Ctrl-C again: force exit"
    } else if area.width < 100 {
        "Click: sort/select · wheel/↑↓: rows · ←→: ε · Tab/r: sort · q: stop"
    } else {
        "Click header: sort ↕ · click row: details · wheel/↑↓/Pg: scroll · ←→: ε · Tab/r: sort/reverse · Ctrl-C/q/Esc: stop"
    };
    frame.render_widget(Paragraph::new(text).style(colors.foreground(GOLD)), area);
}

fn compact(
    frame: &mut Frame<'_>,
    area: Rect,
    data: &Cached,
    view: &mut View,
    colors: ColorPolicy,
    cancelling: bool,
) {
    let parts = Layout::vertical([
        Constraint::Length(5),
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(4),
        Constraint::Length(2),
    ])
    .split(area);
    let (re, im) = indices(data, view.order(data));
    let total = vec![
        Line::from(format!(
            "ε{} · {}",
            number::superscript(view.order(data)),
            source(data)
        )),
        Line::from(format!("Re {}", value(data, None, re))),
        Line::from(format!("Im {}", value(data, None, im))),
    ];
    frame.render_widget(
        Paragraph::new(total).block(card(
            if data.scope.is_full_integral() {
                " FastSecDec · full sum "
            } else {
                " FastSecDec · selected sum "
            },
            TEAL,
            colors,
        )),
        parts[0],
    );
    frame.render_widget(
        Paragraph::new(super::weights::line(data, view.order(data))).style(colors.foreground(GOLD)),
        parts[1],
    );
    super::sectors::render(frame, parts[2], data, view, colors);
    let fs = fractions(Some(&data.operational.diagnostics));
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(format!("f64 {} · DF106 {}", fs[0], fs[1])),
            Line::from(format!(
                "{} {} · Unstable {}",
                crate::display::diagnostic_summary::arbitrary_label(data.stability_mode),
                fs[2],
                fs[3]
            )),
        ])
        .style(colors.foreground(PURPLE))
        .block(card(" Invocation outcomes ", PURPLE, colors)),
        parts[3],
    );
    help(frame, parts[4], colors, cancelling);
}
