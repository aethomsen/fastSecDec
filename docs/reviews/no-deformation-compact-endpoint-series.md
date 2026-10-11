# Shared native endpoint Taylor preparation

The normal-numerator preparation now computes admitted moving-root and finite
source-argument series once per chart. A single Symbolica function-series
template carries a normal-independent record tag, substituted exactly for each
original numerator. Demand-driven source derivative definitions retain their
existing owner and the original numerator bodies remain composed.

The public API/source audit and focused Rust probes reuse Symbolica's
`Atom::series`, `Atom::derivative`, native implicit root hooks, literal
replacement and `Integer::factorial`. Earlier triangular-root jets enter later
root arguments; differentiation happens before restriction to the face.
The outer native series supplies mixed source derivatives and factorials.
There is no new CAS, automatic differentiation, root solver or codec.

Finite-argument, selected-branch, closed-minor and denominator admission are
unchanged. The tag avoids the entire source/chart/section symbol inventory.
The mixed-face implementation is unchanged. Cancellation and node/time limits
are checked around native operations; an individual native symbolic call is
still cooperative and cannot be interrupted by these checks.

Independent source, mathematical and ecosystem review passed. Maintained tests
compare all coefficients through cubic order against the original native
composed series for two source tags and a moving root. They also test resource
refusal and cancellation. All 30 numerator tests pass, including inherited
complex coefficients, mixed derivatives, finite-zero integer powers and native
codec restoration. Strict workspace/all-target Clippy passes.

The exploratory physical-geometry arithmetic probe completed in 1.929 seconds
with about 115 MiB child peak RSS, after the previous untruncated approach hit
its limit. Separate exact root-equation residuals and cubic mutation controls
passed. This is preparation evidence, not a complete physical integral or a
production-owner benchmark. The new physical-source run remains a separate gate;
complete closed-face continuation and the general endpoint atlas are unresolved.

Review identity: `d5e3e84690a7b769f2b8b456ff2fa272fb8daafdd9712cb6d98f2a2a16d68dc3`.
Final source inventory: `bcf14b549f475b0f6f921f208e7920d2825c7bd5b6577690227ab5cc6d1cccaf`.
The local raw receipts and proof payloads are intentionally untracked.
