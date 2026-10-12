//! Validate issued native receipt identity without decoding algebra in the coordinator.
use super::*;
use fastsecdec::generation::streaming as native;
use std::{collections::BTreeSet, io::Read};

pub(in super::super) fn validate(
    request: &Request,
    response: &Response,
    root: &Path,
) -> CliResult<()> {
    match (request, response) {
        (
            Request::PrepareThreshold {
                input,
                overrides,
                attempt,
                prior,
                ..
            }
            | Request::RetryThreshold {
                input,
                overrides,
                attempt,
                prior,
                ..
            },
            Response::PreparedThreshold(prepared),
        ) => {
            let mut card: crate::config::RunCard = toml::from_str(&fs::read_to_string(input)?)?;
            overrides.apply(&mut card);
            if !card.generation.threshold_enabled()
                || prepared.attempt != *attempt
                || prepared.generation.mode
                    != Some(fastsecdec::generation::GenerationMode::Symbolic)
            {
                return Err("threshold preparation differs from issued capability/attempt".into());
            }
            if matches!(request, Request::RetryThreshold { .. }) != prepared.retry.is_some() {
                return Err("threshold retry response capability mismatch".into());
            }
            validate_provenance(input, &prepared.provenance)?;
            if let Request::RetryThreshold { retry, .. } = request {
                let lineage = retry.bind(
                    &super::super::jobs::threshold::rooted(root, &prepared.directory)?,
                    prepared
                        .retry
                        .as_ref()
                        .ok_or("missing retry response lineage")?,
                )?;
                if serde_json::to_value(Some(&lineage))?
                    != serde_json::to_value(&prepared.generation.endpoint_retry)?
                {
                    return Err("retry generation history differs from native lineage".into());
                }
            } else if prepared.generation.endpoint_retry.is_some() {
                return Err("ordinary preparation claimed retry history".into());
            }
            super::super::jobs::threshold::validate_prepared(root, prepared)?;
            if let Some(prior) = prior
                && (prepared.native.publication.source_identity
                    != prior.native.publication.source_identity
                    || prepared.native.publication.prepared_identity
                        != prior.native.publication.prepared_identity)
            {
                return Err("threshold resume mathematical identity changed".into());
            }
        }
        (
            Request::CompileThreshold {
                directory,
                work,
                output,
            },
            Response::CompiledThreshold(compiled),
        ) => {
            if compiled.directory != *directory
                || compiled.data != *output
                || compiled.receipt.work() != work
            {
                return Err("threshold compile response differs from issued work".into());
            }
            super::super::jobs::threshold::validate_compiled(compiled)?;
        }

        (
            Request::Prepare {
                input, overrides, ..
            },
            Response::Prepared(prepared),
        ) => {
            let mut card: crate::config::RunCard = toml::from_str(&fs::read_to_string(input)?)?;
            overrides.apply(&mut card);
            if prepared.native.program_recipe != card.generation.program_recipe() {
                return Err("prepared program recipe differs from its issued job".into());
            }
            validate_provenance(input, &prepared.provenance)?;
            validate_preparation(&prepared.native, root)?;
        }
        (Request::PreparePrograms { input, recipes, .. }, Response::PreparedPrograms(prepared)) => {
            let requested = recipes.iter().copied().collect::<BTreeSet<_>>();
            let returned = prepared
                .native
                .recipes
                .iter()
                .map(|recipe| recipe.program_recipe)
                .collect::<BTreeSet<_>>();
            if requested.is_empty()
                || requested.len() != recipes.len()
                || returned.len() != prepared.native.recipes.len()
                || requested != returned
            {
                return Err("prepared recipe set differs from its issued job".into());
            }
            validate_provenance(input, &prepared.provenance)?;
            let first = &prepared.native.recipes[0];
            for recipe in &prepared.native.recipes {
                if recipe.source_identity != prepared.native.source_identity
                    || recipe.dimension != first.dimension
                    || recipe.mode != first.mode
                    || recipe.charts.len() != first.charts.len()
                    || recipe
                        .charts
                        .iter()
                        .zip(&first.charts)
                        .any(|(a, b)| a.index != b.index || a.map != b.map)
                {
                    return Err(
                        "recipe contexts do not share their issued physical geometry".into(),
                    );
                }
                validate_preparation(recipe, root)?;
            }
        }
        (
            Request::PrepareChartSource {
                source_identity,
                dimension,
                map,
                ..
            },
            Response::ChartSource(source),
        ) => {
            if source.index != map.index
                || source.map != map.map
                || source.source_identity != *source_identity
                || source.dimension != *dimension
            {
                return Err("chart source completion differs from its issued physical map".into());
            }
            source.map.verify(root)?;
            source.record.verify(root)?;
        }
        (
            Request::Discover {
                program_recipe,
                source_id,
                dimension,
                index,
                ..
            },
            Response::Discovered(chart),
        ) if *index == chart.index => {
            if chart.program_recipe != *program_recipe
                || chart.source_id != *source_id
                || chart.dimension != *dimension
            {
                return Err("discovery completion belongs to a different source or recipe".into());
            }
            chart.record.verify(root)?;
        }
        (
            Request::DiscoverPrepared {
                program_recipe,
                source_id,
                source,
                ..
            },
            Response::Discovered(chart),
        ) => {
            if chart.index != source.index
                || chart.dimension != source.dimension
                || chart.program_recipe != *program_recipe
                || chart.source_id != *source_id
            {
                return Err(
                    "prepared discovery completion differs from its issued source or recipe".into(),
                );
            }
            source.record.verify(root)?;
            chart.record.verify(root)?;
        }
        (
            Request::Symmetry {
                chart, candidates, ..
            },
            Response::Symmetry(assignment),
        ) if assignment.source == chart.index
            && assignment.program_recipe == chart.program_recipe
            && assignment.source_id == chart.source_id =>
        {
            if assignment.representative != chart.index
                && !candidates.iter().any(|c| {
                    c.index == assignment.representative
                        && c.program_recipe == chart.program_recipe
                        && c.source_id == chart.source_id
                })
            {
                return Err("symmetry completion contains an unissued candidate".into());
            }
            if assignment.permutation.len() != chart.dimension
                || assignment
                    .permutation
                    .iter()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    != (0..chart.dimension).collect()
            {
                return Err("symmetry completion has an invalid permutation".into());
            }
        }
        (Request::Formula { chart, .. }, Response::Formula(formula))
            if chart.formula_key.as_ref() == Some(&formula.key)
                && chart.program_recipe == formula.program_recipe
                && chart.source_id == formula.source_id =>
        {
            formula.record.verify(root)?;
        }
        (Request::Sector { job, output, .. }, Response::Compiled(compiled))
            if compiled.source_index == job.index
                && &compiled.data == output
                && compiled.program_recipe == job.program_recipe
                && compiled.source_id == job.source.blake3 =>
        {
            if compiled.receipts.is_empty()
                || compiled
                    .receipts
                    .iter()
                    .any(|receipt| receipt.program_recipe() != job.program_recipe)
            {
                return Err("compiled sector omits or changes its issued program recipe".into());
            }
            let expected = job
                .charts
                .iter()
                .map(|entry| entry.chart.index)
                .collect::<std::collections::BTreeSet<_>>();
            let mut returned = std::collections::BTreeSet::new();
            for index in compiled
                .receipts
                .iter()
                .flat_map(|receipt| &receipt.source_indices)
            {
                if !returned.insert(*index) {
                    return Err("compiled sector repeats a source chart".into());
                }
            }
            if expected.len() != job.charts.len()
                || returned != expected
                || compiled
                    .source_chart_modes
                    .keys()
                    .copied()
                    .collect::<std::collections::BTreeSet<_>>()
                    != expected
            {
                return Err("compiled sector source charts differ from its issued job".into());
            }
            let mut file = File::open(&compiled.data)?;
            let mut buffer = [0u8; 64 * 1024];
            for receipt in &compiled.receipts {
                let mut remaining = receipt.length;
                let mut hash = blake3::Hasher::new();
                while remaining > 0 {
                    let n = remaining.min(buffer.len() as u64) as usize;
                    file.read_exact(&mut buffer[..n])?;
                    hash.update(&buffer[..n]);
                    remaining -= n as u64;
                }
                if hash.finalize().to_hex().as_str() != receipt.digest {
                    return Err("compiled sector digest mismatch".into());
                }
            }
            if file.read(&mut buffer[..1])? != 0 {
                return Err("compiled sector has trailing unclaimed data".into());
            }
        }
        _ => return Err("generation completion does not match its issued job".into()),
    }
    Ok(())
}

pub(in super::super) fn validate_provenance(
    input: &Path,
    provenance: &crate::artifact::Provenance,
) -> CliResult<()> {
    if provenance.dependencies != artifact::dependencies() {
        return Err("prepared dependencies changed".into());
    }
    let base = input.parent().unwrap_or_else(|| Path::new("."));
    for source in &provenance.sources {
        if source
            .fingerprint
            .hash(&fs::read(base.join(&source.path))?)?
            != source.blake3
        {
            return Err(
                format!("generation input {} changed since preparation", source.path).into(),
            );
        }
    }
    Ok(())
}

fn validate_preparation(prepared: &native::Preparation, root: &Path) -> CliResult<()> {
    if let Some(scope) = &prepared.source_scope {
        scope.validate(prepared.charts.len())?;
        if scope.chart_source_sectors() != scope.selection().source_sectors() {
            return Err("prepared source selection is incomplete".into());
        }
    }
    if prepared.source_identity.len() != 64
        || !prepared
            .source_identity
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        || prepared
            .charts
            .iter()
            .enumerate()
            .any(|(index, map)| map.index != index)
    {
        return Err("prepared physical source or map directory is invalid".into());
    }
    prepared.source.verify(root)?;
    for map in &prepared.charts {
        map.map.verify(root)?;
    }
    Ok(())
}
