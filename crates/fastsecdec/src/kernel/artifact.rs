//! Strict versioned envelopes around native symbolic/evaluator serialization.
//! Symbolica owns decoding of native programs from trusted cache producers.
mod binary;
mod cached;
pub(crate) mod indexed;
#[cfg(test)]
mod load_tests;
mod native;
mod sector_identity;

use super::{CompilationSettings, KernelError, KernelLoadOptions, KernelSet, PrecisionPolicy};
use symbolica::atom::{Atom, AtomCore, AtomView, Symbol};

pub(super) fn atom(expression: String) -> Result<Atom, KernelError> {
    // Recover native callbacks before expressions are parsed in a cold host.
    let _ = symbolica::transcendental::gamma();
    crate::contour::functions::register();
    Atom::parse(expression, "fastsecdec::artifact", Default::default())
        .map_err(KernelError::Artifact)
}

fn parameters(values: Vec<String>) -> Result<Vec<Symbol>, KernelError> {
    let parameters = values
        .into_iter()
        .map(|value| {
            let value = atom(value)?;
            let AtomView::Var(variable) = value.as_view() else {
                return Err(KernelError::Artifact(
                    "sector parameter is not a symbol".into(),
                ));
            };
            Ok(variable.get_symbol())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parameters
        .iter()
        .collect::<std::collections::BTreeSet<_>>()
        .len()
        != parameters.len()
    {
        return Err(KernelError::Artifact("duplicate sector parameters".into()));
    }
    Ok(parameters)
}

fn validate_orders(orders: &[i32], exact_len: usize) -> Result<(), KernelError> {
    if orders.is_empty()
        || orders
            .windows(2)
            .any(|pair| pair[0].checked_add(1) != Some(pair[1]))
        || exact_len != orders.len()
    {
        return Err(KernelError::Artifact(
            "invalid Laurent output layout".into(),
        ));
    }
    Ok(())
}

fn validate_content_id(value: &str) -> Result<(), KernelError> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(KernelError::Artifact(
            "invalid content identity format".into(),
        ));
    }
    Ok(())
}

fn parameter_names(parameters: &[Symbol]) -> Vec<String> {
    parameters
        .iter()
        .map(|p| Atom::var(*p).to_canonical_string())
        .collect()
}

/// Indexed readers already own exactly one record buffer. Transfer it only
/// when the returned standalone owner needs byte-identical later saving;
/// resident assembly intentionally discards it after moving the native owners.
fn owned_record_with_parent(
    bytes: Vec<u8>,
    options: KernelLoadOptions,
    retain: bool,
    parent: Option<&crate::kernel::ThresholdMetadata>,
) -> Result<KernelSet, KernelError> {
    let progress =
        &mut |_: &crate::kernel::CompilationProgress| std::ops::ControlFlow::Continue(());
    let mut kernels = if bytes.starts_with(cached::MAGIC) {
        cached::load_with_parent(&bytes, options, false, progress, parent)?
    } else {
        binary::load_with_primary_and_parent(&bytes, options, None, None, false, progress, parent)?
    };
    if retain && parent.is_none() {
        kernels.portable_artifact = Some(bytes.into());
    }
    Ok(kernels)
}

impl crate::generation::GeneratedIntegral {
    /// Build and save portable native exact evaluator programs without JIT.
    /// This performs native expression-to-IR translation; it does not materialize
    /// aliased coefficients or compile host executable machine code.
    pub fn to_kernel_bytes(&self, precision: PrecisionPolicy) -> Result<Vec<u8>, KernelError> {
        self.to_kernel_bytes_with_settings(precision, CompilationSettings::default())
    }

    /// Save native evaluator programs using caller-selected optimization controls.
    pub fn to_kernel_bytes_with_settings(
        &self,
        precision: PrecisionPolicy,
        settings: CompilationSettings,
    ) -> Result<Vec<u8>, KernelError> {
        binary::generated(self, precision, settings)
    }
}

impl KernelSet {
    pub(super) fn initialize_artifact(&mut self) -> Result<(), KernelError> {
        self.prepare_contour_checks()?;
        let (id, bytes) = binary::compiled(self)?;
        self.content_id = id;
        self.portable_artifact = Some(bytes.into());
        Ok(())
    }

    /// Save immutable portable programs, metadata and numerical policy. Runtime
    /// parameter values are excluded: these bytes identify `template_content_id()`.
    /// Legacy loaded artifacts retain their original bytes and identities.
    /// Native records may retain the owner's saved JIT application alongside
    /// exact IR. Mutable evaluator stacks, bindings and pilot state are excluded.
    pub fn to_bytes(&self) -> Result<Vec<u8>, KernelError> {
        match self.portable_artifact.as_deref() {
            Some(bytes) => Ok(bytes.to_vec()),
            // Exact setup deliberately releases chart bodies. Its retained
            // generation scope is valid native metadata, but is not a complete
            // indexed chart catalogue. Save that exact owner directly rather
            // than manufacturing missing source-chart receipts.
            None if self.sectors.is_empty()
                && self.metadata.as_ref().is_some_and(|metadata| {
                    metadata.charts.is_empty() && metadata.source_scope.is_some()
                }) =>
            {
                binary::compiled(self).map(|(_, bytes)| bytes)
            }
            None if self.threshold.is_some() => binary::compiled(self).map(|(_, bytes)| bytes),
            None => indexed::to_bytes(self).map(|(bytes, _)| bytes),
        }
    }

    /// Borrow the retained portable artifact without copying its native programs.
    /// Loaded artifacts retain their exact original bytes, including formatting.
    pub fn artifact_bytes(&self) -> Result<&[u8], KernelError> {
        self.portable_artifact
            .as_deref()
            .map(Vec::as_slice)
            .ok_or_else(|| KernelError::Artifact("kernel artifact was not initialized".into()))
    }

    /// Load a cache produced by a trusted, compatible FastSecDec/Symbolica builder.
    /// Format, numerical policy and evaluator layouts are checked. Recomputing
    /// content identities and symbolic metadata proofs requires `validate` in
    /// [`KernelLoadOptions`]. In either mode,
    /// upstream Symbolica's native IR decoder does not validate every internal
    /// instruction index. A valid content hash is not proof of safe provenance;
    /// do not pass attacker-created or manually modified native program bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, KernelError> {
        Self::from_bytes_with_options(bytes, KernelLoadOptions::default())
    }

    /// Restore saved programs with optional content and symbolic metadata checks.
    pub fn from_bytes_with_options(
        bytes: &[u8],
        options: KernelLoadOptions,
    ) -> Result<Self, KernelError> {
        Self::from_bytes_with_options_and_progress(bytes, options, |_| {
            std::ops::ControlFlow::Continue(())
        })
    }

    /// Restore a trusted artifact with caller-owned progress and cancellation.
    /// The callback runs before decoding, between sequential sector restorations,
    /// and after final admission. It cannot interrupt an individual native call.
    /// Supported artifacts restore exact sector IR without regenerating or optimizing
    /// sector expressions. Historical expression-only v1/v2 artifacts are rejected.
    /// Exact offsets are evaluated directly by the native
    /// expression owner, once at a supplied physical point, without compilation.
    pub fn from_bytes_with_progress(
        bytes: &[u8],
        progress: impl FnMut(&super::KernelLoadProgress) -> std::ops::ControlFlow<()>,
    ) -> Result<Self, KernelError> {
        Self::from_bytes_with_options_and_progress(bytes, KernelLoadOptions::default(), progress)
    }

    /// Restore with optional validation and caller-owned progress/cancellation.
    pub fn from_bytes_with_options_and_progress(
        bytes: &[u8],
        options: KernelLoadOptions,
        mut progress: impl FnMut(&super::KernelLoadProgress) -> std::ops::ControlFlow<()>,
    ) -> Result<Self, KernelError> {
        use super::KernelLoadProgress;
        if progress(&KernelLoadProgress::Decoding).is_break() {
            return Err(KernelError::Cancelled);
        }
        let mut restoring =
            |step: &super::CompilationProgress| progress(&KernelLoadProgress::Restoring(*step));
        let kernels = if bytes.starts_with(indexed::programs::MAGIC)
            || bytes.starts_with(indexed::programs::MAGIC_V3)
            || bytes.starts_with(indexed::programs::MAGIC_V4)
        {
            indexed::programs::from_bytes(bytes, options, &mut progress)
        } else if bytes.starts_with(indexed::MAGIC) {
            indexed::from_bytes(bytes, options, &mut progress)
        } else if bytes.starts_with(cached::MAGIC) {
            cached::load(bytes, options, &mut restoring)
        } else if bytes.starts_with(binary::PREFIX) {
            binary::load_with_progress(bytes, options, &mut restoring)
        } else {
            // Dispatch does not replace either codec's strict owned schema.
            // Ignore the large program arrays here instead of allocating a second
            // full JSON representation before the selected codec reads them.
            #[derive(serde::Deserialize)]
            struct Envelope {
                payload: Version,
            }
            #[derive(serde::Deserialize)]
            struct Version {
                version: u32,
            }
            let envelope: Envelope = serde_json::from_slice(bytes)?;
            match envelope.payload.version {
                1 | 2 => Err(KernelError::Artifact(
                    "expression-only kernel artifacts are unsupported; regenerate native evaluator IR".into(),
                )),
                3 => native::load(bytes, options, &mut restoring),
                _ => Err(KernelError::Artifact(
                    "unsupported kernel artifact version".into(),
                )),
            }
        }?;
        if progress(&KernelLoadProgress::Complete).is_break() {
            return Err(KernelError::Cancelled);
        }
        Ok(kernels)
    }
}

pub(crate) fn compiler_policy_with_settings(settings: CompilationSettings) -> String {
    native::compiler_policy_with_settings(settings)
}

#[cfg(all(test, feature = "threshold-decomposition"))]
pub(super) fn test_standalone(kernels: &KernelSet) -> Vec<u8> {
    let (_, bytes) = binary::compiled(kernels).unwrap();
    let restored = KernelSet::from_bytes(&bytes).unwrap();
    assert_eq!(bytes, restored.to_bytes().unwrap());
    let refreshed = cached::refresh(&restored).unwrap();
    assert!(
        KernelSet::from_bytes(&refreshed)
            .unwrap()
            .requires_validated_algebraic_callbacks()
    );
    bytes
}
