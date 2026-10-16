//! Bounded observations of the candidates actually used by a checked native
//! evaluation. Plain factories never call this module. No solver runs here.
use super::requests::Bundle;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};
use symbolica::domains::rational::Rational;

// A resource allowance for distinct rounded results, not an estimate of native
// callback multiplicity: an alias body may execute repeatedly. Exact repeats
// consume no additional storage or certificate work. Exceeding this allowance
// fails the complete attempt; candidates are never dropped or tolerance-merged.
const MAX_CANDIDATES_PER_REQUEST: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Candidate {
    pub lambda: Rational,
    pub bits: u32,
}

#[derive(Clone, Debug)]
pub(crate) struct Requests(Arc<BTreeMap<Bundle, usize>>);

impl Requests {
    pub(crate) fn new(bundles: impl IntoIterator<Item = Bundle>) -> Result<Self, String> {
        let mut unique = BTreeMap::new();
        for bundle in bundles {
            if bundle.0.is_empty() {
                return Err("empty candidate request bundle".into());
            }
            let next = unique.len();
            if unique.insert(bundle, next).is_some() {
                return Err("duplicate candidate request identity".into());
            }
        }
        Ok(Self(Arc::new(unique)))
    }
    pub(crate) fn bundles(&self) -> impl Iterator<Item = (&Bundle, usize)> {
        self.0.iter().map(|(bundle, index)| (bundle, *index))
    }
    pub(crate) fn begin(&self) -> Attempt {
        Attempt {
            previous: ACTIVE.replace(Some(Active {
                requests: self.clone(),
                values: vec![BTreeSet::new(); self.0.len()],
                error: None,
            })),
            _thread: PhantomData,
        }
    }
}

struct Active {
    requests: Requests,
    values: Vec<BTreeSet<Candidate>>,
    error: Option<String>,
}

thread_local! {
    static ACTIVE:RefCell<Option<Active>>=const {RefCell::new(None)};
}

/// Dropping or unwinding an attempt always restores its surrounding context;
/// neither stale samples nor a failed lower-precision retry can leak outward.
pub(crate) struct Attempt {
    previous: Option<Active>,
    _thread: PhantomData<Rc<()>>,
}
impl Drop for Attempt {
    fn drop(&mut self) {
        ACTIVE.replace(self.previous.take());
    }
}
impl Attempt {
    pub(crate) fn finish(self) -> Result<Vec<(usize, Candidate)>, String> {
        let active = ACTIVE.take().ok_or("missing dynamic candidate attempt")?;
        if let Some(error) = active.error {
            return Err(error);
        }
        for (index, candidates) in active.values.iter().enumerate() {
            if candidates.is_empty() {
                return Err(format!(
                    "dynamic candidate request {index} was not evaluated"
                ));
            }
        }
        Ok(active
            .values
            .into_iter()
            .enumerate()
            .flat_map(|(index, candidates)| {
                candidates
                    .into_iter()
                    .map(move |candidate| (index, candidate))
            })
            .collect())
    }
}

pub(crate) fn record(bundle: &Bundle, lambda: Rational, bits: u32) -> Result<(), String> {
    ACTIVE.with_borrow_mut(|state| {
        let active = state
            .as_mut()
            .ok_or("checked dynamic callback requires an active candidate attempt")?;
        if let Some(error) = &active.error {
            return Err(error.clone());
        }
        let result = (|| {
            if lambda <= 0 || bits < 2 {
                return Err("invalid observed dynamic candidate".to_owned());
            }
            let index = *active
                .requests
                .0
                .get(bundle)
                .ok_or("unrecognized dynamic candidate request")?;
            let candidate = Candidate { lambda, bits };
            let values = &mut active.values[index];
            if values.contains(&candidate) {
                return Ok(());
            }
            if values.len() == MAX_CANDIDATES_PER_REQUEST {
                return Err(format!("dynamic candidate request exceeds the limit of {MAX_CANDIDATES_PER_REQUEST} distinct candidates"));
            }
            // Equivalent coefficient functions can follow different arithmetic
            // paths across native jets and Laurent outputs. Each call returns
            // its own rounded value and tracked uncertainty. Every distinct
            // actual centre/precision must reach the independent certificates;
            // this is observation deduplication, never numeric memoization.
            values.insert(candidate);
            Ok(())
        })();
        if let Err(error) = &result {
            active.error.get_or_insert_with(|| error.clone());
        }
        result
    })
}

#[cfg(test)]
mod tests {
    use super::super::requests::Request;
    use super::*;
    fn bundle(namespace: char) -> Bundle {
        Bundle(vec![Request {
            namespace: namespace.to_string().repeat(64),
            face: vec![],
        }])
    }
    #[test]
    fn candidate_attempts_deduplicate_exact_pairs_and_preserve_request_indices() {
        let a = bundle('a');
        let b = bundle('b');
        // Request indices follow admission order, not sorted Bundle order.
        let requests = Requests::new([b.clone(), a.clone()]).unwrap();
        let attempt = requests.begin();
        record(&b, Rational::from((1, 4)), 106).unwrap();
        record(&a, Rational::from((1, 2)), 53).unwrap();
        record(&a, Rational::from((1, 2)), 53).unwrap();
        record(&a, Rational::from((1, 2)), 106).unwrap();
        record(&a, Rational::from((1, 3)), 53).unwrap();
        let accepted = attempt.finish().unwrap();
        assert_eq!(accepted.len(), 4);
        assert_eq!(
            accepted[0],
            (
                0,
                Candidate {
                    lambda: Rational::from((1, 4)),
                    bits: 106
                }
            )
        );
        assert!(accepted[1..].iter().all(|(index, _)| *index == 1));
        assert_eq!(accepted[1].1.lambda, Rational::from((1, 3)));
        assert_eq!(accepted[2].1.bits, 53);
        assert_eq!(accepted[3].1.bits, 106);
    }
    #[test]
    fn missing_unknown_and_invalid_candidates_fail_the_whole_attempt() {
        let a = bundle('a');
        let requests = Requests::new([a.clone()]).unwrap();
        let attempt = requests.begin();
        assert!(record(&bundle('c'), Rational::from(1), 53).is_err());
        assert!(
            record(&a, Rational::from(1), 53)
                .unwrap_err()
                .contains("unrecognized")
        );
        assert!(attempt.finish().unwrap_err().contains("unrecognized"));
        assert!(
            requests
                .begin()
                .finish()
                .unwrap_err()
                .contains("not evaluated")
        );
        for (lambda, bits) in [
            (Rational::from(0), 53),
            (Rational::from(-1), 53),
            (Rational::from(1), 1),
        ] {
            let attempt = requests.begin();
            assert!(record(&a, lambda, bits).unwrap_err().contains("invalid"));
            assert!(attempt.finish().unwrap_err().contains("invalid"));
        }
        assert!(record(&a, Rational::from(1), 53).is_err());
    }
    #[test]
    fn candidate_limit_is_exact_and_overflow_is_sticky() {
        let a = bundle('a');
        let requests = Requests::new([a.clone()]).unwrap();
        for overflow in [false, true] {
            let attempt = requests.begin();
            for value in 1..=MAX_CANDIDATES_PER_REQUEST {
                record(&a, Rational::from(value as i64), 53).unwrap();
            }
            // Repetition at capacity neither consumes work nor fails.
            record(&a, Rational::from(1), 53).unwrap();
            if overflow {
                let error = record(&a, Rational::from(1), 106).unwrap_err();
                assert!(error.contains("limit of 1024"));
                assert_eq!(record(&a, Rational::from(1), 53).unwrap_err(), error);
                ACTIVE.with_borrow(|state| {
                    assert_eq!(
                        state.as_ref().unwrap().values[0].len(),
                        MAX_CANDIDATES_PER_REQUEST
                    );
                });
                assert_eq!(attempt.finish().unwrap_err(), error);
            } else {
                assert_eq!(attempt.finish().unwrap().len(), MAX_CANDIDATES_PER_REQUEST);
            }
        }
    }
    #[test]
    fn nested_retry_and_unwind_preserve_the_outer_candidate() {
        let a = bundle('a');
        let requests = Requests::new([a.clone()]).unwrap();
        let outer = requests.begin();
        record(&a, Rational::from((1, 3)), 53).unwrap();
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let _inner = requests.begin();
                record(&a, Rational::from((1, 5)), 192).unwrap();
                panic!("injected callback unwind");
            }))
            .is_err()
        );
        let retry = requests.begin();
        record(&a, Rational::from((1, 7)), 384).unwrap();
        assert_eq!(retry.finish().unwrap()[0].1.lambda, Rational::from((1, 7)));
        assert_eq!(outer.finish().unwrap()[0].1.lambda, Rational::from((1, 3)));
    }
    #[test]
    fn concurrent_attempts_do_not_share_candidates_or_failures() {
        let a = bundle('a');
        let requests = Requests::new([a.clone()]).unwrap();
        let attempt = requests.begin();
        record(&a, Rational::from((1, 3)), 53).unwrap();
        std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    let failed = requests.begin();
                    assert!(record(&bundle('b'), Rational::from(1), 53).is_err());
                    assert!(failed.finish().is_err());
                    let retry = requests.begin();
                    record(&a, Rational::from((1, 5)), 106).unwrap();
                    assert_eq!(retry.finish().unwrap()[0].1.lambda, Rational::from((1, 5)));
                })
                .join()
                .unwrap();
        });
        assert_eq!(
            attempt.finish().unwrap()[0].1.lambda,
            Rational::from((1, 3))
        );
    }
}
