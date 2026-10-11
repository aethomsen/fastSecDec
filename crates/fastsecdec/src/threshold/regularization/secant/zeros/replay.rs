//! Bounded local arithmetic replay; no geometry authority is reconstructed.
use super::*;

fn fits(p: &Poly, limits: meromorphic::Limits) -> bool {
    p.nterms() <= limits.terms && (0..p.nvars()).all(|i| p.degree(i) <= u32::from(limits.degree))
}
fn product(a: &Poly, b: &Poly, limits: meromorphic::Limits) -> Option<Poly> {
    if a.nterms()
        .checked_mul(b.nterms())
        .is_none_or(|n| n > limits.terms)
        || (0..a.nvars()).any(|i| {
            a.degree(i)
                .checked_add(b.degree(i))
                .is_none_or(|d| d > u32::from(limits.degree))
        })
    {
        return None;
    }
    let p = a * b;
    fits(&p, limits).then_some(p)
}

pub(super) fn validate(r: &Receipt) -> Result<Option<String>> {
    let limits = meromorphic::Limits::default();
    if r.branches[0] == r.branches[1]
        || r.branches
            .iter()
            .any(|s| s.len() != 64 || !s.bytes().all(|b| b.is_ascii_hexdigit()))
        || r.gap_power > 2
        || r.atoms.len() < FIXED
        || r.atoms.len() > limits.terms
        || r.variables.len() < 3
        || r.variables.len() > usize::from(limits.degree)
        || r.inverse_units.len() > limits.terms
        || r.variables.iter().copied().collect::<BTreeSet<_>>().len() != r.variables.len()
    {
        return Ok(None);
    }
    // Inspect the complete inventory before polynomial conversion or loops.
    let mut total_power = 0u32;
    for (index, power) in &r.inverse_units {
        if *index >= r.atoms.len() - FIXED || *power == 0 || *power > u32::from(limits.degree) {
            return Ok(None);
        }
        let Some(next) = total_power.checked_add(*power) else {
            return Ok(None);
        };
        if next > u32::from(limits.degree) {
            return Ok(None);
        }
        total_power = next;
    }
    for a in &r.atoms {
        if meromorphic::preflight_expression(a, limits).is_err() {
            return Ok(None);
        }
    }
    let vars = Arc::new(
        r.variables
            .iter()
            .copied()
            .map(PolyVariable::from)
            .collect::<Vec<_>>(),
    );
    let Some(p) = r
        .atoms
        .iter()
        .map(|a| polynomial(a, &vars))
        .collect::<Option<Vec<_>>>()
    else {
        return Ok(None);
    };
    if p.iter().any(|p| !fits(p, limits))
        || p[1].is_zero()
        || p[4].is_zero()
        || product(&p[1], &p[3], limits).as_ref() != Some(&p[2])
    {
        return Ok(None);
    }
    let expected_gap = polynomial(
        &(Atom::var(r.variables[1]) - Atom::var(r.variables[0])),
        &vars,
    )
    .unwrap();
    if p[4] != expected_gap {
        return Ok(None);
    }
    let mut inverses = p[0].one();
    for (i, power) in &r.inverse_units {
        for _ in 0..*power {
            let Some(next) = product(&inverses, &p[FIXED + *i], limits) else {
                return Ok(None);
            };
            inverses = next;
        }
    }
    if inverses != p[2] {
        return Ok(None);
    }
    let mut cleared = p[0].clone();
    for _ in 0..r.gap_power {
        let Some(next) = product(&cleared, &p[4], limits) else {
            return Ok(None);
        };
        cleared = next;
    }
    let (Some(qa), Some(qb)) = (product(&p[7], &p[5], limits), product(&p[8], &p[6], limits))
    else {
        return Ok(None);
    };
    if qa
        .nterms()
        .checked_add(qb.nterms())
        .is_none_or(|n| n > limits.terms)
        || &qa + &qb != cleared
    {
        return Ok(None);
    }
    let rows = p
        .iter()
        .map(|p| {
            p.into_iter()
                .map(|m| (m.coefficient.to_string(), m.exponents.to_vec()))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let data = serde_json::to_vec(&(
        1u32,
        r.order,
        r.variables.len(),
        r.gap_power,
        &r.branches,
        &r.inverse_units,
        rows,
    ))
    .map_err(native)?;
    let mut h = blake3::Hasher::new();
    h.update(b"threshold-local-paired-zero-v1");
    h.update(&data);
    Ok(Some(h.finalize().to_hex().to_string()))
}
