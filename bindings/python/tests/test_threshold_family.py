"""Represented native family point; actual rebuilt HEPKit host required."""
import gc
import json
import math
from symbolica import E, S
from symbolica.community import hepkit as hep
from symbolica.community.hepkit import sector_decomposition as sd


def test_represented_family_frontend_replay_and_full_complex_oracle():
    k,p,u,D,eps,s,m = S("hepkit_family_point::k", "hepkit_family_point::p", "hepkit_family_point::unused", "hepkit_family_point::D", "hepkit_family_point::eps", "hepkit_family_point::s", "hepkit_family_point::m2")
    kin=hep.Kinematics(D,momenta=[k,p,u]).with_scalar_product(p,p,s).with_scalar_product(u,u,E("7"))
    family=hep.IntegralFamily([k],[p],[kin.scalar_product(k,k)-m,kin.scalar_product(k+p,k+p)-m],kinematics=kin)
    gamma=E("gamma")
    normalization=gamma(1-2*eps)/(gamma(1+eps)*gamma(1-eps)**2)
    fixed=kin.with_scalar_product(p,p,E("16.0")).with_scalar_product(u,u,E("7.0"))
    args=dict(regulator=eps,dimension=4-2*eps,powers=[1,1],numerator=E("0.5"),
              kinematics=fixed,scalar_values={s:E("16.0"),m:E("3.0")},measure_multiplier=E("2.0")*normalization)
    integral=sd.Integral.from_family(family,**args)
    assert "Native integral" in integral._repr_html_()
    try:integral.generate(progress=None)
    except sd.FastSecDecError:pass
    else:raise AssertionError("ordinary family silently converted Float")
    settings=sd.ThresholdSettings(numerical_meaning="represented_values")
    owner=integral.generation_session(threshold_decomposition=True,threshold_settings=settings,subtraction="integrate_by_parts")
    owner.prepare(observer=lambda _:False,progress=None)
    assert owner.preparation_progress_json() is None
    interrupt=KeyboardInterrupt("family conversion")
    def conversion(event):
        if "Native family Numerator" in event.detail:raise interrupt
    try:owner.prepare(observer=conversion,progress=None)
    except KeyboardInterrupt as caught:assert caught is interrupt
    else:raise AssertionError("family conversion callback missing")
    assert owner.failed is None and not owner.prepared
    marker=RuntimeError("family raw checkpoint")
    def after_raw(event):
        if event.detail=="Native threshold Verify":raise marker
    try:owner.prepare(observer=after_raw,progress=None)
    except RuntimeError as caught:assert caught is marker
    else:raise AssertionError("family durable callback missing")
    assert json.loads(owner.preparation_progress_json())["receipts"]["raw"] is not None
    seen=[]
    owner.prepare(observer=lambda event:seen.append(event.detail),progress=None)
    assert owner.prepared and "Native threshold Solve" not in seen
    assert any("Native family Parameterization" in detail for detail in seen)
    assert owner.snapshot().elapsed_seconds>=owner.snapshot().timings.parametrization_seconds>0
    receipt=json.loads(owner.preparation_receipt_json())
    owner.compile_next(100,progress=None)
    assert owner.complete
    kernels=owner.result.select("threshold")
    source=receipt["publication"]["source_identity"]
    prepared=family.sector_decompose(**args,threshold_decomposition=True,threshold_settings=settings,
        subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)
    assert type(prepared) is sd.PreparedThreshold and prepared.snapshot().kernels==0
    assert json.loads(prepared.preparation_receipt_json())["publication"]["source_identity"]==source
    free=sd.sector_decompose(family,**args,threshold_decomposition=True,threshold_settings=settings,
        subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)
    assert json.loads(free.preparation_receipt_json())["publication"]["source_identity"]==source
    exact_args=dict(args,kinematics=kin.with_scalar_product(p,p,E("16")),
                    numerator=E("1/2"),measure_multiplier=2*normalization,
                    scalar_values={s:E("16"),m:E("3")})
    exact=family.sector_decompose(**exact_args,threshold_decomposition=True,
        subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)
    assert json.loads(exact.preparation_receipt_json())["publication"]["source_identity"]!=source
    changed=dict(args,kinematics=fixed.with_scalar_product(u,u,E("9.0")))
    changed_owner=family.sector_decompose(**changed,threshold_decomposition=True,threshold_settings=settings,
        subtraction="integrate_by_parts",coefficient_expansion="coefficient_series",progress=None)
    assert json.loads(changed_owner.preparation_receipt_json())["publication"]["source_identity"]!=source
    # Binding changes must not reinterpret already-consumed native external products.
    try:family.sector_decompose(**dict(args,kinematics=fixed.with_scalar_product(p,p,E("17.0"))),
        threshold_decomposition=True,threshold_settings=settings,progress=None)
    except sd.FastSecDecError as error:assert "cannot change existing family external products" in str(error)
    else:raise AssertionError("foreign point accepted")
    bad=sd.Integral.from_family(family,**args,runtime_parameters=[s])
    try:bad.generation_session(threshold_decomposition=True,threshold_settings=settings).prepare(progress=None)
    except ValueError as error:assert "no runtime_parameters" in str(error)
    else:raise AssertionError("runtime family implicitly specialized")
    payload=kernels.to_bytes();identity=kernels.content_id
    del owner,prepared,free,exact,changed_owner,integral,bad,family,kernels
    gc.collect()
    restored=sd.Kernels.from_bytes(payload)
    assert restored.content_id==identity
    integration=restored.session(sd.QmcSettings(points=1024,shifts=8,seed=202610118051))
    while not integration.complete:integration.step(16)
    result=integration.snapshot().estimate
    expected=[1.,0.,2.-1.5*math.log(3.),math.pi/2]
    assert result.production_complete and len(result.mean)==len(expected)
    for value,error,truth in zip(result.mean,result.standard_error,expected):
        assert abs(value-truth)<8*error+1e-6,(value,error,truth)
    assert len(result.covariance_of_mean)==16
    print("represented family frontend: native original point, callbacks/replay, existing three entrypoints, original-context identity and full complex bubble PASS")
