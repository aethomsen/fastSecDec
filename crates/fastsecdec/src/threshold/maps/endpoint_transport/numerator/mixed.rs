//! Mixed jets of an issued finite clearing-normal coefficient. The original
//! normal ledger is unchanged; no density, unit or analytic-strip authority.
pub mod program;
#[cfg(test)]
mod test_fixture;
#[cfg(test)]
mod tests;
use super::normal::{FiniteArgument, SelectedExtension, admission, definitions};
use super::{Error, Limits, NumeratorFace, Result, Work, native};
use crate::{
    kernel::algebraic::Scope,
    threshold::{
        continued::Definition,
        maps::{endpoint_transport::cover::IssuedEndpointChart, regular::Budget},
    },
};
use std::{collections::BTreeSet, ops::ControlFlow, sync::Arc};
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    evaluate::OptimizationSettings,
    poly::series::SeriesDepth,
};

/// Absolute lower-normal Taylor coefficients of one issued G_j. Here
/// G=c^(k-v)N(phi). Entry m in row j is [u^m c^j]G, so the
/// corresponding mixed derivative at the joint face is m! j! times that entry.
/// This is not a derivative of unnormalized N when k-v is nonzero.
pub struct MixedCoefficient {
    source_record: usize,
    clearing_order: usize,
    values: Vec<Atom>,
}
impl MixedCoefficient {
    pub fn source_record(&self) -> usize {
        self.source_record
    }
    pub fn clearing_order(&self) -> usize {
        self.clearing_order
    }
    pub fn coefficients(&self) -> &[Atom] {
        &self.values
    }
}
pub struct MixedNumeratorFace {
    face: Arc<NumeratorFace>,
    chart: IssuedEndpointChart,
    lower_normal: Symbol,
    lower_depth: u32,
    directions: Vec<Symbol>,
    formals: Vec<Symbol>,
    arguments: Vec<FiniteArgument>,
    sections: Vec<SelectedExtension>,
    coefficients: Vec<MixedCoefficient>,
    definitions: Vec<Definition>,
    head: Symbol,
    scope: Scope,
}
impl MixedNumeratorFace {
    pub fn face(&self) -> &Arc<NumeratorFace> {
        &self.face
    }
    pub fn chart(&self) -> &IssuedEndpointChart {
        &self.chart
    }
    pub fn lower_normal(&self) -> Symbol {
        self.lower_normal
    }
    pub fn lower_depth(&self) -> u32 {
        self.lower_depth
    }
    pub fn direction_units(&self) -> &[Symbol] {
        &self.directions
    }
    pub fn coefficients(&self) -> &[MixedCoefficient] {
        &self.coefficients
    }
    /// Demanded formal arguments followed by inverse direction-divisor
    /// arguments. All are admitted at the same relative joint face.
    pub fn finite_arguments(&self) -> &[FiniteArgument] {
        &self.arguments
    }
    pub fn selected_extensions(&self) -> &[SelectedExtension] {
        &self.sections
    }
    pub fn closed_density_certified(&self) -> bool {
        false
    }
    pub fn prepare(
        face: Arc<NumeratorFace>,
        lower_normal: Symbol,
        lower_depth: u32,
        settings: OptimizationSettings,
        limits: &Limits,
        mut progress: impl FnMut() -> ControlFlow<()>,
    ) -> Result<Arc<Self>> {
        let mut work = Work {
            limits,
            budget: Budget::new(limits.algebra.clone()),
            callback: &mut progress,
        };
        work.check()?;
        let chart = face.transport().issued_chart(face.chart_index())?;
        if face.normal() == lower_normal || !chart.target_units().contains(&lower_normal) {
            return Err(Error::Invalid(
                "mixed face requires distinct owned normals".into(),
            ));
        }
        if lower_depth == 0 || lower_depth > limits.max_relative_depth {
            return Err(Error::ResourceIncomplete("mixed lower-normal depth".into()));
        }
        let entries = face
            .coefficients()
            .len()
            .checked_mul(face.depth() as usize)
            .and_then(|n| n.checked_mul(lower_depth as usize))
            .ok_or_else(|| Error::ResourceIncomplete("mixed coefficient count overflow".into()))?;
        work.entries(entries)?;
        let mut formals = face.finite_coordinates().to_vec();
        let mut images = face.finite_images().to_vec();
        formals.extend_from_slice(face.direction_units());
        images.extend(face.direction_units().iter().copied().map(Atom::var));
        formals.extend_from_slice(face.root_symbols());
        images.extend(face.root_symbols().iter().copied().map(Atom::var));
        if formals.iter().copied().collect::<BTreeSet<_>>().len() != formals.len() {
            return Err(Error::Invalid(
                "mixed independent formal roles collide".into(),
            ));
        }
        let nformals = formals.len();
        // The original face grants only normal independence, not a unit at the
        // new lower boundary. Admit every original direction divisor there.
        let mut occupied = face
            .catalogue()
            .records()
            .iter()
            .flat_map(|r| r.body().get_all_symbols(true))
            .collect::<BTreeSet<_>>();
        occupied.extend(formals.iter().copied());
        occupied.extend(chart.target_units().iter().copied());
        for (i, row) in face.coefficients().iter().enumerate() {
            let label = (0usize..)
                .map(|n| symbolica::symbol!(format!("fastsecdec::mixed_divisor_{n}::d{i}")))
                .find(|s| !occupied.contains(s))
                .expect("finite occupied role inventory");
            occupied.insert(label);
            formals.push(label);
            images.push(Atom::one() / row.direction_divisor());
        }
        if images
            .iter()
            .any(|a| a.contains(Atom::var(face.normal()).as_view()))
        {
            return Err(Error::Invalid(
                "mixed finite coefficient argument retains clearing normal".into(),
            ));
        }
        let admitted = admission::prepare_arguments(
            face.catalogue(),
            &chart,
            lower_normal,
            &formals,
            &images,
            settings,
            &mut work,
        )?;
        formals.truncate(nformals);
        let allowed = formals
            .iter()
            .copied()
            .chain(
                face.catalogue()
                    .records()
                    .iter()
                    .flat_map(|r| r.body().get_all_symbols(true)),
            )
            .collect::<BTreeSet<_>>();
        let head = definitions::fresh_head(face.catalogue(), &chart);
        let mut coefficients = Vec::new();
        let mut bodies = Vec::new();
        for row in face.coefficients() {
            for (j, body) in row.coefficients().iter().enumerate() {
                if body.contains(Atom::var(face.normal()).as_view())
                    || !body
                        .get_all_symbols(true)
                        .iter()
                        .all(|s| allowed.contains(s))
                {
                    return Err(Error::Invalid("foreign finite coefficient role".into()));
                }
                let index = bodies.len();
                bodies.push(Some(body));
                let mut args = vec![Atom::num(index)];
                args.extend(admitted.images[..nformals].iter().cloned());
                let call = symbolica::atom::FunctionBuilder::new(head)
                    .add_args(&args)
                    .finish();
                work.nodes(&call, limits.max_coefficient_nodes)?;
                let series = call
                    .series(
                        lower_normal,
                        0,
                        SeriesDepth::absolute(lower_depth as i64 - 1),
                    )
                    .map_err(native)?;
                let values = (0..lower_depth)
                    .map(|k| series.coefficient(k.into()).unwrap_or_else(Atom::zero))
                    .collect::<Vec<_>>();
                for a in &values {
                    work.nodes(a, limits.max_coefficient_nodes)?;
                }
                coefficients.push(MixedCoefficient {
                    source_record: row.source_record(),
                    clearing_order: j,
                    values,
                });
            }
        }
        let definitions = definitions::collect_bodies(
            &formals,
            &bodies,
            head,
            coefficients.iter().flat_map(|c| &c.values),
            lower_depth,
            &mut work,
        )?;
        let directions = chart
            .target_units()
            .iter()
            .copied()
            .filter(|s| *s != lower_normal && *s != face.normal())
            .collect();
        work.check()?;
        Ok(Arc::new(Self {
            face,
            chart,
            lower_normal,
            lower_depth,
            directions,
            formals,
            arguments: admitted.arguments,
            sections: admitted.sections,
            coefficients,
            definitions,
            head,
            scope: admitted.scope,
        }))
    }
}
