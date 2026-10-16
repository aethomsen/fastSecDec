//! One checked native numerical attempt. The immutable specification is
//! captured at preparation; each numerical owner has independent ball scratch
//! and counters. No validation state is installed on an unchecked hot path.
use super::{checker::Check, radius::RadiusFailure};
use crate::{
    contour::{
        ContourMode,
        functions::dynamic::observation::{Candidate, Requests},
        functions::dynamic::requests::Bundle,
    },
    kernel::{
        KernelError, NativeProgramDescriptor, program::ExactProgram, recipe::DynamicCheckProgram,
    },
};
use std::{cell::RefCell, collections::BTreeSet, marker::PhantomData, rc::Rc, sync::Arc};
use symbolica::{atom::Symbol, domains::rational::Rational};

#[derive(Clone)]
struct Source {
    certificate_index: usize,
    saved: Arc<DynamicCheckProgram>,
    level: ExactProgram,
    projection: Vec<Option<usize>>,
}

type FaceRestriction = Vec<(usize, u8)>;
/// Source index and fixed coordinates for one candidate's certificate check.
type SourceFaceBinding = (usize, FaceRestriction);

pub(in crate::kernel) struct Specification {
    sources: Vec<Source>,
    runtime: Vec<f64>,
    deformation: ContourMode,
    requests: Requests,
    bindings: Vec<Vec<SourceFaceBinding>>,
    dimension: usize,
    root_relative_width: Rational,
    maximum_bits: u32,
}

impl Specification {
    pub(in crate::kernel) fn new(
        descriptor: &NativeProgramDescriptor,
        parameters: &[Symbol],
        runtime: Vec<f64>,
        deformation: ContourMode,
        bundles: BTreeSet<Bundle>,
        precision: &crate::kernel::PrecisionPolicy,
    ) -> Result<Arc<Self>, KernelError> {
        precision.validate()?;
        if descriptor.recipe() != deformation.program_recipe() {
            return Err(KernelError::Contour(
                "checked numerical owner has a different recipe".into(),
            ));
        }
        let certificates = descriptor.certificates().ok_or_else(|| {
            KernelError::Contour("dynamic program lacks saved certificates".into())
        })?;
        let requests = Requests::new(bundles).map_err(KernelError::Contour)?;
        let mut sources = Vec::<Source>::new();
        let mut source_indices = Vec::<usize>::new();
        let mut bindings = vec![Vec::new(); requests.bundles().count()];
        for (bundle, candidate) in requests.bundles() {
            for request in &bundle.0 {
                let mut found = false;
                for (saved_index, saved) in certificates
                    .iter()
                    .enumerate()
                    .filter(|(_, saved)| saved.namespace == request.namespace)
                {
                    found = true;
                    let index = if let Some(index) =
                        source_indices.iter().position(|i| *i == saved_index)
                    {
                        index
                    } else {
                        let projection = saved
                            .coordinates
                            .iter()
                            .map(|name| {
                                Symbol::parse(name, "fastsecdec::artifact")
                                    .map(|symbol| {
                                        parameters.iter().position(|input| *input == symbol)
                                    })
                                    .map_err(KernelError::Artifact)
                            })
                            .collect::<Result<Vec<_>, _>>()?;
                        let level = descriptor
                            .root_helper(&saved.structure.helper_digest)
                            .ok_or_else(|| {
                                KernelError::Contour("certificate root helper is absent".into())
                            })?
                            .exact_program()
                            .clone();
                        let index = sources.len();
                        source_indices.push(saved_index);
                        sources.push(Source {
                            certificate_index: saved_index,
                            saved: Arc::new(saved.clone()),
                            level,
                            projection,
                        });
                        index
                    };
                    let projection = &sources[index].projection;
                    if request
                        .face
                        .iter()
                        .any(|(axis, value)| *axis >= projection.len() || *value > 1)
                    {
                        return Err(KernelError::Contour(
                            "dynamic request cannot project its full-source coordinates".into(),
                        ));
                    }
                    // Equal namespaces can have different native programs.
                    // Check every retained context; never select by byte order.
                    bindings[candidate].push((index, request.face.clone()));
                }
                if !found {
                    return Err(KernelError::Contour(
                        "executed radius lacks its saved source certificate".into(),
                    ));
                }
            }
        }
        Ok(Arc::new(Self {
            sources,
            runtime,
            deformation,
            requests,
            bindings,
            dimension: parameters.len(),
            root_relative_width: Rational::try_from(precision.relative_tolerance)
                .map_err(|_| KernelError::PrecisionPolicy)?,
            maximum_bits: precision.max_bits,
        }))
    }

    #[cfg(test)]
    pub(in crate::kernel) fn enter(self: &Arc<Self>) -> Preparation {
        enter_optional(Some(self.clone()))
    }

    pub(in crate::kernel) fn admit_callbacks(
        &self,
        callbacks: &[crate::kernel::program::Callback],
    ) -> Result<(), String> {
        let mut actual = BTreeSet::new();
        for callback in callbacks {
            if callback.symbol == crate::contour::functions::dynamic::symbol() {
                return Err(
                    "checked stochastic program contains an unassociated mathematical root".into(),
                );
            }
            if callback.symbol != crate::contour::functions::dynamic::requested::symbol() {
                continue;
            }
            if callback.fixed_args.is_some() || callback.tags.len() != 3 {
                return Err(
                    "checked stochastic root requires three tags and runtime arguments".into(),
                );
            }
            actual.insert(Bundle::from_atom(callback.tags[2].as_view())?);
        }
        let expected = self
            .requests
            .bundles()
            .map(|(bundle, _)| bundle.clone())
            .collect::<BTreeSet<_>>();
        if actual != expected {
            return Err(
                "checked numerical program differs from its surviving request coverage".into(),
            );
        }
        Ok(())
    }
}

thread_local! { static ACTIVE: RefCell<Option<Arc<Specification>>> = const { RefCell::new(None) }; }
pub(in crate::kernel) struct Preparation(Option<Arc<Specification>>, PhantomData<Rc<()>>);
impl Drop for Preparation {
    fn drop(&mut self) {
        ACTIVE.replace(self.0.take());
    }
}
pub(in crate::kernel) fn enter_optional(specification: Option<Arc<Specification>>) -> Preparation {
    Preparation(ACTIVE.replace(specification), PhantomData)
}
pub(in crate::kernel) fn capture() -> Option<Arc<Specification>> {
    ACTIVE.with_borrow(Clone::clone)
}

#[derive(Clone, Debug, Default)]
pub(in crate::kernel) struct Coverage {
    pub bundles: BTreeSet<Bundle>,
    pub checked_arguments: usize,
    pub maximum_bits: u32,
    pub contexts: Vec<ContextCoverage>,
}

#[derive(Clone, Debug)]
pub(in crate::kernel) struct ContextCoverage {
    /// Position in this numerical owner's retained descriptor only.
    pub certificate_index: usize,
    pub namespace: String,
    pub chart_index: Option<usize>,
    pub checked_arguments: usize,
    pub maximum_bits: u32,
}

pub(in crate::kernel) struct Validation {
    specification: Arc<Specification>,
    checks: Option<Vec<Check>>,
    pub checked_arguments: usize,
    pub maximum_bits: u32,
    pub last_error: Option<String>,
    coverage: Option<Coverage>,
    attempt_bits: u32,
    attempt_contexts: Vec<ContextCoverage>,
}
impl Clone for Validation {
    fn clone(&self) -> Self {
        Self {
            specification: self.specification.clone(),
            checks: self.checks.clone(),
            checked_arguments: 0,
            maximum_bits: 0,
            last_error: None,
            coverage: None,
            attempt_bits: 0,
            attempt_contexts: Vec::new(),
        }
    }
}
impl Validation {
    pub(in crate::kernel) fn new(specification: Arc<Specification>) -> Self {
        Self {
            specification,
            checks: None,
            checked_arguments: 0,
            maximum_bits: 0,
            last_error: None,
            coverage: None,
            attempt_bits: 0,
            attempt_contexts: Vec::new(),
        }
    }
    /// Return false for callback or certificate failure even if later arithmetic
    /// masks the failed value. Nested scopes are restored on errors and unwind.
    pub(in crate::kernel) fn evaluate(&mut self, point: &[f64], evaluate: impl FnOnce()) -> bool {
        self.last_error = None;
        self.coverage = None;
        let attempt = self.begin();
        let (_, failure) = crate::contour::functions::dynamic::isolated_attempt(evaluate);
        if let Some(reason) = failure {
            self.last_error = Some(format!(
                "dynamic callback at coordinates {point:?}: {reason}"
            ));
            false
        } else {
            self.finish(point, attempt).is_ok()
        }
    }
    pub(in crate::kernel) fn begin(
        &self,
    ) -> crate::contour::functions::dynamic::observation::Attempt {
        self.specification.requests.begin()
    }
    pub(in crate::kernel) fn finish(
        &mut self,
        point: &[f64],
        attempt: crate::contour::functions::dynamic::observation::Attempt,
    ) -> Result<(), String> {
        let candidates = match attempt.finish() {
            Ok(candidates) => candidates,
            Err(reason) => {
                self.coverage = None;
                self.last_error = Some(reason.clone());
                return Err(reason);
            }
        };
        let indexed = candidates
            .iter()
            .map(|(index, candidate)| (*index, candidate))
            .collect::<Vec<_>>();
        let bundles = self
            .specification
            .requests
            .bundles()
            .map(|(bundle, _)| bundle.clone())
            .collect();
        self.certify_attempt(point, &indexed, bundles)
    }

    /// Direct native Atom evaluation retains lazy branches. Only the actual
    /// cached roots are obligations for this attempt; every executed root must
    /// still have an admitted saved association. No stochastic missing-work
    /// rule is applied to unexecuted branches.
    pub(in crate::kernel) fn certify_executed(
        &mut self,
        point: &[f64],
        executed: &[(Bundle, Candidate)],
    ) -> Result<(), String> {
        let mut bundles = BTreeSet::new();
        let indexed = executed
            .iter()
            .map(|(bundle, candidate)| {
                if !bundles.insert(bundle.clone()) {
                    return Err("duplicate actually executed exact root request".to_owned());
                }
                self.specification
                    .requests
                    .bundles()
                    .find(|(expected, _)| *expected == bundle)
                    .map(|(_, index)| (index, candidate))
                    .ok_or_else(|| {
                        "executed exact radius lacks a saved certificate association".to_owned()
                    })
            })
            .collect::<Result<Vec<_>, _>>();
        match indexed {
            Ok(indexed) => self.certify_attempt(point, &indexed, bundles),
            Err(reason) => {
                self.coverage = None;
                self.last_error = Some(reason.clone());
                Err(reason)
            }
        }
    }

    pub(in crate::kernel) fn coverage(&self) -> Option<&Coverage> {
        self.coverage.as_ref()
    }
    pub(in crate::kernel) fn clear_attempt(&mut self) {
        self.last_error = None;
        self.coverage = None;
    }

    fn certify_attempt(
        &mut self,
        point: &[f64],
        candidates: &[(usize, &Candidate)],
        bundles: BTreeSet<Bundle>,
    ) -> Result<(), String> {
        self.last_error = None;
        self.coverage = None;
        self.attempt_bits = 0;
        self.attempt_contexts.clear();
        let before = self.checked_arguments;
        let result = self.certify(point, candidates);
        match &result {
            Ok(()) => {
                self.coverage = Some(Coverage {
                    bundles,
                    checked_arguments: self.checked_arguments - before,
                    maximum_bits: self.attempt_bits,
                    contexts: self.attempt_contexts.clone(),
                })
            }
            Err(reason) => self.last_error = Some(reason.clone()),
        }
        result
    }

    fn certify(&mut self, point: &[f64], candidates: &[(usize, &Candidate)]) -> Result<(), String> {
        let specification = &self.specification;
        if point.len() != specification.dimension + specification.runtime.len()
            || point[specification.dimension..] != specification.runtime
            || point[..specification.dimension]
                .iter()
                .any(|x| !x.is_finite() || !(0.0..=1.0).contains(x))
        {
            return Err("dynamic checked point differs from its bound numerical owner".into());
        }
        if candidates.is_empty() {
            return Ok(());
        }
        if self.checks.is_none() {
            self.checks = Some(
                specification
                    .sources
                    .iter()
                    .map(|source| {
                        Check::prepare(
                            source.saved.clone(),
                            source.level.clone(),
                            specification.runtime.clone(),
                            specification.deformation,
                            specification.root_relative_width.clone(),
                        )
                    })
                    .collect::<Result<_, _>>()
                    .map_err(|error| error.to_string())?,
            );
        }
        let checks = self.checks.as_mut().unwrap();
        for (request, candidate) in candidates {
            for (index, face) in &specification.bindings[*request] {
                let source = &specification.sources[*index];
                let coordinates = source
                    .projection
                    .iter()
                    .map(|input| input.map(|index| point[index]))
                    .collect::<Vec<_>>();
                let mut attempted_bits = Vec::new();
                let mut bits = 96.min(specification.maximum_bits);
                let failure = loop {
                    self.maximum_bits = self.maximum_bits.max(bits);
                    self.attempt_bits = self.attempt_bits.max(bits);
                    attempted_bits.push(bits);
                    match checks[*index].evaluate_projected(
                        &coordinates,
                        face,
                        &candidate.lambda,
                        bits,
                    ) {
                        Ok(_) => break None,
                        Err(error @ (RadiusFailure::Invalid(_) | RadiusFailure::Unsafe(_))) => {
                            break Some(error);
                        }
                        Err(error) if bits == specification.maximum_bits => break Some(error),
                        Err(_) => {}
                    }
                    bits = bits.saturating_mul(2).min(specification.maximum_bits);
                };
                self.checked_arguments += 1;
                if let Some(reason) = failure {
                    return Err(format!(
                        "dynamic certificate chart {:?}, namespace {}, coordinates {coordinates:?}, face {face:?}, candidate precision {}, certificate precisions {attempted_bits:?}, factors {:?}: {reason:?}",
                        source.saved.chart_index,
                        source.saved.namespace,
                        candidate.bits,
                        source.saved.factors
                    ));
                }
                if let Some(context) = self
                    .attempt_contexts
                    .iter_mut()
                    .find(|context| context.certificate_index == source.certificate_index)
                {
                    context.checked_arguments += 1;
                    context.maximum_bits = context.maximum_bits.max(bits);
                } else {
                    self.attempt_contexts.push(ContextCoverage {
                        certificate_index: source.certificate_index,
                        namespace: source.saved.namespace.clone(),
                        chart_index: source.saved.chart_index,
                        checked_arguments: 1,
                        maximum_bits: bits,
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
