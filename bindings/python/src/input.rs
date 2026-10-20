use std::{collections::BTreeMap, sync::Arc};

use fastsecdec::{
    Atom, EdgeId, Symbol,
    input::{GraphIntegral, RuntimeModelBindings},
    kernel::RuntimeMassConstraint,
    parametric::ParametricIntegrand,
};
use feynkit_py::{PyFeynmanDiagram, PyKinematics};
use pyo3::{prelude::*, types::PyDict};
use spynso3::expression::as_tensor;
use symbolica::{api::python::PythonExpression, atom::AtomView};

use super::error;

/// Copy a complete native diagram, replacing only the supplied expressions.
/// Native numerator validation and all graph/model ownership are preserved.
#[cfg_attr(
    feature = "python_stubgen",
    pyo3_stub_gen::derive::gen_stub_pyfunction(
        module = "symbolica.community.hepkit.sector_decomposition"
    )
)]
#[pyfunction]
#[pyo3(signature = (diagram, *, numerator=None, projector=None, overall_factor=None))]
pub(crate) fn with_diagram_expressions(
    py: Python<'_>,
    diagram: &PyFeynmanDiagram,
    numerator: Option<&Bound<'_, PythonExpression>>,
    projector: Option<&Bound<'_, PythonExpression>>,
    overall_factor: Option<&PythonExpression>,
) -> PyResult<PyFeynmanDiagram> {
    let mut copied = diagram.as_diagram()?.clone();
    if let Some(value) = numerator {
        let tensor = as_tensor(py, value.as_any())?;
        copied = copied
            .with_numerator(tensor.borrow(py).structured().clone())
            .map_err(|e| error::native(py, "input", e))?;
    }
    if let Some(value) = projector {
        let tensor = as_tensor(py, value.as_any())?;
        copied = copied.with_projector(tensor.borrow(py).structured().clone());
    }
    if let Some(value) = overall_factor {
        copied = copied.with_overall_factor(value.expr.clone());
    }
    Ok(copied.into())
}

/// Native diagram, assumptions and explicit normalized loop measure.
#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pyclass)]
#[pyclass(
    name = "Integral",
    module = "symbolica.community.hepkit.sector_decomposition",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyIntegral {
    pub(crate) graph: GraphIntegral,
    pub(crate) regulator: Symbol,
    pub(crate) dimension: Atom,
    pub(crate) runtime_parameters: Vec<Symbol>,
    pub(crate) runtime_model: Option<RuntimeModelBindings>,
}

#[derive(Clone, Default)]
pub(crate) struct RuntimeInputs {
    pub(crate) parameters: Vec<Symbol>,
    pub(crate) masses: Vec<RuntimeMassConstraint>,
    pub(crate) defaults: BTreeMap<String, f64>,
}

impl PyIntegral {
    pub(crate) fn parametrize(
        &self,
        py: Python<'_>,
    ) -> PyResult<(ParametricIntegrand, RuntimeInputs)> {
        let parameters = (0..self.graph.powers().len())
            .map(|i| symbolica::symbol!(format!("fastsecdec::hepkit::x{i}")))
            .collect();
        let input = ParametricIntegrand::from_graph(
            &self.graph,
            parameters,
            self.regulator,
            self.dimension.clone(),
        )
        .map_err(|e| error::native(py, "parametrization", e))?;
        let mut runtime = RuntimeInputs {
            parameters: self.runtime_parameters.clone(),
            ..Default::default()
        };
        if let Some(mut model) = self.runtime_model.clone() {
            model.retain_used(&input);
            runtime.parameters.extend(model.symbols());
            runtime.masses = model.mass_constraints().to_vec();
            runtime.defaults = model.defaults();
        }
        let mut seen = std::collections::BTreeSet::new();
        if runtime
            .parameters
            .iter()
            .any(|symbol| !seen.insert(*symbol))
        {
            return Err(error::native(py, "input", "runtime symbols must be unique"));
        }
        Ok((input, runtime))
    }
}

pub(crate) fn symbol(py: Python<'_>, value: &PythonExpression, name: &str) -> PyResult<Symbol> {
    match value.expr.as_view() {
        AtomView::Var(value) => Ok(value.get_symbol()),
        _ => Err(error::native(
            py,
            "input",
            format!("{name} must be a Symbolica symbol"),
        )),
    }
}

/// Transport Python expressions only; native input owners validate and apply
/// the complete scalar point consistently to their physics data.
pub(crate) fn scalar_bindings(
    py: Python<'_>,
    values: Option<&Bound<'_, PyDict>>,
) -> PyResult<BTreeMap<Symbol, Atom>> {
    let mut bindings = BTreeMap::new();
    if let Some(values) = values {
        for (key, value) in values.iter() {
            let key = key.extract::<PyRef<'_, PythonExpression>>()?;
            let value = value.extract::<PyRef<'_, PythonExpression>>()?;
            bindings.insert(symbol(py, &key, "scalar binding key")?, value.expr.clone());
        }
    }
    Ok(bindings)
}

#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pymethods)]
#[pymethods]
impl PyIntegral {
    /// Kinematics retains its symbolic tensor dimension; dimension defaults to 4-2*regulator.
    #[new]
    #[pyo3(signature = (diagram, kinematics, *, regulator, dimension=None, powers=None, scalar_values=None, auxiliary_momenta=None, measure_multiplier=None, runtime_parameters=None, model_parameters="runtime"))]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        py: Python<'_>,
        diagram: &PyFeynmanDiagram,
        kinematics: &PyKinematics,
        regulator: &PythonExpression,
        dimension: Option<&PythonExpression>,
        powers: Option<BTreeMap<usize, u32>>,
        scalar_values: Option<&Bound<'_, PyDict>>,
        auxiliary_momenta: Option<Vec<PythonExpression>>,
        measure_multiplier: Option<&PythonExpression>,
        runtime_parameters: Option<Vec<PythonExpression>>,
        model_parameters: &str,
    ) -> PyResult<Self> {
        let regulator = symbol(py, regulator, "regulator")?;
        let bindings = scalar_bindings(py, scalar_values)?;
        let runtime_parameters = runtime_parameters
            .unwrap_or_default()
            .iter()
            .map(|p| symbol(py, p, "runtime parameter"))
            .collect::<PyResult<Vec<_>>>()?;
        let diagram = Arc::new(diagram.as_diagram()?.clone());
        let runtime_model = match model_parameters {
            "runtime" => Some(
                RuntimeModelBindings::new(&diagram, None, &bindings)
                    .map_err(|e| error::native(py, "input", e))?,
            ),
            "fixed" => None,
            _ => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "model_parameters must be runtime or fixed",
                ));
            }
        };
        let mut declared = runtime_parameters.clone();
        if let Some(model) = &runtime_model {
            declared.extend(model.symbols());
        }
        let graph = GraphIntegral::new_with_runtime_scalar_values(
            diagram,
            kinematics.as_kinematics(),
            runtime_model
                .as_ref()
                .map_or(&bindings, RuntimeModelBindings::values),
            &declared,
        )
        .map_err(|e| error::native(py, "input", e))?;
        let powers = powers
            .unwrap_or_default()
            .into_iter()
            .map(|(id, power)| (EdgeId(id), power))
            .collect();
        let momenta: Vec<_> = auxiliary_momenta
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.expr)
            .collect();
        let graph = graph
            .with_auxiliary_external_momenta(&momenta)
            .and_then(|g| g.with_powers(&powers))
            .map_err(|e| error::native(py, "input", e))?
            .with_measure_multiplier(measure_multiplier.map_or_else(Atom::one, |v| v.expr.clone()));
        Ok(Self {
            graph,
            regulator,
            dimension: dimension.map_or_else(
                || Atom::num(4) - Atom::num(2) * Atom::var(regulator),
                |v| v.expr.clone(),
            ),
            runtime_parameters,
            runtime_model,
        })
    }

    #[getter]
    fn regulator(&self) -> PythonExpression {
        PythonExpression {
            expr: Atom::var(self.regulator),
        }
    }
    #[getter]
    fn dimension(&self) -> PythonExpression {
        PythonExpression {
            expr: self.dimension.clone(),
        }
    }
    #[getter]
    fn powers(&self) -> Vec<(usize, u32)> {
        self.graph
            .propagator_edges()
            .iter()
            .zip(self.graph.powers())
            .map(|(e, p)| (e.0, *p))
            .collect()
    }
}
