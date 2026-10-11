"""Explicit primitive numerical interpretation on native index-bearing graphs.

The embedded test supplies native_payload_diagram using the same public native
FeynKit fragment/projector builders. No graph or conversion implementation is
mocked in Python, and no installed wheel is replaced.
"""
import gc
import json
import math
import os
from pathlib import Path
from symbolica import E, S
from symbolica.community import hepkit as hep
from symbolica.community.hepkit import sector_decomposition as sd

ROOT = Path(os.environ["FASTSECDEC_TEST_ROOT"]) if "FASTSECDEC_TEST_ROOT" in os.environ else Path(__file__).resolve().parents[3]

def test_graph_payload_primitive_policy_native_host():
    if "native_payload_diagram" not in globals():
        import pytest
        pytest.skip("requires the source-matched native graph-builder fixture in the embedded host")
    model=hep.Model(ROOT/"examples/models/scalar.json")
    base=hep.FeynmanDiagram.from_dot(model,(ROOT/"examples/graphs/bubble.dot").read_text())
    diagram=native_payload_diagram(base,True)
    original=diagram.to_json()
    eps,dim=S("payload_host::eps","payload_host::D")
    p,k=hep.Kinematics.external_momentum(),hep.Kinematics.loop_momentum()
    kin=hep.Kinematics(dim,momenta=[p(1),k(0)]).with_scalar_product(p(1),p(1),E("12.0"))
    gamma=E("gamma")
    args=dict(regulator=eps,dimension=4-2*eps,kinematics=kin,model_parameters="fixed",
              scalar_values={S("UFO::mt"):E("1.5")},
              measure_multiplier=gamma(1-2*eps)/(gamma(1+eps)*gamma(1-eps)**2))
    for values in ({"graph_payloads":"primitive_fragments_v1"},
                   {"numerical_meaning":"represented_values","graph_payloads":"guess_rationals"}):
        try:sd.ThresholdSettings(**values)
        except (ValueError,sd.FastSecDecError):pass
        else:raise AssertionError("invalid payload settings accepted")
    ordinary=sd.ThresholdSettings(numerical_meaning="represented_values")
    assert "graph_payloads" not in json.loads(ordinary.to_json())
    settings=sd.ThresholdSettings(numerical_meaning="represented_values",graph_payloads="primitive_fragments_v1")
    assert json.loads(settings.to_json())["graph_payloads"]=="primitive_fragments_v1"
    integral=sd.Integral(diagram,**args)
    try:integral.generation_session(threshold_decomposition=True,threshold_settings=ordinary).prepare(progress=None)
    except sd.FastSecDecError as error:assert "Float outside" in str(error)
    else:raise AssertionError("default payload meaning changed")
    session=integral.generation_session(threshold_decomposition=True,threshold_settings=settings,subtraction="integrate_by_parts")
    session.prepare(observer=lambda _:False,progress=None)
    assert session.preparation_progress_json() is None
    marker=KeyboardInterrupt("payload conversion")
    def cancel_payload(snapshot):
        if "Native graph Payload" in snapshot.detail:raise marker
    try:session.prepare(observer=cancel_payload,progress=None)
    except KeyboardInterrupt as error:assert error is marker
    else:raise AssertionError("missing payload cancellation stage")
    raw=RuntimeError("payload raw checkpoint")
    def cancel_raw(snapshot):
        if snapshot.detail=="Native threshold Verify":raise raw
    try:session.prepare(observer=cancel_raw,progress=None)
    except RuntimeError as error:assert error is raw
    else:raise AssertionError("missing raw checkpoint")
    phases=[]
    session.prepare(observer=lambda x:phases.append(x.detail),progress=None)
    assert session.prepared and "Native threshold Solve" not in phases
    session.compile_next(100,progress=None)
    kernels=session.result.select("threshold")
    identity=kernels.content_id
    saved=kernels.to_bytes()
    sampler=kernels.session(sd.QmcSettings(points=1024,shifts=8,seed=77113))
    while not sampler.complete:sampler.step(16)
    estimate=sampler.snapshot().estimate
    w=.1*.2*.3*.4*.7*12
    expected=[w,0,w*(2-math.log(9/4)-.5*math.log(3)),w*math.pi/2]
    assert len(estimate.mean)==4
    for got,se,want in zip(estimate.mean,estimate.standard_error,expected):
        assert abs(got-want)<=6*se+2e-8,(got,se,want)
    assert diagram.to_json()==original
    for call in (lambda:diagram.sector_decompose(**args,threshold_decomposition=True,threshold_settings=settings,
                  subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None),
                 lambda:sd.sector_decompose(diagram,**args,threshold_decomposition=True,threshold_settings=settings,
                  subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)):
        prepared=call()
        assert prepared.archive is None
        assert prepared.compile(progress=None).content_id==identity
    del session,integral,kernels,sampler,prepared
    gc.collect()
    assert sd.Kernels.from_bytes(saved).content_id==identity
    bad=sd.Integral(native_payload_diagram(base,False),**args)
    try:bad.generation_session(threshold_decomposition=True,threshold_settings=settings).prepare(progress=None)
    except sd.FastSecDecError as error:assert "Float under a function" in str(error)
    else:raise AssertionError("Float function metadata silently converted")
    print("explicit primitive payload host: native tensor, callbacks, replay, full complex oracle and saved reload PASS")
