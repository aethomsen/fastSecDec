//! Compact recipe selection. Native owners validate layouts and decode records;
//! this module only connects their directories to the CLI manifest.
use super::*;
use fastsecdec::{
    kernel::indexed::{
        KernelCatalogue, ProgramArchiveCatalogue, ProgramArchiveReader, ProgramRecipe,
        ProgramRecipeCatalogue, RecordDescriptor,
    },
    status::CoefficientComponent,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramStorage {
    pub data_file: String,
    pub default_recipe: ProgramRecipe,
    pub catalogue: ProgramArchiveCatalogue,
    /// Per-recipe observations, excluded from mathematical identities.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub generation: BTreeMap<ProgramRecipe, ProgramGeneration>,
    /// Bounded optional human previews, never recipe identity or native proof.
    #[serde(
        default,
        skip_serializing_if = "BTreeMap::is_empty",
        deserialize_with = "super::inspection::deserialize_indices"
    )]
    pub inspection: BTreeMap<ProgramRecipe, InspectionIndex>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramGeneration {
    pub generation: GenerationRecord,
    /// Wall observations for this recipe. Shared family stages live on the artifact.
    pub timings: GenerationTimings,
}
impl ProgramStorage {
    pub(super) fn validate(&self, integrity: bool) -> CliResult<()> {
        validate_filename(&self.data_file)?;
        if !matches!(self.catalogue.version, 2..=4) {
            return Err("recipe manifest requires a supported native program directory".into());
        }
        self.catalogue.validate(integrity)?;
        self.catalogue.recipe(self.default_recipe)?;
        for recipe in self.generation.keys().chain(self.inspection.keys()) {
            self.catalogue.recipe(*recipe)?;
        }
        Ok(())
    }
}

pub(super) fn validate_filename(name: &str) -> CliResult<()> {
    let path = Path::new(name);
    if name.is_empty()
        || path.components().count() != 1
        || !matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err("indexed data filename must be a single relative basename".into());
    }
    Ok(())
}

/// Borrowed logical layout, with no invented physical ranges or v1 catalogue.
/// Physical range validation belongs to the complete native archive directory.
#[derive(Clone, Copy)]
pub struct CatalogueView<'a> {
    pub content_id: &'a String,
    pub orders: &'a Vec<i32>,
    pub components: &'a Vec<CoefficientComponent>,
    pub runtime_parameters: &'a Vec<String>,
    pub records: &'a Vec<RecordDescriptor>,
    program: Option<&'a ProgramRecipeCatalogue>,
}
impl<'a> CatalogueView<'a> {
    pub fn threshold(self) -> Option<&'a fastsecdec::kernel::indexed::ThresholdArchiveSummary> {
        self.program.and_then(|p| p.threshold.as_ref())
    }
    pub fn threshold_scope(&self) -> CliResult<Option<fastsecdec::kernel::ThresholdResultScope>> {
        self.program.map_or(Ok(None), |program| {
            program.threshold_scope().map_err(Into::into)
        })
    }
    pub fn source_selection(&self) -> Option<&fastsecdec::generation::SourceSectorSelection> {
        self.records
            .first()
            .and_then(|record| record.receipt.source_scope.as_ref())
            .map(|scope| scope.selection())
    }
    pub fn sector_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.sector.is_some())
            .count()
    }
    pub fn sector(&self, id: usize) -> CliResult<&'a RecordDescriptor> {
        self.records
            .iter()
            .find(|record| record.sector == Some(id))
            .ok_or_else(|| format!("unknown sector {id} in selected recipe").into())
    }
}
impl<'a> From<&'a KernelCatalogue> for CatalogueView<'a> {
    fn from(value: &'a KernelCatalogue) -> Self {
        Self {
            content_id: &value.content_id,
            orders: &value.orders,
            components: &value.components,
            runtime_parameters: &value.runtime_parameters,
            records: &value.records,
            program: None,
        }
    }
}
impl<'a> From<&'a ProgramRecipeCatalogue> for CatalogueView<'a> {
    fn from(value: &'a ProgramRecipeCatalogue) -> Self {
        Self {
            content_id: &value.content_id,
            orders: &value.orders,
            components: &value.components,
            runtime_parameters: &value.runtime_parameters,
            records: &value.records,
            program: Some(value),
        }
    }
}

impl Artifact {
    pub fn is_indexed(&self) -> bool {
        self.indexed.is_some() || self.programs.is_some()
    }

    pub(super) fn indexed_data_file(&self) -> CliResult<Option<&str>> {
        match (&self.indexed, &self.programs) {
            (Some(indexed), None) => {
                indexed.validate(false)?;
                Ok(Some(&indexed.data_file))
            }
            (None, Some(programs)) => {
                programs.validate(false)?;
                Ok(Some(&programs.data_file))
            }
            (None, None) => Ok(None),
            _ => Err("artifact contains conflicting native directories".into()),
        }
    }

    pub fn selected_recipe(&self) -> Option<ProgramRecipe> {
        if let Some(programs) = &self.programs {
            return Some(self.selected_recipe.unwrap_or(programs.default_recipe));
        }
        // Compatibility classification lives in the native archive owner.
        self.indexed
            .as_ref()
            .map(|indexed| indexed.catalogue.program_recipe())
    }

    pub fn select_recipe(&mut self, recipe: ProgramRecipe) -> CliResult<()> {
        if let Some(programs) = &self.programs {
            programs.catalogue.recipe(recipe)?;
            self.selected_recipe = Some(recipe);
            self.generation = programs
                .generation
                .get(&recipe)
                .map(|observation| observation.generation.clone());
        } else if self.selected_recipe() != Some(recipe) {
            return Err(format!(
                "artifact lacks {} capability; regenerate with that recipe enabled",
                recipe.name()
            )
            .into());
        }
        Ok(())
    }

    pub fn catalogue(&self) -> Option<CatalogueView<'_>> {
        if let Some(programs) = &self.programs {
            return programs
                .catalogue
                .recipe(self.selected_recipe()?)
                .ok()
                .map(Into::into);
        }
        self.indexed
            .as_ref()
            .map(|indexed| (&indexed.catalogue).into())
    }

    /// Open and compare only native directory metadata. Sector bytes remain on
    /// disk until the caller explicitly selects and loads a record.
    pub fn open_program_archive(
        &self,
        base: &Path,
        options: KernelLoadOptions,
    ) -> CliResult<ProgramArchiveReader<File>> {
        let reader =
            ProgramArchiveReader::from_reader(File::open(self.data_path(base)?)?, options)?;
        if reader.catalogue().content_id != self.kernel_content_id {
            return Err("native archive identity differs from its manifest".into());
        }
        let expected = if let Some(programs) = &self.programs {
            programs.catalogue.clone()
        } else if let Some(indexed) = &self.indexed {
            ProgramArchiveCatalogue::from_legacy(indexed.catalogue.clone(), false)?
        } else {
            return Err("artifact has no indexed native directory".into());
        };
        // Directory consistency is essential even when optional payload digests
        // are disabled: the coordinator and child must use the same layout.
        if serde_json::to_value(reader.catalogue())? != serde_json::to_value(expected)? {
            return Err("native archive directory differs from its manifest".into());
        }
        Ok(reader)
    }

    /// Attach bounded observations for each recipe without changing identities.
    pub fn set_program_generation(
        &mut self,
        generation: BTreeMap<ProgramRecipe, ProgramGeneration>,
    ) -> CliResult<()> {
        let programs = self
            .programs
            .as_mut()
            .ok_or("artifact has no recipe directory")?;
        if generation
            .keys()
            .copied()
            .ne(programs.catalogue.recipes.iter().map(|r| r.recipe))
        {
            return Err("generation observations do not cover the complete recipe family".into());
        }
        self.generation = Some(generation[&programs.default_recipe].generation.clone());
        programs.generation = generation;
        Ok(())
    }

    /// Publish a completed recipe archive through the same immutable-file and
    /// atomic-manifest transaction retained from historical v1 generation.
    pub fn from_program_archive(
        staged: &Path,
        catalogue: ProgramArchiveCatalogue,
        default_recipe: ProgramRecipe,
        provenance: Provenance,
    ) -> CliResult<Self> {
        if !matches!(catalogue.version, 2..=4) {
            return Err("new recipe artifacts require a supported native program archive".into());
        }
        let reader = ProgramArchiveReader::from_reader(
            File::open(staged)?,
            KernelLoadOptions { validate: true },
        )?;
        if serde_json::to_value(reader.catalogue())? != serde_json::to_value(&catalogue)? {
            return Err("staged recipe directory differs from completed receipts".into());
        }
        let summary = summary(catalogue.recipe(default_recipe)?.into());
        let mut artifact = Self {
            format_version: 5,
            content_id: String::new(),
            kernel_content_id: catalogue.content_id.clone(),
            provenance,
            kernel: RawValue::from_string(serde_json::to_string(&summary)?)?,
            data: Vec::new(),
            indexed: None,
            programs: Some(ProgramStorage {
                data_file: String::new(),
                default_recipe,
                catalogue,
                generation: BTreeMap::new(),
                inspection: BTreeMap::new(),
            }),
            selected_recipe: None,
            staged_data: Some(staged.into()),
            source_root: PathBuf::from("."),
            generation_timings: None,
            generation: None,
            reference: None,
            loading_seconds: 0.,
            validation: ArtifactValidation::default(),
            inspection: None,
        };
        artifact.content_id = artifact.identity()?;
        Ok(artifact)
    }
}

pub(super) fn summary(catalogue: CatalogueView<'_>) -> KernelSummary {
    let sectors: Vec<_> = catalogue
        .records
        .iter()
        .filter(|r| r.sector.is_some())
        .collect();
    KernelSummary {
        source_selection: catalogue.source_selection().cloned(),
        sectors: sectors.len(),
        orders: catalogue.orders.clone(),
        components: catalogue.components.clone(),
        dimensions: Some(
            sectors
                .iter()
                .map(|r| r.receipt.dimension.expect("validated sector dimension"))
                .collect(),
        ),
        evaluator_statistics: Some(
            sectors
                .iter()
                .map(|r| {
                    r.receipt
                        .statistics
                        .clone()
                        .expect("validated sector statistics")
                })
                .collect(),
        ),
        runtime_parameters: Some(catalogue.runtime_parameters.clone()),
    }
}
