"""Contour bindings retain native settings and explicit caller-owned pilot work."""
import json
from pathlib import Path
import sys

import pytest
from symbolica.community.hepkit import sector_decomposition as sd

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "examples/hepkit"))
from showcase import inputs
from _fixtures import fixed_arguments


def test_contour_settings_are_native_and_reject_invalid_strengths():
    default = sd.ContourSettings()
    assert default.mode == "off" and default.lambda_value is None
    assert default.safety_fraction is default.lambda_cap is default.displacement_cap is None
    assert default.construction is None
    assert default.validation == "off"
    assert sd.ContourSettings.fixed(0.01).validation == "off"
    assert sd.ContourSettings.dynamical(0.8).validation == "off"
    fixed = sd.ContourSettings.fixed(0.01, validation="pilot", pilot_points=2)
    assert fixed.mode == "fixed" and fixed.lambda_value == 0.01
    assert sd.ContourSettings.from_json(fixed.to_json()).to_json() == fixed.to_json()
    for strength in (0.0, -1.0, float("inf"), float("nan")):
        with pytest.raises(sd.FastSecDecError):
            sd.ContourSettings.fixed(strength)
    with pytest.raises(ValueError):
        sd.ContourSettings.fixed(0.1, validation="unknown")
    with pytest.raises(sd.FastSecDecError):
        sd.ContourSettings.fixed(0.1, validation="always", pilot_points=0)
    assert sd.ContourSettings.fixed(0.1, pilot_points=0).validation == "off"


@pytest.mark.parametrize("construction", ["polynomial", "sign_aware"])
@pytest.mark.parametrize("validation", ["always", "pilot", "off"])
def test_dynamical_settings_roundtrip_native_prescription(construction, validation):
    settings = sd.ContourSettings.dynamical(
        0.8, lambda_cap=0.25, displacement_cap=2.0,
        construction=construction, validation=validation, pilot_points=4,
    )
    assert settings.mode == "dynamical" and settings.lambda_value is None
    assert settings.safety_fraction == 0.8
    assert settings.lambda_cap == 0.25 and settings.displacement_cap == 2.0
    assert settings.construction == construction and settings.validation == validation
    restored = sd.ContourSettings.from_json(settings.to_json())
    assert restored.to_json() == settings.to_json()
    assert "safety_fraction=0.8" in repr(settings)
    payload = json.loads(settings.to_json())
    assert payload["deformation"] == {
        "mode": "dynamical", "safety_fraction": 0.8,
        "lambda_cap": 0.25, "displacement_cap": 2.0,
        "construction": construction,
    }
    with pytest.raises(AttributeError):
        settings.safety_fraction = 0.5


def test_dynamical_settings_defaults_and_native_range_admission():
    settings = sd.ContourSettings(mode="dynamical", safety_fraction=0.8)
    assert settings.to_json() == sd.ContourSettings.dynamical(0.8).to_json()
    assert settings.lambda_cap == settings.displacement_cap == 1.0
    assert settings.construction == "sign_aware"
    for fraction in (-1.0, 0.0, 1.0, float("inf"), float("nan")):
        with pytest.raises(sd.FastSecDecError):
            sd.ContourSettings.dynamical(fraction)
    for name in ("lambda_cap", "displacement_cap"):
        for invalid in (-1.0, 0.0, float("inf"), float("nan")):
            with pytest.raises(sd.FastSecDecError):
                sd.ContourSettings.dynamical(0.8, **{name: invalid})
    with pytest.raises(ValueError, match="construction"):
        sd.ContourSettings.dynamical(0.8, construction="unknown")


@pytest.mark.parametrize("arguments", [
    {"mode": "dynamical"},
    {"mode": "dynamical", "safety_fraction": 0.8, "lambda_value": 0.1},
    {"mode": "fixed", "lambda_value": 0.1, "safety_fraction": 0.8},
    {"mode": "off", "lambda_cap": 1.0},
])
def test_contour_constructor_rejects_incompatible_prescriptions(arguments):
    with pytest.raises(ValueError):
        sd.ContourSettings(**arguments)


@pytest.mark.parametrize("deformation", [
    {"mode": "dynamical", "safety_fraction": 0.8, "lambda_cpa": 2.0},
    {"mode": "off", "lambda": 0.1},
])
def test_contour_json_rejects_unknown_prescription_fields(deformation):
    with pytest.raises(sd.FastSecDecError, match="unknown field"):
        sd.ContourSettings.from_json(json.dumps({"deformation": deformation}))


@pytest.fixture(scope="module")
def generated():
    integral = sd.Integral(**fixed_arguments(inputs.massive_triangle()))
    return integral.generate(contour=True, coefficient_expansion="coefficient_series", progress=None)


def test_contour_inspection_retains_independent_native_face_views(generated):
    before = generated.snapshot().elapsed_seconds
    for chart in generated.metadata.charts:
        recipe = chart.contour
        assert recipe is not None
        assert recipe.source_index == chart.source_index
        assert recipe.version == 1
        assert recipe.function_definition_count == 0
        assert recipe.function_definitions == []
        # The finite massive triangle needs the full map, with no endpoint
        # subtraction. The native recipe retains the interior explicitly.
        assert recipe.validation_faces == [[]]
        copied_faces = recipe.validation_faces
        copied_faces[0].append((0, 0))
        assert recipe.validation_faces == [[]]
        with pytest.raises(AttributeError):
            recipe.validation_faces = []
    assert generated.snapshot().elapsed_seconds == before


def test_contour_capability_and_caller_owned_pilot_survive_reload(generated):
    assert generated.contour_capable
    template = generated.compile(settings=sd.CompilationSettings(horner_iterations=0), progress=None)
    assert template.contour_capable
    assert not template.runtime_parameters  # Strength is separate from physical inputs.
    restored = sd.Kernels.from_bytes(template.to_bytes())
    assert [r.source_index for r in restored.contour_recipes] == [
        r.source_index for r in template.contour_recipes]
    assert all(r.function_definitions == [] for r in restored.contour_recipes)
    configured = restored.with_parameters({}, contour=sd.ContourSettings.fixed(
        0.01, validation="pilot", pilot_points=2))
    assert configured.contour_settings.lambda_value == 0.01
    identity = configured.content_id
    assert not configured.contour_validation_report().pilot_complete
    with pytest.raises(sd.FastSecDecError, match="pilot"):
        configured.finish_contour_pilot()
    for chart in configured.contour_validation_charts:
        assert chart.kernel_sectors == [chart.kernel_sector]
        assert not chart.includes_exact  # This finite triangle has no exact offset.
        copied_sectors = chart.kernel_sectors
        copied_sectors.append(10000)
        assert chart.kernel_sectors == [chart.kernel_sector]
        for coordinate in (0.3, 0.6):
            report = configured.validate_contour_point(chart.chart_index, [coordinate] * chart.dimension)
            assert report.maximum_bits >= 96
            assert report.checked_arguments > 0
    pilot = configured.finish_contour_pilot()
    assert pilot.pilot_complete and pilot.required_charts == pilot.validated_charts
    assert json.loads(pilot.to_json())["accepted_pilot_points"] == pilot.accepted_pilot_points
    unchecked = configured.with_contour_validation("off", pilot_points=2)
    always = configured.with_contour_validation("always", pilot_points=2)
    assert unchecked.content_id == identity
    assert always.content_id == identity
    assert unchecked.contour_validation_report().pilot_complete
    assert always.contour_validation_report().pilot_complete
    settings = sd.QmcSettings(points=32, shifts=2, package_points=32, rule="hkkn_alpha3")
    checked_session, unchecked_session = configured.session(settings), unchecked.session(settings)
    always_session = always.session(settings)
    assert checked_session.contour_provenance.pilots[0].seed is None
    for session in (checked_session, unchecked_session, always_session):
        while not session.complete:
            session.step()
    assert checked_session.snapshot().estimate.mean == unchecked_session.snapshot().estimate.mean
    assert checked_session.snapshot().estimate.mean == always_session.snapshot().estimate.mean
    production_checks = always_session.snapshot().evaluation_diagnostics.contour.production
    assert production_checks.checked_arguments > 0
    assert production_checks.maximum_bits >= 96
    resumed = unchecked.restore(checked_session.checkpoint())
    assert resumed.contour_provenance.validation == "off"
    assert resumed.contour_provenance.pilots[0].complete
    assert resumed.contour_provenance.pilots[0].seed is None
    assert resumed.snapshot().estimate.mean == checked_session.snapshot().estimate.mean
    assert checked_session.snapshot().evaluation_diagnostics.contour.production.checked_arguments == 0
    assert unchecked_session.snapshot().evaluation_diagnostics.contour is None
    checked_mc = configured.mc_session(sd.HavanaDiscreteSettings(points_per_batch=32, batches=2))
    while not checked_mc.complete:
        checked_mc.step()
    resumed_mc = unchecked.restore_mc(checked_mc.checkpoint())
    assert resumed_mc.contour_provenance.validation == "off"
    assert resumed_mc.contour_provenance.pilots[0].complete


def test_contour_flag_reaches_retained_and_native_diagram_entrypoints():
    arguments = fixed_arguments(inputs.massive_triangle())
    integral = sd.Integral(**arguments)
    owner = integral.generation_session(contour=True, compilation_settings=sd.CompilationSettings(horner_iterations=0))
    assert owner.contour and owner.generated is owner.kernels is None
    while not owner.complete:
        owner.step()
    assert owner.generated.contour_capable and owner.kernels.contour_capable
    diagram = arguments.pop("diagram")
    direct = diagram.sector_decompose(**arguments, contour=True, progress=None)
    assert direct.contour_capable


@pytest.fixture(scope="module")
def exact_contour_template():
    from symbolica import E, S
    from symbolica.community import hepkit as hep

    d, k, eps = S("exact_contour::D", "exact_contour::k", "exact_contour::eps")
    kin = hep.Kinematics(d, momenta=[k])
    family = hep.IntegralFamily([k], [], [kin.scalar_product(k, k) - E("1")],
                               kinematics=kin)
    generated = family.sector_decompose(
        powers=[1], numerator=E("1"), regulator=eps, dimension=2 - 2 * eps,
        max_order=0, contour=True, coefficient_expansion="coefficient_series", progress=None,
    )
    kernels = generated.compile(settings=sd.CompilationSettings(horner_iterations=0), progress=None)
    assert kernels.sector_count == 0
    return sd.Kernels.from_bytes(kernels.to_bytes())


@pytest.mark.parametrize("policy", ["always", "pilot"])
@pytest.mark.parametrize("lane", ["qmc", "mc"])
def test_exact_only_contour_sessions_require_pilot_on_creation_and_restore(
        exact_contour_template, policy, lane):
    configured = exact_contour_template.with_parameters({}, contour=sd.ContourSettings.fixed(
        0.01, validation=policy, pilot_points=2))
    assert any(value != 0 for value in configured.exact_coefficients)
    create = (lambda owner: owner.session(sd.QmcSettings(points=32, shifts=2, rule="hkkn_alpha3"))) if lane == "qmc" else (
        lambda owner: owner.mc_session(sd.HavanaDiscreteSettings(points_per_batch=32, batches=2)))
    restore = (lambda owner, state: owner.restore(state)) if lane == "qmc" else (
        lambda owner, state: owner.restore_mc(state))
    with pytest.raises(sd.FastSecDecError, match="pilot"):
        create(configured)
    for chart in configured.contour_validation_charts:
        for _ in range(2):
            configured.validate_contour_point(chart.chart_index, [0.5] * chart.dimension)
    configured.finish_contour_pilot()
    session = create(configured)
    assert session.complete
    checkpoint = session.checkpoint()
    unready = exact_contour_template.with_parameters({}, contour=sd.ContourSettings.fixed(
        0.01, validation=policy, pilot_points=2))
    with pytest.raises(sd.FastSecDecError, match="pilot"):
        restore(unready, checkpoint)
    unchecked = unready.with_contour_validation("off", pilot_points=2)
    assert create(unchecked).snapshot().estimate.mean == session.snapshot().estimate.mean
    assert restore(unchecked, checkpoint).snapshot().estimate.mean == session.snapshot().estimate.mean
