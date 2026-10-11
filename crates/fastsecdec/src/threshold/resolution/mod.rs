//! Local resolution certificates and constructive contact/monomial stages.
//!
//! Native Q-polynomial charts, supplied inverse frames and controlled blowups
//! are checked alongside affine quotient/etale relative derivations. Inclusive
//! derivative ideals construct algebraic maximum-order loci and finite contact
//! opens without inferring a boundary ledger. Verified relative boundary
//! geometry supports all monomial blowup charts and local exceptional histories.
//! The complete BM invariant/producer, global center gluing, real branch atlas,
//! orientation and closed-face endpoint certificates remain separate gates.
mod algebra;
mod blowup;
mod chart;
mod companion;
mod companion_stage;
mod component_factor;
mod components;
mod contact;
mod cover;
mod differential;
mod elimination;
mod etale;
mod geometry;
mod iteration;
mod localization;
mod localized;
mod marked;
mod marked_equivalence;
mod matrix;
mod native_basis;
mod prepared_membership;
mod producer;
mod recursive;
mod transform;
pub use algebra::{Budget, Error, Ideal, Limits, Poly, Ring};
pub use blowup::{
    AdaptedCover, AdaptedOpen, AdaptedProduction, AdaptedProgress, BirthContext, BlowupProduction,
    BlowupProgress, CompletedContinuedCharts, ContinuedChartAdvance, ContinuedChartState,
    ContinuedFrontierCompletion, ContinuedSupportedFrontier, DivisorReceipt, EmbeddedProblemCenter,
    EmbeddedProblemProduction, EmptyAdaptedOpen, EmptySupportRefinement, HistoryCenter,
    HistoryChartStep, HistoryStep, MonomialBlowup, MonomialBlowupChart, OriginalLevelOpen,
    OriginalLevelPullback, OriginalLowerLevel, OriginalRecursionTree, OriginalTreePullback,
    RefinedSupportCover, RefinedSupportOpen, ResolutionHistory, SupportFrameMap,
    UnresolvedSupportFrame, produce_adapted_cover, produce_monomial_blowup,
};
pub use chart::{Boundary, Chart, FrameCertificate, Guard, Map, VerifiedFrame};
pub use companion::{
    CartierDivision, ComponentResidualOrder, ComponentResidualProduction, FactorProduction,
    FactorProgress, OldBoundaryCoefficient, OldBoundaryProduction, OldBoundaryProgress,
    OldIncidence, QuotientMethod, RelativeOrderLayer, ResidualOrderProduction,
    ResidualOrderProgress, RestrictedResidualOrder, VerifiedCartierQuotient,
    WholeCartierFactorization, divide_cartier, factor_whole_cartier_equations,
    produce_component_residual_order, produce_old_boundary_coefficient,
    produce_restricted_residual_order,
};
pub use companion_stage::{
    CompanionCoefficientChart, CompanionCoefficientProduction, CompanionContactOpen,
    CompanionOpenProduction, CompanionOpenProgress, construct_companion_coefficient,
    construct_companion_coefficient_for_cycle, produce_companion_open,
};
pub use component_factor::{
    BoundaryMaximality, CompletedComponentFactors, ComponentFactorCompletion,
    ComponentFactorFrontier, ComponentFactorLimits, FactorAdvance, FactorChild, FactorData,
    FactorLeaf, FactorNode, FactorPath, FactorPendingEvidence,
};
pub use components::{
    BoundaryAlgebra, ComponentOpen, ComponentPattern, ComponentProduction, ComponentProgress,
    EliminationEvidence, RegularAlgebra, RegularOrigin, VerifiedAnnihilator,
    VerifiedComponentSplit, produce_component_split,
};
pub(crate) use contact::clear_units;
pub use contact::{
    ContactJet, ContactProduction, ContactProgress, ContactQuotient, RingExtension, UnitClearing,
    construct_contact_quotient,
};
pub use cover::{OpenCoverCertificate, VerifiedOpenCover};
pub use etale::{EtaleCertificate, EtaleFrame};
pub use geometry::{
    BoundaryId, InitialDivisor, IntersectionCertificate, MonomialCandidate, MonomialProduction,
    MonomialProgress, MonomialWitness, NormalizationOutcome, NormalizationProgress,
    ProducedMonomialCenter, QuotientNormalizer, SncProduction, SncProgress,
    VerifiedQuotientNormalization, VerifiedRelativeSnc, produce_monomial_center,
    verify_initial_relative_snc,
};
pub use iteration::{
    CarriedMonomialWitness, CompletedMonomialResolution, IterationAdvance, IterationCompletion,
    IterationLimits, MonomialFrontier, NodePath, PendingEvidence, ResolutionNode,
    carry_monomial_witness, exponent_rank,
};
pub use localization::{
    CoverLocalizationProduction, LocalizationProduction, LocalizationProgress,
    LocalizedCoverHistory, LocalizedHistory, VerifiedCoverOpen, VerifiedPrincipalOpen,
    localize_component_open, localize_verified_cover_open,
};
pub use localized::LocalizedAlgebra;
pub use marked::{
    CenterCertificate, CoefficientRestriction, CompanionArithmetic, MarkedIdeal, RecordedInvariant,
    VerifiedCenter,
};
pub use marked_equivalence::{
    PrincipalPowerFactor, PrincipalPowerOutcome, PrincipalPowerPresentation,
    PrincipalPowerProgress, PrincipalPowerUnchanged, PrincipalPowerUnchangedReason,
    produce_principal_power_presentation,
};
pub use producer::{
    ContactCandidate, DerivativeStage, OrderProduction, ProducedContactCover, ProductionProgress,
    produce_ordinary_contact_cover,
};
pub use recursive::{
    AuxiliaryRecursionOrigin, BoundaryFreeFirstCenter, CarriedFirstBlowup, CarriedFirstChart,
    CompanionCenter, CompanionCenterOutcome, CompanionFirstCenter, ComponentLocalization,
    ComponentLocalizationAdvance, ComponentLocalizationFrontier, ComponentLocalizationState,
    ControlledGenerator, CycleOrigin, CycleSnapshot, FirstBlowup, InitialProblem,
    InitialProblemRecursion, LiftReceipt, LocalizedComponentCenter, LocalizedComponentSources,
    PreparedProblemCenter, ProblemConstruction, ProblemRecursionRecord, RecursiveAdvance,
    RecursiveBlowupChart, RecursiveCenter, RecursiveComponentCover, RecursiveLevel,
    RecursiveOutcome, ResidualDrop, carry_first_blowup, first_coordinate_blowup,
    prove_first_residual_drop,
};
pub use transform::{ControlledTransformCertificate, DivisorTransform, VerifiedTransform};

#[cfg(test)]
mod etale_tests;
#[cfg(test)]
mod tests;

pub use blowup::{
    CarriedCompanionChart, CarriedCompanionSupport, CarriedMonomialCenter, CheckedRecursiveCenter,
    CompletedLocalContinuation, ContinuationAdvance, ContinuationCompletion, ContinuationLimits,
    ContinuationNode, EmbeddedChildCycle, InducedCenterProduction, InducedChildChart,
    LocalCompanionContinuation, LocalPrincipalization, PrincipalizationProduction,
    RecursiveAdaptation, RecursiveAdaptedOpen, RecursiveCenterOrigin, RelativeBlowupGeometry,
    RelativeMarkedTransform, RelativeRecursiveBlowup, RelativeRecursiveChart, SaturatedSupport,
    StrictContactSupport, StrictSupportOpen, SupportedGenerator, SupportedMarkedTransform,
    adapt_recursive_center, blowup_recursive_center, carry_companion_chart,
    certify_local_principalization, induce_child_chart, produce_induced_monomial_center,
    restrict_relative_chart, transport_contact_support,
};
pub use blowup::{
    CompletedEmbeddedChild, EmbeddedCenterProduction, EmbeddedMonomialCenter, EmbeddedPresentation,
    EmbeddedTransition, EmbeddedTransitionOpen, IncidenceContinuation, IncidenceDrop,
    MarkedCosupport, PhysicalSupportCover, PhysicalSupportOpen, PhysicalSupportRestriction,
    SupportEmbedding, produce_embedded_monomial_center,
};

pub use blowup::OriginalProblemCenter;

pub use blowup::{AnchoredOriginalTree, OriginalAnchorCenter};

pub use blowup::{SupportedProblemAncestor, SupportedProblemChart, SupportedProblemOpen};

pub use recursive::{SupportedDropInventory, SupportedDropRecord, SupportedResidualDrop};

pub use blowup::{ContinuedSupportedCenter, ContinuedSupportedChart, ContinuedSupportedProduction};

pub use blowup::{ContinuedRefinementFrontier, ContinuedRefinementState, StrictSupportRefinement};
pub use blowup::{StrictSupportPreparation, SupportGuardPullback};

pub use blowup::{
    AccumulatedExceptional, AmbientContinuationCenter, AmbientCycleCarry, AmbientDropInventory,
    AmbientDropRecord, AmbientNextChart, AmbientResidualDrop, OriginalCompanionPullback,
    OriginalDivisorTerminal, OriginalFactorTerminal, OriginalGeneratorPullback,
    OriginalPhysicalProblem, OriginalPhysicalRoot, OriginalPhysicalState,
    OriginalTerminalInventory, OriginalTerminalRecord, PhysicalChartAdvance, PhysicalChartFrontier,
    PhysicalChartWork, PhysicalContinuationCenter, PhysicalCycleCompletion, PhysicalCycleStart,
    PhysicalDropInventory, PhysicalLineageEdge, PhysicalResidualDrop, PhysicalRestartOwner,
    PhysicalTerminalInventory, RetiredPhysicalCycle,
};

pub use blowup::{CenterOverlap, GluedAdmissibleCenter, LocalCenter, PrincipalCenterAtlas};

pub use blowup::{
    MonomialCenterRestriction, MonomialMaximumSelection, SelectedCenterState, SelectedMonomialCover,
};
