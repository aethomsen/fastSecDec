//! V3 uses one native exact coefficient shift per request. Original coefficient
//! derivatives and endpoint controls remain tied to the same certified source.
use super::{RootProgram, Scope, attempt, numeric::Number};
use crate::threshold::maps::regular::RegularSection;
use std::sync::Arc;
use symbolica::{
    atom::{Atom, AtomCore, Symbol},
    domains::{
        float::{Complex, DoubleFloat, ErrorPropagatingFloat as Epf, Float},
        rational::Rational,
    },
    evaluate::{EvaluationDomain, OptimizationSettings},
    symbol,
};

mod accuracy;
mod artifacts;
mod codec;
mod narrow;
use accuracy::{evaluator, exact_fiber_check};
use narrow::narrow_sections;
