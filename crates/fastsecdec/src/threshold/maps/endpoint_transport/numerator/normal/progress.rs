//! Caller-owned, sparse observations around preparation operations.
use super::{Error, Result, Work};
use std::ops::ControlFlow;

/// An operation in symbolic normal preparation. Indices refer to the retained
/// owner inventories, not a numerical sample or a percentage of work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationStage {
    Preparation,
    Validation,
    Admission,
    SelectedParameters { section: usize },
    RootProgram { section: usize },
    RootCall { section: usize },
    ArgumentDomain,
    FiniteArgument { argument: usize },
    RationalArgument { argument: usize },
    Denominator { argument: usize },
    ArgumentComposition { argument: usize },
    Template,
    TemplateRoles,
    RootJet { section: usize },
    RootArguments { section: usize },
    RootDerivative { section: usize, order: u32 },
    RootRestriction { section: usize, order: u32 },
    ArgumentSeries { argument: usize },
    OuterSeries,
    Record { record: usize },
    Definitions,
    DefinitionScan { coefficient: usize },
    SourcePartial { record: usize, demand: usize },
    Assembly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreparationState {
    Started,
    Completed,
}

/// A sparse transition. Callers supply timing, storage and presentation.
/// Stages may nest. Completed means that operation returned successfully;
/// errors and cancellation do not synthesize completion. Native calls remain
/// cooperative and cannot be interrupted by this observer while executing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PreparationProgress {
    pub stage: PreparationStage,
    pub state: PreparationState,
}

pub(super) type Observer<'a> = dyn FnMut(PreparationProgress) -> ControlFlow<()> + 'a;

pub(super) fn emit(
    stage: PreparationStage,
    state: PreparationState,
    work: &mut Work<'_>,
    observer: &mut Observer<'_>,
) -> Result<()> {
    work.check()?;
    if observer(PreparationProgress { stage, state }).is_break() {
        return Err(Error::Cancelled);
    }
    // Observer work is caller time within the same cooperative budget.
    work.budget.check()?;
    Ok(())
}

pub(super) fn ignore(_: PreparationProgress) -> ControlFlow<()> {
    ControlFlow::Continue(())
}
