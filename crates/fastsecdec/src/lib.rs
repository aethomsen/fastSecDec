//! Native sector integration built on HEPKit graphs and Symbolica expressions.
//!
//! Graphs, momentum routing and algebra retain their existing ecosystem types.
//! This crate owns only integral-specific normalization and generation state.

#![forbid(unsafe_code)]
// Retained resolution ancestry needs a deeper compiler auto-trait query stack.
#![cfg_attr(feature = "threshold-decomposition", recursion_limit = "256")]

#[cfg(all(feature = "native", feature = "portable"))]
compile_error!("select exactly one FastSecDec backend: native or portable");
#[cfg(not(any(feature = "native", feature = "portable")))]
compile_error!("select a FastSecDec backend: native (default) or portable");

pub mod contour;
pub mod diagnostics;
pub mod error;
pub mod generation;
pub mod input;
pub mod integration;
pub mod kernel;
pub mod parametric;
pub mod reference;
pub mod results;
pub mod status;
#[cfg(feature = "threshold-decomposition")]
pub mod threshold;

pub use error::{Error, Result};
pub use feynkit_graph::{EdgeId, FeynmanDiagram, IntegralFamily};
pub use feynkit_kinematics::Kinematics;
pub use feynkit_model::{Model, ParameterCard};
pub use symbolica::atom::{Atom, AtomCore, Symbol};
