//! Native selected-root arithmetic; no geometric proof or source graph is resident.
use super::{RootProgram, failed, resolve};
use std::sync::{Arc, LazyLock, Mutex};
use symbolica::{
    atom::{Atom, AtomCore, AtomView, EvalFn, EvaluationInfo, Symbol},
    domains::float::{
        Complex, DoubleFloat, ErrorPropagatingFloat, Float, Real, RealLike, SingleFloat,
    },
    evaluate::{EvaluationDomain, ExpressionEvaluator},
    solve::{BracketedRootConvergence, BracketedRootOptions, nsolve_bracketed},
    symbol,
};
pub(super) static ROOT: LazyLock<Symbol> = LazyLock::new(|| {
    symbol!(
        "fastsecdec::algebraic::selected_root_v1",
        der = |view, index, out| {
            let Some(args) = view.as_fun_view() else {
                return;
            };
            if index < 3 {
                **out = Atom::zero();
                return;
            }
            let Some(count) = args.get_nargs().checked_sub(3) else {
                return;
            };
            if index >= args.get_nargs() || count < 2 {
                return;
            }
            let root = view.to_owned();
            let derivative: Atom = (1..count)
                .map(|i| Atom::num(i) * args.get(i + 3).to_owned() * root.pow((i - 1) as i64))
                .sum();
            **out = -root.pow((index - 3) as i64) / derivative;
        },
        eval = EvaluationInfo::new()
            .with_tags(3)
            .register_tagged(real::<f64, 1>)
            .register_tagged(real::<DoubleFloat, 1>)
            .register_tagged(real::<Float, 1>)
            .register_tagged(complex::<f64, 1>)
            .register_tagged(complex::<DoubleFloat, 1>)
            .register_tagged(complex::<Float, 1>)
            .register_tagged(real::<ErrorPropagatingFloat<f64>, 1>)
            .register_tagged(real::<ErrorPropagatingFloat<Float>, 1>)
            .register_tagged(complex::<ErrorPropagatingFloat<f64>, 1>)
            .register_tagged(complex::<ErrorPropagatingFloat<Float>, 1>)
    )
});
pub(super) static ROOT_V2: LazyLock<Symbol> = LazyLock::new(|| {
    symbol!(
        "fastsecdec::algebraic::selected_root_v2",
        der = |view, index, out| {
            let Some(args) = view.as_fun_view() else {
                return;
            };
            if index < 3 {
                **out = Atom::zero();
                return;
            }
            let Some(count) = args.get_nargs().checked_sub(5) else {
                return;
            };
            if index >= args.get_nargs() || count < 2 {
                return;
            }
            if index >= count + 3 {
                // Source-derived endpoint controls do not define an independent
                // coefficient. Public construction derives them from c(x).
                **out = Atom::zero();
                return;
            }
            let root = view.to_owned();
            let derivative: Atom = (1..count)
                .map(|i| Atom::num(i) * args.get(i + 3).to_owned() * root.pow((i - 1) as i64))
                .sum();
            **out = -root.pow((index - 3) as i64) / derivative;
        },
        eval = EvaluationInfo::new()
            .with_tags(3)
            .register_tagged(real::<f64, 2>)
            .register_tagged(real::<DoubleFloat, 2>)
            .register_tagged(real::<Float, 2>)
            .register_tagged(complex::<f64, 2>)
            .register_tagged(complex::<DoubleFloat, 2>)
            .register_tagged(complex::<Float, 2>)
            .register_tagged(real::<ErrorPropagatingFloat<f64>, 2>)
            .register_tagged(real::<ErrorPropagatingFloat<Float>, 2>)
            .register_tagged(complex::<ErrorPropagatingFloat<f64>, 2>)
            .register_tagged(complex::<ErrorPropagatingFloat<Float>, 2>)
    )
});
// Same minimal precision-constructor adapter as existing contour callbacks;
// arithmetic, root iteration, and derivatives remain native owner operations.
pub trait Number: Real + RealLike + PartialOrd + EvaluationDomain + Send + Sync {
    fn one_at(bits: u32) -> Self;
    fn invalid(bits: u32) -> Self;
    fn exact_zero_at(bits: u32) -> Self {
        Self::one_at(bits).from_rational(&symbolica::domains::rational::Rational::zero())
    }
    fn valid(&self) -> bool {
        self.is_finite()
    }
    fn exact_control(&self) -> bool {
        true
    }
    fn rational_at(value: &symbolica::domains::rational::Rational, bits: u32) -> Self {
        Self::one_at(bits).from_rational(value)
    }
}
impl Number for f64 {
    fn one_at(_: u32) -> Self {
        1.
    }
    fn invalid(_: u32) -> Self {
        f64::NAN
    }
}
impl Number for DoubleFloat {
    fn one_at(_: u32) -> Self {
        Self::from(1.)
    }
    fn invalid(_: u32) -> Self {
        Self::from(f64::NAN)
    }
}
impl Number for Float {
    fn one_at(bits: u32) -> Self {
        Self::with_val(bits, 1)
    }
    fn invalid(bits: u32) -> Self {
        Self::with_val(bits, f64::NAN)
    }
}
impl<T: Number> Number for ErrorPropagatingFloat<T>
where
    Self: EvaluationDomain,
{
    fn one_at(bits: u32) -> Self {
        Self::new_with_accuracy(T::one_at(bits), f64::INFINITY)
    }
    fn invalid(bits: u32) -> Self {
        Self::new_with_accuracy(T::invalid(bits), f64::INFINITY)
    }
    fn valid(&self) -> bool {
        self.is_finite() && self.get_absolute_error().is_finite()
    }
    fn exact_control(&self) -> bool {
        self.get_absolute_error() == 0.
    }
    fn rational_at(value: &symbolica::domains::rational::Rational, bits: u32) -> Self {
        let center = T::rational_at(value, bits);
        // Native exact round-trip proves this immutable literal needs no
        // rounding uncertainty in every supported domain (at least 53 bits).
        // Runtime coefficient/input uncertainties do not pass through here.
        let primary = value.to_f64();
        if T::FIXED_PRECISION.unwrap_or(bits) >= 53
            && primary.is_finite()
            && symbolica::domains::rational::Rational::try_from(primary)
                .ok()
                .as_ref()
                == Some(value)
        {
            Self::new_with_accuracy(center, f64::INFINITY)
        } else {
            Self::new(center, f64::from(bits) * std::f64::consts::LOG10_2 - 2.)
        }
    }
}
pub(super) fn endpoint_control<T: Number>(coordinate: &T, one: &T) -> Option<usize> {
    if !coordinate.exact_control() {
        None
    } else if coordinate.is_zero() {
        Some(0)
    } else if coordinate == one {
        Some(1)
    } else {
        None
    }
}
#[derive(Clone)]
struct Scratch<T> {
    program: ExpressionEvaluator<Complex<T>>,
    input: Vec<Complex<T>>,
    output: Vec<Complex<T>>,
    endpoints: [Complex<T>; 2],
    options: BracketedRootOptions<T>,
    lower: T,
    upper: T,
}
struct Workspace<T> {
    owner: Arc<RootProgram>,
    scratch: Mutex<Scratch<T>>,
    bits: u32,
    poisoned: bool,
}
impl<T: Clone> Clone for Workspace<T> {
    fn clone(&self) -> Self {
        let (scratch, poisoned) = match self.scratch.lock() {
            Ok(s) => (s.clone(), self.poisoned),
            Err(e) => (e.into_inner().clone(), true),
        };
        Self {
            owner: self.owner.clone(),
            scratch: Mutex::new(scratch),
            bits: self.bits,
            poisoned,
        }
    }
}
impl<T: Number> Workspace<T>
where
    Complex<T>: EvaluationDomain,
{
    fn new(tags: &[AtomView<'_>], version: u32) -> std::result::Result<Self, String> {
        let (owner, requested) = resolve(tags)?;
        if owner.version != version {
            return Err("root callback recipe differs from its selected owner".into());
        }
        let bits = T::FIXED_PRECISION.unwrap_or(requested);
        if bits < 53 {
            return Err("static root bracket recipe requires at least 53 bits".into());
        }
        let one = T::one_at(bits);
        let zero = T::exact_zero_at(bits);
        let program = owner.exact.clone().map_coeff_with_prec(
            &|c| Complex::new(T::rational_at(&c.re, bits), T::rational_at(&c.im, bits)),
            bits,
        );
        // This power of two is a known exact solver control, not computed
        // density data. Construct it natively before tracked arithmetic.
        let tolerance = one.from_rational(
            &symbolica::domains::rational::Rational::from((1, 2))
                .pow(u64::from(bits.saturating_sub(5))),
        );
        let scratch = Scratch {
            program,
            input: vec![Complex::new(zero.clone(), zero.clone()); owner.degree + 2],
            output: vec![
                Complex::new(zero.clone(), zero.clone());
                if version == 1 { 2 } else { 3 }
            ],
            endpoints: [
                Complex::new(zero.clone(), zero.clone()),
                Complex::new(zero.clone(), zero.clone()),
            ],
            lower: if version == 1 {
                one.from_rational(&owner.lower)
            } else {
                zero.clone()
            },
            upper: if version == 1 {
                one.from_rational(&owner.upper)
            } else {
                one
            },
            options: BracketedRootOptions {
                absolute_tolerance: zero,
                relative_tolerance: tolerance,
                max_iterations: (bits as usize).saturating_mul(4).max(256),
                initial_guess: None,
                convergence: BracketedRootConvergence::Bracket,
            },
        };
        Ok(Self {
            owner,
            scratch: Mutex::new(scratch),
            bits,
            poisoned: false,
        })
    }
    fn evaluate(
        &self,
        count: usize,
        value: impl Fn(usize) -> Complex<T>,
    ) -> std::result::Result<Complex<T>, String> {
        if self.poisoned {
            return Err("regular-section callback scratch poisoned".into());
        }
        let coefficient_count = self.owner.degree + 1;
        let affine = self.owner.version == 2;
        if count != coefficient_count + if affine { 2 } else { 0 } {
            return Err("regular-section coefficient shape".into());
        }
        let mut s = self
            .scratch
            .lock()
            .map_err(|_| "regular-section callback scratch poisoned")?;
        for i in 0..count {
            let v = value(i);
            if !v.re.is_finite() || !v.im.is_finite() || !v.im.is_zero() {
                return Err("regular-section inputs must have finite real centres".into());
            }
            if !v.re.valid() || !v.im.valid() {
                return Err(
                    "regular-section coefficient uncertainty exceeds native tracking range".into(),
                );
            }
            if i < coefficient_count {
                s.input[i + 1] = v;
            } else {
                s.endpoints[i - coefficient_count] = v;
            }
        }
        if !s.lower.valid() || !s.upper.valid() || !s.options.relative_tolerance.valid() {
            return Err("regular-section precision/error-tracking range is unsupported".into());
        }
        if self.owner.degree == 1 && !affine {
            if s.input[2].re.is_zero() {
                return Err("regular-section linear leading coefficient vanished".into());
            }
            let root = -s.input[1].clone() / s.input[2].clone();
            if !root.re.valid()
                || !root.im.valid()
                || !root.im.is_zero()
                || root.re < s.lower
                || root.re > s.upper
            {
                return Err("regular-section linear root outside bracket".into());
            }
            return Ok(root);
        }
        let Scratch {
            program,
            input,
            output,
            endpoints,
            lower,
            upper,
            options,
        } = &mut *s;
        let mut invalid_tracking = false;
        let solved = nsolve_bracketed(lower.clone(), upper.clone(), options, |z| {
            if !z.valid() {
                invalid_tracking = true;
                return (T::invalid(self.bits), T::invalid(self.bits));
            }
            input[0] = Complex::new(z.clone(), T::exact_zero_at(self.bits));
            if program.try_evaluate(input, output).is_err() {
                return (T::invalid(self.bits), T::invalid(self.bits));
            }
            if affine && let Some(index) = endpoint_control(z, upper) {
                output[0] = endpoints[index].clone();
            }
            if output.iter().any(|v| !v.re.valid() || !v.im.valid()) {
                invalid_tracking = true;
                return (T::invalid(self.bits), T::invalid(self.bits));
            }
            (output[0].re.clone(), output[1].re.clone())
        });
        if invalid_tracking {
            return Err("regular-section root refinement exceeded native uncertainty range".into());
        }
        let root = solved
            .map_err(|e| {
                format!(
                    "regular-section native root refinement at {} bits: {e}",
                    self.bits
                )
            })?
            .root;
        if !root.valid() || root < *lower || root > *upper {
            return Err("regular-section nonfinite/outside bracket root".into());
        }
        // Reuse contour's native correction: retain imaginary uncertainty whose centre is zero.
        input[0] = Complex::new(root.clone(), T::exact_zero_at(self.bits));
        program
            .try_evaluate(input, output)
            .map_err(|e| e.to_string())?;
        if affine && let Some(index) = endpoint_control(&root, upper) {
            output[0] = endpoints[index].clone();
        }
        if output.iter().any(|v| !v.re.valid() || !v.im.valid()) {
            return Err(
                "regular-section corrected evaluation exceeded native uncertainty range".into(),
            );
        }
        let mut correction = output[0].clone() / &output[1];
        let physical = if affine {
            // Helper derivative is Q_s=w*P_z. The returned root is physical:
            // imaginary sensitivity is -P/P_z, hence the explicit factor w.
            correction *= Complex::new(
                T::rational_at(&(&self.owner.upper - &self.owner.lower), self.bits),
                T::exact_zero_at(self.bits),
            );
            output[2].re.clone()
        } else {
            root
        };
        let answer = Complex::new(physical, -correction.im);
        if !answer.re.valid() || !answer.im.valid() || !answer.im.is_zero() {
            return Err("regular-section invalid corrected root".into());
        }
        Ok(answer)
    }
}

fn real<T: Number, const VERSION: u32>(tags: &[AtomView<'_>]) -> EvalFn<T>
where
    Complex<T>: EvaluationDomain,
{
    let workspace = Workspace::<T>::new(tags, VERSION);
    Box::new(move |values: &[T]| {
        let result = match &workspace {
            Ok(w) => w
                .evaluate(values.len(), |i| {
                    Complex::new(values[i].clone(), T::exact_zero_at(w.bits))
                })
                .map(|v| v.re),
            Err(e) => Err(e.clone()),
        };
        result.unwrap_or_else(|e| {
            failed(e);
            T::invalid(workspace.as_ref().map_or(53, |w| w.bits))
        })
    })
}
fn complex<T: Number, const VERSION: u32>(tags: &[AtomView<'_>]) -> EvalFn<Complex<T>>
where
    Complex<T>: EvaluationDomain,
{
    let workspace = Workspace::<T>::new(tags, VERSION);
    Box::new(move |values: &[Complex<T>]| {
        let result = match &workspace {
            Ok(w) => w.evaluate(values.len(), |i| values[i].clone()),
            Err(e) => Err(e.clone()),
        };
        result.unwrap_or_else(|e| {
            failed(e);
            let v = T::invalid(workspace.as_ref().map_or(53, |w| w.bits));
            Complex::new(v.clone(), v)
        })
    })
}
