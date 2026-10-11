use super::*;
use crate::threshold::gcad::VerifiedDecomposition;
use std::collections::BTreeSet;
use symgcad::output::RootBound;

/// Exact input roles. Low-level maps use [parameters, units]; kernels use
/// [units, parameters]. Parameter values never contribute an integration width.
#[derive(Debug)]
pub struct ParameterInputLayout {
    parameters: Vec<Symbol>,
    units: Vec<Symbol>,
    raw_symbols: Vec<Symbol>,
    raw_to_runtime: Vec<Option<usize>>,
    raw_to_unit: Vec<Option<usize>>,
}
impl ParameterInputLayout {
    fn prepare(map: &CellMap) -> Result<Self> {
        let parameters = map.owner.request().kinematics().runtime_parameters.clone();
        let units = map.unit_coordinates.clone();
        let mut raw_to_runtime = Vec::with_capacity(map.axes.len());
        let mut raw_to_unit = Vec::with_capacity(map.axes.len());
        for axis in &map.axes {
            match axis.role {
                AliasRole::RuntimeParameter => {
                    let index = parameters
                        .iter()
                        .position(|s| *s == axis.symbol)
                        .ok_or_else(|| {
                            GcadError::Invalid("parameter prefix lost public runtime role".into())
                        })?;
                    if axis.unit_index.is_some() {
                        return Err(GcadError::Invalid(
                            "parameter carries an integration unit".into(),
                        ));
                    }
                    raw_to_runtime.push(Some(index));
                    raw_to_unit.push(None);
                }
                AliasRole::IntegrationCoordinate => {
                    let index = axis.unit_index.ok_or_else(|| {
                        GcadError::Invalid("integration axis lost its unit".into())
                    })?;
                    if index >= units.len() {
                        return Err(GcadError::Invalid(
                            "integration unit index outside layout".into(),
                        ));
                    }
                    raw_to_runtime.push(None);
                    raw_to_unit.push(Some(index));
                }
            }
        }
        if raw_to_runtime
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>()
            != (0..parameters.len()).collect::<BTreeSet<_>>()
            || raw_to_unit
                .iter()
                .flatten()
                .copied()
                .collect::<BTreeSet<_>>()
                != (0..units.len()).collect::<BTreeSet<_>>()
        {
            return Err(GcadError::Invalid(
                "incomplete parameter/integration input layout".into(),
            ));
        }
        Ok(Self {
            parameters,
            units,
            raw_symbols: map.axes.iter().map(|a| a.symbol).collect(),
            raw_to_runtime,
            raw_to_unit,
        })
    }
    pub fn parameters(&self) -> &[Symbol] {
        &self.parameters
    }
    pub fn units(&self) -> &[Symbol] {
        &self.units
    }
    pub fn raw_symbols(&self) -> &[Symbol] {
        &self.raw_symbols
    }
    pub fn raw_to_runtime(&self) -> &[Option<usize>] {
        &self.raw_to_runtime
    }
    pub fn raw_to_unit(&self) -> &[Option<usize>] {
        &self.raw_to_unit
    }
    pub fn map_inputs(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.parameters.iter().chain(&self.units).copied()
    }
    pub fn kernel_inputs(&self) -> impl Iterator<Item = Symbol> + '_ {
        self.units.iter().chain(&self.parameters).copied()
    }
    /// Each low-level map input's index in the kernel's input array.
    pub fn map_from_kernel(&self) -> impl Iterator<Item = usize> + '_ {
        (self.units.len()..self.units.len() + self.parameters.len()).chain(0..self.units.len())
    }
}

/// One common parameter prefix over the complete actual cell inventory.
/// This is chamber geometry only; uniform density continuation is issued by
/// NormalizedFamily and cannot be inferred from this owner alone.
#[derive(Debug)]
pub struct ParameterChamber {
    owner: Arc<VerifiedDecomposition>,
    prefixes: Vec<Arc<ParameterPrefix>>,
    layout: ParameterInputLayout,
}
impl ParameterChamber {
    pub(crate) fn prepare(mut maps: Vec<Arc<CellMap>>) -> Result<Arc<Self>> {
        let first = maps
            .first()
            .ok_or_else(|| GcadError::Invalid("empty parameter chamber inventory".into()))?;
        let owner = first.owner.clone();
        let layout = ParameterInputLayout::prepare(first)?;
        if layout.parameters.is_empty() {
            return Err(GcadError::Unsupported(
                "parameter chamber requires runtime parameters".into(),
            ));
        }
        if maps.len() != owner.cells().len()
            || maps.iter().map(|m| m.cell_index).collect::<BTreeSet<_>>()
                != (0..owner.cells().len()).collect::<BTreeSet<_>>()
            || maps
                .iter()
                .any(|m| !Arc::ptr_eq(&m.owner, &owner) || m.unit_coordinates != layout.units)
        {
            return Err(GcadError::Invalid(
                "parameter chamber needs every cell of one exact source".into(),
            ));
        }
        maps.sort_by_key(|m| m.cell_index);
        let p = layout.parameters.len();
        for map in &maps {
            if map.axes.len() != layout.raw_symbols.len()
                || map.axes.iter().zip(&maps[0].axes).any(|(a, z)| {
                    a.symbol != z.symbol || a.role != z.role || a.unit_index != z.unit_index
                })
                || map
                    .axes
                    .iter()
                    .take_while(|a| a.role == AliasRole::RuntimeParameter)
                    .count()
                    != p
            {
                return Err(GcadError::Invalid(
                    "parameter chamber changes source input roles".into(),
                ));
            }
            for (axis, first) in map.native_axes()[..p]
                .iter()
                .zip(&maps[0].native_axes()[..p])
            {
                if !same_bound(&axis.lower, &first.lower) || !same_bound(&axis.upper, &first.upper)
                {
                    return Err(GcadError::Unsupported(
                        "multiple parameter chambers require a separate complete chamber atlas"
                            .into(),
                    ));
                }
            }
        }
        // Same owner/prefix selectors above bind every guard expression to the
        // identical native polynomial table. Prepare the shared arithmetic once.
        let first = ParameterPrefix::prepare(maps[0].clone())?;
        let mut prefixes = vec![first.clone()];
        for map in maps.into_iter().skip(1) {
            let mut prefix = ParameterPrefix::geometry(map)?;
            prefix.guard = first.guard.clone();
            prefixes.push(Arc::new(prefix));
        }
        Ok(Arc::new(Self {
            owner,
            prefixes,
            layout,
        }))
    }
    pub(crate) fn guard(&self) -> &Arc<ExactParameterGuard> {
        self.prefixes[0]
            .guard()
            .expect("nonempty parameter chamber")
    }
    pub fn admit(&self, values: &BTreeMap<Symbol, Rational>) -> Result<AdmittedParameterFiber> {
        self.prefixes[0].admit(values)
    }
    pub fn decomposition(&self) -> &Arc<VerifiedDecomposition> {
        &self.owner
    }
    pub fn prefixes(&self) -> &[Arc<ParameterPrefix>] {
        &self.prefixes
    }
    pub fn layout(&self) -> &ParameterInputLayout {
        &self.layout
    }
}

// Sample isolating intervals are not part of section identity. Both indices
// refer to the same actual native polynomial table of the shared owner.
fn same_bound(a: &Option<RootBound>, b: &Option<RootBound>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => {
            a.polynomial == b.polynomial && a.index == b.index && a.index_domain == b.index_domain
        }
        _ => false,
    }
}
