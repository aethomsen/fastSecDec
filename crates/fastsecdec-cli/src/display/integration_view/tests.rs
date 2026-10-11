use super::*;
use fastsecdec::integration::{
    IntegrationProblem, Periodization, QmcSession, QmcSettings, RuleSource, SectorSpec,
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer};

fn qmc() -> QmcSession {
    QmcSession::democratic(
        IntegrationProblem::new_with_components(
            "dashboard-layout".into(),
            vec![0, 0],
            vec![CoefficientComponent::Real, CoefficientComponent::Imag],
            vec![SectorSpec {
                id: 3,
                dimension: 1,
            }],
            vec![0.0; 2],
        )
        .unwrap(),
        QmcSettings {
            points: 8,
            shifts: 2,
            seed: 17,
            package_points: 8,
            periodization: Periodization::None,
            rule: RuleSource::Supplied(vec![1]),
        },
    )
    .unwrap()
}
fn shift(session: &mut QmcSession) {
    let task = session.next_work().unwrap().unwrap();
    let result = session
        .worker_context(task.sector_id())
        .unwrap()
        .evaluate_weighted(task, |point, _, output| {
            output.copy_from_slice(&[point[0], -2.0 * point[0]]);
            Ok::<_, String>(())
        })
        .unwrap();
    session.submit(result).unwrap();
}
fn cached(session: &QmcSession) -> Cached {
    Cached {
        observation: session.diagnostic_observation().unwrap(),
        previous_completed: None,
        live: Some(session.live_observation().unwrap()),
        operational: OperationalMetrics::default(),
        stability_mode: fastsecdec::kernel::StabilityMode::Distance,
        memory: memory::Snapshot::default(),
        serial: None,
        elapsed: 1.0,
        scope: Default::default(),
        workers: vec![],
    }
}
fn draw(data: &Cached, width: u16, height: u16, sectors_only: bool) -> (Buffer, View) {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let mut view = View::default();
    view.table.select(Some(0));
    let colors = ColorPolicy::for_stream(true, false);
    terminal
        .draw(|frame| {
            if sectors_only {
                sectors::render(frame, frame.area(), data, &mut view, colors);
            } else {
                render(frame, data, &mut view, colors, false);
            }
        })
        .unwrap();
    (terminal.backend().buffer().clone(), view)
}
fn line(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, y)].symbol())
        .collect()
}
fn text(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| line(buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn mean_only_qmc_labels_fit_then_disappear_when_native_error_is_available() {
    let mut session = qmc();
    shift(&mut session);
    let first = cached(&session);
    assert_eq!(
        first.live.as_ref().unwrap().total.status,
        LiveStatus::MeanOnly
    );
    assert!(first.live.as_ref().unwrap().total.standard_error.is_none());
    let initial = first.live.clone();
    for width in [80, 120, 160, 240] {
        let (buffer, _) = draw(&first, width, 36, false);
        let rendered = text(&buffer);
        // Both full-sum components and both sector components retain the label.
        assert_eq!(
            rendered.matches("(σ n/a)").count(),
            4,
            "{width}:\n{rendered}"
        );
        assert!(rendered.contains("Await shifts"));
    }
    assert_eq!(first.live, initial);
    shift(&mut session);
    let second = cached(&session);
    assert_eq!(
        second.live.as_ref().unwrap().total.status,
        LiveStatus::Available
    );
    assert!(second.live.as_ref().unwrap().total.standard_error.is_some());
    for width in [80, 120, 160, 240] {
        let (buffer, _) = draw(&second, width, 36, false);
        assert!(!text(&buffer).contains("σ"));
    }
}

#[test]
fn accepted_weight_and_norm_line_waits_for_evidence_and_preserves_snapshot() {
    let mut session = qmc();
    let initial = cached(&session);
    assert!(super::weights::line(&initial, 0).contains("C —"));
    assert!(super::weights::line(&initial, 0).contains("norm rel —"));
    shift(&mut session);
    shift(&mut session);
    let accepted = cached(&session);
    let before = serde_json::to_value(&accepted.observation).unwrap();
    let line = super::weights::line(&accepted, 0);
    assert!(line.contains("C 1.000"), "{line}");
    assert!(!line.contains("max/|I| —"), "{line}");
    assert!(!line.contains("norm rel —"), "{line}");
    for width in [80, 120, 160, 240] {
        let (buffer, _) = draw(&accepted, width, 36, false);
        let rendered = text(&buffer);
        assert!(rendered.contains("C 1.000"), "{width}:\n{rendered}");
        assert!(rendered.contains("max/|I|"), "{width}:\n{rendered}");
        assert!(rendered.contains("norm rel"), "{width}:\n{rendered}");
    }
    assert_eq!(serde_json::to_value(&accepted.observation).unwrap(), before);
}

#[test]
fn sector_columns_keep_complete_unicode_suffixes_and_matching_mouse_regions() {
    let mut session = qmc();
    shift(&mut session);
    let mut data = cached(&session);
    let template = data.observation.contributions.sectors[0].clone();
    let numbers = [1.234567, -1.234567e-10, f64::from_bits(1), -1.234567e100];
    data.observation.contributions.sectors = numbers
        .iter()
        .enumerate()
        .map(|(i, _)| {
            let mut row = template.clone();
            row.progress.id = i as u64;
            row
        })
        .collect();
    data.live.as_mut().unwrap().sectors = numbers
        .iter()
        .enumerate()
        .map(|(i, &value)| fastsecdec::integration::LiveSector {
            id: i as u64,
            estimate: fastsecdec::integration::LiveEstimate {
                mean: Some(vec![value, -value]),
                standard_error: None,
                points: 8,
                replicas: 1,
                status: LiveStatus::MeanOnly,
            },
        })
        .collect();
    for width in [64, 65, 80, 119, 120, 160, 240] {
        let (buffer, mut view) = draw(&data, width, 28, true);
        let rendered = text(&buffer);
        // Never render an apparently complete truncated '(σ' or exponent.
        for row in rendered.lines() {
            for suffix in row.split("(σ").skip(1) {
                assert!(suffix.starts_with(" n/a)"), "{width}: {row}");
            }
        }
        if width < 65 {
            continue;
        }
        let mut anchors = None;
        for (area, _) in &view.hits.rows {
            let dots = (0..width)
                .filter(|&x| buffer[(x, area.y)].symbol() == "·")
                .collect::<Vec<_>>();
            assert_eq!(dots.len(), 2, "{width}:\n{rendered}");
            if let Some(expected) = &anchors {
                assert_eq!(&dots, expected);
            } else {
                anchors = Some(dots);
            }
        }
        if width >= 80 {
            assert_eq!(
                rendered.matches("(σ n/a)").count(),
                8,
                "{width}:\n{rendered}"
            );
            assert!(rendered.contains("·10⁻³²⁴ (σ n/a)"));
        }
        let first_dots = anchors.unwrap();
        for (x, sort) in [(first_dots[0], Sort::Real), (first_dots[1], Sort::Imag)] {
            assert!(view.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: x,
                    row: 1,
                    modifiers: crossterm::event::KeyModifiers::NONE
                },
                &data
            ));
            assert!(view.sort == sort);
        }
    }
}

#[test]
fn dominating_errors_and_summary_columns_never_show_truncated_number_tokens() {
    let mut session = qmc();
    shift(&mut session);
    shift(&mut session);
    let mut data = cached(&session);
    let live = data.live.as_mut().unwrap();
    // A long but ordinary last-digit uncertainty and an extreme finite one.
    live.sectors[0].estimate.mean = Some(vec![0.123, 1e-200]);
    live.sectors[0].estimate.standard_error = Some(vec![12434.43, 1e100]);
    live.total.mean = Some(vec![f64::from_bits(1), -1e100]);
    live.total.standard_error = None;
    live.total.status = LiveStatus::MeanOnly;
    let expected = number::uncertainty(0.123, Some(12434.43));
    for width in [65, 80, 120, 160, 240] {
        let (buffer, _) = draw(&data, width, 36, false);
        let rendered = text(&buffer);
        assert!(
            rendered.contains('…'),
            "long mantissa should be explicitly omitted: {width}"
        );
        for row in rendered.lines() {
            for suffix in row.split("(σ").skip(1) {
                assert!(suffix.starts_with(" n/a)"), "{width}: {row}");
            }
        }
        if width >= 120 {
            let (mantissa, exponent) = sectors::split(expected.clone());
            assert!(rendered.contains(&mantissa), "{width}:\n{rendered}");
            assert!(rendered.contains(&exponent));
            assert!(rendered.contains("·10⁻³²⁴ (σ n/a)"));
        }
    }
}
