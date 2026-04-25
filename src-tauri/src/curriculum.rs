//! Curated exam-prep ramps (PE-012). Pre-packaged content bundles for
//! the two highest-stakes CZ/SK exams students face:
//!
//!   * **Cermat** — státní přijímačky on the move from základka to
//!     gymnázium; separate testy for Czech language (ČJL) and maths (M).
//!     Commonly taken in 9th grade (sometimes 7th / 5th for víceleté).
//!   * **Maturita** — státní maturitní zkouška, school-leaving; we
//:    prepare common rails: ČJL didaktický test, matematika, angličtina.
//!
//! Each ramp is a small set of `RampLesson`s — ordered passages + key-
//! term lists students should be able to type from memory. These live
//! alongside the Lessons curriculum but are *topical*, not mechanical:
//! typing them repeatedly drills both keys and the material itself.
//!
//! Content is kept CC-BY-licensable: paraphrased facts, no copyrighted
//! test extracts. Full attribution via the `citation` field.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExamKind {
    CermatCzechLanguage,
    CermatMath,
    MaturitaCzechLanguage,
    MaturitaEnglish,
    MaturitaMath,
}

impl ExamKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ExamKind::CermatCzechLanguage => "cermat_cjl",
            ExamKind::CermatMath => "cermat_math",
            ExamKind::MaturitaCzechLanguage => "maturita_cjl",
            ExamKind::MaturitaEnglish => "maturita_en",
            ExamKind::MaturitaMath => "maturita_math",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExamRamp {
    pub id: &'static str,
    pub kind: ExamKind,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub lessons: Vec<RampLesson>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RampLesson {
    pub id: &'static str,
    pub title: &'static str,
    pub passages: Vec<&'static str>,
    /// Short attribution line for the facts (e.g. "Cermat veřejná
    /// specifikace 2025 / Wikipedia — paraphrased").
    pub citation: &'static str,
}

/// Return every available exam ramp. Order is stable — UI uses it.
pub fn all_ramps() -> Vec<ExamRamp> {
    vec![
        // --------- Cermat — český jazyk a literatura ---------
        ExamRamp {
            id: "cermat_cjl",
            kind: ExamKind::CermatCzechLanguage,
            title: "Cermat · Český jazyk a literatura",
            subtitle: "Přijímací zkouška na SŠ — pravopis, literatura, sloh.",
            lessons: vec![
                RampLesson {
                    id: "pravopis_vyjmenovana",
                    title: "Pravopis — vyjmenovaná slova",
                    passages: vec![
                        "Po obojetných souhláskách píšeme tvrdé y nebo měkké i podle vyjmenovaných slov.",
                        "Slovesa být, bydlet, obývat, byt, příbytek, dobytek, obyvatelstvo, byl.",
                        "Po písmenu b: být, bydlit, obyvatel, byt, příbytek, dobytek, nábytek.",
                        "Po písmenu l: slyšet, mlýn, blýskat se, polykat, plynout, plýtvat, vzlykat.",
                        "Po písmenu m: my, mýlit se, myslit, mýt, hmyz, myš, smýkat, zamykat, smysl.",
                    ],
                    citation: "Učebnice ČJL — paraphrased summary.",
                },
                RampLesson {
                    id: "interpunkce",
                    title: "Interpunkce — čárka před spojkami",
                    passages: vec![
                        "Čárku píšeme před spojkami ale, avšak, však, nýbrž, neboť, tedy.",
                        "Před spojkou a čárku obvykle nepíšeme, pokud spojuje pouze dva členy v prostém výčtu.",
                        "Před spojkou a čárku píšeme, pokud za a stojí vložená vedlejší věta.",
                        "V souvětí oddělujeme vedlejší věty od hlavní čárkami z obou stran.",
                        "Přímou řeč uvozujeme dvojtečkou a uzavíráme uvozovkami, čárka patří uvnitř uvozovek.",
                    ],
                    citation: "Internetová jazyková příručka ÚJČ AV ČR — paraphrased.",
                },
                RampLesson {
                    id: "shoda_pridavneho_prisudku",
                    title: "Shoda podmětu s přísudkem",
                    passages: vec![
                        "Přísudek se shoduje s podmětem v rodě a čísle.",
                        "V mužském rodě životném píšeme v množném čísle příčestí s koncovkou -i: muži přišli.",
                        "V mužském rodě neživotném píšeme -y: stromy rostly, stoly stály.",
                        "V ženském rodě píšeme -y: ženy zpívaly, knihy ležely.",
                        "Ve středním rodě píšeme -a: děvčata tančila, kola se otáčela.",
                    ],
                    citation: "Školní mluvnice češtiny — paraphrased.",
                },
                RampLesson {
                    id: "literatura_obrozeni",
                    title: "Literatura — České národní obrození",
                    passages: vec![
                        "České národní obrození probíhalo od konce 18. století do poloviny 19. století.",
                        "Josef Dobrovský položil základy vědecké bohemistiky a sepsal mluvnici češtiny.",
                        "Josef Jungmann vytvořil pětidílný Česko-německý slovník a přeložil Ztracený ráj.",
                        "Karel Hynek Mácha vydal báseň Máj roku 1836, dnes klíčové dílo českého romantismu.",
                        "Božena Němcová napsala Babičku, vyšla roku 1855 a patří mezi nejčtenější česká díla.",
                    ],
                    citation: "Středoškolská učebnice literatury — paraphrased.",
                },
            ],
        },
        // --------- Cermat — matematika ---------
        ExamRamp {
            id: "cermat_math",
            kind: ExamKind::CermatMath,
            title: "Cermat · Matematika",
            subtitle: "Přijímací zkouška na SŠ — zlomky, rovnice, geometrie.",
            lessons: vec![
                RampLesson {
                    id: "zlomky",
                    title: "Zlomky a desetinná čísla",
                    passages: vec![
                        "Zlomek a/b je podíl dvou čísel, kde b je různé od nuly.",
                        "Pro sčítání zlomků se stejným jmenovatelem sčítáme jen čitatele.",
                        "Zlomek a/b rozšiřujeme, když čitatele i jmenovatele násobíme stejným číslem.",
                        "Smíšené číslo převedeme na zlomek tak, že celé číslo vynásobíme jmenovatelem a přičteme čitatele.",
                        "Desetinná čísla zaokrouhlujeme podle první následující číslice.",
                    ],
                    citation: "Učebnice matematiky ZŠ — paraphrased.",
                },
                RampLesson {
                    id: "rovnice",
                    title: "Lineární rovnice",
                    passages: vec![
                        "Lineární rovnice má tvar ax + b = 0, kde a je různé od nuly.",
                        "Při řešení převádíme neznámé na jednu stranu a známé členy na druhou.",
                        "Pokud obě strany vydělíme nulou, rovnice ztrácí smysl.",
                        "Rovnici můžeme násobit i dělit libovolným nenulovým číslem na obou stranách.",
                        "Pokud se po úpravě dostaneme 0 = 0, rovnice má nekonečně mnoho řešení.",
                    ],
                    citation: "Učebnice matematiky ZŠ — paraphrased.",
                },
                RampLesson {
                    id: "obvody_obsahy",
                    title: "Obvody a obsahy základních útvarů",
                    passages: vec![
                        "Obvod obdélníku se vypočítá jako dvakrát součet délek stran.",
                        "Obsah čtverce je druhá mocnina délky jeho strany.",
                        "Obsah obdélníku je součin délek jeho stran.",
                        "Obsah trojúhelníku je polovina součinu strany a výšky k ní.",
                        "Obvod kruhu se rovná součinu pi a průměru kruhu.",
                    ],
                    citation: "Učebnice matematiky ZŠ — paraphrased.",
                },
            ],
        },
        // --------- Maturita — český jazyk ---------
        ExamRamp {
            id: "maturita_cjl",
            kind: ExamKind::MaturitaCzechLanguage,
            title: "Maturita · Český jazyk a literatura",
            subtitle: "Didaktický test + slohová práce — pravopis, literatura 20. stol.",
            lessons: vec![
                RampLesson {
                    id: "meziv_lit",
                    title: "Meziválečná literatura",
                    passages: vec![
                        "Poetismus vznikl v Československu ve dvacátých letech a usiloval o radost ze života.",
                        "Vítězslav Nezval psal v poetistickém duchu a později přešel k surrealismu.",
                        "Karel Čapek napsal drama R.U.R., ve kterém poprvé použil slovo robot.",
                        "Jaroslav Hašek vydal Osudy dobrého vojáka Švejka za světové války a jeho vliv.",
                        "Vladislav Vančura psal ornamentálním jazykem, mezi hlavní díla patří Rozmarné léto.",
                    ],
                    citation: "Dějiny české literatury — paraphrased.",
                },
                RampLesson {
                    id: "normalizace",
                    title: "Normalizace a samizdat",
                    passages: vec![
                        "Po srpnu 1968 nastoupila v Československu normalizace a mnozí autoři byli zakázáni.",
                        "Samizdat byl neoficiální způsob šíření zakázaných knih rukopisem nebo na stroji.",
                        "Václav Havel, Ludvík Vaculík a Pavel Kohout vydávali své texty mimo státní tisk.",
                        "Milan Kundera odešel do Francie, kde vydal Nesnesitelnou lehkost bytí.",
                        "Bohumil Hrabal psal pábitelský jazyk, jeho Obsluhoval jsem anglického krále vyšel v samizdatu.",
                    ],
                    citation: "Dějiny české literatury 2. pol. 20. stol. — paraphrased.",
                },
                RampLesson {
                    id: "slohovy_postup",
                    title: "Slohové postupy",
                    passages: vec![
                        "Vyprávěcí postup řadí události v čase a bývá v dějové literatuře.",
                        "Popisný postup charakterizuje objekt nebo jev bez dějové linky.",
                        "Výkladový postup vysvětluje souvislosti a využívá logických vazeb.",
                        "Úvahový postup autor rozvíjí myšlenku a hodnotí téma z různých úhlů.",
                        "Informační postup sděluje fakta stručně, bez hodnocení a citového zabarvení.",
                    ],
                    citation: "Školní mluvnice češtiny — paraphrased.",
                },
            ],
        },
        // --------- Maturita — English ---------
        ExamRamp {
            id: "maturita_en",
            kind: ExamKind::MaturitaEnglish,
            title: "Maturita · English",
            subtitle: "B1-B2 grammar + vocabulary — common exam topics.",
            lessons: vec![
                RampLesson {
                    id: "present_perfect",
                    title: "Present perfect vs past simple",
                    passages: vec![
                        "Present perfect connects the past with the present and often uses ever, never, just.",
                        "We use past simple when the time is specified, such as yesterday, in 1999 or last week.",
                        "I have lived in Prague for five years means I still live there.",
                        "I lived in Prague for five years means I no longer live there.",
                        "Present perfect continuous stresses the duration of an ongoing action until now.",
                    ],
                    citation: "Cambridge Grammar in Use — paraphrased.",
                },
                RampLesson {
                    id: "conditional",
                    title: "Conditional sentences",
                    passages: vec![
                        "The zero conditional describes general truths, using if plus present simple twice.",
                        "The first conditional describes likely future events, using if plus present and will.",
                        "The second conditional describes hypothetical present situations with past tense and would.",
                        "The third conditional describes hypothetical past situations with past perfect and would have.",
                        "Mixed conditionals combine past hypothetical cause with present hypothetical effect.",
                    ],
                    citation: "Cambridge Grammar in Use — paraphrased.",
                },
            ],
        },
        // --------- Maturita — matematika ---------
        ExamRamp {
            id: "maturita_math",
            kind: ExamKind::MaturitaMath,
            title: "Maturita · Matematika",
            subtitle: "Funkce, derivace, geometrie, pravděpodobnost.",
            lessons: vec![
                RampLesson {
                    id: "goniometrie",
                    title: "Goniometrické funkce",
                    passages: vec![
                        "Sinus úhlu je poměr protilehlé odvěsny k přeponě v pravoúhlém trojúhelníku.",
                        "Kosinus úhlu je poměr přilehlé odvěsny k přeponě.",
                        "Tangens úhlu je poměr sinu a kosinu, respektive protilehlé k přilehlé odvěsně.",
                        "Platí základní vztah sinus na druhou plus kosinus na druhou je rovno jedné.",
                        "Pro sinus a kosinus platí součtové vzorce sinus a plus beta a obdobně pro kosinus.",
                    ],
                    citation: "Středoškolská matematika — paraphrased.",
                },
                RampLesson {
                    id: "derivace",
                    title: "Derivace funkce",
                    passages: vec![
                        "Derivace funkce v bodě udává směrnici tečny ke grafu funkce v daném bodě.",
                        "Derivace konstanty je nula a derivace proměnné x je rovna jedné.",
                        "Derivace mocninné funkce x na n je n krát x na n minus jedna.",
                        "Derivace součtu je součet derivací jednotlivých funkcí.",
                        "Derivace složené funkce se počítá podle řetízkového pravidla.",
                    ],
                    citation: "Středoškolská matematika — paraphrased.",
                },
            ],
        },
    ]
}

pub fn ramp_by_id(id: &str) -> Option<ExamRamp> {
    all_ramps().into_iter().find(|r| r.id == id)
}

pub fn lesson_by_id<'a>(ramp: &'a ExamRamp, id: &str) -> Option<&'a RampLesson> {
    ramp.lessons.iter().find(|l| l.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_ramps_have_unique_ids_and_at_least_two_lessons() {
        let ramps = all_ramps();
        assert!(ramps.len() >= 5, "five exam ramps minimum");
        let mut ids: Vec<&str> = ramps.iter().map(|r| r.id).collect();
        ids.sort();
        let dedup = ids.clone();
        assert_eq!(ids, dedup, "ramp ids must be unique");
        for r in ramps {
            assert!(r.lessons.len() >= 2, "ramp {} has too few lessons", r.id);
            for l in &r.lessons {
                assert!(!l.passages.is_empty(), "lesson {}/{} has no passages", r.id, l.id);
                assert!(
                    !l.citation.is_empty(),
                    "lesson {}/{} missing citation",
                    r.id,
                    l.id
                );
            }
        }
    }

    #[test]
    fn ramp_lookup_works() {
        let r = ramp_by_id("cermat_cjl").expect("cermat cjl ramp");
        assert_eq!(r.kind, ExamKind::CermatCzechLanguage);
        assert!(lesson_by_id(&r, "pravopis_vyjmenovana").is_some());
        assert!(lesson_by_id(&r, "missing").is_none());
    }
}
