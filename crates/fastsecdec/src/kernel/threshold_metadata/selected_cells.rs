//! Compact original-cell scope checks; these never recreate a proof owner.
use super::{validate::invalid, validate::ordered, *};

impl OriginalCellSelectionV1 {
    pub fn validate(&self) -> Result<()> {
        for digest in [
            &self.source,
            &self.original_request_record,
            &self.original_raw_evidence,
            &self.quotient_request_record,
            &self.quotient_raw_evidence,
            &self.quotient,
            &self.coverage,
        ] {
            digest.validate()?;
        }
        ordered(&self.cells, "selected original cells")?;
        if self.original_cell_count == 0
            || self.cells.is_empty()
            || self
                .cells
                .last()
                .is_some_and(|i| *i >= self.original_cell_count)
        {
            return Err(invalid("empty or out-of-range original-cell selection"));
        }
        Ok(())
    }

    pub(super) fn validate_manifest(&self, manifest: &LineageManifestV1) -> Result<()> {
        self.validate()?;
        let [patch] = manifest.patches.as_slice() else {
            return Err(invalid("original-cell selection requires one source patch"));
        };
        if manifest.preparation.strategy != ResolvedStrategy::GcadFirst
            || self.source != manifest.preparation.source_identity
            || self.coverage != manifest.preparation.partition_certificate
            || patch.source != SourceOrigin::OriginalInput
            || patch.coverage != self.coverage
            || patch.map.geometry().certificate != self.quotient
        {
            return Err(invalid(
                "original-cell selection source/quotient association",
            ));
        }
        if manifest.cells.len() != self.cells.len()
            || manifest.cells.iter().any(|cell| {
                cell.patch != patch.id
                    || cell.native.request_record != self.quotient_request_record
                    || cell.native.raw_evidence != self.quotient_raw_evidence
                    || self
                        .cells
                        .binary_search(&cell.native.raw_cell_index)
                        .is_err()
            })
        {
            return Err(invalid(
                "original-cell selection differs from native cell inventory",
            ));
        }
        // The caller has already rejected duplicate native cells. Together
        // with equal lengths and membership this establishes exact inventory.
        Ok(())
    }
}
