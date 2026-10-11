use super::*;
use crate::{
    generation::{GenerationMode, GenerationOptions},
    kernel::{EvaluatorBackend, parameter_template_tests::family_kind},
};
fn fixture(numerator: bool) -> KernelSet {
    let (family, _) = family_kind(numerator, None);
    let continued = family
        .continue_symbolically(&GenerationOptions {
            mode: GenerationMode::Symbolic,
            max_order: 1,
            ..Default::default()
        })
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    KernelSet::compile_threshold_family(
        &continued,
        dir.path(),
        1,
        PrecisionPolicy::default(),
        CompilationSettings {
            backend: EvaluatorBackend::Eager,
            ..Default::default()
        },
    )
    .unwrap()
}
fn altered(bytes: &[u8], change: impl FnOnce(&mut PayloadV18)) -> Vec<u8> {
    let (mut envelope, _): (Envelope, usize) = bincode::decode_from_slice(
        bytes.strip_prefix(MAGIC_V18).unwrap(),
        bincode::config::standard(),
    )
    .unwrap();
    let context = State::import(&mut envelope.state.as_slice(), None).unwrap();
    let (mut payload, _): (PayloadV18, usize) = bincode::decode_from_slice_with_context(
        &envelope.payload,
        bincode::config::standard(),
        context,
    )
    .unwrap();
    change(&mut payload);
    envelope.payload = bincode::encode_to_vec(payload, bincode::config::standard()).unwrap();
    envelope.digest = *digest(MAGIC_V18, &envelope.state, &envelope.payload).as_bytes();
    let mut out = MAGIC_V18.to_vec();
    out.extend(bincode::encode_to_vec(envelope, bincode::config::standard()).unwrap());
    out
}
#[test]
fn uniform_v18_rejects_missing_guard_and_transplanted_parent_or_runtime() {
    let source = fixture(false);
    let bytes = source.to_bytes().unwrap();
    for mutation in [0, 1, 2, 3] {
        let changed = altered(&bytes, |p| match mutation {
            0 => p.validated_policy_version = 2,
            1 => p.base.base.runtime_parameters.clear(),
            2 => {
                p.guard = SavedGuard::Parent {
                    semantic: "0".repeat(64),
                }
            }
            _ => p
                .base
                .base
                .runtime_parameters
                .push(p.base.base.runtime_parameters[0]),
        });
        for validate in [false, true] {
            assert!(
                KernelSet::from_bytes_with_options(&changed, KernelLoadOptions { validate })
                    .is_err()
            );
        }
    }
    let semantic = source
        .threshold_metadata()
        .unwrap()
        .parameter_guard()
        .unwrap()
        .semantic_identity()
        .unwrap();
    let parent_bytes = altered(&bytes, |p| p.guard = SavedGuard::Parent { semantic });
    let (envelope, _): (Envelope, usize) = bincode::decode_from_slice(
        parent_bytes.strip_prefix(MAGIC_V18).unwrap(),
        bincode::config::standard(),
    )
    .unwrap();
    let foreign = fixture(true);
    assert!(
        decode(
            &envelope.payload,
            State::import(&mut envelope.state.as_slice(), None).unwrap(),
            None
        )
        .is_err()
    );
    assert!(
        decode(
            &envelope.payload,
            State::import(&mut envelope.state.as_slice(), None).unwrap(),
            foreign.threshold_metadata()
        )
        .is_err()
    );
    assert!(
        decode(
            &envelope.payload,
            State::import(&mut envelope.state.as_slice(), None).unwrap(),
            source.threshold_metadata()
        )
        .is_ok()
    );
}
