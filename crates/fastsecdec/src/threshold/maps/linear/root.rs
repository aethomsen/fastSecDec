//! Existing native linear-root lowering shared by prefix and complete maps.
use super::{AliasRole, CellMap, GcadError, LinearAxis, LinearRoot, Result};
use std::{collections::BTreeMap, ops::Range, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    id::{Pattern, Replacement},
};
use symgcad::{algebra::Algebra, output::RootBound};

pub(in crate::threshold::maps) fn lower_axes(
    source: &CellMap,
    range: Range<usize>,
) -> Result<Vec<LinearAxis>> {
    lower_geometry(
        &super::super::source::GeometryView::standard(&source.owner),
        source.cell_index,
        &source.axes,
        range,
    )
}

pub(in crate::threshold::maps) fn lower_geometry(
    source: &super::super::source::GeometryView<'_>,
    cell_index: usize,
    map_axes: &[super::super::MapAxis],
    range: Range<usize>,
) -> Result<Vec<LinearAxis>> {
    lower_bound_geometry(source, cell_index, map_axes, range, false)
}

/// Extract the same finite linear roots for a separately admitted open-interval
/// caller. Missing sides remain missing; this alone supplies no interval map.
pub(in crate::threshold::maps) fn lower_open_geometry(
    source: &super::super::source::GeometryView<'_>,
    cell_index: usize,
    map_axes: &[super::super::MapAxis],
    range: Range<usize>,
) -> Result<Vec<LinearAxis>> {
    lower_bound_geometry(source, cell_index, map_axes, range, true)
}

fn lower_bound_geometry(
    source: &super::super::source::GeometryView<'_>,
    cell_index: usize,
    map_axes: &[super::super::MapAxis],
    range: Range<usize>,
    open_intervals: bool,
) -> Result<Vec<LinearAxis>> {
    if range.start > range.end || range.end > map_axes.len() {
        return Err(GcadError::Invalid("linear lowering axis range".into()));
    }
    let raw = source.raw;
    let algebra = Algebra::new(&raw.order).map_err(|e| GcadError::Invalid(e.to_string()))?;
    let mut roots = BTreeMap::<(usize, usize), Arc<LinearRoot>>::new();
    let mut make_root = |axis: usize, bound: &RootBound| -> Result<Arc<LinearRoot>> {
        if bound.index != 0 {
            return Err(GcadError::Unsupported(
                "finite linear roots require native selector ordinal zero".into(),
            ));
        }
        let key = (axis, bound.polynomial);
        // A root can be referenced with different domains; keep that exact
        // choice attached, even though the algebraic expression is shared.
        if let Some(root) = roots.get(&key)
            && root.selector == bound.index_domain
        {
            return Ok(root.clone());
        }
        let text = raw.polynomials.get(bound.polynomial).ok_or_else(|| {
            GcadError::Invalid("root references a missing native polynomial".into())
        })?;
        let polynomial = algebra
            .parse(text)
            .map_err(|e| GcadError::Invalid(e.to_string()))?;
        if polynomial.degree(axis) != 1
            || (axis + 1..raw.order.len()).any(|i| polynomial.degree(i) > 0)
        {
            return Err(GcadError::Unsupported(format!(
                "cell {} axis {} needs a nonlinear or nontriangular root section",
                cell_index, axis
            )));
        }
        let coefficients = polynomial.to_univariate_polynomial_list(axis);
        let numerator = -coefficients
            .iter()
            .find(|(_, d)| *d == 0)
            .map_or_else(|| polynomial.zero(), |(p, _)| p.clone());
        let denominator = coefficients
            .iter()
            .find(|(_, d)| *d == 1)
            .ok_or_else(|| GcadError::Invalid("linear root lost its leading coefficient".into()))?
            .0
            .clone();
        let expression = numerator.to_expression() / denominator.to_expression();
        let expression =
            expression.replace_multiple(raw.order.iter().enumerate().map(|(i, name)| {
                let alias =
                    Symbol::parse(name, "symgcad").expect("native Algebra already validated alias");
                Replacement::new(
                    Pattern::Literal(Atom::var(alias)),
                    Pattern::Literal(Atom::var(map_axes[i].symbol)),
                )
            }));
        let root = Arc::new(LinearRoot {
            polynomial: bound.polynomial,
            selector: bound.index_domain,
            numerator,
            denominator,
            expression,
        });
        roots.insert(key, root.clone());
        Ok(root)
    };
    let mut axes = Vec::with_capacity(range.len());
    for (index, (axis, role)) in raw.cells[cell_index]
        .axes
        .iter()
        .zip(map_axes)
        .enumerate()
        .take(range.end)
        .skip(range.start)
    {
        if !open_intervals
            && role.role == AliasRole::IntegrationCoordinate
            && (axis.lower.is_none() || axis.upper.is_none())
        {
            return Err(GcadError::Unsupported(
                "unbounded integration cells need a separately proved compactification".into(),
            ));
        }
        axes.push(LinearAxis {
            lower: axis
                .lower
                .as_ref()
                .map(|b| make_root(index, b))
                .transpose()?,
            upper: axis
                .upper
                .as_ref()
                .map(|b| make_root(index, b))
                .transpose()?,
        });
    }
    Ok(axes)
}
