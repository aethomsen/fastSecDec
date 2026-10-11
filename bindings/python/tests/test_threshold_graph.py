"""Native represented graph frontend; requires an actual rebuilt HEPKit host."""
import gc
import json
import math
import os
from pathlib import Path
from symbolica import E, S
from symbolica.community import hepkit as hep
from symbolica.community.hepkit import sector_decomposition as sd

# The embedded source-matched gate supplies this environment variable. Installed
# host tests use this repository's ordinary asset layout.
ROOT = Path(os.environ["FASTSECDEC_TEST_ROOT"]) if "FASTSECDEC_TEST_ROOT" in os.environ else Path(__file__).resolve().parents[3]

def test_represented_graph_frontend_replay_and_full_complex_oracle():
    model = hep.Model(ROOT / "examples/models/scalar.json")
    diagram = hep.FeynmanDiagram.from_dot(model, (ROOT / "examples/graphs/bubble.dot").read_text())
    eps, dim = S("hepkit_threshold_graph_py::eps", "hepkit_threshold_graph_py::D")
    p, k = hep.Kinematics.external_momentum(), hep.Kinematics.loop_momentum()
    kinematics = hep.Kinematics(dim, momenta=[p(1), k(0)]).with_scalar_product(p(1), p(1), E("16"))
    gamma = E("gamma")
    normalization = gamma(1-2*eps)/(gamma(1+eps)*gamma(1-eps)**2)
    arguments = dict(regulator=eps, dimension=4-2*eps, kinematics=kinematics,
        model_parameters="fixed", scalar_values={S("UFO::mt"): E("sqrt(3)")},
        measure_multiplier=normalization)
    family = diagram.propagator_family(kinematics=kinematics)
    # Cheap constructor validation remains immediate; no family algebra is needed.
    for override in ({"scalar_values": {dim:E("4")}}, {"powers": {999:1}}, {"powers": {2:0}}):
        try:sd.Integral(diagram,**dict(arguments,**override))
        except (sd.FastSecDecError,ValueError):pass
        else:raise AssertionError(("constructor guard changed",override))

    # Faithful Float graph frontend: construction and repr retain original values.
    float_kin=hep.Kinematics(dim,momenta=[p(1),k(0)]).with_scalar_product(p(1),p(1),E("12.0"))
    float_args=dict(arguments,kinematics=float_kin,scalar_values={S("UFO::mt"):E("1.5")})
    float_integral=sd.Integral(diagram,**float_args)
    assert float_integral.powers==[(2,1),(3,1)]
    assert "Native integral" in float_integral._repr_html_()
    try:float_integral.generate(progress=None)
    except sd.FastSecDecError as error:assert "rational" in str(error)
    else:raise AssertionError("ordinary exact family silently admitted Float")
    represented=sd.ThresholdSettings(numerical_meaning="represented_values",gcad_limits_json='{"memory_mib":1024}')
    assert json.loads(represented.to_json())["gcad_limits"]["workers"]==1
    try:sd.ThresholdSettings(numerical_meaning="uncertainty_bounds")
    except ValueError:pass
    else:raise AssertionError("uncertainty discarded")
    owner=float_integral.generation_session(threshold_decomposition=True,threshold_settings=represented,
        subtraction="integrate_by_parts")
    assert not owner.prepared and owner.snapshot().elapsed_seconds==0
    owner.prepare(observer=lambda _:False,progress=None)
    assert owner.preparation_progress_json() is None
    marker=KeyboardInterrupt("represented conversion")
    def cancel_conversion(event):
        if "Native graph Kinematics" in event.detail:raise marker
    try:owner.prepare(observer=cancel_conversion,progress=None)
    except KeyboardInterrupt as error:assert error is marker
    else:raise AssertionError("conversion callback missing")
    assert not owner.prepared and owner.failed is None
    raw_marker=RuntimeError("represented raw checkpoint")
    def cancel_raw(event):
        if event.detail=="Native threshold Verify":raise raw_marker
    try:owner.prepare(observer=cancel_raw,progress=None)
    except RuntimeError as error:assert error is raw_marker
    else:raise AssertionError("raw evidence callback missing")
    assert json.loads(owner.preparation_progress_json())["receipts"]["raw"] is not None
    stages=[]
    owner.prepare(observer=lambda event:stages.append(event.detail),progress=None)
    assert "Native threshold Solve" not in stages and owner.prepared
    assert any("Native graph Parameterization" in stage for stage in stages)
    assert owner.snapshot().timings.parametrization_seconds > 0
    assert owner.snapshot().elapsed_seconds >= owner.snapshot().timings.parametrization_seconds
    float_receipt=json.loads(owner.preparation_receipt_json())
    owner.compile_next(100,progress=None)
    assert owner.complete
    float_kernels=owner.result.select("threshold")
    float_bytes=float_kernels.to_bytes()
    float_id=float_kernels.content_id
    exact_args=dict(float_args,kinematics=hep.Kinematics(dim,momenta=[p(1),k(0)]).with_scalar_product(p(1),p(1),E("12")),scalar_values={S("UFO::mt"):E("3/2")})
    exact_prepared=diagram.sector_decompose(**exact_args,threshold_decomposition=True,
        subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)
    exact_source=json.loads(exact_prepared.preparation_receipt_json())["publication"]["source_identity"]
    assert float_receipt["publication"]["source_identity"]!=exact_source
    float_prepared=sd.sector_decompose(diagram,**float_args,threshold_decomposition=True,
        threshold_settings=represented,subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)
    assert float_prepared.snapshot().kernels==0 and float_prepared.archive is None
    assert json.loads(float_prepared.preparation_receipt_json())["publication"]["source_identity"]==float_receipt["publication"]["source_identity"]
    float_session=float_kernels.session(sd.QmcSettings(points=1024,shifts=8,seed=7331))
    while not float_session.complete:float_session.step(16)
    float_result=float_session.snapshot().estimate
    expected_float=[1.0,0.0,2-math.log(9/4)-0.5*math.log(3),math.pi/2]
    assert len(float_result.mean)==len(expected_float)
    for got,error,want in zip(float_result.mean,float_result.standard_error,expected_float):
        assert abs(got-want)<max(6*error,3e-3),(got,error,want)
    del owner,float_kernels,float_integral,float_prepared,float_session
    _ = gc.collect()
    restored=sd.Kernels.from_bytes(float_bytes)
    assert restored.content_id==float_id
    print("represented graph frontend, raw replay, full complex oracle and artifact-only reload: PASS")
