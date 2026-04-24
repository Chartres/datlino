//! FSRS — Free Spaced Repetition Scheduler (Jarrett Ye 2022+).
//!
//! The brief flagged plain recency decay as the Week-1 placeholder;
//! proper spaced repetition is what "learning how to learn" literature
//! (Bjork, Roediger, Dunlosky) consistently identifies as the single
//! biggest lever for long-term retention. FSRS is the state-of-the-art
//! open scheduler — it replaces SM-2 / Anki's default with a
//! two-variable model of *stability* (how long you'll remember) and
//! *difficulty* (how hard this item is for you).
//!
//! Our adaptation for Datlino: one row of state per (user, chunk).
//! After each typing attempt we grade the chunk on a 1–4 scale derived
//! from accuracy and WPM vs the lesson target, feed it into the
//! update rule, and write back the new stability / difficulty /
//! due_at. The session generator prefers chunks whose `due_at` is in
//! the past (overdue cards first), falling back to novel chunks when
//! the due queue is empty.
//!
//! We use the FSRS-4.5 simplified parameter set. They're tuned on the
//! reference Anki dataset; they won't be optimal for typing but are a
//! sane starting point that beats naive recency. A learner-specific
//! re-fit is a later feature.
//!
//! References: Jarrett Ye, "A stochastic shortest path algorithm for
//! spaced repetition", 2022; open implementation at github.com/open-spaced-repetition/fsrs.

use serde::Serialize;

/// 17 FSRS-4.5 weights. Ordered as in the reference implementation.
/// These are the community default; we can re-fit per-user once we
/// have enough observations.
const W: [f64; 17] = [
    0.4072, 1.1829, 3.1262, 15.4722, 7.2102, 0.5316, 1.0651, 0.0234, 1.616, 0.1544, 1.0824,
    1.9813, 0.0953, 0.2975, 2.2042, 0.2407, 2.9466,
];

/// Retention target — how likely the student should be to recall a
/// due item. 0.9 is the FSRS default; higher means more reviews.
const REQUEST_RETENTION: f64 = 0.9;

/// A grade from the most recent attempt — higher is better.
///   Again = 1 — wrong or barely typed
///   Hard  = 2 — finished but under the accuracy/WPM target
///   Good  = 3 — hit the target
///   Easy  = 4 — significantly above target (faster and cleaner)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[repr(u8)]
pub enum Grade {
    Again = 1,
    Hard = 2,
    Good = 3,
    Easy = 4,
}

impl Grade {
    fn as_f64(self) -> f64 {
        self as u8 as f64
    }
}

/// Convert an attempt's accuracy % + WPM into a 1–4 grade against the
/// lesson's / content's target. The thresholds are generous: perfectly
/// typed = Easy, target-hit = Good, below target but finished = Hard,
/// anything worse = Again.
pub fn grade_from_attempt(
    accuracy_pct: f64,
    wpm: f64,
    target_accuracy: f64,
    target_wpm: f64,
) -> Grade {
    if accuracy_pct < 60.0 {
        return Grade::Again;
    }
    let acc_ratio = accuracy_pct / target_accuracy.max(1.0);
    let wpm_ratio = wpm / target_wpm.max(1.0);
    let combined = (acc_ratio + wpm_ratio) / 2.0;
    if combined >= 1.25 && accuracy_pct >= 95.0 {
        Grade::Easy
    } else if combined >= 1.0 {
        Grade::Good
    } else if accuracy_pct >= 70.0 {
        Grade::Hard
    } else {
        Grade::Again
    }
}

/// Per (user, chunk) scheduler state. `due_at` is a unix timestamp in
/// seconds — the session generator orders overdue chunks first.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct State {
    /// Memory stability — days until retrievability drops to
    /// `REQUEST_RETENTION`. Grows when reviews succeed.
    pub stability: f64,
    /// Item difficulty on a 1–10 scale. Drifts toward 5 (mean) after
    /// each review, offset by the grade.
    pub difficulty: f64,
    /// Unix seconds of the next scheduled review.
    pub due_at: i64,
    /// Unix seconds of the most recent review (0 if never).
    pub last_review: i64,
    /// Count of completed reviews. Used to distinguish "first review"
    /// from repeats (initial stability calc is different).
    pub reps: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            stability: 0.0,
            difficulty: 0.0,
            due_at: 0,
            last_review: 0,
            reps: 0,
        }
    }
}

/// Apply a grade to an existing state. `now` is unix seconds.
/// First-review items (reps == 0) use the FSRS initial-stability table;
/// repeats use the full update rule.
pub fn update(state: &State, grade: Grade, now: i64) -> State {
    let g = grade.as_f64();
    if state.reps == 0 {
        let stability = initial_stability(grade);
        let difficulty = initial_difficulty(grade);
        let interval_days = next_interval_days(stability);
        return State {
            stability,
            difficulty,
            due_at: now + (interval_days * 86_400.0) as i64,
            last_review: now,
            reps: 1,
        };
    }

    // Elapsed days since last review — used to compute current
    // retrievability before the grade is applied.
    let elapsed_days = ((now - state.last_review).max(0) as f64) / 86_400.0;
    let retrievability = (1.0 + elapsed_days / (9.0 * state.stability.max(0.1))).powf(-1.0);

    // New difficulty — drifts toward 5 after each review.
    let mut difficulty = state.difficulty - W[6] * (g - 3.0);
    // Mean reversion.
    difficulty = W[7] * 5.0 + (1.0 - W[7]) * difficulty;
    difficulty = difficulty.clamp(1.0, 10.0);

    // New stability depends on whether we lapsed (Again) or recalled.
    let stability = if grade == Grade::Again {
        W[11]
            * state.difficulty.powf(-W[12])
            * ((state.stability + 1.0).powf(W[13]) - 1.0)
            * (W[14] * (1.0 - retrievability)).exp()
    } else {
        let hard_penalty = if grade == Grade::Hard { W[15] } else { 1.0 };
        let easy_bonus = if grade == Grade::Easy { W[16] } else { 1.0 };
        state.stability
            * (1.0
                + (W[8]).exp()
                    * (11.0 - difficulty)
                    * state.stability.powf(-W[9])
                    * ((W[10] * (1.0 - retrievability)).exp() - 1.0)
                    * hard_penalty
                    * easy_bonus)
    };
    let stability = stability.max(0.1);

    let interval_days = next_interval_days(stability);

    State {
        stability,
        difficulty,
        due_at: now + (interval_days * 86_400.0) as i64,
        last_review: now,
        reps: state.reps + 1,
    }
}

fn initial_stability(grade: Grade) -> f64 {
    match grade {
        Grade::Again => W[0],
        Grade::Hard => W[1],
        Grade::Good => W[2],
        Grade::Easy => W[3],
    }
}

fn initial_difficulty(grade: Grade) -> f64 {
    (W[4] - W[5] * (grade.as_f64() - 3.0)).clamp(1.0, 10.0)
}

/// Desired interval given a stability and the global retention target.
fn next_interval_days(stability: f64) -> f64 {
    let factor = 9.0 * (REQUEST_RETENTION.powf(-1.0) - 1.0);
    let interval = stability * factor;
    // Clamp to [1 day, 3 years]. A clean 1-day minimum protects the
    // "tomorrow" spacing that typing practice benefits from.
    interval.clamp(1.0, 3.0 * 365.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grade_thresholds_are_sane() {
        // Perfect run — always Easy.
        assert_eq!(grade_from_attempt(100.0, 60.0, 90.0, 40.0), Grade::Easy);
        // Exactly on target — Good.
        assert_eq!(grade_from_attempt(90.0, 40.0, 90.0, 40.0), Grade::Good);
        // Accuracy OK, speed slow — Hard (combined ratio < 1).
        let g = grade_from_attempt(88.0, 20.0, 90.0, 40.0);
        assert!(matches!(g, Grade::Hard | Grade::Again));
        // Sub-60% accuracy — Again regardless of speed.
        assert_eq!(grade_from_attempt(55.0, 60.0, 90.0, 40.0), Grade::Again);
    }

    #[test]
    fn first_review_sets_initial_stability_from_grade() {
        let s0 = State::default();
        let now = 1_700_000_000;
        let good = update(&s0, Grade::Good, now);
        let again = update(&s0, Grade::Again, now);
        assert!(good.stability > again.stability);
        assert_eq!(good.reps, 1);
        assert!(good.due_at > now);
    }

    #[test]
    fn consecutive_good_reviews_grow_stability() {
        let mut s = State::default();
        let mut now = 1_700_000_000;
        s = update(&s, Grade::Good, now);
        let first = s.stability;
        now += (s.due_at - now).max(0);
        s = update(&s, Grade::Good, now);
        let second = s.stability;
        now += (s.due_at - now).max(0);
        s = update(&s, Grade::Good, now);
        let third = s.stability;
        assert!(second > first, "second review grows stability");
        assert!(third > second, "third review keeps growing");
    }

    #[test]
    fn lapse_resets_stability_low() {
        let mut s = State::default();
        let now = 1_700_000_000;
        s = update(&s, Grade::Good, now);
        s = update(&s, Grade::Good, s.due_at);
        let before = s.stability;
        s = update(&s, Grade::Again, s.due_at);
        assert!(s.stability < before, "lapse shrinks stability");
    }

    #[test]
    fn difficulty_drifts_toward_five() {
        // Consistent Good reviews should pull difficulty toward ~5.
        let mut s = State::default();
        s = update(&s, Grade::Good, 0);
        for _ in 0..20 {
            s = update(&s, Grade::Good, s.due_at);
        }
        assert!(
            (s.difficulty - 5.0).abs() < 2.0,
            "difficulty converges: got {}",
            s.difficulty
        );
    }
}
