//! Sealed borrowed compiler input. Only already issued continuation owners enter
//! the shared record/compilation pipeline; this is not an importable proof DTO.
use super::regularization::{
    BoundContinuation, projective_family::ContinuedProjectiveFamily,
    projective_pair_family::ContinuedProjectivePairFamily, secant::ContinuedFamily,
};
use crate::{
    generation::{EndpointProfileRow, GenerationOptions},
    kernel::algebraic::Scope,
};
use symbolica::atom::{Atom, Symbol};
mod source;
pub(crate) use source::SourceAuthority;
#[derive(Clone)]
pub(crate) struct Definition {
    pub head: Symbol,
    pub tags: Vec<Atom>,
    pub formals: Vec<Symbol>,
    pub body: Atom,
    /// Native tag derivatives followed by source-coordinate mixed orders.
    pub orders: Option<Vec<usize>>,
    pub always_inline: bool,
}
#[derive(Clone, Copy)]
pub(crate) enum ContinuedSource<'a> {
    Rational(&'a BoundContinuation<'a>),
    Algebraic(&'a ContinuedFamily<'a>),
    Projective(&'a ContinuedProjectiveFamily<'a>),
    PositiveProjective(&'a ContinuedProjectivePairFamily<'a>),
}
impl ContinuedSource<'_> {
    pub fn authority(&self) -> SourceAuthority<'_> {
        match self {
            Self::Rational(b) => {
                SourceAuthority::Request(b.certificate().decomposition().request())
            }
            Self::Algebraic(b) => SourceAuthority::Request(b.certificate().owner().request()),
            Self::Projective(b) => SourceAuthority::Request(b.certificate().owner().request()),
            Self::PositiveProjective(b) => {
                SourceAuthority::PositiveProjective(b.certificate().owner())
            }
        }
    }
    pub fn coordinates(&self) -> Vec<Symbol> {
        match self {
            Self::Rational(b) => b.coordinates().to_vec(),
            Self::Algebraic(b) => b.certificate().coordinates().to_vec(),
            Self::Projective(b) => b.certificate().coordinates().to_vec(),
            Self::PositiveProjective(b) => b.certificate().coordinates().to_vec(),
        }
    }
    pub fn runtime_parameters(&self) -> &[Symbol] {
        match self {
            Self::Rational(_) => &[],
            Self::Algebraic(b) => b.certificate().runtime_parameters(),
            Self::Projective(b) => b.certificate().runtime_parameters(),
            Self::PositiveProjective(b) => b.certificate().runtime_parameters(),
        }
    }
    pub fn regulators(&self) -> Vec<Symbol> {
        match self {
            Self::Rational(b) => b.regulators().to_vec(),
            Self::Algebraic(b) => b.certificate().regulators().to_vec(),
            Self::Projective(b) => b.certificate().regulators().to_vec(),
            Self::PositiveProjective(b) => b.certificate().regulators().to_vec(),
        }
    }
    pub fn charts(&self) -> &[Atom] {
        match self {
            Self::Rational(b) => b.chart_expressions(),
            Self::Algebraic(b) => b.charts(),
            Self::Projective(b) => b.charts(),
            Self::PositiveProjective(b) => b.charts(),
        }
    }
    pub fn profiles(&self) -> &[Vec<EndpointProfileRow>] {
        match self {
            Self::Rational(b) => b.profiles(),
            Self::Algebraic(b) => b.profiles(),
            Self::Projective(b) => b.profiles(),
            Self::PositiveProjective(b) => b.profiles(),
        }
    }
    pub fn options(&self) -> &GenerationOptions {
        match self {
            Self::Rational(b) => b.generation_options(),
            Self::Algebraic(b) => b.options(),
            Self::Projective(b) => b.options(),
            Self::PositiveProjective(b) => b.options(),
        }
    }
    pub fn definitions(&self, chart: usize) -> Vec<Definition> {
        match self {
            Self::Rational(b) => b
                .definitions()
                .iter()
                .map(|d| Definition {
                    head: d.head(),
                    tags: d.tags().to_vec(),
                    formals: d.formals().to_vec(),
                    body: d.body().clone(),
                    orders: d.derivative_order().map(|n| vec![0, n]),
                    always_inline: d.derivative_order().is_some(),
                })
                .collect(),
            Self::Algebraic(b) => b.definitions().to_vec(),
            Self::Projective(b) => b.definitions(),
            Self::PositiveProjective(b) => b.definitions(chart),
        }
    }
    pub fn roots(&self, chart: usize) -> Scope {
        match self {
            Self::Rational(_) => Scope::default(),
            Self::Algebraic(b) => b.certificate().callback_scope().clone(),
            Self::Projective(b) => b.certificate().callback_scope(),
            Self::PositiveProjective(b) => b.roots(chart),
        }
    }
    /// Root coefficients remain sampled constants; only the historical exact
    /// rational owner supplies its existing scalar materialization capability.
    pub fn rational(&self) -> Option<&BoundContinuation<'_>> {
        match self {
            Self::Rational(b) => Some(b),
            Self::Algebraic(_) | Self::Projective(_) | Self::PositiveProjective(_) => None,
        }
    }
}
