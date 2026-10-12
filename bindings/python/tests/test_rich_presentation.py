"""Rich native object protocols read retained data without changing science."""
import sys
from pathlib import Path

import pytest
from symbolica import E
from symbolica.community.hepkit import sector_decomposition as sd

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "examples/hepkit"))
from showcase import inputs


def html(owner):
    text = owner._repr_html_()
    assert isinstance(text, str) and 'class="fsd-native"' in text
    assert '<script' not in text
    return text


@pytest.fixture(scope="module")
def owners():
    arguments = inputs.massive_triangle().integral_arguments()
    arguments["measure_multiplier"] = E("1/1000000000000")
    integral = sd.Integral(**arguments)
    generation = integral.generation_session(max_order=1)
    events = []
    before = generation.snapshot().elapsed_seconds
    html(integral)
    html(generation)
    assert generation.snapshot().elapsed_seconds == before == 0
    assert generation.generated is generation.kernels is None
    while not generation.complete:
        generation.step(observer=events.append)
    generated, template = generation.generated, generation.kernels
    kernels = template.with_parameters(generated.runtime_parameter_defaults,
        stability=sd.StabilitySettings(f64_distance=1e-12, double_float_distance=1e-16, large_weight_threshold=None))
    qmc = kernels.session(sd.QmcSettings(points=32, shifts=3, package_points=8, rule="hkkn_alpha3"))
    initial = qmc.live_observation()
    saved = qmc.checkpoint()
    html(qmc)
    html(initial)
    assert qmc.checkpoint() == saved
    while not qmc.complete:
        qmc.step(max_packages=4)
    mc = kernels.mc_session(sd.HavanaDiscreteSettings(points_per_batch=16, batches=2), pilot=True)
    return integral, generation, generated, kernels, qmc, mc, events, initial


def test_native_html_covers_owners_settings_events_and_readonly_metadata(owners):
    integral, generation, generated, kernels, qmc, mc, events, initial = owners
    objects = [integral, generation, generated, kernels, qmc, mc,
        sd.CompilationSettings(cpe_rounds=None), sd.StabilitySettings(),
        sd.StabilitySettings(mode="validated"), qmc.settings, mc.settings,
        generated.metadata, generated.metadata.domain, *generated.metadata.domain.factors]
    for event in events:
        objects.extend([event, event.timings])
        if event.coefficient_expansion is not None:
            objects.extend([event.coefficient_expansion, event.coefficient_expansion.requests])
    for chart in generated.metadata.charts:
        objects.extend([chart, chart.coordinates, chart.geometry])
        if chart.pre_subtraction is not None:
            objects.append(chart.pre_subtraction)
            for term in chart.pre_subtraction.terms:
                objects.extend([term, *term.powers])
    for sector in generated.sectors:
        objects.extend([sector, sector.map, *sector.aliased_coefficients])
    for statistics in kernels.sector_statistics:
        objects.extend([statistics, statistics.operations])
    saved = kernels.to_bytes(), kernels.content_id, qmc.checkpoint()
    for owner in objects:
        html(owner)
    assert saved == (kernels.to_bytes(), kernels.content_id, qmc.checkpoint())
    assert 'Unlimited' in html(sd.CompilationSettings(cpe_rounds=None))
    assert 'inactive' in html(sd.StabilitySettings(mode="validated"))
    assert 'certificate' not in html(generated.metadata.domain).lower()


def test_estimates_keep_full_laurent_vector_covariance_and_small_values(owners):
    _, _, _, kernels, qmc, mc, _, initial = owners
    snapshot = qmc.snapshot()
    observation = qmc.observation()
    live = qmc.live_observation()
    objects = [snapshot, observation, live, live.total, *live.sectors,
        observation.total, *observation.sectors, *snapshot.sectors,
        snapshot.evaluation_diagnostics, snapshot.evaluation_diagnostics.f64_timing,
        snapshot.evaluation_diagnostics.double_float_timing,
        snapshot.evaluation_diagnostics.arbitrary_timing,
        snapshot.evaluation_diagnostics.conditioning_timing,
        initial, initial.total, *initial.sectors]
    for sector in live.sectors:
        objects.append(sector.estimate)
    for sector in snapshot.sectors:
        if sector.discrete_allocation is not None:
            objects.append(sector.discrete_allocation)
    before = observation.to_json(), qmc.checkpoint()
    for owner in objects:
        html(owner)
    assert before == (qmc.observation().to_json(), qmc.checkpoint())
    estimate = observation.total
    result = html(estimate)
    assert 'Full native covariance' in result
    for order in estimate.orders:
        assert f'ε order {order}' in result
    for value in estimate.mean:
        assert 0 < abs(value) < 1e-9
        assert f'{value:.6e}' in result
    assert 'Waiting for native mean coverage' in html(initial.total)
    assert 'Provisional observation only' in html(initial)
    assert not kernels.stability_settings.mode == "validated"


def test_selected_integrand_uses_scoped_native_pager_without_mutating_artifact(owners):
    import marimo as mo
    from showcase import sectors
    from showcase.notebook import Study

    _, _, generated, kernels, qmc, _, _, _ = owners
    saved = kernels.to_bytes(), kernels.content_id, qmc.checkpoint()
    study = Study()
    study.expression_viewer = sectors.expression_viewer(generated, 0, 0)
    pager = study.expression_viewer
    assert not pager._cache and pager._widget is None
    rendered = mo.as_html(pager).text
    assert "marimo-anywidget" in rendered
    assert len(pager._cache) <= 3
    assert saved == (kernels.to_bytes(), kernels.content_id, qmc.checkpoint())
    study.close_inspection()
    assert pager._closed and pager._source is None and not pager._cache
    assert study.expression_viewer is None and study.inspection is None
