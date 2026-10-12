//! Presentation of the native threshold lineage, distinct from legacy charts.
//! Saved descriptors identify proof records; loading them is not proof replay.
use crate::{
    CliResult,
    artifact::Artifact,
    generation_report::{facts_table, heading},
    terminal_policy::ColorPolicy,
};
use fastsecdec::kernel::{
    KernelSet,
    threshold_metadata::{ResidentSelection, SourceExtent},
};
use tabled::settings::Color;

pub(super) fn native_document(kernels: &KernelSet) -> Option<serde_json::Value> {
    kernels.threshold_metadata().map(|m| serde_json::json!({
        "lineage": m.lineage(),
        "resident": m.resident(),
        "result_scope": m.result_scope(),
        "global_proof_replayed": false,
        "proof_note": "Saved maps and certificate identities are retained; global proof replay requires external generation evidence.",
    }))
}

fn heading_line(width: usize, colors: ColorPolicy) -> String {
    format!(
        "\n{}",
        heading("Threshold lineage", width, colors, Color::FG_GREEN)
    )
}

pub(super) fn catalogue_facts(
    artifact: &Artifact,
    selected: Option<usize>,
    width: usize,
    colors: ColorPolicy,
) -> CliResult<String> {
    let Some(catalogue) = artifact.catalogue() else {
        return Ok(String::new());
    };
    let Some(summary) = catalogue.threshold() else {
        return Ok(String::new());
    };
    let mut facts = vec![
        [
            "Source extent".into(),
            match &summary.source_extent {
                SourceExtent::Full { .. } => "Full input",
                SourceExtent::Selected { .. } => "Selected sources",
                SourceExtent::SelectedOriginalCellsV1 { .. } => "Selected original cells (partial)",
            }
            .into(),
        ],
        [
            "Contributions".into(),
            summary.contributions.len().to_string(),
        ],
        [
            "Proofs".into(),
            "Certificate identities; global proof not replayed".into(),
        ],
    ];
    if let Some(id) = selected
        && let Some(receipt) = &catalogue.sector(id)?.receipt.threshold
    {
        let ids = receipt
            .lineage
            .as_ref()
            .map(|r| {
                r.contributions
                    .iter()
                    .map(|c| c.0.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .unwrap_or_else(|| "Setup carrier".into());
        facts.push(["Record contributions".into(), ids]);
    }
    let mut out = heading_line(width, colors);
    out.push_str(&facts_table(facts, width, colors));
    out.push_str(&heading(
        "Metadata only; --deep restores the selected record's native maps and lineage.",
        width,
        colors,
        Color::FG_BRIGHT_BLACK,
    ));
    Ok(out)
}

pub(super) fn native_facts(kernels: &KernelSet, width: usize, colors: ColorPolicy) -> String {
    let Some(metadata) = kernels.threshold_metadata() else {
        return String::new();
    };
    let lineage = metadata.lineage();
    let mut facts = vec![
        [
            "Parent projective patches".into(),
            lineage.patches.len().to_string(),
        ],
        [
            "Parent threshold cells".into(),
            lineage.cells.len().to_string(),
        ],
        [
            "Parent endpoint charts".into(),
            lineage.endpoint_charts.len().to_string(),
        ],
        [
            "Parent continuation groups".into(),
            lineage.continuation_groups.len().to_string(),
        ],
        [
            "Proofs".into(),
            "Certificate identities; global proof not replayed".into(),
        ],
    ];
    if let ResidentSelection::Selected {
        stochastic_contributions,
        ..
    } = &metadata.resident().selection
    {
        facts.push([
            "Resident stochastic contributions".into(),
            stochastic_contributions
                .iter()
                .map(|c| c.0.to_string())
                .collect::<Vec<_>>()
                .join(", "),
        ]);
    }
    let mut out = heading_line(width, colors);
    out.push_str(&facts_table(facts, width, colors));
    out.push_str(&heading(
        "Use --json for saved map descriptors, lineage and certificate identities.",
        width,
        colors,
        Color::FG_BRIGHT_BLACK,
    ));
    out
}
