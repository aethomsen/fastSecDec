use super::work::Work;
use crate::{
    error, family::archive::PyRecipeArchive, input::PyIntegral, progress::GenerationProgress,
    status::PyGenerationSnapshot,
};
use fastsecdec::{
    status::{GenerationSnapshot, GenerationStage},
    threshold::generation::{PreparationOptions, Progress, Stage},
};
use pyo3::{prelude::*, types::PyString};
use std::{ops::ControlFlow, time::Instant};

#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pyclass)]
#[pyclass(
    name = "ThresholdGenerationSession",
    module = "symbolica.community.hepkit.sector_decomposition",
    unsendable
)]
pub(crate) struct PyThresholdGenerationSession {
    input: PyIntegral,
    work: Work,
    status: GenerationSnapshot,
    result: Option<Py<PyRecipeArchive>>,
    failed: Option<String>,
}
impl PyThresholdGenerationSession {
    pub(super) fn new(input: PyIntegral, options: PreparationOptions, limit: u64) -> Self {
        Self {
            input,
            work: Work::new(options, limit),
            status: GenerationSnapshot {
                stage: GenerationStage::Input,
                completed: 0,
                total: None,
                sectors: 0,
                kernels: 0,
                elapsed_seconds: 0.0,
                timings: Default::default(),
                coefficient_expansion: None,
                formula_preparation: None,
                detail: "Ready; call prepare(), then compile_next()".into(),
            },
            result: None,
            failed: None,
        }
    }
    fn check(&self, py: Python<'_>) -> PyResult<()> {
        if let Some(e) = &self.failed {
            Err(error::native(py, "threshold generation", e))
        } else {
            Ok(())
        }
    }
    pub(super) fn compile_remaining(
        &mut self,
        py: Python<'_>,
        observer: Option<Py<PyAny>>,
        progress: Option<Py<PyAny>>,
    ) -> PyResult<PyGenerationSnapshot> {
        let remaining = self.work.jobs().saturating_sub(self.work.next).max(1);
        self.compile_next(py, remaining, observer, progress)
    }
    fn prepare_inner(
        &mut self,
        py: Python<'_>,
        progress: &mut GenerationProgress<'_>,
    ) -> PyResult<()> {
        self.check(py)?;
        if self.work.receipt.is_some() {
            return Ok(());
        }
        // False or throwing callbacks have no parameterization/storage side effects.
        if !progress.observe(py, &self.status)? {
            return Ok(());
        }
        if !self.work.initialized() {
            crate::citations::mark_threshold_generation();
            if self.work.options.represented.is_some() {
                let source = self.input.represented_point()?;
                self.work
                    .initialize_represented(source)
                    .map_err(|e| error::native(py, "threshold input", e))?;
            } else {
                self.status.stage = GenerationStage::Parametrization;
                self.status.detail = "Parametrizing the existing native HEPKit input".into();
                let started = Instant::now();
                let (input, runtime) = self.input.parametrize(py)?;
                let seconds = started.elapsed().as_secs_f64();
                self.status.timings.parametrization_seconds += seconds;
                self.status.elapsed_seconds += seconds;
                self.status.timings.total_seconds = self.status.elapsed_seconds;
                if !runtime.parameters.is_empty() || !runtime.masses.is_empty() {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "threshold preparation currently requires fixed physical input; use model_parameters='fixed' and no runtime_parameters",
                    ));
                }
                self.work
                    .initialize(input)
                    .map_err(|e| error::native(py, "threshold preparation", e))?;
            }
        }
        let elapsed = self.status.elapsed_seconds;
        let started = Instant::now();
        let status = &mut self.status;
        let mut callback_error = None;
        let mut input_parametrization_started: Option<Instant> = None;
        let result = self.work.prepare(|native| {
            let parameterizing = parameterizing(native);
            if parameterizing && input_parametrization_started.is_none() {
                input_parametrization_started = Some(Instant::now());
            } else if !parameterizing && let Some(started) = input_parametrization_started.take() {
                status.timings.parametrization_seconds += started.elapsed().as_secs_f64();
            }
            update(status, native, elapsed + started.elapsed().as_secs_f64());
            match progress.observe(py, status) {
                Ok(true) => ControlFlow::Continue(()),
                Ok(false) => ControlFlow::Break(()),
                Err(e) => {
                    callback_error = Some(e);
                    ControlFlow::Break(())
                }
            }
        });
        if let Some(started) = input_parametrization_started.take() {
            self.status.timings.parametrization_seconds += started.elapsed().as_secs_f64();
        }
        self.status.elapsed_seconds = elapsed + started.elapsed().as_secs_f64();
        self.status.timings.total_seconds = self.status.elapsed_seconds;
        if let Some(e) = callback_error {
            return Err(e);
        }
        match result {
            Ok(true) => {
                let receipt = self
                    .work
                    .receipt
                    .as_ref()
                    .expect("completed native preparation");
                self.status.stage = GenerationStage::Compilation;
                self.status.completed = 0;
                self.status.total = Some(receipt.jobs);
                self.status.sectors = receipt.endpoint_charts;
                self.status.detail = "Verified preparation ready; call compile_next()".into();
                Ok(())
            }
            Ok(false) => {
                self.status.detail="Preparation paused; call prepare() to continue (durable raw evidence is reverified)".into();
                Ok(())
            }
            Err(e) => {
                self.failed = Some(e.to_string());
                Err(error::native(py, "threshold preparation", e))
            }
        }
    }
    fn compile_inner(
        &mut self,
        py: Python<'_>,
        max_units: usize,
        progress: &mut GenerationProgress<'_>,
    ) -> PyResult<()> {
        self.check(py)?;
        if self.result.is_some() {
            return Ok(());
        }
        if self.work.receipt.is_none() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "call prepare() before compile_next()",
            ));
        }
        for _ in 0..max_units {
            if !progress.observe(py, &self.status)? {
                break;
            }
            let started = Instant::now();
            let compiled = self.work.compile_one();
            let seconds = started.elapsed().as_secs_f64();
            self.status.timings.compilation_seconds += seconds;
            self.status.elapsed_seconds += seconds;
            self.status.timings.total_seconds = self.status.elapsed_seconds;
            if let Err(e) = compiled {
                self.failed = Some(e.to_string());
                return Err(error::native(py, "threshold compilation", e));
            }
            self.status.completed = self.work.next;
            self.status.kernels = self.work.next.saturating_sub(1);
            self.status.detail = format!(
                "Compiled {} of {} native threshold records",
                self.work.next,
                self.work.jobs()
            );
            if self.work.next == self.work.jobs() {
                // Publication is transactional at the Python owner boundary.
                // A consumed native writer cannot be retried after IO/allocation failure.
                let candidate = (|| -> PyResult<(GenerationSnapshot, Py<PyRecipeArchive>)> {
                    let started = Instant::now();
                    let (writer, catalogue) = self
                        .work
                        .finish()
                        .map_err(|e| error::native(py, "threshold archive", e))?;
                    let count = catalogue
                        .recipe(fastsecdec::kernel::ProgramRecipe::ThresholdV1)
                        .map_err(|e| error::native(py, "threshold archive", e))?
                        .sector_count();
                    let mut status = self.status.clone();
                    let seconds = started.elapsed().as_secs_f64();
                    status.timings.compilation_seconds += seconds;
                    status.elapsed_seconds += seconds;
                    status.timings.total_seconds = status.elapsed_seconds;
                    status.stage = GenerationStage::Complete;
                    status.sectors = count;
                    status.kernels = count;
                    status.detail =
                        "Native threshold archive ready; selection and integration are explicit"
                            .into();
                    let storage = self.work.storage.as_ref().expect("native storage").clone();
                    let path = storage.path().join("integral.fsd");
                    let archive = PyRecipeArchive::threshold_generated(
                        py,
                        storage,
                        path,
                        writer,
                        catalogue,
                        status.clone(),
                    )?;
                    Ok((status, Py::new(py, archive)?))
                })();
                match candidate {
                    Ok((status, archive)) => {
                        self.status = status;
                        self.result = Some(archive);
                    }
                    Err(e) => {
                        self.failed = Some(e.to_string());
                        return Err(e);
                    }
                }
                // A final observer exception retains the accepted complete archive.
                progress.observe(py, &self.status)?;
                break;
            }
        }
        Ok(())
    }
}
fn parameterizing(p: &fastsecdec::threshold::generation::Progress) -> bool {
    p.graph_input.is_some_and(|d| {
        d.stage == fastsecdec::threshold::represented::graph::Stage::Parameterization
    }) || p.family_input.is_some_and(|d| {
        d.stage == fastsecdec::threshold::represented::family::Stage::Parameterization
    })
}
fn update(status: &mut GenerationSnapshot, p: &Progress, elapsed: f64) {
    status.stage = match p.stage {
        Stage::Input => GenerationStage::Input,
        Stage::Solve | Stage::Verify => GenerationStage::Geometry,
        Stage::Regularize => GenerationStage::Mapping,
        Stage::Continue => GenerationStage::Subtraction,
        Stage::Bind | Stage::StageVectors => GenerationStage::CoefficientExpansion,
        Stage::Complete => GenerationStage::Compilation,
    };
    status.completed = p.completed;
    status.total = p.total;
    status.elapsed_seconds = elapsed;
    status.timings.total_seconds = elapsed;
    status.detail = p.graph_input.map_or_else(
        || {
            p.family_input.map_or_else(
                || format!("Native threshold {:?}", p.stage),
                |detail| {
                    format!(
                        "Native family {:?}; {} represented literals",
                        detail.stage, detail.converted_literals
                    )
                },
            )
        },
        |detail| {
            format!(
                "Native graph {:?}; {} represented literals",
                detail.stage, detail.converted_literals
            )
        },
    );
    if parameterizing(p) {
        status.stage = GenerationStage::Parametrization;
    }
}
#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pymethods)]
#[pymethods]
impl PyThresholdGenerationSession {
    /// Use the existing caller-stepped workflow. Preparation consumes one unit;
    /// each remaining unit compiles one local record. Integration stays separate.
    #[pyo3(signature=(max_units=1,*,observer=None,progress=Some(Python::attach(|py| PyString::new(py,"auto").into_any().unbind()))))]
    fn step(
        &mut self,
        py: Python<'_>,
        max_units: usize,
        observer: Option<Py<PyAny>>,
        progress: Option<Py<PyAny>>,
    ) -> PyResult<PyGenerationSnapshot> {
        if max_units == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "max_units must be positive",
            ));
        }
        let mut presentation =
            GenerationProgress::new(py, progress.as_ref(), observer.as_ref(), self.prepared())?;
        let result = (|| {
            self.check(py)?;
            let mut remaining = max_units;
            if !self.prepared() {
                self.prepare_inner(py, &mut presentation)?;
                remaining -= 1;
                if !self.prepared() {
                    return Ok(self.snapshot());
                }
            }
            if remaining > 0 {
                self.compile_inner(py, remaining, &mut presentation)?;
            }
            Ok(self.snapshot())
        })();
        presentation.finish(py, result)
    }
    /// Synchronous native work on this caller's thread. Cancellation occurs at
    /// native boundaries; an in-progress CAS call is not preemptible here.
    #[pyo3(signature=(*,observer=None,progress=Some(Python::attach(|py| PyString::new(py,"auto").into_any().unbind()))))]
    pub(super) fn prepare(
        &mut self,
        py: Python<'_>,
        observer: Option<Py<PyAny>>,
        progress: Option<Py<PyAny>>,
    ) -> PyResult<PyGenerationSnapshot> {
        let mut presentation =
            GenerationProgress::new(py, progress.as_ref(), observer.as_ref(), false)?;
        let result = self
            .prepare_inner(py, &mut presentation)
            .map(|()| self.snapshot());
        presentation.finish(py, result)
    }
    /// Compile at most max_units local records. Each native compilation is one
    /// indivisible caller-owned unit; no thread/process pool is started here.
    #[pyo3(signature=(max_units=1,*,observer=None,progress=Some(Python::attach(|py| PyString::new(py,"auto").into_any().unbind()))))]
    fn compile_next(
        &mut self,
        py: Python<'_>,
        max_units: usize,
        observer: Option<Py<PyAny>>,
        progress: Option<Py<PyAny>>,
    ) -> PyResult<PyGenerationSnapshot> {
        if max_units == 0 {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "max_units must be positive",
            ));
        }
        let mut presentation =
            GenerationProgress::new(py, progress.as_ref(), observer.as_ref(), true)?;
        let result = self
            .compile_inner(py, max_units, &mut presentation)
            .map(|()| self.snapshot());
        presentation.finish(py, result)
    }
    pub(super) fn snapshot(&self) -> PyGenerationSnapshot {
        PyGenerationSnapshot {
            inner: self.status.clone(),
        }
    }
    #[getter]
    pub(super) fn compilation_settings(&self) -> crate::settings::PyCompilationSettings {
        crate::settings::PyCompilationSettings {
            inner: self.work.options.compilation,
        }
    }
    #[getter]
    pub(super) fn prepared(&self) -> bool {
        self.work.receipt.is_some()
    }
    #[getter]
    pub(super) fn complete(&self) -> bool {
        self.result.is_some()
    }
    #[getter]
    fn failed(&self) -> Option<String> {
        self.failed.clone()
    }
    #[getter]
    pub(super) fn result(&self, py: Python<'_>) -> Option<Py<PyRecipeArchive>> {
        self.result.as_ref().map(|r| r.clone_ref(py))
    }
    fn preparation_progress_json(&self) -> PyResult<Option<String>> {
        self.work
            .progress
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
    pub(super) fn preparation_receipt_json(&self) -> PyResult<Option<String>> {
        self.work
            .receipt
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }
    fn __repr__(&self) -> String {
        format!(
            "ThresholdGenerationSession(prepared={}, complete={}, compiled={}/{}, scope='admitted rational fixed fiber')",
            self.prepared(),
            self.complete(),
            self.work.next,
            self.work.jobs()
        )
    }
}
