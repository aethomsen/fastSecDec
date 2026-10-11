//! Native algebraic-kernel owner, precision, transport and full-vector controls.
use super::program::PreparedCoefficientVector;
use super::{
    CompilationSettings, EvaluatorBackend, PrecisionPolicy, SectorKernel, StabilitySettings,
};
use crate::{
    generation,
    parametric::{
        FactorRole, FactorSemantics, ParametricDomain, ParametricIntegrand, ParametricTerm,
        PolynomialFactor,
    },
    threshold::{
        gcad::{GcadKinematics, GcadRequest, SolverOptions},
        regularization::secant,
    },
};
use std::sync::Arc;
use symbolica::{
    atom::{AliasedAtom, Atom, AtomCore},
    domains::rational::Rational,
    symbol,
};

mod fixtures;
use fixtures::*;
mod artifacts;
mod brackets;
mod precision;
mod science;

mod paired;
