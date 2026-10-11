//! The existing numerical point identity, shared by binding and association.
pub(super) fn identity(template: &str, ordered: &[f64]) -> String {
    if ordered.is_empty() {
        return template.to_owned();
    }
    let mut hash = blake3::Hasher::new();
    hash.update(b"fastsecdec-runtime-parameter-point-v1");
    hash.update(template.as_bytes());
    for value in ordered {
        hash.update(&value.to_bits().to_le_bytes());
    }
    hash.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn established_point_bytes_and_signed_zero_are_preserved() {
        let template = "a".repeat(64);
        assert_eq!(identity(&template, &[]), template);
        for values in [vec![0.0], vec![-0.0], vec![16., -3.25, f64::MIN_POSITIVE]] {
            let mut old = blake3::Hasher::new();
            old.update(b"fastsecdec-runtime-parameter-point-v1");
            old.update(template.as_bytes());
            for value in &values {
                old.update(&f64::to_bits(*value).to_le_bytes());
            }
            assert_eq!(
                identity(&template, &values),
                old.finalize().to_hex().as_str()
            );
        }
        assert_ne!(identity(&template, &[0.]), identity(&template, &[-0.]));
        assert_ne!(
            identity(&template, &[1., 2.]),
            identity(&template, &[2., 1.])
        );
    }
}
