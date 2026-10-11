//! Native-only caller-owned threshold preparation and incremental compilation.
//! No graph conversion, mathematical admission, estimator or worker pool here.
mod prepared;
mod session;
mod work;
use crate::{error, input::PyIntegral, settings::PyCompilationSettings};
use fastsecdec::threshold::{gcad, generation::PreparationOptions, represented};
use pyo3::prelude::*;

#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pyclass)]
#[pyclass(
    name = "ThresholdSettings",
    module = "symbolica.community.hepkit.sector_decomposition",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyThresholdSettings {
    limits: gcad::Limits,
    solver: gcad::SolverOptions,
    transport_bytes: u64,
    represented: Option<(represented::NumericalMeaning, represented::Limits)>,
    graph_payloads: Option<represented::graph::GraphPayloadPolicy>,
}
impl Default for PyThresholdSettings {
    fn default() -> Self {
        Self {
            limits: gcad::GcadRequest::default_limits(),
            solver: Default::default(),
            transport_bytes: 1 << 30,
            represented: None,
            graph_payloads: None,
        }
    }
}
#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pymethods)]
#[pymethods]
impl PyThresholdSettings {
    /// Native GCAD JSON schemas, without a second settings implementation.
    /// limits.workers must remain one; schedule processes in the caller.
    /// Byte limits cover transport, not native decoded/compiler RSS.
    /// numerical_meaning="represented_values" explicitly preserves the stored
    /// Float values in fixed kinematics, scalar bindings and loop measure.
    /// An already exact IntegralFamily also accepts an explicit represented
    /// scalar numerator. Original native owners and precision are retained,
    /// without guessing rationals or inferring uncertainty. Stored family basis
    /// and assumptions must be exact; this cannot recover numerical arithmetic
    /// performed before supplying them. Graph payloads remain exact unless
    /// graph_payloads="primitive_fragments_v1" explicitly chooses conversion of
    /// each stored local fragment before recomputing the native aggregate.
    /// The original rounded aggregate is provenance only under that policy.
    /// Float arguments of functions (including scalar f(0.1), index/momentum
    /// labels and shorthand arguments) are currently refused.
    /// Omit numerical_meaning to retain the ordinary exact-input path.
    #[new]
    #[pyo3(signature=(*, gcad_limits_json=None, solver_options_json=None, maximum_transport_bytes=1_073_741_824, numerical_meaning=None, represented_limits_json=None, graph_payloads=None))]
    fn new(
        py: Python<'_>,
        gcad_limits_json: Option<&str>,
        solver_options_json: Option<&str>,
        maximum_transport_bytes: u64,
        numerical_meaning: Option<&str>,
        represented_limits_json: Option<&str>,
        graph_payloads: Option<&str>,
    ) -> PyResult<Self> {
        if maximum_transport_bytes == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "maximum_transport_bytes must be positive",
            ));
        }
        let mut defaults = serde_json::to_value(gcad::GcadRequest::default_limits())
            .map_err(|e| error::native(py, "threshold settings", e))?;
        if let Some(json) = gcad_limits_json {
            let overrides: serde_json::Value = serde_json::from_str(json)
                .map_err(|e| error::native(py, "threshold settings", e))?;
            let values = overrides.as_object().ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err("gcad_limits_json must be an object")
            })?;
            defaults
                .as_object_mut()
                .expect("native limits object")
                .extend(values.clone());
        }
        let limits: gcad::Limits = serde_json::from_value(defaults)
            .map_err(|e| error::native(py, "threshold settings", e))?;
        if limits.workers != 1 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "native GCAD requires workers=1; schedule processes in the caller",
            ));
        }
        let represented = match numerical_meaning {
            None => {
                if represented_limits_json.is_some() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "represented_limits_json requires explicit numerical_meaning",
                    ));
                }
                None
            }
            Some(value) => {
                // Deserialize the existing native policy; no second numerical
                // interpretation or Float-to-rational conversion lives here.
                let meaning: represented::NumericalMeaning =
                    serde_json::from_value(serde_json::Value::String(value.to_owned()))
                        .map_err(|e| error::native(py, "represented input policy", e))?;
                if meaning != represented::NumericalMeaning::RepresentedValues {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "uncertainty bounds require an interval/error contract; use represented_values only for the actual stored values",
                    ));
                }
                let mut limits = serde_json::to_value(represented::Limits::default())
                    .map_err(|e| error::native(py, "represented input limits", e))?;
                if let Some(json) = represented_limits_json {
                    let overrides: serde_json::Value = serde_json::from_str(json)
                        .map_err(|e| error::native(py, "represented input limits", e))?;
                    let values = overrides.as_object().ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err(
                            "represented_limits_json must be an object",
                        )
                    })?;
                    limits
                        .as_object_mut()
                        .expect("native represented limits object")
                        .extend(values.clone());
                }
                Some((
                    meaning,
                    serde_json::from_value(limits)
                        .map_err(|e| error::native(py, "represented input limits", e))?,
                ))
            }
        };
        let graph_payloads = graph_payloads
            .map(|value| {
                if represented.is_none() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "graph_payloads requires numerical_meaning='represented_values'",
                    ));
                }
                serde_json::from_value(serde_json::Value::String(value.to_owned()))
                    .map_err(|e| error::native(py, "graph payload policy", e))
            })
            .transpose()?;
        Ok(Self {
            limits,
            solver: solver_options_json
                .map(serde_json::from_str)
                .transpose()
                .map_err(|e| error::native(py, "threshold settings", e))?
                .unwrap_or_default(),
            transport_bytes: maximum_transport_bytes,
            represented,
            graph_payloads,
        })
    }
    fn to_json(&self) -> PyResult<String> {
        let mut value = serde_json::json!({"gcad_limits":self.limits,"solver_options":self.solver,"maximum_transport_bytes":self.transport_bytes,"represented":self.represented});
        if let Some(policy) = self.graph_payloads {
            value["graph_payloads"] = serde_json::to_value(policy)
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        }
        serde_json::to_string(&value)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
}
#[pymethods]
impl PyIntegral {
    /// Inert native work. prepare() performs parametrization/verified preparation;
    /// compile_next() performs compilation. Neither action starts integration.
    /// Initially supports fixed inputs with one admitted rational integration axis.
    #[pyo3(signature=(max_order=0, *, coefficient_expansion="coefficient_series", subtraction="integrate_by_parts", compilation_settings=None, threshold_settings=None))]
    pub(crate) fn threshold_generation_session(
        &self,
        py: Python<'_>,
        max_order: i32,
        coefficient_expansion: &str,
        subtraction: &str,
        compilation_settings: Option<&PyCompilationSettings>,
        threshold_settings: Option<&PyThresholdSettings>,
    ) -> PyResult<session::PyThresholdGenerationSession> {
        let mut options =
            PreparationOptions::new(symbolica::symbol!("fastsecdec::hepkit::threshold_unit"));
        options.generation = crate::generation::options(
            max_order,
            coefficient_expansion,
            "symbolic",
            subtraction,
            false,
            "symbolic",
        )?;
        options.compilation = compilation_settings.cloned().unwrap_or_default().inner;
        options
            .compilation
            .validate()
            .map_err(|e| error::native(py, "threshold compilation", e))?;
        if options.compilation.contour_jacobian != fastsecdec::contour::ContourJacobian::Symbolic {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "threshold generation does not use a contour Jacobian",
            ));
        }
        let settings = threshold_settings.cloned().unwrap_or_default();
        options.threshold.gcad_limits = settings.limits;
        options.threshold.solver = settings.solver;
        options.represented = settings.represented;
        options.graph_payloads = settings.graph_payloads;
        if options.graph_payloads.is_some()
            && !matches!(
                self.represented_point()?,
                crate::input::RepresentedPoint::Graph(_)
            )
        {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "graph_payloads requires an original graph input, not an IntegralFamily",
            ));
        }
        Ok(session::PyThresholdGenerationSession::new(
            self.clone(),
            options,
            settings.transport_bytes,
        ))
    }
}
pub(crate) fn register(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<PyThresholdSettings>()?;
    module.add_class::<prepared::PyPreparedThreshold>()?;
    module.add_class::<session::PyThresholdGenerationSession>()?;
    Ok(())
}

#[cfg(feature = "python_stubgen")]
pyo3_stub_gen::inventory::submit! {
    pyo3_stub_gen::derive::gen_methods_from_python! {
        r#"
import typing
import symbolica.community.hepkit.sector_decomposition

class PyIntegral:
    def threshold_generation_session(self, max_order: int = 0, *,
        coefficient_expansion: str = "coefficient_series",
        subtraction: str = "integrate_by_parts",
        compilation_settings: typing.Optional[symbolica.community.hepkit.sector_decomposition.CompilationSettings] = None,
        threshold_settings: typing.Optional[symbolica.community.hepkit.sector_decomposition.ThresholdSettings] = None,
    ) -> symbolica.community.hepkit.sector_decomposition.ThresholdGenerationSession:
        """Create inert native threshold work; preparation and compilation are explicit."""
"#
    }
}

/// Reuse the session admission and publication path, returning only a prepared
/// owner. The ordinary convenience action's cancellation contract is unchanged.
#[allow(clippy::too_many_arguments)]
pub(crate) fn prepare_entry(
    input: &PyIntegral,
    py: Python<'_>,
    max_order: i32,
    coefficient_expansion: &str,
    mode: &str,
    subtraction: &str,
    contour: bool,
    contour_jacobian: &str,
    settings: Option<&PyCompilationSettings>,
    threshold_settings: Option<&Bound<'_, PyAny>>,
    observer: Option<Py<PyAny>>,
    progress: Option<Py<PyAny>>,
) -> PyResult<Py<PyAny>> {
    if mode != "symbolic" || contour || contour_jacobian != "symbolic" {
        return Err(pyo3::exceptions::PyValueError::new_err(
            "threshold generation requires symbolic endpoints, contour off and symbolic Jacobian policy",
        ));
    }
    let options = threshold_settings
        .map(|s| s.extract::<PyRef<'_, PyThresholdSettings>>())
        .transpose()?;
    let mut session = input.threshold_generation_session(
        py,
        max_order,
        coefficient_expansion,
        subtraction,
        settings,
        options.as_deref(),
    )?;
    session.prepare(py, observer, progress)?;
    Ok(Py::new(
        py,
        prepared::PyPreparedThreshold::from_prepared(py, session)?,
    )?
    .into_any())
}
