//! Verified adapted opens and controlled relative blowup charts.
//!
//! These are overlapping algebraic charts with relative Jacobians and checked
//! local exceptional history. They are not a disjoint real integration atlas,
//! a global BM center-gluing algorithm or a complete endpoint certificate.
mod adapted;
mod embedded_center;
mod embedding;
mod general;
mod normal_presentation;
pub use normal_presentation::CenterNormalPresentation;
mod general_transform;
mod graph;
mod helpers;
mod incidence;
mod presentation;
mod presentation_transition;
mod saturation;
mod standard;
mod support;
pub use embedded_center::{
    EmbeddedCenterProduction, EmbeddedMonomialCenter, produce_embedded_monomial_center,
};
pub use embedding::{
    MatchedPhysicalRestriction, MatchedPhysicalSupport, PhysicalSupportCover, PhysicalSupportOpen,
    PhysicalSupportRestriction, SupportEmbedding,
};
pub use general::{
    CheckedRecursiveCenter, RecursiveAdaptation, RecursiveAdaptedOpen, RecursiveCenterOrigin,
    adapt_recursive_center,
};
pub use general_transform::{
    RelativeBlowupGeometry, RelativeMarkedTransform, RelativeRecursiveBlowup,
    RelativeRecursiveChart, blowup_recursive_center, restrict_relative_chart,
};
pub use incidence::{
    CompletedEmbeddedChild, IncidenceContinuation, IncidenceDrop, MarkedCosupport,
};
pub use presentation::EmbeddedPresentation;
pub use presentation_transition::{EmbeddedTransition, EmbeddedTransitionOpen};
pub use saturation::SaturatedSupport;
pub use support::{
    StrictContactSupport, StrictSupportOpen, UnresolvedSupportFrame, transport_contact_support,
};
mod history;
mod transform;
pub use adapted::{
    AdaptedCover, AdaptedOpen, AdaptedProduction, AdaptedProgress, EmptyAdaptedOpen,
    produce_adapted_cover,
};
pub use history::{BirthContext, HistoryCenter, HistoryChartStep, HistoryStep, ResolutionHistory};
pub use transform::{
    BlowupProduction, BlowupProgress, DivisorReceipt, MonomialBlowup, MonomialBlowupChart,
    produce_monomial_blowup,
};

#[cfg(test)]
mod tests;

mod coefficient;
pub use coefficient::{
    CarriedCompanionChart, CarriedCompanionSupport, SupportedGenerator, SupportedMarkedTransform,
    carry_companion_chart,
};

mod induced;
pub use induced::{
    CarriedMonomialCenter, EmbeddedChildCycle, InducedCenterProduction, InducedChildChart,
    induce_child_chart, produce_induced_monomial_center,
};

mod terminal;
pub use terminal::{
    LocalPrincipalization, PrincipalizationProduction, certify_local_principalization,
};

mod continuation;
pub use continuation::{
    CompletedLocalContinuation, ContinuationAdvance, ContinuationCompletion, ContinuationLimits,
    ContinuationNode, LocalCompanionContinuation,
};

#[cfg(test)]
mod relative_tests;

mod problem_center;
mod refinement;
mod tower;
pub(crate) use problem_center::produce_embedded_problem_center;
pub use problem_center::{EmbeddedProblemCenter, EmbeddedProblemProduction};
pub use refinement::{
    EmptySupportRefinement, RefinedSupportCover, RefinedSupportOpen, SupportFrameMap,
};
pub use tower::{
    AnchoredOriginalTree, OriginalAnchorCenter, OriginalLevelOpen, OriginalLevelPullback,
    OriginalLowerLevel, OriginalRecursionTree, OriginalTreePullback, PowerLevelTransform,
};

mod original_problem;
pub use original_problem::OriginalProblemCenter;

mod lower_boundary;

mod supported_problem;
pub use supported_problem::{
    SupportedProblemAncestor, SupportedProblemChart, SupportedProblemOpen,
};

mod continued_problem;
pub use continued_problem::{
    ContinuedSupportedCenter, ContinuedSupportedChart, ContinuedSupportedProduction,
};

mod continued_frontier;
pub use continued_frontier::{
    CompletedContinuedCharts, ContinuedChartAdvance, ContinuedChartState,
    ContinuedFrontierCompletion, ContinuedSupportedFrontier,
};

mod support_preparation;
pub use support_preparation::{StrictSupportPreparation, SupportGuardPullback};
mod chart_refinement;
pub use chart_refinement::{
    ContinuedRefinementFrontier, ContinuedRefinementState, StrictSupportRefinement,
};

mod physical_cycle;
pub use physical_cycle::{
    AccumulatedExceptional, AmbientContinuationCenter, AmbientCycleCarry, AmbientDropInventory,
    AmbientDropRecord, AmbientNextChart, AmbientResidualDrop, OriginalCompanionPullback,
    OriginalDivisorTerminal, OriginalFactorTerminal, OriginalGeneratorPullback,
    OriginalPhysicalProblem, OriginalPhysicalRoot, OriginalPhysicalState,
    OriginalTerminalInventory, OriginalTerminalRecord, PhysicalChartAdvance, PhysicalChartFrontier,
    PhysicalChartWork, PhysicalContinuationCenter, PhysicalCycleCompletion, PhysicalCycleStart,
    PhysicalDropInventory, PhysicalLineageEdge, PhysicalResidualDrop, PhysicalRestartOwner,
    PhysicalTerminalInventory, RetiredPhysicalCycle,
};

mod descent;
pub use descent::{CenterOverlap, GluedAdmissibleCenter, LocalCenter, PrincipalCenterAtlas};

pub use descent::{
    MonomialCenterRestriction, MonomialMaximumSelection, SelectedCenterState, SelectedMonomialCover,
};

mod carried_power;
pub use carried_power::{
    CarriedPowerAdvance, CarriedPowerCompanion, CarriedPowerState, CarriedPowerTransition,
    PendingPowerTransition, PowerCosupportProof,
};

pub use physical_cycle::{
    PhysicalAuxiliaryAdvance, PhysicalAuxiliaryWork, PhysicalPowerInventory,
    PhysicalPowerPreparation, PhysicalPowerSlot, PhysicalPowerSlotKind, PhysicalPowerTerminal,
    RetiredPowerInventory,
};

pub use embedding::{AmbientSupportRestriction, EmptyAmbientSupport, PulledSupportCover};
