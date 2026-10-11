//! Shared native graph-coordinate adaptation. A successful local chart is not
//! by itself a cover; callers retain and verify their complete open inventory.
use super::super::{contact::clear_units, *};
use super::helpers::*;
use std::{collections::BTreeSet, sync::Arc};

#[derive(Clone, Debug)]
pub(super) struct GraphCoordinateOpen {
    pub source: Arc<EtaleFrame>,
    pub functions: Vec<Poly>,
    pub columns: Vec<usize>,
    pub extension: Extension,
    pub graph: Arc<EtaleFrame>,
    pub coordinate_axes: Vec<usize>,
    pub relative_minor: Poly,
    pub source_open: Poly,
    pub forward_coordinate_jacobian: Poly,
}
#[derive(Clone, Debug)]
pub(super) struct EmptyGraphOpen {
    pub ideal: Ideal,
    pub guards: Vec<Guard>,
}
pub(super) fn graph_coordinates(
    old: Arc<EtaleFrame>,
    functions: &[Poly],
    columns: &[usize],
    extra_units: &[Poly],
    namespace: &str,
    b: &mut Budget,
) -> Result<std::result::Result<Arc<GraphCoordinateOpen>, EmptyGraphOpen>> {
    let k = functions.len();
    if columns.len() != k
        || columns.iter().copied().collect::<BTreeSet<_>>().len() != k
        || columns.iter().any(|c| *c >= old.free_axes().len())
    {
        return Err(Error::Invalid("graph coordinate minor shape"));
    }
    b.reserve_slots(k)?;
    b.reserve_slots(extra_units.len())?;
    for f in functions.iter().chain(extra_units) {
        old.local().supports(f)?;
        b.poly(f)?;
    }
    let slots = k
        .checked_add(extra_units.len())
        .and_then(|n| n.checked_add(1))
        .ok_or(Error::ResourceIncomplete("graph slots"))?;
    let extension = Extension::new(old.local().ring().clone(), slots, namespace, b)?;
    let ring = extension.target();
    let start = extension.source().len();
    let coordinate_axes = (start..start + k).collect::<Vec<_>>();
    let entries = functions
        .iter()
        .flat_map(|f| columns.iter().map(move |j| (f, *j)))
        .map(|(f, j)| old.derivative(j, f, b))
        .collect::<Result<Vec<_>>>()?;
    let relative_minor = determinant(old.local().ring(), entries, k, b)?;
    let mut source_open = relative_minor.clone();
    for f in extra_units {
        source_open = b.mul(&source_open, f)?;
    }
    let mut forward_entries = Vec::new();
    for j in 0..old.free_axes().len() {
        if !columns.contains(&j) {
            for l in 0..old.free_axes().len() {
                forward_entries.push(if l == j {
                    old.local().ring().one()
                } else {
                    old.local().ring().one().zero()
                });
            }
        }
    }
    for f in functions {
        for j in 0..old.free_axes().len() {
            forward_entries.push(old.derivative(j, f, b)?);
        }
    }
    let forward_coordinate_jacobian = determinant(
        old.local().ring(),
        forward_entries,
        old.free_axes().len(),
        b,
    )?;
    let clearing = functions
        .iter()
        .map(|f| clear_units(old.local(), f, b))
        .collect::<Result<Vec<_>>>()?;
    let mut selected = old
        .selected_equations()
        .iter()
        .map(|i| extension.pull(&old.source().ideal().generators()[*i], b))
        .collect::<Result<Vec<_>>>()?;
    let mut equations = Vec::new();
    let mut denominator = ring.one();
    for (i, c) in clearing.iter().enumerate() {
        let u = extension.pull(&c.denominator, b)?;
        denominator = b.mul(&denominator, &u)?;
        equations.push(
            &extension.pull(&c.numerator, b)?
                - &b.mul(&ring.coordinate(coordinate_axes[i])?, &u)?,
        );
    }
    selected.extend(equations.iter().cloned());
    let relations = extension
        .ideal(old.local().ideal(), b)?
        .sum(&Ideal::new(ring.clone(), equations, b)?, b)?;
    let mut guards = extension.guards(old.local().guards(), b)?;
    let mut slot = start + k;
    for f in extra_units {
        let cleared = clear_units(old.local(), f, b)?;
        guards.push(Guard {
            factor: extension.pull(&cleared.numerator, b)?,
            inverse_axis: slot,
        });
        slot += 1;
    }
    let old_minor = extension.pull(old.determinant(), b)?;
    let pulled_minor = extension.pull(&relative_minor, b)?;
    let expected = b.mul(&old_minor, &denominator)?;
    let expected = b.mul(&expected, &pulled_minor)?;
    let minor_clearing = clear_units(old.local(), &relative_minor, b)?;
    let mut probe_guards = guards.clone();
    probe_guards.push(Guard {
        factor: extension.pull(&minor_clearing.numerator, b)?,
        inverse_axis: slot,
    });
    if empty(&relations, &probe_guards, b)? {
        return Ok(Err(EmptyGraphOpen {
            ideal: relations,
            guards: probe_guards,
        }));
    }
    let mut axes = old.local().axes().to_vec();
    axes.extend(&coordinate_axes);
    let local = LocalizedAlgebra::new(relations.clone(), axes, guards, b)?;
    let mut dependent = old.dependent_axes().to_vec();
    dependent.extend(columns.iter().map(|j| old.free_axes()[*j]));
    let mut free = old
        .free_axes()
        .iter()
        .enumerate()
        .filter(|(j, _)| !columns.contains(j))
        .map(|(_, a)| *a)
        .collect::<Vec<_>>();
    free.extend(&coordinate_axes);
    let graph = Arc::new(
        EtaleCertificate {
            source: local,
            equations: indices(&relations, &selected)?,
            dependent_axes: dependent,
            free_axes: free,
            determinant_inverse_axis: if selected.is_empty() {
                None
            } else {
                Some(slot)
            },
        }
        .verify(b)?,
    );
    let mut membership =
        super::super::prepared_membership::PreparedLocalZero::new(graph.local(), b)?;
    if !membership.zero(&(graph.determinant() - &expected), b)? {
        return Err(Error::Invalid("adapted graph Schur minor"));
    }
    if !unit(
        graph.local(),
        &extension.pull(&forward_coordinate_jacobian, b)?,
        b,
    )? {
        return Err(Error::Invalid("adapted coordinate Jacobian not unit"));
    }
    for (i, c) in clearing.iter().enumerate() {
        let h = extension.pull(&c.original, b)?;
        if !membership.zero(&(&h - &ring.coordinate(coordinate_axes[i])?), b)? {
            return Err(Error::Invalid("adapted coordinate identity"));
        }
        for (j, axis) in graph.free_axes().iter().enumerate() {
            let want = if *axis == coordinate_axes[i] {
                ring.one()
            } else {
                ring.one().zero()
            };
            if !membership.zero(&(graph.derivative(j, &h, b)? - want), b)? {
                return Err(Error::Invalid("adapted derivative identity"));
            }
        }
    }
    for f in extra_units {
        if !unit(graph.local(), &extension.pull(f, b)?, b)? {
            return Err(Error::Invalid("graph complement factor not unit"));
        }
    }
    Ok(Ok(Arc::new(GraphCoordinateOpen {
        source: old,
        functions: functions.to_vec(),
        columns: columns.to_vec(),
        extension,
        graph,
        coordinate_axes,
        relative_minor,
        source_open,
        forward_coordinate_jacobian,
    })))
}
