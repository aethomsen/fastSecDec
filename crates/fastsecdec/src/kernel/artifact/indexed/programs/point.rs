use super::{ProgramRecipe, ProgramRecipeCatalogue};
use crate::{
    kernel::{
        KernelError, KernelSet, artifact, parameter_point, threshold_metadata::ResidentSelection,
    },
    results::ExactContributionPolicy,
};
use std::collections::BTreeMap;
use symbolica::atom::Symbol;

impl ProgramRecipeCatalogue {
    /// Associate an already-bound exact-only threshold carrier with this complete
    /// catalogue and return the complete numerical point identity.
    ///
    /// The carrier must include all exact contributions and no stochastic ones,
    /// with the same native parent, layout, runtime schema and committed point.
    /// Fixed templates accept an empty map and keep their catalogue identity.
    /// This reads no program records, evaluates no offsets and changes neither
    /// the carrier's binding nor its selected scope. It does not replay geometry.
    pub fn bound_threshold_content_id(
        &self,
        exact: &KernelSet,
        values: &BTreeMap<Symbol, f64>,
    ) -> Result<String, KernelError> {
        let fail =
            |message: &str| KernelError::Artifact(format!("threshold catalogue point: {message}"));
        if self.recipe != ProgramRecipe::ThresholdV1
            || exact.program_recipe() != ProgramRecipe::ThresholdV1
        {
            return Err(fail("requires threshold catalogue and carrier"));
        }
        self.threshold_scope()?;
        let summary = self
            .threshold
            .as_ref()
            .ok_or_else(|| fail("missing complete inventory"))?;
        let owner = exact
            .threshold_metadata()
            .ok_or_else(|| fail("missing native carrier owner"))?;
        if !exact.sectors().is_empty()
            || !matches!(&owner.resident().selection,
                ResidentSelection::Selected { stochastic_contributions, exact_policy }
                if stochastic_contributions.is_empty()
                    && *exact_policy == ExactContributionPolicy::IncludeAll)
        {
            return Err(fail("requires exact-only IncludeAll selection"));
        }
        if exact.orders() != self.orders
            || exact.components() != self.components
            || exact.exact_coefficients().len() != self.orders.len()
            || exact
                .runtime_parameters()
                .iter()
                .map(|symbol| symbol.get_name())
                .ne(self.runtime_parameters.iter().map(String::as_str))
        {
            return Err(fail("carrier layout or runtime schema differs"));
        }
        summary.check_parent(owner)?;
        let policy = artifact::compiler_policy_with_settings(exact.compilation_settings);
        let selected_template = owner.semantic_identity(&policy, &exact.precision)?;
        if selected_template != exact.template_content_id() {
            return Err(fail("carrier template identity differs"));
        }
        let complete_template = owner.semantic_identity_for_selection(
            &policy,
            &exact.precision,
            &ResidentSelection::Complete,
        )?;
        if complete_template != self.content_id {
            return Err(fail("complete catalogue template identity differs"));
        }
        let ordered = exact.ordered_runtime_parameters(values)?;
        owner.admit_parameter_point(exact.runtime_parameters(), &ordered)?;
        if !exact.parameters_bound() {
            return Err(fail("runtime carrier is not bound"));
        }
        if parameter_point::identity(&selected_template, &ordered) != exact.content_id() {
            return Err(fail("carrier is bound to a different point"));
        }
        Ok(parameter_point::identity(&complete_template, &ordered))
    }
}
