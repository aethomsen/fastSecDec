use super::failure;
use crate::{
    kernel::{EvaluatorStatistics, KernelError},
    status::CoefficientComponent,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Small receipt returned by a completed worker. It contains no native Atoms,
/// evaluator instructions, expression text or executable owner.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordReceipt {
    /// Original-geometry subset and this record's local chart lineage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_scope: Option<crate::generation::GenerationSourceScope>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub threshold: Option<super::ThresholdRecordReceipt>,
    pub version: u32,
    /// Version-two receipts identify the native v10 recipe explicitly. Legacy
    /// receipts keep their original JSON shape and schema classification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipe: Option<super::ProgramRecipe>,
    pub length: u64,
    pub digest: String,
    pub native_content_id: String,
    pub orders: Vec<i32>,
    pub components: Vec<CoefficientComponent>,
    pub runtime_parameters: Vec<String>,
    pub dimension: Option<usize>,
    pub statistics: Option<EvaluatorStatistics>,
    /// Local chart ordinal -> compact chart ordinal in this generated dataset.
    /// A partial generation's original geometry IDs live in `source_scope`.
    pub source_indices: Vec<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecordDescriptor {
    pub offset: u64,
    pub receipt: RecordReceipt,
    /// Local numerical output -> the complete catalogue vector.
    pub output_indices: Vec<usize>,
    pub sector: Option<usize>,
}

/// Complete immutable index. Local record layouts remain unchanged when later
/// records extend the Laurent range or introduce imaginary components.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelCatalogue {
    pub version: u32,
    pub content_id: String,
    pub orders: Vec<i32>,
    pub components: Vec<CoefficientComponent>,
    pub runtime_parameters: Vec<String>,
    pub records: Vec<RecordDescriptor>,
    /// Bytes before the compact footer. Record ranges never include the footer.
    pub records_end: u64,
}

impl RecordReceipt {
    pub fn program_recipe(&self) -> super::ProgramRecipe {
        self.recipe
            .unwrap_or_else(|| super::ProgramRecipe::legacy(&self.runtime_parameters))
    }
    pub(crate) fn validate(&self) -> Result<(), KernelError> {
        if let Some(threshold) = &self.threshold {
            threshold.validate(self)?;
        }
        if self.recipe == Some(super::programs::ProgramRecipe::ThresholdV1)
            && self.threshold.is_none()
        {
            return Err(failure(
                "legacy source receipt cannot store threshold lineage",
            ));
        }
        if let Some(scope) = &self.source_scope {
            scope.validate(self.source_indices.len()).map_err(failure)?;
        }
        if !(matches!((self.version, self.recipe), (1, None) | (2, Some(_)))
            && self.threshold.is_none()
            || matches!(self.version, 3 | 4) && self.threshold.is_some())
            || self.length == 0
        {
            return Err(failure("unsupported or empty worker record"));
        }
        if let Some(recipe) = self.recipe {
            recipe.validate_runtime_schema(&self.runtime_parameters)?;
        }
        super::super::validate_content_id(&self.digest)?;
        super::super::validate_content_id(&self.native_content_id)?;
        validate_layout(&self.orders, &self.components)?;
        if self.dimension.is_some() != self.statistics.is_some()
            || self.dimension == Some(0)
            || self
                .runtime_parameters
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != self.runtime_parameters.len()
            || self
                .source_indices
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
        {
            return Err(failure("invalid worker record descriptor"));
        }
        Ok(())
    }
}

pub(super) fn validate_layout(
    orders: &[i32],
    components: &[CoefficientComponent],
) -> Result<(), KernelError> {
    use CoefficientComponent::{Imag, Real};
    if orders.is_empty() || orders.len() != components.len() {
        return Err(failure("empty or inconsistent coefficient layout"));
    }
    let complex = components.contains(&Imag);
    let width = if complex { 2 } else { 1 };
    if !orders.len().is_multiple_of(width) {
        return Err(failure("inconsistent complex coefficient layout"));
    }
    let mut previous = None;
    for (orders, components) in orders.chunks(width).zip(components.chunks(width)) {
        if components[0] != Real
            || (complex && (components[1] != Imag || orders[1] != orders[0]))
            || previous.is_some_and(|value: i32| value.checked_add(1) != Some(orders[0]))
        {
            return Err(failure("invalid Laurent/component ordering"));
        }
        previous = Some(orders[0]);
    }
    Ok(())
}

impl KernelCatalogue {
    pub fn source_selection(&self) -> Option<&crate::generation::SourceSectorSelection> {
        self.records
            .first()
            .and_then(|record| record.receipt.source_scope.as_ref())
            .map(|scope| scope.selection())
    }
    /// Classify the legacy v1 runtime schema without copying its records.
    /// V2 archives use their explicit recipe directories instead.
    pub fn program_recipe(&self) -> super::ProgramRecipe {
        super::ProgramRecipe::legacy(&self.runtime_parameters)
    }

    pub fn sector_count(&self) -> usize {
        self.records
            .iter()
            .filter(|record| record.sector.is_some())
            .count()
    }
    pub fn sector(&self, sector: usize) -> Result<&RecordDescriptor, KernelError> {
        self.records
            .iter()
            .find(|record| record.sector == Some(sector))
            .ok_or_else(|| failure(format!("unknown sector {sector}")))
    }
    pub(crate) fn finish(
        records: Vec<RecordDescriptor>,
        records_end: u64,
    ) -> Result<Self, KernelError> {
        if records.iter().any(|r| r.receipt.threshold.is_some()) {
            return Err(failure(
                "legacy indexed catalogue cannot own threshold records",
            ));
        }
        let super::layout::RecordLayout {
            orders,
            components,
            runtime_parameters,
            records,
        } = super::layout::finish_layout(records)?;
        let mut catalogue = Self {
            version: 1,
            content_id: String::new(),
            orders,
            components,
            runtime_parameters,
            records,
            records_end,
        };
        catalogue.content_id = catalogue.identity()?;
        catalogue.validate(true)?;
        Ok(catalogue)
    }

    fn identity(&self) -> Result<String, KernelError> {
        // Byte offsets, context-dependent transport digests and completion order
        // are intentionally absent: native semantic IDs already bind each record.
        let mut hash = blake3::Hasher::new();
        hash.update(b"fastsecdec-indexed-semantic-v1\0");
        serde_json::to_writer(
            &mut hash,
            &(&self.orders, &self.components, &self.runtime_parameters),
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

    /// Cheap schema/range checks are mandatory; semantic digest recomputation is
    /// controlled by the caller's existing trusted-cache validation policy.
    pub fn validate(&self, integrity: bool) -> Result<(), KernelError> {
        if self.records.iter().any(|r| r.receipt.threshold.is_some()) {
            return Err(failure(
                "threshold records need the new program archive capability",
            ));
        }
        if self.version != 1 || self.records.is_empty() {
            return Err(failure("unsupported or empty catalogue"));
        }
        super::super::validate_content_id(&self.content_id)?;
        if self
            .records
            .iter()
            .any(|record| record.receipt.program_recipe() != self.program_recipe())
        {
            return Err(failure(
                "legacy catalogue cannot contain a different explicit native recipe",
            ));
        }
        super::layout::validate_records(
            &self.orders,
            &self.components,
            &self.runtime_parameters,
            &self.records,
        )?;
        super::layout::validate_ranges(
            self.records.iter(),
            super::MAGIC.len() as u64,
            self.records_end,
        )?;
        if integrity && self.identity()? != self.content_id {
            return Err(failure("catalogue content identity differs"));
        }
        Ok(())
    }
}
