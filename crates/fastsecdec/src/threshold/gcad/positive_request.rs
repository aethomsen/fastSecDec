//! Original homogeneous solve requests bound to the complete native density.
//!
//! This is a source/preparation boundary, not an endpoint resolver. In particular
//! a successfully solved quotient does not authorize publishing an integral.
use super::{GcadError, PositiveProjectiveQuotient, Result, SourceProvenance, anchor::source_poly};
use crate::{
    parametric::{FactorRole, FactorSemantics, ParametricIntegrand},
    threshold::{
        projective::PositiveProjectivePreparation, represented::graph::ExactRepresentedGraphInput,
    },
};
use std::{collections::BTreeSet, sync::Arc};
use symgcad::{Limits, Problem, SolverOptions, algebra::Algebra, output::SolveResult};

/// A complete original homogeneous problem. The source owner, anchor and exact
/// native problem remain associated through solving, staging and fresh replay.
#[derive(Clone, Debug)]
pub struct PositiveProjectiveRequest {
    preparation: Arc<PositiveProjectivePreparation>,
    provenance: SourceProvenance,
    problem: Problem,
    problem_bytes: Vec<u8>,
}

impl PositiveProjectiveRequest {
    pub fn exact(
        input: Arc<ParametricIntegrand>,
        anchor_index: usize,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        let preparation = Arc::new(
            PositiveProjectivePreparation::anchor(input, anchor_index)
                .map_err(|e| GcadError::Invalid(e.to_string()))?,
        );
        Self::build(preparation, SourceProvenance::Exact, solver, limits)
    }

    /// Retain the actual pre-parametric HEPKit input, including the numerical
    /// interpretation and complete numerator. Geometry remains fixed over QQ.
    pub fn graph(
        source: Arc<ExactRepresentedGraphInput>,
        anchor_index: usize,
        solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        let preparation = Arc::new(
            PositiveProjectivePreparation::anchor(source.exact().clone(), anchor_index)
                .map_err(|e| GcadError::Invalid(e.to_string()))?,
        );
        Self::build(
            preparation,
            SourceProvenance::PreparametricGraph(source),
            solver,
            limits,
        )
    }

    fn build(
        preparation: Arc<PositiveProjectivePreparation>,
        provenance: SourceProvenance,
        mut solver: SolverOptions,
        limits: Limits,
    ) -> Result<Self> {
        if preparation.coordinates().is_empty() {
            return Err(GcadError::Unsupported(
                "zero-dimensional projective input needs no GCAD solve".into(),
            ));
        }
        if limits.workers != 1 {
            return Err(GcadError::Unsupported(
                "positive-projective solving is synchronous; native workers must be one".into(),
            ));
        }
        if solver.ordering != "given"
            || solver.search_time_secs.is_some()
            || solver.max_rollouts != SolverOptions::default().max_rollouts
            || solver.monotone_negative_target
        {
            return Err(GcadError::Unsupported(
                "positive-projective requests require a given complete order and full sign decomposition; ordering searches remain caller-owned".into(),
            ));
        }
        let names = (0..preparation.input().parameters().len())
            .map(|i| format!("x{i}"))
            .collect::<Vec<_>>();
        let anchor = &names[preparation.anchor_index()];
        if solver
            .homogeneous_anchor
            .as_ref()
            .is_some_and(|a| a != anchor)
            || (!solver.homogeneous_block.is_empty()
                && (solver.homogeneous_block.len() != names.len()
                    || solver.homogeneous_block.iter().collect::<BTreeSet<_>>()
                        != names.iter().collect::<BTreeSet<_>>()))
        {
            return Err(GcadError::Invalid(
                "solver anchor/block differ from the original projective source".into(),
            ));
        }
        if solver.order.is_empty() {
            solver.order = std::iter::once(anchor.clone())
                .chain(names.iter().filter(|n| *n != anchor).cloned())
                .collect();
        }
        if solver.order.first() != Some(anchor)
            || solver.order.len() != names.len()
            || solver.order.iter().collect::<BTreeSet<_>>() != names.iter().collect::<BTreeSet<_>>()
        {
            return Err(GcadError::Invalid(
                "native lifting order must include each original coordinate once, with its anchor first".into(),
            ));
        }
        solver.homogeneous_anchor = Some(anchor.clone());
        solver.homogeneous_block = names.clone();
        let algebra = Algebra::new(&names).map_err(native)?;
        let aliases = preparation
            .input()
            .parameters()
            .iter()
            .copied()
            .zip(names.iter().cloned())
            .collect::<Vec<_>>();
        let mut splits = Vec::new();
        for term in preparation.input().terms() {
            for factor in term.factors() {
                if factor.exponent().is_zero() || factor.role() == FactorRole::Polynomial {
                    continue;
                }
                // Reuse the existing exact owner conversion: foreign aliases,
                // functions and free coefficients cannot become geometry axes.
                let polynomial = source_poly(factor.polynomial(), &aliases, &algebra)?;
                if polynomial.is_zero() {
                    return Err(GcadError::Invalid("zero active source factor".into()));
                }
                match factor.semantics() {
                    FactorSemantics::Causal => {
                        let content = polynomial.content().abs();
                        let primitive = polynomial.div_coeff(&content);
                        if !splits.contains(&primitive) {
                            splits.push(primitive);
                        }
                    }
                    // Do not add U>0 as a constraint: verification proves the
                    // original factor positive on the complete positive orthant.
                    FactorSemantics::Positive => {}
                    FactorSemantics::Generic => {
                        return Err(GcadError::Unsupported(
                            "generic singular factor lacks a positive or causal branch".into(),
                        ));
                    }
                }
            }
        }
        let problem = Problem {
            variables: names.clone(),
            constraints: names.iter().map(|n| format!("{n}>0")).collect(),
            split: splits.iter().map(|p| algebra.format(p)).collect(),
            variable_groups: vec![names],
            solver,
            limits,
            ..Problem::default()
        };
        problem.validate().map_err(native)?;
        let problem_bytes = serde_json::to_vec(&problem).map_err(native)?;
        Ok(Self {
            preparation,
            provenance,
            problem,
            problem_bytes,
        })
    }

    pub fn preparation(&self) -> &Arc<PositiveProjectivePreparation> {
        &self.preparation
    }
    pub fn provenance(&self) -> &SourceProvenance {
        &self.provenance
    }
    pub fn problem(&self) -> &Problem {
        &self.problem
    }
    pub(super) fn matches_native_problem(&self, problem: &Problem) -> bool {
        serde_json::to_vec(problem).is_ok_and(|bytes| bytes == self.problem_bytes)
    }
    pub fn source_identity(&self) -> Result<String> {
        match &self.provenance {
            SourceProvenance::Exact => {
                crate::generation::source_identity(self.preparation.input(), &[], &[])
                    .map_err(native)
            }
            SourceProvenance::PreparametricGraph(graph) => {
                if !Arc::ptr_eq(graph.exact(), self.preparation.input()) {
                    return Err(GcadError::RequestMismatch);
                }
                Ok(graph.source_identity().to_owned())
            }
            _ => Err(GcadError::Unsupported(
                "positive-projective request provenance".into(),
            )),
        }
    }
    /// Save the returned native result before verification, including failed or
    /// incomplete statuses. Hard resource/process limits belong to the caller.
    pub fn solve(&self) -> Result<SolveResult> {
        symgcad::solver::solve(&self.problem).map_err(|e| GcadError::Solve(e.to_string()))
    }
    /// Fresh native verification; request settings cannot be changed by replay.
    /// Reduced-only geometry never substitutes for the original homogeneous proof.
    pub fn verify(&self, original: Arc<SolveResult>) -> Result<PositiveProjectiveQuotient> {
        if !self.matches_native_problem(&original.problem) {
            return Err(GcadError::RequestMismatch);
        }
        match &self.provenance {
            SourceProvenance::Exact => {
                PositiveProjectiveQuotient::verify(self.preparation.clone(), original)
            }
            SourceProvenance::PreparametricGraph(graph) => {
                self.source_identity()?;
                PositiveProjectiveQuotient::verify_graph(
                    graph.clone(),
                    self.preparation.anchor_index(),
                    original,
                )
            }
            _ => Err(GcadError::Unsupported(
                "positive-projective request provenance".into(),
            )),
        }
    }
}

fn native(error: impl std::fmt::Display) -> GcadError {
    GcadError::Invalid(error.to_string())
}

#[cfg(test)]
mod tests;
