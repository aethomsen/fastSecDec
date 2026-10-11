use super::super::{KernelCatalogue, RecordDescriptor, failure, layout};
use crate::{kernel::KernelError, status::CoefficientComponent};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Explicit mathematical program selection. Known recipe IDs can be inspected;
/// their executable admission is checked separately by the native descriptor.
/// Unknown/new versions never fall back to an existing recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProgramRecipe {
    #[serde(rename = "undeformed-v1")]
    UndeformedV1,
    #[serde(rename = "fixed-v1")]
    FixedV1,
    #[serde(rename = "dynamic-polynomial-v1")]
    DynamicPolynomialV1,
    #[serde(rename = "dynamic-sign-aware-v1")]
    DynamicSignAwareV1,
    #[serde(rename = "threshold-v1")]
    ThresholdV1,
}
impl ProgramRecipe {
    pub fn name(self) -> &'static str {
        match self {
            Self::ThresholdV1 => "threshold-v1",
            Self::UndeformedV1 => "undeformed-v1",
            Self::FixedV1 => "fixed-v1",
            Self::DynamicPolynomialV1 => "dynamic-polynomial-v1",
            Self::DynamicSignAwareV1 => "dynamic-sign-aware-v1",
        }
    }
    pub fn recipe_parameters(self) -> &'static [&'static str] {
        match self {
            Self::UndeformedV1 | Self::ThresholdV1 => &[],
            Self::FixedV1 => &["fastsecdec::contour::lambda"],
            Self::DynamicPolynomialV1 | Self::DynamicSignAwareV1 => &[
                "fastsecdec::contour::dynamic::safety_fraction",
                "fastsecdec::contour::dynamic::lambda_cap",
                "fastsecdec::contour::dynamic::displacement_cap",
            ],
        }
    }
    pub fn is_dynamic(self) -> bool {
        matches!(self, Self::DynamicPolynomialV1 | Self::DynamicSignAwareV1)
    }
    /// Whether this recipe uses a complex contour map. Other regularization
    /// strategies must not acquire contour parameters or checks implicitly.
    pub fn is_contour(self) -> bool {
        matches!(
            self,
            Self::FixedV1 | Self::DynamicPolynomialV1 | Self::DynamicSignAwareV1
        )
    }
    pub(crate) fn validate_runtime_schema(self, parameters: &[String]) -> Result<(), KernelError> {
        let expected = self.recipe_parameters();
        if expected
            .iter()
            .any(|parameter| !parameters.iter().any(|actual| actual == parameter))
            || parameters.iter().any(|parameter| {
                parameter.starts_with("fastsecdec::contour::")
                    && !expected.contains(&parameter.as_str())
            })
        {
            return Err(failure(
                "runtime schema differs from the explicit mathematical recipe",
            ));
        }
        Ok(())
    }
    pub(in crate::kernel::artifact::indexed) fn legacy(parameters: &[String]) -> Self {
        if parameters
            .iter()
            .any(|p| p == Self::FixedV1.recipe_parameters()[0])
        {
            Self::FixedV1
        } else {
            Self::UndeformedV1
        }
    }
}

/// One complete mathematical recipe. Physical byte ranges belong to the outer
/// archive; layouts, source/chart IDs and sector IDs belong to this recipe.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramRecipeCatalogue {
    pub recipe: ProgramRecipe,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<super::super::ThresholdArchiveSummary>,
    pub content_id: String,
    pub orders: Vec<i32>,
    pub components: Vec<CoefficientComponent>,
    /// Exact native input order; physical/recipe lists classify this same schema.
    pub runtime_parameters: Vec<String>,
    pub physics_parameters: Vec<String>,
    pub recipe_parameters: Vec<String>,
    pub records: Vec<RecordDescriptor>,
}
impl ProgramRecipeCatalogue {
    /// Scope of this complete, structurally checked contribution catalogue.
    /// Use it only with the full catalogue sector inventory and exact carrier;
    /// `load_exact()` and selected records retain their own narrower scope.
    /// This view does not replay global geometry/continuation proofs.
    pub fn threshold_scope(
        &self,
    ) -> Result<Option<crate::kernel::ThresholdResultScope>, KernelError> {
        let Some(summary) = &self.threshold else {
            if self.recipe == ProgramRecipe::ThresholdV1 {
                return Err(failure("threshold catalogue lacks its closed inventory"));
            }
            return Ok(None);
        };
        self.validate(&summary.source_identity, true)?;
        crate::kernel::ThresholdResultScope::complete_catalogue(summary.source_extent.clone())
            .map(Some)
    }
    pub fn source_selection(&self) -> Option<&crate::generation::SourceSectorSelection> {
        self.records
            .first()
            .and_then(|record| record.receipt.source_scope.as_ref())
            .map(|scope| scope.selection())
    }
    pub fn runtime_parameters(&self) -> Vec<String> {
        self.runtime_parameters.clone()
    }
    pub fn sector_count(&self) -> usize {
        self.records.iter().filter(|r| r.sector.is_some()).count()
    }
    pub fn sector(&self, sector: usize) -> Result<&RecordDescriptor, KernelError> {
        self.records
            .iter()
            .find(|record| record.sector == Some(sector))
            .ok_or_else(|| failure(format!("unknown {} sector {sector}", self.recipe.name())))
    }
    pub(super) fn finish(
        recipe: ProgramRecipe,
        records: Vec<RecordDescriptor>,
        source: &str,
    ) -> Result<Self, KernelError> {
        let layout = layout::finish_layout(records)?;
        let parameters = recipe
            .recipe_parameters()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>();
        if layout
            .records
            .iter()
            .any(|record| record.receipt.program_recipe() != recipe)
        {
            return Err(failure(
                "record runtime schema differs from its advertised recipe",
            ));
        }
        recipe.validate_runtime_schema(&layout.runtime_parameters)?;
        let physics_parameters = layout
            .runtime_parameters
            .iter()
            .filter(|p| !parameters.contains(p))
            .cloned()
            .collect();
        let mut catalogue = Self {
            recipe,
            threshold: None,
            content_id: String::new(),
            orders: layout.orders,
            components: layout.components,
            runtime_parameters: layout.runtime_parameters,
            physics_parameters,
            recipe_parameters: parameters,
            records: layout.records,
        };
        catalogue.content_id = catalogue.identity(source)?;
        catalogue.validate(source, true)?;
        Ok(catalogue)
    }
    pub(super) fn finish_threshold(
        records: Vec<RecordDescriptor>,
        summary: super::super::ThresholdArchiveSummary,
    ) -> Result<Self, KernelError> {
        let layout = layout::finish_layout(records)?;
        let value = Self {
            recipe: ProgramRecipe::ThresholdV1,
            content_id: summary.complete_content_id.clone(),
            orders: layout.orders,
            components: layout.components,
            runtime_parameters: layout.runtime_parameters.clone(),
            physics_parameters: layout.runtime_parameters,
            recipe_parameters: Vec::new(),
            records: layout.records,
            threshold: Some(summary),
        };
        value.validate(
            value.threshold.as_ref().unwrap().source_identity.as_str(),
            true,
        )?;
        Ok(value)
    }
    fn identity(&self, source: &str) -> Result<String, KernelError> {
        if let Some(threshold) = &self.threshold {
            if threshold.source_identity != source {
                return Err(failure("threshold source identity differs"));
            }
            return Ok(threshold.complete_content_id.clone());
        }

        let mut hash = blake3::Hasher::new();
        hash.update(b"fastsecdec-program-recipe-v1\0");
        serde_json::to_writer(
            &mut hash,
            &(
                source,
                self.recipe,
                &self.orders,
                &self.components,
                &self.runtime_parameters,
                &self.physics_parameters,
                &self.recipe_parameters,
            ),
        )?;
        for record in &self.records {
            serde_json::to_writer(
                &mut hash,
                &(
                    &record.receipt.native_content_id,
                    &record.receipt.source_indices,
                    &record.output_indices,
                    record.sector,
                ),
            )?;
        }
        Ok(hash.finalize().to_hex().to_string())
    }
    fn validate_schema(&self) -> Result<(), KernelError> {
        if (self.recipe == ProgramRecipe::ThresholdV1) != self.threshold.is_some() {
            return Err(failure("threshold archive capability missing or mixed"));
        }
        if let Some(threshold) = &self.threshold {
            threshold.validate(&self.records)?;
        }

        super::super::super::validate_content_id(&self.content_id)?;
        self.recipe
            .validate_runtime_schema(&self.runtime_parameters)?;
        if self
            .records
            .iter()
            .any(|record| record.receipt.program_recipe() != self.recipe)
        {
            return Err(failure("native record recipe differs from its directory"));
        }
        if self.recipe_parameters.iter().map(String::as_str).ne(self
            .recipe
            .recipe_parameters()
            .iter()
            .copied())
            || self
                .physics_parameters
                .iter()
                .any(|p| p.starts_with("fastsecdec::contour::"))
        {
            return Err(failure("invalid physics/recipe parameter schema"));
        }
        let mut classified = self
            .physics_parameters
            .iter()
            .chain(&self.recipe_parameters)
            .collect::<Vec<_>>();
        classified.sort();
        let mut actual = self.runtime_parameters.iter().collect::<Vec<_>>();
        actual.sort();
        if classified != actual || actual.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(failure(
                "runtime schema differs from physical/recipe partition",
            ));
        }
        layout::validate_records(
            &self.orders,
            &self.components,
            &self.runtime_parameters(),
            &self.records,
        )?;
        Ok(())
    }
    fn validate(&self, source: &str, integrity: bool) -> Result<(), KernelError> {
        self.validate_schema()?;
        if integrity && self.identity(source)? != self.content_id {
            return Err(failure("recipe content identity differs"));
        }
        Ok(())
    }
}

/// Metadata-only directory. Version-one archives are exposed as a single
/// legacy recipe with no invented original-source identity.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramArchiveCatalogue {
    pub version: u32,
    pub source_identity: Option<String>,
    pub content_id: String,
    pub records_end: u64,
    pub recipes: Vec<ProgramRecipeCatalogue>,
}
impl ProgramArchiveCatalogue {
    pub fn recipe(&self, recipe: ProgramRecipe) -> Result<&ProgramRecipeCatalogue, KernelError> {
        self.recipes
            .iter()
            .find(|entry| entry.recipe == recipe)
            .ok_or_else(|| {
                failure(format!(
                    "artifact lacks {} capability; regenerate with that recipe enabled",
                    recipe.name()
                ))
            })
    }
    pub(super) fn finish(
        source_identity: String,
        mut recipes: Vec<ProgramRecipeCatalogue>,
        records_end: u64,
    ) -> Result<Self, KernelError> {
        recipes.sort_by_key(|recipe| recipe.recipe);
        let mut catalogue = Self {
            version: if recipes.iter().any(|r| {
                r.threshold
                    .as_ref()
                    .is_some_and(|s| s.runtime_guard.is_some())
            }) {
                4
            } else if recipes.iter().any(|r| r.threshold.is_some()) {
                3
            } else {
                2
            },
            source_identity: Some(source_identity),
            content_id: String::new(),
            records_end,
            recipes,
        };
        catalogue.content_id = catalogue.identity()?;
        catalogue.validate(true)?;
        Ok(catalogue)
    }
    fn identity(&self) -> Result<String, KernelError> {
        let mut hash = blake3::Hasher::new();
        hash.update(b"fastsecdec-program-archive-v2\0");
        serde_json::to_writer(&mut hash, &self.source_identity)?;
        for recipe in &self.recipes {
            serde_json::to_writer(&mut hash, &(recipe.recipe, &recipe.content_id))?;
        }
        Ok(hash.finalize().to_hex().to_string())
    }
    pub fn validate(&self, integrity: bool) -> Result<(), KernelError> {
        if self.version == 1 {
            if self.source_identity.is_some() || self.recipes.len() != 1 {
                return Err(failure("invalid legacy recipe directory"));
            }
            let recipe = &self.recipes[0];
            if recipe.content_id != self.content_id
                || ProgramRecipe::legacy(&recipe.runtime_parameters()) != recipe.recipe
            {
                return Err(failure("legacy recipe identity differs"));
            }
            recipe.validate_schema()?;
            return self.legacy_catalogue()?.validate(integrity);
        }
        if !matches!(self.version, 2..=4) || self.recipes.is_empty() {
            return Err(failure("unsupported or empty program archive"));
        }
        if (self.version >= 3) != self.recipes.iter().any(|r| r.threshold.is_some())
            || self.version >= 3 && self.recipes.len() != 1
        {
            return Err(failure("threshold archive recipe capability"));
        }
        if (self.version == 4)
            != self.recipes.iter().any(|r| {
                r.threshold
                    .as_ref()
                    .is_some_and(|s| s.runtime_guard.is_some())
            })
        {
            return Err(failure("uniform parameter archive version"));
        }
        let source = self
            .source_identity
            .as_ref()
            .ok_or_else(|| failure("missing archive source identity"))?;
        super::super::super::validate_content_id(source)?;
        super::super::super::validate_content_id(&self.content_id)?;
        let mut unique = BTreeSet::new();
        let source_selection = self.recipes[0].source_selection();
        for recipe in &self.recipes {
            if !unique.insert(recipe.recipe) {
                return Err(failure("duplicate archive recipe"));
            }
            recipe.validate(source, integrity)?;
            if recipe.source_selection() != source_selection {
                return Err(failure(
                    "archive recipes have different original source selections",
                ));
            }
        }
        if self.recipes.windows(2).any(|w| w[0].recipe >= w[1].recipe) {
            return Err(failure("noncanonical recipe ordering"));
        }
        layout::validate_ranges(
            self.recipes.iter().flat_map(|r| &r.records),
            super::MAGIC.len() as u64,
            self.records_end,
        )?;
        if integrity && self.identity()? != self.content_id {
            return Err(failure("archive content identity differs"));
        }
        Ok(())
    }
    /// Adapt and validate an existing v1 catalogue without changing its byte
    /// ranges, record identities or mathematical recipe. Structural checks are
    /// mandatory; `integrity` additionally verifies the catalogue identity.
    pub fn from_legacy(catalogue: KernelCatalogue, integrity: bool) -> Result<Self, KernelError> {
        let recipe = ProgramRecipe::legacy(&catalogue.runtime_parameters);
        let recipe_parameters = recipe
            .recipe_parameters()
            .iter()
            .map(|s| (*s).to_owned())
            .collect::<Vec<_>>();
        let physics_parameters = catalogue
            .runtime_parameters
            .iter()
            .filter(|p| !recipe_parameters.contains(p))
            .cloned()
            .collect();
        let directory = Self {
            version: 1,
            source_identity: None,
            content_id: catalogue.content_id.clone(),
            records_end: catalogue.records_end,
            recipes: vec![ProgramRecipeCatalogue {
                recipe,
                threshold: None,
                content_id: catalogue.content_id,
                orders: catalogue.orders,
                components: catalogue.components,
                runtime_parameters: catalogue.runtime_parameters,
                physics_parameters,
                recipe_parameters,
                records: catalogue.records,
            }],
        };
        directory.validate(integrity)?;
        Ok(directory)
    }
    pub(super) fn legacy_catalogue(&self) -> Result<KernelCatalogue, KernelError> {
        let recipe = self
            .recipes
            .first()
            .ok_or_else(|| failure("empty legacy recipe directory"))?;
        Ok(KernelCatalogue {
            version: 1,
            content_id: self.content_id.clone(),
            orders: recipe.orders.clone(),
            components: recipe.components.clone(),
            runtime_parameters: recipe.runtime_parameters(),
            records: recipe.records.clone(),
            records_end: self.records_end,
        })
    }
}

#[cfg(test)]
mod recipe_wire_tests {
    use super::*;
    #[test]
    fn existing_recipe_ordinals_and_names_are_unchanged() {
        let recipes = [
            ProgramRecipe::UndeformedV1,
            ProgramRecipe::FixedV1,
            ProgramRecipe::DynamicPolynomialV1,
            ProgramRecipe::DynamicSignAwareV1,
            ProgramRecipe::ThresholdV1,
        ];
        let names = [
            "undeformed-v1",
            "fixed-v1",
            "dynamic-polynomial-v1",
            "dynamic-sign-aware-v1",
            "threshold-v1",
        ];
        for (index, (recipe, name)) in recipes.into_iter().zip(names).enumerate() {
            let bytes = bincode::serde::encode_to_vec(recipe, bincode::config::standard()).unwrap();
            assert_eq!(bytes, vec![index as u8]);
            let (restored, used): (ProgramRecipe, usize) =
                bincode::serde::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
            assert_eq!(restored, recipe);
            assert_eq!(used, bytes.len());
            assert_eq!(serde_json::to_value(recipe).unwrap(), name);
        }
    }
}
