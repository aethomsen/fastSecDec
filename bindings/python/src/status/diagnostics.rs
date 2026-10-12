use fastsecdec::status::EvaluationDiagnostics;
use pyo3::prelude::*;

/// Native caller-aggregated evaluation counters, including failed attempts.
#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pyclass)]
#[pyclass(
    frozen,
    from_py_object,
    module = "symbolica.community.hepkit.sector_decomposition",
    name = "EvaluationDiagnostics"
)]
#[derive(Clone)]
pub(crate) struct PyEvaluationDiagnostics {
    pub(super) inner: EvaluationDiagnostics,
}

#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pymethods)]
#[pymethods]
impl PyEvaluationDiagnostics {
    /// Optional native work observations, separate from causal validation.
    /// None means observations were not recorded, not zero work.
    #[getter]
    fn contour_runtime(&self) -> Option<crate::contour::PyContourRuntimeDiagnostics> {
        self.inner.contour_runtime.as_deref().map(|inner| {
            crate::contour::PyContourRuntimeDiagnostics {
                inner: inner.clone(),
            }
        })
    }
    #[getter]
    fn contour(&self) -> Option<crate::contour::PyContourEvaluationDiagnostics> {
        self.inner
            .contour
            .map(|inner| crate::contour::PyContourEvaluationDiagnostics { inner })
    }
    #[getter]
    fn f64_points(&self) -> u64 {
        self.inner.f64_points
    }
    #[getter]
    fn double_float_points(&self) -> u64 {
        self.inner.double_float_points
    }
    #[getter]
    fn arbitrary_points(&self) -> u64 {
        self.inner.arbitrary_points
    }
    #[getter]
    fn unstable_points(&self) -> u64 {
        self.inner.unstable_points
    }
    #[getter]
    fn cutoff_zero_points(&self) -> u64 {
        self.inner.cutoff_zero_points
    }
    #[getter]
    fn unclassified_points(&self) -> u64 {
        self.inner.unclassified_points()
    }
    #[getter]
    fn f64_timing(&self) -> PyEvaluatorTiming {
        PyEvaluatorTiming {
            inner: self.inner.f64_timing,
        }
    }
    #[getter]
    fn double_float_timing(&self) -> PyEvaluatorTiming {
        PyEvaluatorTiming {
            inner: self.inner.double_float_timing,
        }
    }
    #[getter]
    fn arbitrary_timing(&self) -> PyEvaluatorTiming {
        PyEvaluatorTiming {
            inner: self.inner.arbitrary_timing,
        }
    }
    /// Tracked evaluator work, including algebraic root conditioning and rescue checks.
    /// A default bucket may mean unrecorded or disabled work rather than zero cost.
    #[getter]
    fn conditioning_timing(&self) -> PyEvaluatorTiming {
        PyEvaluatorTiming {
            inner: self.inner.conditioning_timing,
        }
    }
    #[getter]
    fn evaluations(&self) -> u64 {
        self.inner.evaluations
    }

    #[getter]
    fn conditioning_checks(&self) -> u64 {
        self.inner.conditioning_checks
    }

    #[getter]
    fn rescues(&self) -> u64 {
        self.inner.rescues
    }

    #[getter]
    fn max_precision_bits(&self) -> u32 {
        self.inner.max_precision_bits
    }

    #[getter]
    fn failures(&self) -> u64 {
        self.inner.failures
    }

    #[getter]
    fn weighted_checks(&self) -> u64 {
        self.inner.weighted_checks
    }

    #[getter]
    fn additional_replays(&self) -> u64 {
        self.inner.additional_replays
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pyclass)]
#[pyclass(
    name = "EvaluatorTiming",
    module = "symbolica.community.hepkit.sector_decomposition",
    frozen,
    from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyEvaluatorTiming {
    inner: fastsecdec::kernel::EvaluatorTiming,
}
#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pymethods)]
#[pymethods]
impl PyEvaluatorTiming {
    /// Evaluated points, including speculative or repeated numerical attempts.
    #[getter]
    fn calls(&self) -> u64 {
        self.inner.calls
    }
    #[getter]
    fn nanoseconds(&self) -> u64 {
        self.inner.nanoseconds
    }
    #[getter]
    fn matrix_invocations(&self) -> u64 {
        self.inner.matrix_invocations
    }
    #[getter]
    fn matrix_points(&self) -> u64 {
        self.inner.matrix_points
    }
}
