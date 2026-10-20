use std::{collections::BTreeMap, sync::Arc};

use fastsecdec::{
    Atom, EdgeId, FeynmanDiagram, IntegralFamily, Kinematics, Symbol,
    input::{GraphIntegral, RuntimeModelBindings, prepare_family_input},
    kernel::RuntimeMassConstraint,
    parametric::ParametricIntegrand,
};
use feynkit_py::{PyFeynmanDiagram, PyIntegralFamily, PyKinematics};
use idenso::tensor::SymbolicTensor;
use pyo3::{prelude::*, types::PyDict};
use symbolica::{
    api::python::PythonExpression,
    atom::{AtomCore, AtomView},
};

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
    numerator: Option<&PythonExpression>,
    projector: Option<&PythonExpression>,
    overall_factor: Option<&PythonExpression>,
) -> PyResult<PyFeynmanDiagram> {
    let mut copied = diagram.as_diagram()?.clone();
    if let Some(value) = numerator {
        let numerator =
            SymbolicTensor::infer(value.expr.clone()).map_err(|e| error::native(py, "input", e))?;
        copied = copied
            .with_numerator(numerator)
            .map_err(|e| error::native(py, "input", e))?;
    }
    if let Some(value) = projector {
        let projector =
            SymbolicTensor::infer(value.expr.clone()).map_err(|e| error::native(py, "input", e))?;
        copied = copied.with_projector(projector);
    }
    if let Some(value) = overall_factor {
        copied = copied.with_overall_factor(value.expr.clone());
    }
    Ok(copied.into())
}

#[derive(Clone)]
enum IntegralInput {
    Graph(Arc<GraphInput>),
    Family(Arc<FamilyInput>),
}

/// Original native graph point. Storing it performs no propagator-family
/// arithmetic; exact and represented generation consume the same original roles.
struct GraphInput {
    diagram: Arc<FeynmanDiagram>,
    kinematics: Arc<Kinematics>,
    scalar_values: BTreeMap<Symbol, Atom>,
    auxiliary_momenta: Vec<Atom>,
    powers: BTreeMap<EdgeId, u32>,
    propagator_edges: Vec<EdgeId>,
    measure_multiplier: Atom,
}

/// Retained native data only. Specialization, projection and numerator algebra
/// are deferred until the caller explicitly starts generation.
struct FamilyInput {
    family: Arc<IntegralFamily>,
    powers: Vec<i32>,
    numerator: Atom,
    measure_multiplier: Atom,
    kinematics: Option<Kinematics>,
    scalar_values: BTreeMap<Symbol, Atom>,
    auxiliary_momenta: Vec<Atom>,
}

/// Native diagram or integral family, assumptions and explicit loop measure.
///
/// Construction retains the original native input without propagator-family
/// arithmetic. Native basis, width and scalar-point validation occurs on explicit
/// generation/parametrization; type, symbol-role and graph-view checks remain
/// immediate. Display and snapshots never parameterize or compile the input.
#[cfg_attr(feature = "python_stubgen", pyo3_stub_gen::derive::gen_stub_pyclass)]
#[pyclass(
    name = "Integral",
    module = "symbolica.community.hepkit.sector_decomposition",
    frozen,
    skip_from_py_object
)]
#[derive(Clone)]
pub(crate) struct PyIntegral {
    input: IntegralInput,
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

#[cfg(feature = "native")]
pub(crate) enum RepresentedPoint {
    Graph(Arc<fastsecdec::threshold::represented::graph::GraphPoint>),
    Family(Arc<fastsecdec::threshold::represented::family::FamilyPoint>),
}

impl PyIntegral {
    #[cfg(feature = "native")]
    pub(crate) fn represented_point(&self) -> PyResult<RepresentedPoint> {
        if !self.runtime_parameters.is_empty() || self.runtime_model.is_some() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "represented threshold preparation requires model_parameters='fixed' and no runtime_parameters",
            ));
        }
        match &self.input {
            IntegralInput::Graph(source) => Ok(RepresentedPoint::Graph(Arc::new(
                fastsecdec::threshold::represented::graph::GraphPoint {
                    diagram: source.diagram.clone(),
                    kinematics: source.kinematics.clone(),
                    scalar_values: source.scalar_values.clone(),
                    auxiliary_momenta: source.auxiliary_momenta.clone(),
                    powers: source.powers.clone(),
                    measure_multiplier: source.measure_multiplier.clone(),
                    coordinates: (0..source.propagator_edges.len())
                        .map(|i| symbolica::symbol!(format!("fastsecdec::hepkit::x{i}")))
                        .collect(),
                    regulator: self.regulator,
                    dimension: self.dimension.clone(),
                },
            ))),
            IntegralInput::Family(source) => Ok(RepresentedPoint::Family(Arc::new(
                fastsecdec::threshold::represented::family::FamilyPoint {
                    family: source.family.clone(),
                    powers: source.powers.clone(),
                    numerator: source.numerator.clone(),
                    measure_multiplier: source.measure_multiplier.clone(),
                    kinematics: source.kinematics.as_ref().map(|k| Arc::new(k.clone())),
                    scalar_values: source.scalar_values.clone(),
                    auxiliary_momenta: source.auxiliary_momenta.clone(),
                    coordinates: (0..source.powers.iter().filter(|p| **p > 0).count())
                        .map(|i| symbolica::symbol!(format!("fastsecdec::hepkit::x{i}")))
                        .collect(),
                    regulator: self.regulator,
                    dimension: self.dimension.clone(),
                },
            ))),
        }
    }

    pub(crate) fn parametrize(
        &self,
        py: Python<'_>,
    ) -> PyResult<(ParametricIntegrand, RuntimeInputs)> {
        let parameters = |count| {
            (0..count)
                .map(|i| symbolica::symbol!(format!("fastsecdec::hepkit::x{i}")))
                .collect()
        };
        let input = match &self.input {
            IntegralInput::Graph(source) => {
                let mut declared = self.runtime_parameters.clone();
                if let Some(model) = &self.runtime_model {
                    declared.extend(model.symbols());
                }
                let graph = GraphIntegral::new_with_runtime_scalar_values(
                    source.diagram.clone(),
                    &source.kinematics,
                    self.runtime_model
                        .as_ref()
                        .map_or(&source.scalar_values, RuntimeModelBindings::values),
                    &declared,
                )
                .and_then(|g| g.with_auxiliary_external_momenta(&source.auxiliary_momenta))
                .and_then(|g| g.with_powers(&source.powers))
                .map_err(|e| error::native(py, "input", e))?
                .with_measure_multiplier(source.measure_multiplier.clone());
                ParametricIntegrand::from_graph(
                    &graph,
                    parameters(graph.powers().len()),
                    self.regulator,
                    self.dimension.clone(),
                )
            }
            IntegralInput::Family(source) => {
                let (family, powers, numerator) = prepare_family_input(
                    &source.family,
                    &source.powers,
                    &source.numerator * &source.measure_multiplier,
                    source.kinematics.as_ref(),
                    &source.scalar_values,
                    &source.auxiliary_momenta,
                )
                .map_err(|e| error::native(py, "input", e))?;
                ParametricIntegrand::from_family(
                    &family,
                    &powers,
                    numerator,
                    parameters(powers.len()),
                    self.regulator,
                    self.dimension.clone(),
                )
            }
        }
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
        // This role check needs no family construction and remains a constructor
        // error. Full scalar-point/model/basis admission occurs at parametrization.
        let tensor_dimension = kinematics.as_kinematics().dimension().to_symbolic();
        if bindings
            .keys()
            .any(|key| tensor_dimension.contains(Atom::var(*key).as_view()))
        {
            return Err(error::native(
                py,
                "input",
                "scalar bindings cannot replace the tensor dimension",
            ));
        }
        let propagator_edges = GraphIntegral::propagator_edge_ids(&diagram);
        let powers = powers
            .unwrap_or_default()
            .into_iter()
            .map(|(id, power)| (EdgeId(id), power))
            .collect::<BTreeMap<_, _>>();
        let momenta: Vec<_> = auxiliary_momenta
            .unwrap_or_default()
            .into_iter()
            .map(|v| v.expr)
            .collect();
        for (&edge, &power) in &powers {
            if !propagator_edges.contains(&edge) {
                return Err(error::native(
                    py,
                    "input",
                    fastsecdec::Error::UnknownPropagator(edge),
                ));
            }
            if power == 0 {
                return Err(error::native(
                    py,
                    "input",
                    fastsecdec::Error::InvalidPower { edge, power },
                ));
            }
        }
        let graph = GraphInput {
            diagram,
            kinematics: Arc::new(kinematics.as_kinematics().clone()),
            scalar_values: bindings,
            auxiliary_momenta: momenta,
            powers,
            propagator_edges,
            measure_multiplier: measure_multiplier.map_or_else(Atom::one, |v| v.expr.clone()),
        };
        Ok(Self {
            input: IntegralInput::Graph(Arc::new(graph)),
            regulator,
            dimension: dimension.map_or_else(
                || Atom::num(4) - Atom::num(2) * Atom::var(regulator),
                |v| v.expr.clone(),
            ),
            runtime_parameters,
            runtime_model,
        })
    }

    /// Retain an existing native IntegralFamily with explicitly weighted physics.
    ///
    /// Powers follow its zero-based denominator order. Positive powers select
    /// propagators, zero powers omit them, and negative powers multiply the
    /// numerator. No graph/projector weight is inferred. Native specialization,
    /// projection and parametrization occur only when generation is requested.
    #[staticmethod]
    #[pyo3(signature=(family, *, regulator, powers, numerator, kinematics=None, dimension=None, scalar_values=None, auxiliary_momenta=None, measure_multiplier=None, runtime_parameters=None))]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_family(
        py: Python<'_>,
        family: &PyIntegralFamily,
        regulator: &PythonExpression,
        powers: Vec<i32>,
        numerator: &PythonExpression,
        kinematics: Option<&PyKinematics>,
        dimension: Option<&PythonExpression>,
        scalar_values: Option<&Bound<'_, PyDict>>,
        auxiliary_momenta: Option<Vec<PythonExpression>>,
        measure_multiplier: Option<&PythonExpression>,
        runtime_parameters: Option<Vec<PythonExpression>>,
    ) -> PyResult<Self> {
        let regulator = symbol(py, regulator, "regulator")?;
        Ok(Self {
            input: IntegralInput::Family(Arc::new(FamilyInput {
                family: Arc::new(family.as_family().clone()),
                powers,
                numerator: numerator.expr.clone(),
                measure_multiplier: measure_multiplier.map_or_else(Atom::one, |v| v.expr.clone()),
                kinematics: kinematics.map(|v| v.as_kinematics().clone()),
                scalar_values: scalar_bindings(py, scalar_values)?,
                auxiliary_momenta: auxiliary_momenta
                    .unwrap_or_default()
                    .into_iter()
                    .map(|v| v.expr)
                    .collect(),
            })),
            regulator,
            dimension: dimension.map_or_else(
                || Atom::num(4) - Atom::num(2) * Atom::var(regulator),
                |v| v.expr.clone(),
            ),
            runtime_parameters: runtime_parameters
                .unwrap_or_default()
                .iter()
                .map(|p| symbol(py, p, "runtime parameter"))
                .collect::<PyResult<_>>()?,
            runtime_model: None,
        })
    }

    #[getter]
    fn input_kind(&self) -> &'static str {
        match &self.input {
            IntegralInput::Graph(_) => "graph",
            IntegralInput::Family(_) => "family",
        }
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
    /// Graph powers use stable edge IDs; family powers use zero-based native
    /// denominator indices and retain signed powers, including omitted slots.
    fn powers(&self) -> Vec<(usize, i64)> {
        match &self.input {
            IntegralInput::Graph(graph) => graph
                .propagator_edges
                .iter()
                .map(|e| (e.0, i64::from(*graph.powers.get(e).unwrap_or(&1))))
                .collect(),
            IntegralInput::Family(source) => source
                .powers
                .iter()
                .enumerate()
                .map(|(i, p)| (i, i64::from(*p)))
                .collect(),
        }
    }
}
