//! The shared generate-then-check engine for random-character passwords.
//!
//! A [`Policy`] is an alphabet of printable ASCII characters, a length and
//! a set of constraints. Generation draws every character independently and
//! uniformly from the alphabet (rejection sampling, see [`crate::rng`]) and
//! then checks the whole candidate. A candidate that fails any constraint
//! is wiped and replaced by a completely fresh one. The output is therefore
//! uniformly distributed over the set of strings that satisfy the
//! constraints: no position is forced, and no class is favored.
//!
//! Before generating, the caller estimates the acceptance probability
//! (`acceptance_*`, [`Overlapping`]) and refuses policies below
//! [`MIN_ACCEPTANCE`], so the retry loop finishes quickly and
//! [`MAX_ATTEMPTS`] is not reached in practice.
//!
//! Each estimate is exact for its own kind of constraint: per-class minimums
//! over disjoint classes, overlapping `required` sets, the
//! `max-consecutive`/`max-sequential` runs, and `max-repeating`. Site rules
//! that combine kinds multiply the estimates, which assumes the constraints
//! are independent; they are not, so that product is an **approximation**.
//! It decides only which length to use and whether to refuse up front. The
//! output stays uniform over the qualifying strings either way, and a
//! badly overestimated combination ends in [`Error::RetryLimit`].

use zeroize::Zeroizing;

use crate::entropy::Entropy;
use crate::error::{Error, Result};
use crate::rng::{RandomSource, uniform_index};
use crate::secret::GeneratedPassword;

/// Policies whose estimated acceptance probability is below this are
/// refused (expected attempts above 1024).
pub(crate) const MIN_ACCEPTANCE: f64 = 1.0 / 1024.0;

/// Upper bound on candidates per call. With an acceptance probability of
/// at least 2^-10 the chance of reaching it is below e^-48. Exhausting it
/// costs at most `50_000 × 128` uniform draws; the draws come from a
/// buffered OS CSPRNG (one `getrandom` call per 64 draws, see
/// [`crate::rng::OsRandom`]).
pub(crate) const MAX_ATTEMPTS: u32 = 50_000;

/// Most distinct requirement sets the inclusion-exclusion estimate handles
/// (2^12 terms). Real site rules have far fewer (the vendored ones at
/// most 8 `required` properties).
pub(crate) const MAX_OVERLAPPING_REQUIREMENTS: usize = 12;

/// The space character, as a bit in an ASCII mask.
pub(crate) const SPACE_BIT: u128 = 1 << 0x20;

/// A set of ASCII characters as a bit mask: bit `c` is character `c`.
pub(crate) type AsciiMask = u128;

/// The number of characters in `mask`.
pub(crate) fn mask_len(mask: AsciiMask) -> u32 {
    mask.count_ones()
}

/// Whether byte `b` is in `mask`.
pub(crate) fn mask_contains(mask: AsciiMask, b: u8) -> bool {
    b < 128 && mask & (1u128 << b) != 0
}

/// The characters of `mask` in ascending order.
pub(crate) fn mask_bytes(mask: AsciiMask) -> Vec<u8> {
    (0u8..128).filter(|&b| mask_contains(mask, b)).collect()
}

/// "At least `min` characters from `members`".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Requirement {
    pub(crate) members: AsciiMask,
    pub(crate) min: u32,
}

/// Run-length constraints from site rules.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(
    clippy::struct_field_names,
    reason = "the names mirror the passwordrules properties"
)]
pub(crate) struct Runs {
    /// Longest allowed run of one repeated character (`aaa` is 3).
    pub(crate) max_consecutive: Option<u32>,
    /// Most occurrences of any one character in the whole password.
    pub(crate) max_repeating: Option<u32>,
    /// Longest allowed run of ascending or descending code points
    /// (`abc` and `321` are 3).
    pub(crate) max_sequential: Option<u32>,
}

impl Runs {
    /// Whether any limit can reject a string of `length` characters.
    pub(crate) fn binds(&self, length: u32) -> bool {
        [
            self.max_consecutive,
            self.max_repeating,
            self.max_sequential,
        ]
        .into_iter()
        .flatten()
        .any(|k| k < length)
    }
}

/// An alphabet, a length and constraints, ready to generate.
#[derive(Debug)]
pub(crate) struct Policy {
    alphabet: Vec<u8>,
    length: u32,
    requirements: Vec<Requirement>,
    runs: Runs,
}

impl Policy {
    /// `alphabet` must be a non-empty mask of printable ASCII.
    pub(crate) fn new(
        alphabet: AsciiMask,
        length: u32,
        requirements: Vec<Requirement>,
        runs: Runs,
    ) -> Self {
        Self {
            alphabet: mask_bytes(alphabet),
            length,
            requirements,
            runs,
        }
    }

    #[cfg(test)]
    pub(crate) fn length(&self) -> u32 {
        self.length
    }

    /// `log2(|alphabet|) × length`, an upper bound whenever a constraint
    /// can reject a candidate.
    pub(crate) fn entropy(&self) -> Entropy {
        let constrained =
            self.requirements.iter().any(|r| r.min > 0) || self.runs.binds(self.length);
        let size = u32::try_from(self.alphabet.len()).unwrap_or(u32::MAX);
        Entropy::for_characters(size, self.length, constrained)
    }

    /// Whether `candidate` meets every constraint.
    pub(crate) fn accepts(&self, candidate: &[u8]) -> bool {
        let requirements_met = self.requirements.iter().all(|r| {
            let count = candidate
                .iter()
                .filter(|&&b| mask_contains(r.members, b))
                .count();
            u32::try_from(count).unwrap_or(u32::MAX) >= r.min
        });
        requirements_met && runs_ok(&self.runs, candidate.iter().map(|&b| u32::from(b)))
    }

    /// Generates a password: fresh uniform candidates until one is accepted.
    ///
    /// # Errors
    /// [`Error::Randomness`] if the source fails, [`Error::RetryLimit`]
    /// after [`MAX_ATTEMPTS`] rejected candidates.
    pub(crate) fn generate<R: RandomSource + ?Sized>(
        &self,
        rng: &mut R,
    ) -> Result<GeneratedPassword> {
        let len = usize::try_from(self.length).map_err(|_| Error::RetryLimit)?;
        let mut candidate = Zeroizing::new(vec![0u8; len]);
        for _ in 0..MAX_ATTEMPTS {
            for slot in candidate.iter_mut() {
                let i = uniform_index(rng, self.alphabet.len())?;
                *slot = self.alphabet.get(i).copied().ok_or(Error::Randomness)?;
            }
            if self.accepts(&candidate) {
                let mut out = GeneratedPassword::with_capacity(len, self.entropy());
                for &b in candidate.iter() {
                    out.push(char::from(b));
                }
                return Ok(out);
            }
        }
        Err(Error::RetryLimit)
    }
}

/// Checks the run-length limits on `s` (a sequence of code points) in one
/// streaming pass, without copying `s`.
///
/// `s` is a secret (a candidate, or a user's password in
/// `PasswordRules::is_satisfied_by`), so the only per-character state kept
/// is wiped afterwards: the `max-repeating` counts live in a zeroizing
/// array for ASCII and, for other code points, in a zeroizing vector that
/// never reallocates without wiping its old buffer.
pub(crate) fn runs_ok(runs: &Runs, s: impl IntoIterator<Item = u32>) -> bool {
    if *runs == Runs::default() {
        return true;
    }
    let mut ascii_counts = Zeroizing::new([0u32; 128]);
    let mut other: Zeroizing<Vec<u32>> = Zeroizing::new(Vec::new());
    let mut prev: Option<u32> = None;
    // Lengths of the current equal, ascending and descending runs.
    let (mut same, mut up, mut down) = (0u32, 0u32, 0u32);
    for c in s {
        let (is_same, is_up, is_down) = prev.map_or((false, false, false), |p| {
            (
                p == c,
                c.checked_sub(p) == Some(1),
                p.checked_sub(c) == Some(1),
            )
        });
        same = if is_same { same.saturating_add(1) } else { 1 };
        up = if is_up { up.saturating_add(1) } else { 1 };
        down = if is_down { down.saturating_add(1) } else { 1 };
        prev = Some(c);
        if runs.max_consecutive.is_some_and(|k| same > k)
            || runs.max_sequential.is_some_and(|k| up > k || down > k)
        {
            return false;
        }
        if let Some(k) = runs.max_repeating {
            match usize::try_from(c)
                .ok()
                .and_then(|i| ascii_counts.get_mut(i))
            {
                Some(count) => {
                    *count = count.saturating_add(1);
                    if *count > k {
                        return false;
                    }
                }
                None => push_wiping(&mut other, c),
            }
        }
    }
    if let Some(k) = runs.max_repeating {
        // Non-ASCII code points: sort in place and count equal neighbours.
        other.sort_unstable();
        let mut run = 0u32;
        let mut last: Option<u32> = None;
        for &c in other.iter() {
            run = if last == Some(c) {
                run.saturating_add(1)
            } else {
                1
            };
            last = Some(c);
            if run > k {
                return false;
            }
        }
    }
    true
}

/// Pushes onto a zeroizing vector. When it is full, the contents move into
/// a new, larger zeroizing vector and the old buffer is wiped on drop
/// (`Vec::push` would free it unwiped).
fn push_wiping(v: &mut Zeroizing<Vec<u32>>, x: u32) {
    if v.len() == v.capacity() {
        let mut grown: Zeroizing<Vec<u32>> =
            Zeroizing::new(Vec::with_capacity(v.capacity().saturating_mul(2).max(16)));
        grown.extend_from_slice(v);
        *v = grown;
    }
    v.push(x);
}

/// `0! ..= n!` as `f64` (exact enough; 128! is about 3.9e215).
fn factorials(n: u32) -> Vec<f64> {
    let mut out = Vec::with_capacity(usize::try_from(n).unwrap_or(0).saturating_add(1));
    let mut acc = 1.0f64;
    out.push(acc);
    for i in 1..=n {
        acc *= f64::from(i);
        out.push(acc);
    }
    out
}

/// Exact probability that a uniform string of `length` characters over an
/// alphabet of `alphabet` characters meets every requirement, when the
/// requirement sets are pairwise **disjoint** (random-character mode).
///
/// Multinomial DP over the classes: `P = n! Σ Π (p_i^k_i / k_i!) ·
/// p_rest^r / r!` over all counts with `k_i ≥ min_i`.
pub(crate) fn acceptance_disjoint(alphabet: u32, requirements: &[Requirement], length: u32) -> f64 {
    let a = f64::from(alphabet);
    let fact = factorials(length);
    let n = usize::try_from(length).unwrap_or(0);
    let mut dp = vec![0.0f64; n + 1];
    if let Some(first) = dp.first_mut() {
        *first = 1.0;
    }
    let mut covered = 0u32;
    for r in requirements {
        let size = mask_len(r.members);
        covered = covered.saturating_add(size);
        let p = f64::from(size) / a;
        let min = usize::try_from(r.min).unwrap_or(usize::MAX);
        let mut next = vec![0.0f64; n + 1];
        for (j, &mass) in dp.iter().enumerate() {
            if mass == 0.0 {
                continue;
            }
            for k in min..=n.saturating_sub(j) {
                let term = p.powi(i32::try_from(k).unwrap_or(i32::MAX))
                    / fact.get(k).copied().unwrap_or(f64::INFINITY);
                if let Some(slot) = next.get_mut(j + k) {
                    *slot += mass * term;
                }
            }
        }
        dp = next;
    }
    let p_rest = f64::from(alphabet.saturating_sub(covered)) / a;
    let total: f64 = dp
        .iter()
        .enumerate()
        .map(|(j, &mass)| {
            let r = n - j;
            mass * p_rest.powi(i32::try_from(r).unwrap_or(i32::MAX))
                / fact.get(r).copied().unwrap_or(f64::INFINITY)
        })
        .sum();
    (total * fact.get(n).copied().unwrap_or(0.0)).clamp(0.0, 1.0)
}

/// The probability that a uniform string over `alphabet` contains at least
/// one character from each of some (possibly overlapping) sets, by
/// inclusion-exclusion: `P(n) = Σ_{S ⊆ sets} (-1)^|S| ((|A| - |∪S|) / |A|)^n`.
/// The terms do not depend on the length, so they are computed once.
///
/// The formula is exact; the `f64` sum of up to 2^12 alternating terms of
/// magnitude at most 1 has an absolute rounding error of order
/// `2^12 × 2^-53`, about `5e-13`, negligible next to [`MIN_ACCEPTANCE`]
/// (a test compares it with a cancellation-free computation).
pub(crate) struct Overlapping {
    /// `(fraction of the alphabet outside ∪S, |S| is odd)` per subset `S`.
    terms: Vec<(f64, bool)>,
}

impl Overlapping {
    /// `None` if there are more than [`MAX_OVERLAPPING_REQUIREMENTS`] sets.
    pub(crate) fn new(alphabet: AsciiMask, sets: &[AsciiMask]) -> Option<Self> {
        if sets.len() > MAX_OVERLAPPING_REQUIREMENTS {
            return None;
        }
        let a = f64::from(mask_len(alphabet));
        let terms = (0u32..(1u32 << sets.len()))
            .map(|subset| {
                let union = sets
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| subset & (1 << i) != 0)
                    .fold(0, |acc, (_, &m)| acc | (m & alphabet));
                let outside = f64::from(mask_len(alphabet & !union)) / a;
                (outside, subset.count_ones() % 2 == 1)
            })
            .collect();
        Some(Self { terms })
    }

    /// The acceptance probability for strings of `length` characters.
    pub(crate) fn probability(&self, length: u32) -> f64 {
        let n = i32::try_from(length).unwrap_or(i32::MAX);
        self.terms
            .iter()
            .map(|&(outside, odd)| {
                let term = outside.powi(n);
                if odd { -term } else { term }
            })
            .sum::<f64>()
            .clamp(0.0, 1.0)
    }
}

/// The probability that a uniform string over an alphabet has no run of
/// one repeated character longer than `max_consecutive` and no ascending or
/// descending run of code points longer than `max_sequential` (the
/// [`runs_ok`] checks), for any length up to the `max_len` it was built for.
/// Exact, except that limits with a negligible chance of being reached at
/// all are dropped (an error below `2 × RUN_NEGLIGIBLE`, 2e-12).
///
/// A Markov chain over (last character, the one run that is longer than 1,
/// its length): after an equal step the ascending and descending runs
/// restart, and so on, so at most one run is longer than 1 at a time. One
/// step per character, `O(|alphabet| × (max_consecutive + 2 ×
/// max_sequential))` each. Lengths are computed on demand and remembered,
/// so a length search pays only for the lengths it asks about.
pub(crate) struct RunsAcceptance {
    /// `probs[n]`: the probability at length `n`, for the lengths so far.
    probs: Vec<f64>,
    max_len: usize,
    /// `None` when no step can fail (no limits) or every step fails.
    chain: Option<RunChain>,
}

/// Run limits whose total chance of being broken is below this are not
/// tracked by [`RunsAcceptance`].
const RUN_NEGLIGIBLE: f64 = 1e-12;

/// The chain behind [`RunsAcceptance`]. Per character `d`, a block of
/// `stride` masses at `d × stride`: `[fresh, same[0..ws], up[0..wq],
/// down[0..wq]]`, where `fresh` has every run at length 1 and `same[r - 2]`
/// (likewise `up`, `down`) has the equal (ascending, descending) run at
/// length `r`.
struct RunChain {
    inv: f64,
    same_limited: bool,
    seq_limited: bool,
    ws: usize,
    wq: usize,
    stride: usize,
    /// The index of the character one code point below / above, if any.
    pred: Vec<Option<usize>>,
    succ: Vec<Option<usize>>,
    state: Vec<f64>,
    next: Vec<f64>,
}

/// One character's masses: `(fresh, same, up, down)`.
type Block<'a> = (f64, &'a [f64], &'a [f64], &'a [f64]);

/// A block split into its parts.
fn split_block(block: &[f64], ws: usize) -> Option<Block<'_>> {
    let (&fresh, rest) = block.split_first()?;
    let (same, rest) = rest.split_at_checked(ws)?;
    let (up, down) = rest.split_at_checked(rest.len() / 2)?;
    Some((fresh, same, up, down))
}

impl RunsAcceptance {
    pub(crate) fn new(
        alphabet: AsciiMask,
        max_consecutive: Option<u32>,
        max_sequential: Option<u32>,
        max_len: u32,
    ) -> Self {
        let n = usize::try_from(max_len).unwrap_or(0);
        let bytes = mask_bytes(alphabet);
        // A limit k is broken only by a run of k + 1, which has probability
        // at most |A|^-k per position. When even max_len positions together
        // stay below RUN_NEGLIGIBLE, the limit is not tracked: that keeps
        // the chain small (k ≤ 8 for 94 characters) and changes the result
        // by at most 2 × RUN_NEGLIGIBLE per limit (ascending and descending).
        let size = f64::from(u32::try_from(bytes.len()).unwrap_or(u32::MAX));
        let reachable = |k: &u32| {
            f64::from(max_len) * size.powi(-i32::try_from(*k).unwrap_or(i32::MAX)) >= RUN_NEGLIGIBLE
        };
        let max_consecutive = max_consecutive.filter(reachable);
        let max_sequential = max_sequential.filter(reachable);
        let impossible =
            bytes.is_empty() || max_consecutive == Some(0) || max_sequential == Some(0);
        let unlimited = max_consecutive.is_none() && max_sequential.is_none();
        let mut probs = vec![1.0];
        if impossible || unlimited {
            probs.resize(n + 1, if impossible { 0.0 } else { 1.0 });
            if let Some(first) = probs.first_mut() {
                *first = 1.0;
            }
            return Self {
                probs,
                max_len: n,
                chain: None,
            };
        }
        let a = bytes.len();
        let inv = 1.0 / f64::from(u32::try_from(a).unwrap_or(u32::MAX));
        // Run lengths 2..=k are tracked; no run is longer than the string.
        let tracked = |k: Option<u32>| {
            k.map_or(0, |k| {
                usize::try_from(k.min(max_len))
                    .unwrap_or(n)
                    .saturating_sub(1)
            })
        };
        let ws = tracked(max_consecutive);
        let wq = tracked(max_sequential);
        let stride = 1 + ws + 2 * wq;
        let index_of = |b: Option<u8>| b.and_then(|b| bytes.binary_search(&b).ok());
        let pred = bytes.iter().map(|&b| index_of(b.checked_sub(1))).collect();
        let succ = bytes.iter().map(|&b| index_of(b.checked_add(1))).collect();
        // Length 1: every character, all runs at length 1.
        let mut state = vec![0.0; a * stride];
        for block in state.chunks_exact_mut(stride) {
            if let Some(fresh) = block.first_mut() {
                *fresh = inv;
            }
        }
        if n >= 1 {
            probs.push(1.0);
        }
        Self {
            probs,
            max_len: n,
            chain: Some(RunChain {
                inv,
                same_limited: max_consecutive.is_some(),
                seq_limited: max_sequential.is_some(),
                ws,
                wq,
                stride,
                pred,
                succ,
                next: vec![0.0; state.len()],
                state,
            }),
        }
    }

    /// The probability at length `n` (0 beyond the `max_len` it was built
    /// for).
    pub(crate) fn probability(&mut self, n: u32) -> f64 {
        let Ok(n) = usize::try_from(n) else {
            return 0.0;
        };
        if n > self.max_len {
            return 0.0;
        }
        while self.probs.len() <= n {
            let Some(chain) = self.chain.as_mut() else {
                break;
            };
            self.probs.push(chain.step().clamp(0.0, 1.0));
        }
        self.probs.get(n).copied().unwrap_or(0.0)
    }
}

impl RunChain {
    /// Appends one character to every surviving string; returns the
    /// surviving mass.
    fn step(&mut self) -> f64 {
        let Self {
            inv,
            same_limited,
            seq_limited,
            ws,
            wq,
            stride,
            pred,
            succ,
            state,
            next,
        } = self;
        let (inv, ws, stride) = (*inv, *ws, *stride);
        // (fresh, Σ same, Σ up, Σ down) per character.
        let parts: Vec<[f64; 4]> = state
            .chunks_exact(stride)
            .map(|block| {
                split_block(block, ws).map_or([0.0; 4], |(f, s, u, d)| {
                    [f, s.iter().sum(), u.iter().sum(), d.iter().sum()]
                })
            })
            .collect();
        let total = |i: Option<usize>| {
            i.and_then(|i| parts.get(i))
                .map_or(0.0, |p| p.iter().sum::<f64>())
        };
        let all: f64 = parts.iter().flatten().sum();
        let block_of = |i: Option<usize>| {
            i.and_then(|i| state.get(i * stride..(i + 1) * stride))
                .and_then(|b| split_block(b, ws))
        };
        for (d, out) in next.chunks_exact_mut(stride).enumerate() {
            // Runs with no predecessor (no neighbour) stay at zero.
            out.fill(0.0);
            let below = pred.get(d).copied().flatten();
            let above = succ.get(d).copied().flatten();
            // Steps into d that start no limited run: from every character
            // except d itself (an equal step) and its neighbours (an
            // ascending or descending step), when those runs are limited.
            let mut excluded = 0.0;
            if *same_limited {
                excluded += total(Some(d));
            }
            if *seq_limited {
                excluded += total(below) + total(above);
            }
            let Some((fresh, rest)) = out.split_first_mut() else {
                continue;
            };
            *fresh = ((all - excluded) * inv).max(0.0);
            let Some((same, rest)) = rest.split_at_mut_checked(ws) else {
                continue;
            };
            let Some((up, down)) = rest.split_at_mut_checked(*wq) else {
                continue;
            };
            if let Some((f, s, u, dn)) = block_of(Some(d)) {
                // An equal step: the equal run grows, the others restart.
                let start = f + u.iter().sum::<f64>() + dn.iter().sum::<f64>();
                extend_run(same, start, s, inv);
            }
            if let Some((f, s, u, dn)) = block_of(below) {
                // An ascending step from d - 1.
                let start = f + s.iter().sum::<f64>() + dn.iter().sum::<f64>();
                extend_run(up, start, u, inv);
            }
            if let Some((f, s, u, dn)) = block_of(above) {
                // A descending step from d + 1.
                let start = f + s.iter().sum::<f64>() + u.iter().sum::<f64>();
                extend_run(down, start, dn, inv);
            }
        }
        core::mem::swap(state, next);
        state.iter().sum()
    }
}

/// One step of a tracked run: mass `start` begins a run of length 2, and a
/// run of length `r` in `previous` becomes `r + 1`. Runs past the limit
/// (the end of `target`) are dropped.
fn extend_run(target: &mut [f64], start: f64, previous: &[f64], inv: f64) {
    let mut carry = start;
    for (slot, &prev) in target.iter_mut().zip(previous) {
        *slot = carry * inv;
        carry = prev;
    }
}

/// The probability, for every length `0..=max_len` (the index), that no
/// character occurs more than `max_repeating` times in a uniform string over
/// `alphabet` characters:
/// `P(n) = n! [x^n] (Σ_{j ≤ k} (x/|A|)^j / j!)^|A|`. Exact, except that a
/// limit with a negligible chance of being broken (below `RUN_NEGLIGIBLE`)
/// counts as no limit.
pub(crate) fn acceptance_repeating(alphabet: u32, max_repeating: u32, max_len: u32) -> Vec<f64> {
    let n = usize::try_from(max_len).unwrap_or(0);
    if max_repeating >= max_len {
        return vec![1.0; n + 1];
    }
    let inv = 1.0 / f64::from(alphabet.max(1));
    // Union bound on breaking the limit: some character at some k + 1 of
    // the positions, |A| × C(max_len, k + 1) × |A|^-(k + 1). Below
    // RUN_NEGLIGIBLE the limit is ignored, which bounds the work below.
    let k1 = max_repeating.saturating_add(1);
    let binomial = (0..k1).fold(1.0f64, |acc, i| {
        acc * f64::from(max_len - i) / f64::from(i + 1)
    });
    let tail = f64::from(alphabet) * binomial * inv.powi(i32::try_from(k1).unwrap_or(i32::MAX));
    if tail < RUN_NEGLIGIBLE {
        return vec![1.0; n + 1];
    }
    let fact = factorials(max_len);
    let k = usize::try_from(max_repeating).unwrap_or(n);
    // One character's factor: (x/|A|)^j / j! for j = 0..=k.
    let factor: Vec<f64> = (0..=k)
        .map(|j| {
            inv.powi(i32::try_from(j).unwrap_or(i32::MAX))
                / fact.get(j).copied().unwrap_or(f64::INFINITY)
        })
        .collect();
    let mut poly = vec![0.0f64; n + 1];
    if let Some(first) = poly.first_mut() {
        *first = 1.0;
    }
    for _ in 0..alphabet {
        let mut next = vec![0.0f64; n + 1];
        for (i, &c) in poly.iter().enumerate() {
            if c == 0.0 {
                continue;
            }
            for (j, &f) in factor.iter().enumerate() {
                if let Some(slot) = next.get_mut(i + j) {
                    *slot += c * f;
                }
            }
        }
        poly = next;
    }
    poly.iter()
        .zip(&fact)
        .map(|(&c, &f)| (c * f).clamp(0.0, 1.0))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::SplitMix64;
    use crate::rng::testing::{Constant, Failing, assert_uniform};

    const fn bit(c: u8) -> AsciiMask {
        1u128 << c
    }

    fn mask_of(s: &str) -> AsciiMask {
        s.bytes().fold(0, |m, b| m | bit(b))
    }

    /// Brute force: the fraction of all strings over `alphabet` of length
    /// `n` that `policy` accepts.
    #[allow(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "small test sizes"
    )]
    fn brute_force(policy: &Policy) -> f64 {
        let a = policy.alphabet.len();
        let n = policy.length as usize;
        let total = a.pow(n as u32);
        let mut ok = 0usize;
        let mut buf = vec![0u8; n];
        for mut code in 0..total {
            for slot in &mut buf {
                *slot = policy.alphabet[code % a];
                code /= a;
            }
            if policy.accepts(&buf) {
                ok += 1;
            }
        }
        ok as f64 / total as f64
    }

    #[test]
    fn mask_helpers() {
        let m = mask_of("ba");
        assert_eq!(mask_bytes(m), b"ab");
        assert_eq!(mask_len(m), 2);
        assert!(mask_contains(m, b'a'));
        assert!(!mask_contains(m, b'c'));
        assert!(!mask_contains(m, 200));
    }

    fn ok(runs: &Runs, s: &str) -> bool {
        runs_ok(runs, s.chars().map(u32::from))
    }

    /// [`RunsAcceptance`] at every length `0..=max_len`, asked in a
    /// scrambled order to exercise the on-demand extension.
    fn runs_vec(
        alphabet: AsciiMask,
        max_consecutive: Option<u32>,
        max_sequential: Option<u32>,
        max_len: u32,
    ) -> Vec<f64> {
        let mut r = RunsAcceptance::new(alphabet, max_consecutive, max_sequential, max_len);
        let mid = r.probability(max_len / 2);
        let all: Vec<f64> = (0..=max_len).map(|n| r.probability(n)).collect();
        assert_eq!(all[(max_len / 2) as usize], mid);
        assert_eq!(r.probability(max_len + 1), 0.0);
        all
    }

    #[test]
    fn runs_checks() {
        let none = Runs::default();
        assert!(ok(&none, "aaaa"));
        let consecutive = Runs {
            max_consecutive: Some(2),
            ..Runs::default()
        };
        assert!(ok(&consecutive, "aabaa"));
        assert!(!ok(&consecutive, "abbb"));
        assert!(ok(&consecutive, ""));
        let repeating = Runs {
            max_repeating: Some(2),
            ..Runs::default()
        };
        assert!(ok(&repeating, "abab"));
        assert!(!ok(&repeating, "abaca"));
        let zero = Runs {
            max_repeating: Some(0),
            ..Runs::default()
        };
        assert!(!ok(&zero, "a"));
        assert!(!ok(&zero, "ä"));
        assert!(ok(&zero, ""));
        let sequential = Runs {
            max_sequential: Some(2),
            ..Runs::default()
        };
        assert!(ok(&sequential, "abxba"));
        assert!(!ok(&sequential, "xabc"));
        assert!(!ok(&sequential, "321"));
        assert!(!ok(&sequential, "9876a"));
        // An equal step ends an ascending run, and a run of one character
        // is not sequential.
        assert!(ok(&sequential, "abbc"));
        let three = Runs {
            max_sequential: Some(3),
            max_consecutive: Some(3),
            ..Runs::default()
        };
        assert!(ok(&three, "aabbbcd"));
        assert!(!ok(&three, "abcd"));
        assert!(!ok(&three, "bbbb"));
        assert!(
            Runs {
                max_consecutive: Some(3),
                ..Runs::default()
            }
            .binds(4)
        );
        assert!(
            !Runs {
                max_consecutive: Some(4),
                ..Runs::default()
            }
            .binds(4)
        );
        assert!(!none.binds(100));
    }

    #[test]
    fn disjoint_estimate_matches_brute_force() {
        let alphabet = mask_of("abcde01");
        for (reqs, n) in [
            (vec![(mask_of("ab"), 1), (mask_of("01"), 1)], 3),
            (vec![(mask_of("ab"), 2), (mask_of("01"), 1)], 4),
            (vec![(mask_of("abcde"), 1), (mask_of("01"), 2)], 4),
            (vec![(mask_of("abcde01"), 5)], 5),
            (vec![(mask_of("a"), 3)], 3),
            (vec![], 3),
        ] {
            let reqs: Vec<Requirement> = reqs
                .into_iter()
                .map(|(members, min)| Requirement { members, min })
                .collect();
            let p = acceptance_disjoint(7, &reqs, n);
            let policy = Policy::new(alphabet, n, reqs, Runs::default());
            let exact = brute_force(&policy);
            assert!((p - exact).abs() < 1e-9, "{p} vs {exact}");
        }
    }

    #[test]
    fn overlapping_estimate_matches_brute_force() {
        let alphabet = mask_of("abcd01");
        for (sets, n) in [
            (vec![mask_of("ab"), mask_of("bc"), mask_of("0")], 3),
            (
                vec![mask_of("a"), mask_of("b"), mask_of("c"), mask_of("d")],
                4,
            ),
            (
                vec![mask_of("a"), mask_of("b"), mask_of("c"), mask_of("d")],
                3,
            ),
            (vec![mask_of("abcd01")], 2),
            (vec![], 2),
        ] {
            let p = Overlapping::new(alphabet, &sets).unwrap().probability(n);
            let reqs = sets
                .iter()
                .map(|&members| Requirement { members, min: 1 })
                .collect();
            let exact = brute_force(&Policy::new(alphabet, n, reqs, Runs::default()));
            assert!((p - exact).abs() < 1e-9, "{p} vs {exact}");
        }
        let too_many = vec![bit(b'a'); MAX_OVERLAPPING_REQUIREMENTS + 1];
        assert!(Overlapping::new(alphabet, &too_many).is_none());
    }

    /// The run estimate is exact for `max-consecutive`, `max-sequential` and
    /// both together, at every length, over alphabets with and without
    /// neighbouring code points.
    #[test]
    fn runs_estimate_matches_brute_force() {
        let limits = [None, Some(0), Some(1), Some(2), Some(3), Some(9)];
        for alphabet in ["abc", "ab", "a", "ace", "abcx", "13579ab"] {
            let mask = mask_of(alphabet);
            let max_len = if alphabet.len() > 4 { 5 } else { 6 };
            for &max_consecutive in &limits {
                for &max_sequential in &limits {
                    let p = runs_vec(mask, max_consecutive, max_sequential, max_len);
                    assert_eq!(p.len(), max_len as usize + 1);
                    let runs = Runs {
                        max_consecutive,
                        max_sequential,
                        ..Runs::default()
                    };
                    for n in 1..=max_len {
                        let exact = brute_force(&Policy::new(mask, n, vec![], runs));
                        let got = p[n as usize];
                        assert!(
                            (got - exact).abs() < 1e-9,
                            "{alphabet} c={max_consecutive:?} s={max_sequential:?} n={n}: {got} vs {exact}"
                        );
                    }
                    assert_eq!(p[0], 1.0);
                }
            }
        }
        assert_eq!(runs_vec(mask_of("a"), Some(1), None, 0), vec![1.0]);
        // The widest case: 94 characters, every run length tracked up to 128.
        let printable: AsciiMask = (b'!'..=b'~').fold(0, |m, c| m | bit(c));
        let wide = runs_vec(printable, Some(128), Some(128), 128);
        assert!(
            wide.iter().all(|p| (p - 1.0).abs() < 1e-9),
            "no run can exceed 128"
        );
        let strict = runs_vec(printable, Some(1), Some(1), 128);
        assert!(strict.windows(2).all(|w| w[1] <= w[0]));
        assert!(strict[128] > 0.0 && strict[128] < 1.0);
    }

    #[test]
    fn repeating_estimate_matches_brute_force() {
        for alphabet in ["ab", "abc", "abcd", "a"] {
            let mask = mask_of(alphabet);
            let a = mask_len(mask);
            for k in 0..=4u32 {
                let p = acceptance_repeating(a, k, 6);
                let runs = Runs {
                    max_repeating: Some(k),
                    ..Runs::default()
                };
                for n in 0..=6u32 {
                    let exact = if n == 0 {
                        1.0
                    } else {
                        brute_force(&Policy::new(mask, n, vec![], runs))
                    };
                    let got = p[n as usize];
                    assert!(
                        (got - exact).abs() < 1e-9,
                        "{alphabet} k={k} n={n}: {got} vs {exact}"
                    );
                }
            }
        }
        // Ten characters, each at most once: 10! / 10^10 at length 10.
        let p = acceptance_repeating(10, 1, 11);
        assert!((p[10] - 3_628_800.0 / 1e10).abs() < 1e-12);
        assert_eq!(p[11], 0.0);
        // The finding's case: digits at most 13 times each in 128 positions.
        assert!(acceptance_repeating(10, 13, 128)[128] < MIN_ACCEPTANCE);
    }

    /// The independence product is an approximation: compare it with brute
    /// force for required sets combined with run limits. Over these small
    /// alphabets it stays within a factor of 2 either way, well inside the
    /// margin between [`MIN_ACCEPTANCE`] and [`MAX_ATTEMPTS`].
    #[test]
    #[allow(clippy::cast_possible_truncation, reason = "small test sizes")]
    fn combined_estimate_is_close() {
        let alphabet = mask_of("abc01");
        for (sets, runs, n) in [
            (
                vec![mask_of("ab"), mask_of("01")],
                Runs {
                    max_consecutive: Some(1),
                    ..Runs::default()
                },
                5,
            ),
            (
                vec![mask_of("a"), mask_of("0")],
                Runs {
                    max_consecutive: Some(2),
                    max_sequential: Some(2),
                    ..Runs::default()
                },
                6,
            ),
            (
                vec![mask_of("abc"), mask_of("1")],
                Runs {
                    max_repeating: Some(2),
                    ..Runs::default()
                },
                6,
            ),
        ] {
            let reqs: Vec<Requirement> = sets
                .iter()
                .map(|&members| Requirement { members, min: 1 })
                .collect();
            let exact = brute_force(&Policy::new(alphabet, n, reqs, runs));
            let p_required = Overlapping::new(alphabet, &sets).unwrap().probability(n);
            let p_runs =
                runs_vec(alphabet, runs.max_consecutive, runs.max_sequential, n)[n as usize];
            let p_repeat = runs.max_repeating.map_or(1.0, |k| {
                acceptance_repeating(mask_len(alphabet), k, n)[n as usize]
            });
            let estimate = p_required * p_runs * p_repeat;
            let ratio = estimate / exact;
            assert!((0.5..2.0).contains(&ratio), "{estimate} vs {exact}");
        }
    }

    /// Inclusion-exclusion over 12 overlapping sets in `f64` against a
    /// cancellation-free computation: a DP over which sets the string has
    /// hit so far, which adds only non-negative terms.
    #[test]
    fn twelve_overlapping_sets_lose_no_precision() {
        // 24 characters, each set two of them, overlapping in a ring.
        let chars: Vec<u8> = (b'a'..=b'x').collect();
        let alphabet = chars.iter().fold(0, |m, &c| m | bit(c)) | mask_of("0123456789");
        let sets: Vec<AsciiMask> = (0..12)
            .map(|i| bit(chars[2 * i]) | bit(chars[(2 * i + 3) % 24]))
            .collect();
        let overlapping = Overlapping::new(alphabet, &sets).unwrap();
        let hits: Vec<usize> = mask_bytes(alphabet)
            .into_iter()
            .map(|c| {
                sets.iter()
                    .enumerate()
                    .filter(|(_, s)| mask_contains(**s, c))
                    .fold(0usize, |m, (i, _)| m | (1 << i))
            })
            .collect();
        let size = f64::from(u32::try_from(hits.len()).unwrap());
        let mut dp = vec![0.0f64; 1 << 12];
        dp[0] = 1.0;
        for n in 1..=60u32 {
            let mut next = vec![0.0f64; 1 << 12];
            for (state, &mass) in dp.iter().enumerate() {
                if mass == 0.0 {
                    continue;
                }
                for &hit in &hits {
                    next[state | hit] += mass / size;
                }
            }
            dp = next;
            let exact = dp[(1 << 12) - 1];
            let p = overlapping.probability(n);
            assert!(
                (p - exact).abs() < 1e-12,
                "n={n}: inclusion-exclusion {p} vs {exact}"
            );
        }
    }

    /// Generate-then-check is uniform over the accepted strings: alphabet
    /// `ab01`, length 3, at least one letter and one digit. 48 of the 64
    /// strings qualify and each must be equally likely.
    #[test]
    fn chi_squared_uniform_over_accepted_strings() {
        let alphabet = mask_of("ab01");
        let reqs = vec![
            Requirement {
                members: mask_of("ab"),
                min: 1,
            },
            Requirement {
                members: mask_of("01"),
                min: 1,
            },
        ];
        let policy = Policy::new(alphabet, 3, reqs, Runs::default());
        let accepted: Vec<Vec<u8>> = (0..64usize)
            .map(|code| (0..3).map(|i| b"ab01"[(code >> (2 * i)) & 3]).collect())
            .filter(|s: &Vec<u8>| policy.accepts(s))
            .collect();
        assert_eq!(accepted.len(), 48);
        let mut counts = vec![0u64; 48];
        let mut rng = SplitMix64::new(0x0123_4567_89ab_cdef);
        for _ in 0..48 * 2_000 {
            let p = policy.generate(&mut rng).unwrap();
            let pos = accepted
                .iter()
                .position(|s| s.as_slice() == p.expose_secret().as_bytes())
                .unwrap();
            counts[pos] += 1;
        }
        assert_uniform(&counts, "generate-then-check over 48 strings");
    }

    /// The same check rejects a forced-position generator (put a letter in
    /// position 0 and a digit in position 1, fill the rest freely), which is
    /// the biased technique this crate avoids.
    #[test]
    fn chi_squared_detects_forced_positions() {
        let alphabet = *b"ab01";
        let policy = Policy::new(
            mask_of("ab01"),
            3,
            vec![
                Requirement {
                    members: mask_of("ab"),
                    min: 1,
                },
                Requirement {
                    members: mask_of("01"),
                    min: 1,
                },
            ],
            Runs::default(),
        );
        let accepted: Vec<Vec<u8>> = (0..64usize)
            .map(|code| (0..3).map(|i| alphabet[(code >> (2 * i)) & 3]).collect())
            .filter(|s: &Vec<u8>| policy.accepts(s))
            .collect();
        let mut counts = vec![0u64; 48];
        let mut rng = SplitMix64::new(99);
        for _ in 0..48 * 2_000 {
            let letter = b"ab"[uniform_index(&mut rng, 2).unwrap()];
            let digit = b"01"[uniform_index(&mut rng, 2).unwrap()];
            let free = alphabet[uniform_index(&mut rng, 4).unwrap()];
            let mut s = vec![letter, digit, free];
            // Even shuffling the positions afterwards stays biased.
            let j = uniform_index(&mut rng, 3).unwrap();
            s.swap(0, j);
            let pos = accepted.iter().position(|a| *a == s).unwrap();
            counts[pos] += 1;
        }
        let chi2 = crate::rng::testing::chi_squared(&counts);
        assert!(
            chi2 > crate::rng::testing::chi_squared_critical(47),
            "forced positions should look biased, chi^2 = {chi2}"
        );
    }

    #[test]
    fn retry_limit_and_rng_failure() {
        // The alphabet is sorted ("0a"); a constant odd draw always picks
        // index 1, so every candidate is "aaaa", which never has a digit.
        let policy = Policy::new(
            mask_of("a0"),
            4,
            vec![Requirement {
                members: mask_of("0"),
                min: 1,
            }],
            Runs::default(),
        );
        assert_eq!(
            policy.generate(&mut Constant(1)).unwrap_err(),
            Error::RetryLimit
        );
        assert_eq!(
            policy.generate(&mut Failing).unwrap_err(),
            Error::Randomness
        );
    }

    #[test]
    fn generated_buffer_is_exact_size() {
        let policy = Policy::new(mask_of("abc"), 17, vec![], Runs::default());
        let p = policy.generate(&mut SplitMix64::new(1)).unwrap();
        assert_eq!(p.expose_secret().len(), 17);
        assert_eq!(p.capacity(), 17);
        assert!(!p.entropy().is_upper_bound());
        assert_eq!(policy.length(), 17);
    }
}
