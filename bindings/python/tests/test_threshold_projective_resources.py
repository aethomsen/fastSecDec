"""Real HEPKit graph/family callers of shared native endpoint preparation.

Requires the freshly linked native host. Points/normalization are the maintained
above-threshold overview cards; references are native OneLOop finite terms.
"""
import gc
import json
import math
import os
from pathlib import Path
import tomllib

from symbolica import E, S
from symbolica.community import hepkit as hep
from symbolica.community.hepkit import sector_decomposition as sd

ROOT = Path(os.environ.get("FASTSECDEC_TEST_ROOT", Path(__file__).resolve().parents[3]))


def native_inputs(name):
    filename = "box_no_contour.toml" if name == "box" else "triangle.toml"
    path = ROOT / "examples/no_deformation/benchmarks/overview" / filename
    card = tomllib.loads(path.read_text())
    model = hep.Model((path.parent / card["input"]["model"]).resolve())
    diagram = hep.FeynmanDiagram.from_dot(
        model, (path.parent / card["input"]["graph"]).read_text())
    assert diagram.loop_count == 1 and diagram.symmetry_factor == 1
    eps = S("eps")
    dim = S("hepkit_endpoint_resources::D")
    p, k = hep.Kinematics.external_momentum(), hep.Kinematics.loop_momentum()
    products = card["kinematics"]["products"]
    indices = sorted({row[key] for row in products for key in ("left", "right")})
    kin = hep.Kinematics(dim, momenta=[k(i) for i in range(diagram.loop_count)] + [p(i) for i in indices])
    for row in products:
        kin = kin.with_scalar_product(p(row["left"]), p(row["right"]), E(row["value"]))
    arguments = dict(regulator=eps, dimension=E(card["integral"]["dimension"]),
                     scalar_values={S(key): E(value) for key, value in card["parameters"].items()},
                     measure_multiplier=E(card["integral"]["measure_multiplier"]))
    graph = sd.Integral(diagram, kinematics=kin, model_parameters="fixed", **arguments)
    assert len(graph.powers) == (4 if name == "box" else 3)
    assert all(power == 1 for _, power in graph.powers)
    family = diagram.propagator_family(kinematics=kin)
    # Reuse HEPKit's weighted numerator and contraction, not reconstructed
    # propagators or an assumed graph numerator/overall factor.
    numerator = (diagram.numerator_expression(in_lmb=True)
                 * diagram.projector_expression()
                 * diagram.numerator_prefactor_expression()
                 * diagram.overall_factor_expression()).with_lorentz_dimension(kin.dimension)
    numerator = numerator.simplify_algebra(contract="dots").to_dots()
    assert numerator.is_scalar
    numerator = kin.apply(numerator).to_expression()
    assert numerator == E("1"), "these physical fixtures require a unit scalar numerator"
    retained = sd.Integral.from_family(family, powers=[1] * len(family.denominators),
                                      numerator=numerator, **arguments)
    solver = card.get("threshold_decomposition", {}).get("solver", {})
    return graph, retained, solver


def run_projective(name, reference):
    graph, family, solver = native_inputs(name)
    resources = {"geometry": {"wall_time_secs": 600},
                 "discovery": {"wall_time_secs": 600}, "max_charts": 4096}
    settings = sd.ThresholdSettings(
        solver_options_json=json.dumps(solver),
        gcad_limits_json='{"wall_time_secs":60,"memory_mib":2048}',
        endpoint_resources_json=json.dumps(resources))
    encoded = json.loads(settings.to_json())
    assert encoded["endpoint_resources"]["geometry"]["wall_time_secs"] == 600
    assert encoded["gcad_limits"]["wall_time_secs"] == 60
    compiler = sd.CompilationSettings(backend="eager", horner_iterations=0, cpe_rounds=0)
    results = []
    for kind, integral in [("graph", graph), ("family", family)]:
        owner = integral.generation_session(
            threshold_decomposition=True, threshold_settings=settings,
            compilation_settings=compiler, coefficient_expansion="coefficient_series",
            subtraction="integrate_by_parts")
        assert not owner.prepared and owner.snapshot().elapsed_seconds == 0
        owner.prepare(observer=lambda _: False, progress=None)
        assert owner.preparation_progress_json() is None
        owner.prepare(progress=None)
        receipt = json.loads(owner.preparation_receipt_json())
        assert receipt["integration_dimensions"] == (3 if name == "box" else 2)
        assert receipt["endpoint_charts"] > receipt["cells"] > 0
        assert owner.prepared and not owner.complete
        while not owner.complete:
            owner.compile_next(16, progress=None)
        archive = owner.result
        identity, payload = archive.content_id, archive.to_bytes()
        # Fresh codec and selected runtime ownership, with preparer dropped.
        del owner, archive
        gc.collect()
        restored = sd.RecipeArchive.from_bytes(payload, default_recipe="threshold")
        assert restored.content_id == identity
        kernels = restored.select("threshold")
        session = kernels.session(sd.QmcSettings(points=4096, shifts=8, seed=202610128141))
        assert session.snapshot().completed_points == 0
        while not session.complete:
            session.step(64)
        estimate = session.snapshot().estimate
        assert estimate.production_complete
        assert len(estimate.covariance_of_mean) == len(estimate.mean) ** 2
        actual = {}
        for order, part, mean, error in zip(estimate.orders, estimate.components,
                                           estimate.mean, estimate.standard_error):
            expected = reference[0 if part == "real" else 1] if order == 0 else 0.
            assert math.isfinite(mean) and math.isfinite(error)
            # Same 8x4096 work as the native scalar gate. The small fixed
            # allowance covers binary64 reference/evaluator roundoff only.
            assert abs(mean - expected) < 8 * error + 2e-9, (name, kind, order, part, mean, error, expected)
            actual[(order, part)] = (mean, error)
        assert (0, "real") in actual and (0, "imag") in actual
        results.append(actual)
        print(name, kind, "native prepare/compile/fresh restore/OneLOop PASS", receipt["cells"], receipt["endpoint_charts"], flush=True)
    assert results[0].keys() == results[1].keys()
    for key, (left, error) in results[0].items():
        right, other_error = results[1][key]
        assert abs(left - right) < 8 * (error + other_error) + 2e-9


def test_real_triangle_graph_and_family_automatic_route_resources():
    run_projective("triangle", (-0.8943345118780581, -0.6047086137711147))


def test_real_box_graph_and_family_automatic_route_resources():
    run_projective("box", (0.28470925234030064, 0.2719854522910709))


def test_endpoint_settings_invalid_fields_and_omission():
    assert "endpoint_resources" not in json.loads(sd.ThresholdSettings().to_json())
    for value in [{"geometry": {"wall_time_secs": 0}},
                  {"discovery": {"max_dyadic_level": 31}},
                  {"geometry": {"not_a_native_limit": 2}}]:
        try:
            sd.ThresholdSettings(endpoint_resources_json=json.dumps(value))
        except (ValueError, sd.FastSecDecError):
            pass
        else:
            raise AssertionError(("invalid endpoint settings accepted", value))
