"""Public owner entry points share native physics, metadata and lazy execution."""

from dataclasses import replace
from pathlib import Path
import sys

import pytest
from symbolica import E, S
from symbolica.community import hepkit as hep

sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "examples/hepkit"))
from showcase import inputs
from _fixtures import fixed_arguments

sd = getattr(hep, "sector_decomposition", None)
pytestmark = pytest.mark.skipif(sd is None, reason="requires sector_decomposition wheel")


def arguments(value):
    result = fixed_arguments(value)
    result.pop("diagram")
    return result


def test_public_types_use_canonical_module():
    for name in ("Integral", "GeneratedIntegral", "Kernels", "QmcSettings",
                 "QmcSession", "FastSecDecError", "CancelledError"):
        assert getattr(sd, name).__module__ == "symbolica.community.hepkit.sector_decomposition"


def test_diagram_expression_adapter_preserves_tensor_interfaces_and_ranked_zero():
    from symbolica.community import tensor as sp

    diagram = inputs.massive_triangle().diagram
    vector = sp.TensorName.vector("fastsecdec_input::v")(sp.Representation.mink(4))
    zero = vector.derivative(S("fastsecdec_input::absent"))
    assert zero.rank == 1 and zero.to_expression() == E("0")
    for source in (vector, zero):
        copied = sd.with_diagram_expressions(diagram, projector=source)
        restored = copied.projector_expression()
        assert restored.structure.axes == source.structure.axes
        assert restored.to_expression() == source.to_expression()


def test_weighted_diagram_family_and_compatibility_generation_agree():
    value = inputs.massive_triangle()
    family = value.diagram.propagator_family(kinematics=value.kinematics)
    k, p = family.loop_momenta[0], family.external_momenta[0]
    numerator = value.kinematics.scalar_product(k, k) + 2 * value.kinematics.scalar_product(k, p) + 7
    diagram = sd.with_diagram_expressions(
        value.diagram, numerator=numerator, projector=E("2"), overall_factor=E("3")
    )
    value = replace(value, diagram=diagram)
    args = arguments(value)
    args["measure_multiplier"] = E("5")
    events = []
    direct = sd.sector_decompose(diagram, **args, max_order=0,
                                coefficient_expansion="native_named", observer=events.append)
    assert events[0].stage == "parametrization"
    assert direct.snapshot().kernels == 0 and direct.snapshot().stage == "compilation"
    assert not hasattr(direct, "session")
    assert direct.metadata.charts and any(sum(s.alias_counts) for s in direct.sectors)
    family_args = dict(args)
    family_args.pop("kinematics")  # The native family's own assumptions are authoritative.
    family_args["powers"] = [1] * len(family.denominators)
    family_args["numerator"] = value.scalar_numerator()
    from_family = sd.sector_decompose(family, **family_args, coefficient_expansion="native_named")
    legacy = sd.Integral(diagram, **args).generate(coefficient_expansion="native_named")
    expected = direct.compile().to_bytes()
    assert from_family.compile().to_bytes() == expected
    assert legacy.compile().to_bytes() == expected


def test_diagram_admission_uses_explicit_width_point_before_model_defaults():
    model = hep.Model.standard_model()
    before = model.to_json()
    raw = hep.FeynmanDiagram.from_dot(model, '''digraph top_bubble {
        ext [style=invis];
        ext -> a [particle="g"];
        b -> ext [particle="g"];
        a -> b [particle="t", lmb_id=0];
        b -> a [particle="t"];
    }''')
    diagram = sd.with_diagram_expressions(raw, numerator=E("2"), projector=E("3"),
                                          overall_factor=E("5"))
    dimension, eps = S("atomic_point::D", "atomic_point::eps")
    basis = diagram.loop_momentum_basis
    independent = next(i for i, edge in enumerate(basis.external_edges)
                       if edge not in basis.dependent_externals)
    momentum = hep.Kinematics.external_momentum()(independent)
    kin = hep.Kinematics(dimension).with_scalar_product(
        momentum, momentum, E("-1"))
    mass, width = model.parameter("MT").symbol, model.parameter("WT").symbol
    args = dict(regulator=eps, dimension=2 - 2 * eps, kinematics=kin, max_order=0)
    for values in ({}, {width: E("1")}, {width: S("unknown_width")},
                   {width: E("0"), mass: E("1+1i")}):
        with pytest.raises(sd.FastSecDecError) as caught:
            diagram.sector_decompose(scalar_values=values, **args)
        assert caught.value.stage == "input"
    values = model.scalar_bindings(overrides={mass: E("1"), width: E("0")})
    generated = diagram.sector_decompose(scalar_values=values, **args)
    assert generated.snapshot().kernels == 0  # Compilation remains explicit.
    family = diagram.propagator_family(kinematics=kin)
    control = family.sector_decompose(numerator=E("30"), powers=[1, 1],
                                     scalar_values=values, **args)
    assert generated.compile().to_bytes() == control.compile().to_bytes()
    assert model.to_json() == before
    assert model.parameter("WT").value.real > 0


def family_fixture():
    d, k, p, mass, eps = S("entry_family::D", "entry_family::k", "entry_family::p",
                          "entry_family::m2", "entry_family::eps")
    kin = hep.Kinematics(d, momenta=[k, p]).with_scalar_product(p, p, E("-1"))
    denominator = kin.scalar_product(k, k) - mass
    family = hep.IntegralFamily([k], [p], [denominator], kinematics=kin)
    return family, family.complete(), eps, mass


@pytest.mark.parametrize("auxiliary_power", [0, -2])
def test_completed_family_auxiliary_slots_never_become_denominators(auxiliary_power):
    physical, completed, eps, mass = family_fixture()
    assert len(completed.denominators) == 2
    numerator = E("1") + physical.kinematics.scalar_product(
        physical.loop_momenta[0], physical.external_momenta[0]) ** 2
    args = dict(regulator=eps, dimension=2 - 2 * eps, scalar_values={mass: E("1")},
                measure_multiplier=E("3"), max_order=0)
    projected = sd.sector_decompose(completed, powers=[3, auxiliary_power], numerator=numerator, **args)
    expected = sd.sector_decompose(
        physical, powers=[3],
        numerator=numerator * completed.denominators[1] ** (-auxiliary_power), **args)
    assert projected.metadata.domain.parameters == expected.metadata.domain.parameters
    assert len(projected.metadata.domain.parameters) == 1
    assert projected.compile().to_bytes() == expected.compile().to_bytes()


def test_family_requires_explicit_physics_and_rejects_invalid_specialization():
    family, completed, eps, mass = family_fixture()
    for kwargs in ({"powers": [1]}, {"numerator": E("1")},
                   {"powers": [1, 0], "numerator": E("1")}):
        with pytest.raises((TypeError, ValueError, sd.FastSecDecError)):
            sd.sector_decompose(family, regulator=eps, **kwargs)
    with pytest.raises((ValueError, sd.FastSecDecError)):
        sd.sector_decompose(completed, regulator=eps, powers=[1, -(2**31)], numerator=E("1"))
    for scalar_values in ({family.kinematics.dimension: E("4")},
                          {family.loop_momenta[0]: E("1")},
                          {family.external_momenta[0]: E("1")},
                          {mass: family.external_momenta[0]},
                          {mass: family.kinematics.scalar_product(family.loop_momenta[0], family.loop_momenta[0])},
                          {mass: family.kinematics.scalar_product(family.loop_momenta[0], family.loop_momenta[0]) / 2},
                          {mass: hep.Kinematics(family.kinematics.dimension).scalar_product(
                              family.external_momenta[0], family.external_momenta[0])},
                          {mass: mass + 1}):
        with pytest.raises(sd.FastSecDecError) as caught:
            sd.sector_decompose(family, regulator=eps, powers=[2], numerator=E("1"),
                                scalar_values=scalar_values)
        assert caught.value.stage == "input"
    for powers in ([0], [-1]):
        with pytest.raises(sd.FastSecDecError) as caught:
            sd.sector_decompose(family, regulator=eps, powers=powers, numerator=E("1"))
        assert caught.value.stage == "input"


def test_compatible_auxiliary_kinematics_and_requested_dimension_reach_native_moments():
    family, _, eps, mass = family_fixture()
    k, p = family.loop_momenta[0], family.external_momenta[0]
    e = S("entry_family::polarization")
    dimension = family.kinematics.dimension
    extended = family.kinematics.with_scalar_product(e, e, E("-2"))
    extended = extended.with_scalar_product(p, e, E("0"))
    numerator = dimension * extended.scalar_product(k, e) ** 2
    args = dict(regulator=eps, dimension=2 - 2 * eps, powers=[3], numerator=numerator,
                scalar_values={mass: E("1")}, max_order=0)
    generated = sd.sector_decompose(family, kinematics=extended, auxiliary_momenta=[e], **args)
    explicit = hep.IntegralFamily([k], [p, e], family.denominators, kinematics=extended)
    control = sd.sector_decompose(explicit, **args)
    assert generated.compile().to_bytes() == control.compile().to_bytes()
    # Gaussian contraction D*(k.e)^2 -> e^2*k^2; at D=2, m^2=1,
    # J2=1 and J3=-1/2 in the normalized Minkowski measure. This checks
    # requested dimension and numerator contraction, not just matching paths.
    assert generated.orders == [0]
    assert generated.compile().exact_coefficients == [-1.0]
    # This integral has a pole at D=4. The explicit D=2 above must matter.
    default_dimension = dict(args)
    default_dimension.pop("dimension")
    default = sd.sector_decompose(explicit, **default_dimension)
    assert -1 in default.orders
    conflicting = family.kinematics.with_scalar_product(p, p, E("-2"))
    with pytest.raises(sd.FastSecDecError) as caught:
        sd.sector_decompose(family, regulator=eps, powers=[3], numerator=E("1"),
                            kinematics=conflicting)
    assert caught.value.stage == "input"


def test_generation_owner_methods_and_observer_errors_use_the_same_entry():
    value = inputs.massive_triangle()
    args = arguments(value)
    with pytest.raises(sd.CancelledError) as caught:
        value.diagram.sector_decompose(**args, observer=lambda _: False)
    assert caught.value.stage == "generation"
    family = value.diagram.propagator_family(kinematics=value.kinematics)
    kwargs = dict(regulator=value.regulator, powers=[1] * len(family.denominators),
                  numerator=value.scalar_numerator(), scalar_values=value.scalar_values)

    class MarkerError(Exception):
        pass

    def fail(_):
        raise MarkerError("original observer exception")

    for call in (lambda: sd.sector_decompose(family, observer=fail, **kwargs),
                 lambda: family.sector_decompose(observer=fail, **kwargs)):
        with pytest.raises(MarkerError, match="original observer exception"):
            call()
    events = []
    generated = family.sector_decompose(observer=events.append, **kwargs)
    assert generated.snapshot().kernels == 0 and events[-1].stage == "compilation"
    kernels = generated.compile()
    session = kernels.session(sd.QmcSettings(points=1024, shifts=2, package_points=8))
    assert session.snapshot().completed_points == 0


def test_diagram_does_not_infer_kinematics_or_ignore_an_extra_numerator():
    value = inputs.massive_triangle()
    with pytest.raises((TypeError, ValueError, sd.FastSecDecError)):
        sd.sector_decompose(value.diagram, regulator=value.regulator)
    with pytest.raises((TypeError, ValueError, sd.FastSecDecError)):
        sd.sector_decompose(value.diagram, **arguments(value), numerator=E("7"))
    with pytest.raises(TypeError):
        sd.sector_decompose(object(), regulator=value.regulator)
