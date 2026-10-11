//! Immutable, detached helper ownership and native IR codec. No optimizer runs
//! on restore. The contract digest binds the branch, not just polynomial degree.
use super::Error;
#[cfg(feature = "threshold-decomposition")]
use super::numeric::{ROOT, ROOT_V2, ROOT_V3};
use std::{cell::RefCell, collections::BTreeMap, sync::Arc};
#[cfg(feature = "threshold-decomposition")]
use symbolica::evaluate::{EvaluatorComposer, OptimizationSettings, Slot};
use symbolica::{
    atom::{Atom, AtomCore, AtomView, Symbol},
    domains::{float::Complex, rational::Rational},
    evaluate::{ExpressionEvaluator, Instruction},
    symbol,
};
pub(super) type Exact = ExpressionEvaluator<Complex<Rational>>;
type Result<T> = std::result::Result<T, Error>;
const CONTRACT: &str = "fastsecdec::algebraic::contract_v1_";
#[derive(Debug)]
pub struct RootProgram {
    pub(super) exact: Exact,
    pub(super) shift: Option<Exact>,
    pub(super) version: u32,
    pub(super) lower: Rational,
    pub(super) upper: Rational,
    pub(super) degree: usize,
    tag: Symbol,
    #[cfg(feature = "threshold-decomposition")]
    semantic: String,
    bytes: Vec<u8>,
}
// Explicit transport limits. They bound decoded vectors and scalar literals;
// native evaluator preparation/normalization still needs caller process limits.
pub(super) const MAX_HELPER_BYTES: usize = 8 * 1024 * 1024;
const MAX_CONTRACT_BYTES: usize = 1024 * 1024;
const MAX_SCALAR_BYTES: usize = 4096;
pub(super) const MAX_DEGREE: usize = 4096;
const MAX_RATIONAL_BITS: u64 = 8192;
#[derive(bincode::Encode, bincode::BorrowDecode)]
pub(super) struct Wire<'a> {
    pub(super) version: u32,
    pub(super) degree: usize,
    pub(super) contract: &'a [u8],
    pub(super) bracket: &'a [u8],
    pub(super) exact: &'a [u8],
}
struct Saved {
    version: u32,
    contract: Vec<u8>,
    lower: Rational,
    upper: Rational,
    degree: usize,
    exact: Exact,
    shift: Option<Exact>,
}
fn scalar_admission(q: &Rational) -> bool {
    q.numerator_ref().significant_bits() <= MAX_RATIONAL_BITS
        && q.denominator_ref().significant_bits() <= MAX_RATIONAL_BITS
}
fn header_admission(
    version: u32,
    degree: usize,
    contract: &[u8],
    lower: &Rational,
    upper: &Rational,
) -> Result<()> {
    if !matches!(version, 1..=3)
        || degree == 0
        || degree > MAX_DEGREE
        || contract.is_empty()
        || contract.len() > MAX_CONTRACT_BYTES
        || lower >= upper
        || !scalar_admission(lower)
        || !scalar_admission(upper)
        || (version >= 2 && !scalar_admission(&(upper - lower)))
    {
        return Err(Error::Invalid(
            "root helper bounded preparation schema".into(),
        ));
    }
    if version >= 2 {
        // Endpoint controls contain native exact powers a^i,b^i. Refuse a
        // conservative scalar-size bound before those powers are allocated.
        for q in [lower, upper] {
            let bits = q
                .numerator_ref()
                .significant_bits()
                .max(q.denominator_ref().significant_bits());
            if bits
                .checked_mul(degree as u64)
                .is_none_or(|n| n > MAX_RATIONAL_BITS)
            {
                return Err(Error::Unsupported(
                    "affine endpoint power exceeds scalar preparation bound".into(),
                ));
            }
        }
    }
    // This first arithmetic recipe enters through f64. Its static numerical
    // bracket must survive that mapping exactly; an arbitrary rational rounded
    // inward could exclude an actual closed-face endpoint root. No padding or
    // unproved bracket widening is performed. Geometry may retain a broader
    // rational certificate for a future precision-specific numerical recipe.
    if version == 1 {
        for endpoint in [lower, upper] {
            let value = endpoint.to_f64();
            if !value.is_finite() || Rational::try_from(value).ok().as_ref() != Some(endpoint) {
                return Err(Error::Unsupported(
                    "static root bracket is not exactly representable in the primary f64 domain"
                        .into(),
                ));
            }
        }
    }
    if version == 3 {
        super::shifted::preflight(degree, lower, upper)?;
    }
    Ok(())
}
impl RootProgram {
    #[cfg(feature = "threshold-decomposition")]
    pub(crate) fn prepare_from_certificate(
        degree: usize,
        lower: Rational,
        upper: Rational,
        contract: Vec<u8>,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        Self::prepare_recipe(1, degree, lower, upper, contract, settings)
    }
    #[cfg(feature = "threshold-decomposition")]
    pub(crate) fn prepare_affine_from_certificate(
        degree: usize,
        lower: Rational,
        upper: Rational,
        contract: Vec<u8>,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        Self::prepare_recipe(2, degree, lower, upper, contract, settings)
    }
    #[cfg(feature = "threshold-decomposition")]
    pub(crate) fn prepare_shifted_from_certificate(
        degree: usize,
        lower: Rational,
        upper: Rational,
        contract: Vec<u8>,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        Self::prepare_recipe(3, degree, lower, upper, contract, settings)
    }
    #[cfg(feature = "threshold-decomposition")]
    fn prepare_recipe(
        version: u32,
        degree: usize,
        lower: Rational,
        upper: Rational,
        contract: Vec<u8>,
        settings: OptimizationSettings,
    ) -> Result<Arc<Self>> {
        header_admission(version, degree, &contract, &lower, &upper)?;
        let z = symbol!("fastsecdec::algebraic::root_argument");
        let mut inputs = vec![Atom::var(z)];
        let mut polynomial = Atom::zero();
        for i in 0..=degree {
            let c = Atom::var(symbol!(&format!("fastsecdec::algebraic::coefficient_{i}")));
            polynomial += &c * Atom::var(z).pow(i as i64);
            inputs.push(c);
        }
        let build = |out: &[Atom], inputs: &[Atom]| {
            Atom::evaluator_multiple(out, inputs)
                .optimization_settings(settings.clone())
                .build()
                .map_err(|e| Error::Native(e.to_string()))
        };
        let (exact, shift) = if version == 3 {
            let (body, shift) =
                super::shifted::prepare(&polynomial, z, &inputs, &lower, &upper, &settings)?;
            (body, Some(shift))
        } else {
            let body = build(&[polynomial.clone(), polynomial.derivative(z)], &inputs)?;
            let exact = if version == 1 {
                body
            } else {
                // Measured compact native composition preserves the original
                // coefficient schema without a dense binomial transform.
                let width = &upper - &lower;
                let image = Atom::num(lower.clone()) + Atom::num(width.clone()) * Atom::var(z);
                let affine = build(&[image], &[Atom::var(z)])?;
                let scale = build(&[Atom::num(width) * Atom::var(z)], &[Atom::var(z)])?;
                let mut compose = EvaluatorComposer::new(degree + 2);
                let physical = compose
                    .append(&affine, &[Slot::Param(0)])
                    .map_err(|e| Error::Native(e.to_string()))?[0];
                let mut slots = vec![physical];
                slots.extend((1..degree + 2).map(Slot::Param));
                let values = compose
                    .append(&body, &slots)
                    .map_err(|e| Error::Native(e.to_string()))?;
                let qs = compose
                    .append(&scale, &[values[1]])
                    .map_err(|e| Error::Native(e.to_string()))?[0];
                compose
                    .finish(&[values[0], qs, physical], settings)
                    .map_err(|e| Error::Native(e.to_string()))?
            };
            (exact, None)
        };
        let saved = Saved {
            version,
            contract,
            lower,
            upper,
            degree,
            exact,
            shift,
        };
        let bracket = bincode::serde::encode_to_vec(
            (&saved.lower, &saved.upper),
            bincode::config::standard(),
        )
        .map_err(|e| Error::Native(e.to_string()))?;
        let exact = if let Some(shift) = &saved.shift {
            bincode::serde::encode_to_vec((&saved.exact, shift), bincode::config::standard())
        } else {
            bincode::serde::encode_to_vec(&saved.exact, bincode::config::standard())
        }
        .map_err(|e| Error::Native(e.to_string()))?;
        let bytes = bincode::encode_to_vec(
            Wire {
                version,
                degree,
                contract: &saved.contract,
                bracket: &bracket,
                exact: &exact,
            },
            bincode::config::standard(),
        )
        .map_err(|e| Error::Native(e.to_string()))?;
        if bytes.len() > MAX_HELPER_BYTES {
            return Err(Error::Invalid(
                "root helper exceeds transport byte limit".into(),
            ));
        }
        Self::admit(saved, bytes)
    }
    pub(crate) fn from_bytes(bytes: &[u8]) -> Result<Arc<Self>> {
        if bytes.len() > MAX_HELPER_BYTES {
            return Err(Error::Invalid(
                "root helper exceeds transport byte limit".into(),
            ));
        }
        // Borrow lengths/metadata first: degree, coefficient count and byte
        // limits are checked BEFORE allocating native evaluator vectors.
        let (wire, read): (Wire<'_>, usize) = bincode::borrow_decode_from_slice(
            bytes,
            bincode::config::standard().with_limit::<MAX_HELPER_BYTES>(),
        )
        .map_err(|e| Error::Invalid(e.to_string()))?;
        if read != bytes.len()
            || !matches!(wire.version, 1..=3)
            || wire.degree == 0
            || wire.degree > MAX_DEGREE
            || wire.contract.is_empty()
            || wire.contract.len() > MAX_CONTRACT_BYTES
            || wire.bracket.len() > MAX_SCALAR_BYTES
        {
            return Err(Error::Invalid("root helper bounded wire schema".into()));
        }
        let ((lower, upper), read): ((Rational, Rational), usize) =
            bincode::serde::decode_from_slice(
                wire.bracket,
                bincode::config::standard().with_limit::<MAX_SCALAR_BYTES>(),
            )
            .map_err(|e| Error::Invalid(e.to_string()))?;
        if read != wire.bracket.len() {
            return Err(Error::Invalid("trailing root bracket bytes".into()));
        }
        header_admission(wire.version, wire.degree, wire.contract, &lower, &upper)?;
        let (exact, shift, read) = if wire.version == 3 {
            let ((body, shift), read): ((Exact, Exact), usize) = bincode::serde::decode_from_slice(
                wire.exact,
                bincode::config::standard().with_limit::<MAX_HELPER_BYTES>(),
            )
            .map_err(|e| Error::Invalid(e.to_string()))?;
            (body, Some(shift), read)
        } else {
            let (body, read): (Exact, usize) = bincode::serde::decode_from_slice(
                wire.exact,
                bincode::config::standard().with_limit::<MAX_HELPER_BYTES>(),
            )
            .map_err(|e| Error::Invalid(e.to_string()))?;
            (body, None, read)
        };
        if read != wire.exact.len() {
            return Err(Error::Invalid("trailing root IR bytes".into()));
        }
        Self::admit(
            Saved {
                version: wire.version,
                contract: wire.contract.to_vec(),
                lower,
                upper,
                degree: wire.degree,
                exact,
                shift,
            },
            bytes.to_vec(),
        )
    }
    fn admit(saved: Saved, bytes: Vec<u8>) -> Result<Arc<Self>> {
        if !matches!(saved.version, 1..=3)
            || saved.degree == 0
            || saved.degree > MAX_DEGREE
            || saved.degree.checked_add(2) != Some(saved.exact.get_input_len())
            || saved.exact.get_output_len() != if saved.version == 1 { 2 } else { 3 }
            || saved.lower >= saved.upper
            || saved.contract.is_empty()
        {
            return Err(Error::Invalid("root helper schema".into()));
        }
        if (saved.version == 3) != saved.shift.is_some()
            || saved.shift.as_ref().is_some_and(|shift| {
                shift.get_input_len() != saved.degree + 1
                    || shift.get_output_len() != saved.degree + 1
            })
        {
            return Err(Error::Invalid("root coefficient-shift schema".into()));
        }
        for program in std::iter::once(&saved.exact).chain(saved.shift.as_ref()) {
            let code = program.export_instructions();
            if !code.constant_functions.is_empty()
                || !code.sub_evaluators.is_empty()
                || code.constants.iter().any(|c| c.im != 0)
                || code.instructions.iter().any(|i| {
                    !matches!(
                        i,
                        Instruction::Add(..)
                            | Instruction::Mul(..)
                            | Instruction::Assign(..)
                            | Instruction::Pow(_, _, 0.., _)
                    )
                })
            {
                return Err(Error::Invalid(
                    "root helper must be native real polynomial arithmetic".into(),
                ));
            }
        }
        // Includes complete native IR, preparation recipe and branch contract. No
        // process-local Symbol IDs or global registry are used for owner selection.
        let digest = blake3::hash(&bytes).to_hex().to_string();
        // Arithmetic transport includes optimized IR. Mathematical association
        // includes only the selected polynomial/branch and numeric recipe.
        #[cfg(feature = "threshold-decomposition")]
        let semantic = {
            let identity = serde_json::to_vec(&(
                "fastsecdec-selected-root-mathematics-v1",
                saved.version,
                saved.degree,
                &saved.contract,
                saved.lower.to_string(),
                saved.upper.to_string(),
            ))
            .map_err(|e| Error::Invalid(e.to_string()))?;
            blake3::hash(&identity).to_hex().to_string()
        };
        Ok(Arc::new(Self {
            version: saved.version,
            exact: saved.exact,
            shift: saved.shift,
            lower: saved.lower,
            upper: saved.upper,
            degree: saved.degree,
            tag: symbol!(&format!("{CONTRACT}{digest}")),
            #[cfg(feature = "threshold-decomposition")]
            semantic,
            bytes,
        }))
    }
    #[cfg(feature = "threshold-decomposition")]
    pub(crate) fn semantic_identity(&self) -> &str {
        &self.semantic
    }
    #[cfg(feature = "threshold-decomposition")]
    pub(crate) fn transport_symbol(&self) -> Symbol {
        self.tag
    }
    #[cfg(feature = "threshold-decomposition")]
    pub fn call(&self, tag: Symbol, coefficients: &[Atom]) -> Result<Atom> {
        if coefficients.len() != self.degree + 1 {
            return Err(Error::Invalid("root coefficient shape".into()));
        }
        let mut args = vec![Atom::num(self.degree), Atom::var(tag), Atom::var(self.tag)];
        args.extend(coefficients.iter().cloned());
        if self.version >= 2 {
            // These redundant arithmetic controls are constructed from exactly
            // the same coefficient expressions. They are never public inputs
            // with an independent mathematical derivative role.
            for endpoint in [&self.lower, &self.upper] {
                let residual: Atom = coefficients
                    .iter()
                    .enumerate()
                    .map(|(i, c)| c * Atom::num(endpoint.pow(i as u64)))
                    .sum();
                args.push(residual.expand());
            }
        }
        Ok(match self.version {
            1 => *ROOT,
            2 => *ROOT_V2,
            3 => *ROOT_V3,
            _ => unreachable!("admitted root recipe"),
        }
        .call_args(args))
    }
    pub(crate) fn recipe_version(&self) -> u32 {
        self.version
    }
    pub(crate) fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}
#[derive(Clone, Default)]
pub struct Scope(BTreeMap<(Symbol, Symbol), Arc<RootProgram>>);
thread_local! { static PREPARING: RefCell<Option<(Scope, u32)>> = const { RefCell::new(None) }; }
impl Scope {
    pub fn insert(&mut self, tag: Symbol, program: Arc<RootProgram>) -> Result<()> {
        match self.0.entry((tag, program.tag)) {
            std::collections::btree_map::Entry::Vacant(slot) => {
                slot.insert(program);
                Ok(())
            }
            std::collections::btree_map::Entry::Occupied(_) => {
                Err(Error::Invalid("duplicate root owner tag".into()))
            }
        }
    }
    pub fn enter<R>(&self, bits: u32, run: impl FnOnce() -> R) -> R {
        if self.0.is_empty() {
            return run();
        }
        struct Restore(Option<(Scope, u32)>);
        impl Drop for Restore {
            fn drop(&mut self) {
                PREPARING.replace(self.0.take());
            }
        }
        let _restore = Restore(PREPARING.replace(Some((self.clone(), bits))));
        run()
    }
    pub(in crate::kernel) fn capture_for(
        callbacks: &[crate::kernel::program::Callback],
    ) -> std::result::Result<Self, String> {
        let mut selected = Self::default();
        for callback in callbacks.iter().filter(|c| super::is_root(c.symbol)) {
            let tags = callback.tags.iter().map(Atom::as_view).collect::<Vec<_>>();
            let (owner, _) = resolve(&tags)?;
            if callback.symbol
                != match owner.version {
                    1 => *super::numeric::ROOT,
                    2 => *super::numeric::ROOT_V2,
                    3 => *super::numeric::ROOT_V3,
                    _ => unreachable!("admitted root recipe"),
                }
            {
                return Err("root callback recipe differs from its selected owner".into());
            }
            let tag = tags[1].as_var_view().unwrap().get_symbol();
            if let Some(prior) = selected.0.insert((tag, owner.tag), owner.clone())
                && prior.tag != owner.tag
            {
                return Err("incompatible root owners share a tag".into());
            }
        }
        Ok(selected)
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    pub(crate) fn owners(&self) -> impl Iterator<Item = (Symbol, &Arc<RootProgram>)> {
        self.0.iter().map(|((tag, _), v)| (*tag, v))
    }

    pub(crate) fn merged<'a>(scopes: impl Iterator<Item = &'a Scope>) -> Result<Self> {
        let mut result = Self::default();
        for scope in scopes {
            for (key, owner) in &scope.0 {
                if let Some(old) = result.0.insert(*key, owner.clone())
                    && old.bytes() != owner.bytes()
                {
                    return Err(Error::Invalid(
                        "conflicting algebraic helper identity".into(),
                    ));
                }
            }
        }
        Ok(result)
    }
    pub(crate) fn saved(&self) -> Vec<(Symbol, Vec<u8>)> {
        let mut rows = self
            .owners()
            .map(|(tag, owner)| (tag, owner.bytes().to_vec()))
            .collect::<Vec<_>>();
        rows.sort_by_cached_key(|(tag, bytes)| {
            (
                tag.get_name().to_owned(),
                blake3::hash(bytes).to_hex().to_string(),
            )
        });
        rows
    }
    pub(crate) fn restore(rows: Vec<(Symbol, Vec<u8>)>) -> Result<Self> {
        let mut result = Self::default();
        for (tag, bytes) in rows {
            result.insert(tag, RootProgram::from_bytes(&bytes)?)?;
        }
        Ok(result)
    }
    #[cfg(test)]
    pub fn native_roundtrip(&self) -> Result<Self> {
        let mut scope = Self::default();
        for ((tag, _), owner) in &self.0 {
            scope.insert(*tag, RootProgram::from_bytes(owner.bytes())?)?;
        }
        Ok(scope)
    }
}
pub(super) fn resolve(
    tags: &[AtomView<'_>],
) -> std::result::Result<(Arc<RootProgram>, u32), String> {
    if tags.len() != 3 {
        return Err("root callback requires degree, owner and branch-contract tags".into());
    }
    let tag = tags[1]
        .as_var_view()
        .ok_or("root owner tag must be a symbol")?
        .get_symbol();
    PREPARING.with_borrow(|active| {
        let (scope, bits) = active
            .as_ref()
            .ok_or("root callback requires its selected preparation scope")?;
        let contract = tags[2]
            .as_var_view()
            .ok_or("root contract tag must be a symbol")?
            .get_symbol();
        let owner = scope
            .0
            .get(&(tag, contract))
            .ok_or("root callback owner absent from selected scope")?;
        if tags[0] != Atom::num(owner.degree).as_view() || tags[2] != Atom::var(owner.tag).as_view()
        {
            return Err(
                "root callback degree/branch contract differs from its selected owner".into(),
            );
        }
        Ok((owner.clone(), *bits))
    })
}
