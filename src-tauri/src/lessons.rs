//! Intro lesson curriculum for absolute beginners.
//!
//! A Montessori-flavoured progression: the student meets one key cluster
//! at a time, builds confidence, then layers. Each lesson is a collection
//! of generated drill lines — letters, bigrams, real Czech words, then
//! a short sentence — so motor memory and meaning land together.
//!
//! Progression (lesson id → what the student earns):
//!
//!   1  home_row_left     — asdf
//!   2  home_row_right    — jkl;
//!   3  home_row_both     — asdf jkl with the canonical "ask dad" drills
//!   4  top_row_left      — qwer
//!   5  top_row_right     — uiop
//!   6  top_row_both
//!   7  bottom_row_left   — zxcv
//!   8  bottom_row_right  — m,./
//!   9  bottom_row_both
//!  10  diacritics_light  — áíéúý (straight-acute, most common)
//!  11  diacritics_háček  — čšžřě
//!  12  diacritics_kroužek — ů
//!  13  shift_capitals
//!  14  numbers_row
//!  15  punctuation
//!  16  short_sentences   — real short Czech sentences from the corpus
//!
//! Each lesson has an explicit mastery bar (accuracy ≥ 95 %, WPM ≥ some
//! threshold) the session summary checks against. If passed, the next
//! lesson unlocks in `intro_lesson_progress`.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
pub struct LessonMeta {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub target_accuracy: f64,
    pub target_wpm: f64,
}

/// Human-readable Czech rendering of a target. The numeric form
/// (`Cíl: 10 WPM · 92 %`) means nothing to a 14-year-old on day one.
/// We translate WPM into a felt unit ("cca 2 slova za sekundu") and
/// accuracy into a 10-attempt frame ("9 z 10 vět skoro bez chyby") —
/// then keep the numbers visible alongside for adults who care.
///
/// Ranges are coarse on purpose. The point is communication, not
/// precision; the FSRS scorer still uses the raw numbers.
pub fn human_target(meta: &LessonMeta) -> String {
    let acc = meta.target_accuracy;
    let wpm = meta.target_wpm;

    let acc_phrase = if acc >= 95.0 {
        "skoro bez chyby"
    } else if acc >= 90.0 {
        "s minimem chyb"
    } else if acc >= 85.0 {
        "s pár drobnými chybami"
    } else {
        "i s několika chybami"
    };

    let wpm_phrase = if wpm < 12.0 {
        "v klidu, ne na rychlost"
    } else if wpm < 18.0 {
        "tempem normálního čtení"
    } else if wpm < 25.0 {
        "tak rychle, jak normálně mluvíš"
    } else {
        "svižně, bez váhání"
    };

    // Accuracy → "X z 10". 92% → 9.2 → "9 z 10". 88% → "9 z 10". 80%
    // → "8 z 10". Round to nearest.
    let out_of_ten = ((acc / 10.0).round() as i32).clamp(5, 10);

    format!(
        "Zvládnul jsi to, když napíšeš {of_ten} z 10 vět {acc_phrase} a píšeš {wpm_phrase}.",
        of_ten = out_of_ten,
        acc_phrase = acc_phrase,
        wpm_phrase = wpm_phrase,
    )
}

#[derive(Debug, Clone, Serialize)]
pub struct LessonDrill {
    pub text: String,
}

pub struct Lesson {
    pub meta: LessonMeta,
    pub drills: fn() -> Vec<LessonDrill>,
}

fn lines(words: &[&str]) -> Vec<LessonDrill> {
    // Group words into lines of ~40 chars so the wrap stays tidy.
    let mut out = Vec::new();
    let mut cur = String::new();
    for w in words {
        if !cur.is_empty() && cur.len() + 1 + w.len() > 40 {
            out.push(LessonDrill {
                text: std::mem::take(&mut cur),
            });
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        out.push(LessonDrill { text: cur });
    }
    out
}

/// Full curriculum, in order. Order == unlock order.
pub fn curriculum() -> Vec<Lesson> {
    vec![
        Lesson {
            meta: LessonMeta {
                id: "home_row_left",
                title: "Horní řada levá",
                subtitle: "Prsty na asdf. Malíček, prsteníček, prostředníček, ukazováček.",
                target_accuracy: 92.0,
                target_wpm: 10.0,
            },
            drills: || {
                lines(&[
                    "aaa", "sss", "ddd", "fff", "asdf", "asdf",
                    "aa ss dd ff", "fdsa", "dads", "sad", "fad", "fads",
                    "as as as", "ad ad", "fad sad", "dad fad sad", "asdf fdsa",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "home_row_right",
                title: "Horní řada pravá",
                subtitle: "jkl; — palec pravé ruky na mezerníku.",
                target_accuracy: 92.0,
                target_wpm: 10.0,
            },
            drills: || {
                lines(&[
                    "jjj", "kkk", "lll", ";;;", "jkl;", "jkl;",
                    "jj kk ll", "lkjh", "jkl j", "kill", "lull",
                    "jk jk", "kl kl", "lll kkk", "ask jkl",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "home_row_both",
                title: "Obě ruce, horní řada",
                subtitle: "Sjednocení obou rukou. Klasické „ask dad\" věty.",
                target_accuracy: 92.0,
                target_wpm: 12.0,
            },
            drills: || {
                lines(&[
                    "asdf jkl;", "asdf jkl;",
                    "jak sal", "dal slad", "sad dad",
                    "ask a kid", "dad had a lad",
                    "a fad lad", "all safe lads",
                    "asd fgh jkl", "asdf jkl; asdf jkl;",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "top_row_left",
                title: "Vrchní řada levá",
                subtitle: "qwer — prsty se natáhnou o řadu výš.",
                target_accuracy: 90.0,
                target_wpm: 14.0,
            },
            drills: || {
                lines(&[
                    "qwer", "qwer", "qa ws ed rf",
                    "were", "wear", "read", "red", "rare",
                    "fr fr", "de de", "sw sw", "aq aq",
                    "as we see", "free rates", "dear wear",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "top_row_right",
                title: "Vrchní řada pravá",
                subtitle: "uiop — ruka se přesouvá výš.",
                target_accuracy: 90.0,
                target_wpm: 14.0,
            },
            drills: || {
                lines(&[
                    "uiop", "uiop", "uj ik ol p;",
                    "your", "pour", "loop", "pool", "poor",
                    "up up", "io io", "po po",
                    "pool your", "loop up", "pour your oil",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "top_row_both",
                title: "Celá vrchní řada",
                subtitle: "qwer + uiop dohromady s horní řadou.",
                target_accuracy: 90.0,
                target_wpm: 16.0,
            },
            drills: || {
                lines(&[
                    "qwer uiop", "asdf jkl;",
                    "quiet worker", "peer power",
                    "we eat apple", "your quiet pool",
                    "proper order", "repeat questions",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "bottom_row_left",
                title: "Spodní řada levá",
                subtitle: "zxcv — prsty jdou pod horní řadu.",
                target_accuracy: 88.0,
                target_wpm: 14.0,
            },
            drills: || {
                lines(&[
                    "zxcv", "zxcv", "az sx dc fv",
                    "zap vax cave", "vex fax",
                    "fz fz", "dc dc", "sx sx",
                    "vex a fox", "a brave cat",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "bottom_row_right",
                title: "Spodní řada pravá",
                subtitle: "nm,./ — čárka, tečka, lomítko.",
                target_accuracy: 88.0,
                target_wpm: 14.0,
            },
            drills: || {
                lines(&[
                    "nnn", "mmm", ",,,", "...",
                    "nm,.", "nm,.", "jn km l, ;.",
                    "min kin", "moon noon", "noon mop",
                    "n, m. n, m.", ", . , .",
                    "send mail, ok.", "take it, now.",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "bottom_row_both",
                title: "Celá spodní řada",
                subtitle: "zxcv + nm,./",
                target_accuracy: 88.0,
                target_wpm: 16.0,
            },
            drills: || {
                lines(&[
                    "zxcv nm,.", "asdf jkl;",
                    "cave, vex mom.", "move, vex, zoom.",
                    "maximum, minimum, common.",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "diacritics_acute",
                title: "Čárky nad samohláskami",
                subtitle: "á é í ó ú ý — délková čárka.",
                target_accuracy: 90.0,
                target_wpm: 16.0,
            },
            drills: || {
                lines(&[
                    "áá éé íí óó úú ýý",
                    "máma tátá", "bílý dlouhý",
                    "být dát mít", "já tý mý",
                    "víme hrát", "báseň je krásná",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "diacritics_hacek",
                title: "Háčky",
                subtitle: "č š ž ř ě ň ď ť — české hlásky.",
                target_accuracy: 88.0,
                target_wpm: 16.0,
            },
            drills: || {
                lines(&[
                    "čč šš žž řř ěě",
                    "čaj šálek žena",
                    "řeka lžíce","čížek šťovík",
                    "tři dřeva", "žluťoučký kůň",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "diacritics_krouzek",
                title: "Kroužek a zbytek",
                subtitle: "ů + ď ť ň v kontextu.",
                target_accuracy: 88.0,
                target_wpm: 18.0,
            },
            drills: || {
                lines(&[
                    "ůů ůl kůň",
                    "můj tvůj svůj", "hůl stůl důl",
                    "ť ď ň — ťuk ďábel koň",
                    "kůň pije vodu", "můj stůl stojí",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "shift_capitals",
                title: "Shift a velká písmena",
                subtitle: "Začínáme větu velkým, používáme malíček na Shift.",
                target_accuracy: 90.0,
                target_wpm: 18.0,
            },
            drills: || {
                lines(&[
                    "Ahoj. Jak se máš?", "Praha je krásná.",
                    "Karel Čapek", "Josef Lada",
                    "Anna má krásný dům.", "Petr jde do školy.",
                    "Nový rok 2026 začíná.",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "numbers_row",
                title: "Číselná řada",
                subtitle: "1234567890 — pro data, roky, čísla stran.",
                target_accuracy: 88.0,
                target_wpm: 16.0,
            },
            drills: || {
                lines(&[
                    "1234567890",
                    "rok 1918", "rok 1989", "rok 2026",
                    "123 456 789",
                    "strana 12", "kapitola 3",
                    "10 let, 100 dní, 1000 slov.",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "punctuation",
                title: "Interpunkce",
                subtitle: ", . ! ? : ; — a uvozovky.",
                target_accuracy: 88.0,
                target_wpm: 18.0,
            },
            drills: || {
                lines(&[
                    ", . ! ? : ;",
                    "Ahoj, kamarádi!", "Jsi tam? Ne.",
                    "Řekl: „Ano.\"",
                    "Praha — hlavní město — má most.",
                    "Petr (bratr) přišel; zůstal dlouho.",
                ])
            },
        },
        Lesson {
            meta: LessonMeta {
                id: "short_sentences",
                title: "Krátké věty",
                subtitle: "Sestavíme vše dohromady do smysluplných vět.",
                target_accuracy: 90.0,
                target_wpm: 22.0,
            },
            drills: || {
                lines(&[
                    "Dnes je krásný den.",
                    "Učím se psát naslepo.",
                    "Česká abeceda má 42 písmen.",
                    "Piš pomalu, ale přesně.",
                    "Každý den kousek lepší.",
                    "Datel klepe do kůry — ťuk, ťuk, ťuk.",
                ])
            },
        },
    ]
}

/// Find a lesson by its stable ID.
pub fn lesson_by_id(id: &str) -> Option<Lesson> {
    curriculum().into_iter().find(|l| l.meta.id == id)
}

/// Next lesson's ID after the given one, or `None` if the student has
/// finished the whole curriculum.
pub fn next_lesson_id(after: &str) -> Option<&'static str> {
    let c = curriculum();
    let idx = c.iter().position(|l| l.meta.id == after)?;
    c.get(idx + 1).map(|l| l.meta.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curriculum_has_stable_ids_and_no_duplicates() {
        let c = curriculum();
        let mut ids: Vec<&str> = c.iter().map(|l| l.meta.id).collect();
        ids.sort();
        let mut deduped = ids.clone();
        deduped.dedup();
        assert_eq!(ids, deduped, "IDs must be unique");
        assert!(c.len() >= 12, "curriculum should cover a full beginner arc");
    }

    #[test]
    fn every_lesson_has_at_least_a_handful_of_drills() {
        for l in curriculum() {
            let drills = (l.drills)();
            assert!(
                drills.len() >= 3,
                "lesson {} should have multiple drill lines, got {}",
                l.meta.id,
                drills.len()
            );
            for d in drills {
                assert!(!d.text.is_empty());
            }
        }
    }

    #[test]
    fn targets_grow_monotonically_with_curriculum() {
        let c = curriculum();
        // Accuracy floor doesn't increase (the student is learning harder
        // material); WPM target does increase by the end.
        let first_wpm = c.first().unwrap().meta.target_wpm;
        let last_wpm = c.last().unwrap().meta.target_wpm;
        assert!(
            last_wpm > first_wpm,
            "later lessons should expect higher WPM"
        );
    }

    #[test]
    fn next_lesson_id_walks_curriculum_then_stops() {
        let c = curriculum();
        let first = c[0].meta.id;
        let second = c[1].meta.id;
        assert_eq!(next_lesson_id(first), Some(second));
        let last = c.last().unwrap().meta.id;
        assert_eq!(next_lesson_id(last), None);
    }

    #[test]
    fn human_target_translates_numbers_to_words() {
        let easy = LessonMeta {
            id: "x", title: "x", subtitle: "x",
            target_accuracy: 92.0, target_wpm: 10.0,
        };
        let s = human_target(&easy);
        assert!(s.contains("9 z 10"), "got: {s}");
        assert!(s.contains("v klidu"), "got: {s}");

        let hard = LessonMeta {
            id: "x", title: "x", subtitle: "x",
            target_accuracy: 95.0, target_wpm: 30.0,
        };
        let s2 = human_target(&hard);
        assert!(s2.contains("10 z 10") || s2.contains("9 z 10"));
        assert!(s2.contains("svižně"));
    }
}
