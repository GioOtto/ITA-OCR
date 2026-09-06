//! Post-correzione lessicale conservativa dell'uscita OCR.
//!
//! Il dizionario italiano serve sia a riconoscere le parole valide sia a
//! escludere correzioni ambigue. Una sostituzione viene applicata soltanto se
//! esiste una sola parola valida a distanza di Levenshtein 1 e quella parola
//! e' presente nel lessico locale. Nomi propri, formule, sigle, parole
//! brevi e sillabazioni a fine riga non vengono mai toccati.
//!
//! Nemmeno le parole inglesi: un manoscritto italiano cita spesso termini o
//! passaggi in inglese, e il correttore italiano li vedrebbe come errori con
//! un solo vicino a distanza 1 (`form` diventerebbe `forma`). Un secondo
//! dizionario, en_US, serve solo a riconoscerli e lasciarli stare: non
//! corregge mai niente in inglese, si limita a togliere quelle parole di
//! mezzo. Se non e' presente si degrada al comportamento di prima.

use regex::Regex;
use spellbook::Dictionary;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use unicode_normalization::UnicodeNormalization;

const ALFABETO: &str = "abcdefghijklmnopqrstuvwxyzàáèéìíòóùúäëïöüç";

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Correzione {
    /// Posizioni in caratteri (non byte) nel testo corretto.
    pub inizio: usize,
    pub fine: usize,
    pub originale: String,
    pub corretta: String,
    pub metodo: String,
}

#[derive(Clone, Default)]
pub struct Esito {
    pub testo: String,
    pub correzioni: Vec<Correzione>,
}

pub struct Varianti {
    pub dizionario: Esito,
    pub contesto: Esito,
    pub entrambe: Esito,
}

pub struct Correttore {
    dizionario: Dictionary,
    /// Solo per riconoscere l'inglese, mai per correggerlo. Opzionale: un
    /// pacchetto senza en_US resta funzionante, perde solo questa protezione.
    inglese: Option<Dictionary>,
    frequenze_lessico: HashMap<String, u32>,
    bigrammi: HashMap<(String, String), u32>,
    trigrammi: HashMap<(String, String, String), u32>,
    parola: Regex,
    formula: Regex,
}

impl Correttore {
    /// `cartella_inglese` e' opzionale perche' la sua assenza non e' un errore:
    /// vale la pena correggere in italiano anche senza poter riconoscere
    /// l'inglese, mentre fallire il caricamento lascerebbe il testo grezzo.
    pub fn carica(cartella: &Path, cartella_inglese: Option<&Path>) -> Result<Self, String> {
        let aff = std::fs::read_to_string(cartella.join("it_IT.aff"))
            .map_err(|e| format!("regole del dizionario italiano non leggibili: {e}"))?;
        let dic = std::fs::read_to_string(cartella.join("it_IT.dic"))
            .map_err(|e| format!("dizionario italiano non leggibile: {e}"))?;
        // La distribuzione pubblica usa solo il dizionario pubblico. Risorse
        // lessicali personali possono essere aggiunte localmente; non sono
        // necessarie per caricare il correttore e non vengono distribuite.
        let training = leggi_opzionale(&cartella.join("lessico.txt"))?.unwrap_or_else(|| {
            dic.lines().skip(1).filter_map(|riga| {
                let parola = riga.split('/').next()?.split_whitespace().next()?;
                Some(format!("{parola}\t1\n"))
            }).collect()
        });
        let contesto = leggi_opzionale(&cartella.join("contesto.txt"))?.unwrap_or_default();
        let inglese = cartella_inglese.and_then(|dir| {
            let aff = std::fs::read_to_string(dir.join("en_US.aff")).ok()?;
            let dic = std::fs::read_to_string(dir.join("en_US.dic")).ok()?;
            Some((aff, dic))
        });
        let inglese = inglese
            .as_ref()
            .map(|(aff, dic)| (aff.as_str(), dic.as_str()));
        Self::da_testi(&aff, &dic, &training, &contesto, inglese)
    }

    fn da_testi(
        aff: &str,
        dic: &str,
        training: &str,
        contesto: &str,
        inglese: Option<(&str, &str)>,
    ) -> Result<Self, String> {
        let dizionario = Dictionary::new(aff, dic)
            .map_err(|e| format!("dizionario italiano non valido: {e}"))?;
        let inglese = match inglese {
            Some((aff, dic)) => Some(
                Dictionary::new(aff, dic)
                    .map_err(|e| format!("dizionario inglese non valido: {e}"))?,
            ),
            None => None,
        };
        let frequenze_lessico = training
            .lines()
            .filter_map(|riga| {
                let (parola, frequenza) = riga.split_once('\t')?;
                Some((normalizza(parola), frequenza.parse::<u32>().ok()?))
            })
            .filter(|(p, _)| !p.is_empty())
            .collect::<HashMap<_, _>>();
        let mut bigrammi = HashMap::new();
        let mut trigrammi = HashMap::new();
        for riga in contesto.lines() {
            let campi: Vec<&str> = riga.split('\t').collect();
            match campi.as_slice() {
                ["B", a, b, n] => {
                    if let Ok(n) = n.parse() {
                        bigrammi.insert(((*a).into(), (*b).into()), n);
                    }
                }
                ["T", a, b, c, n] => {
                    if let Ok(n) = n.parse() {
                        trigrammi.insert(((*a).into(), (*b).into(), (*c).into()), n);
                    }
                }
                _ => {}
            }
        }
        let parola = Regex::new(r"[\p{L}]+(?:['’][\p{L}]+)*").map_err(|e| e.to_string())?;
        let formula = Regex::new(
            r"(?s)\$\$.*?\$\$|\$.*?\$|\\\[.*?\\\]|\\\(.*?\\\)|\\begin\{[^}]+\}.*?\\end\{[^}]+\}",
        )
        .map_err(|e| e.to_string())?;
        Ok(Self {
            dizionario,
            inglese,
            frequenze_lessico,
            bigrammi,
            trigrammi,
            parola,
            formula,
        })
    }

    pub fn correggi(&self, testo: &str, continua_dalla_precedente: bool) -> Esito {
        let formule: Vec<(usize, usize)> = self
            .formula
            .find_iter(testo)
            .map(|m| (m.start(), m.end()))
            .collect();
        let mut sostituzioni = Vec::new();

        for m in self.parola.find_iter(testo) {
            let originale = m.as_str();
            let parola = normalizza(originale);
            if parola.chars().count() < 5
                || parola.contains('\'')
                || originale.chars().next().is_some_and(char::is_uppercase)
                || (originale.chars().count() > 1
                    && originale
                        .chars()
                        .all(|c| !c.is_alphabetic() || c.is_uppercase()))
                || formule
                    .iter()
                    .any(|&(inizio, fine)| m.start() < fine && m.end() > inizio)
                || sillabata_dopo(testo, m.end())
                || sillabata_prima(testo, m.start(), continua_dalla_precedente)
                || self.valida(&parola)
                || self.e_inglese(&parola)
            {
                continue;
            }

            let candidati = self.candidati_distanza_uno(&parola);
            if candidati.len() == 1 && self.frequenze_lessico.contains_key(&candidati[0]) {
                sostituzioni.push((m.start(), m.end(), candidati[0].clone(), "dizionario"));
            }
        }

        costruisci_esito(testo, sostituzioni)
    }

    /// Secondo livello: fra piu' alternative italiane a distanza 1 interpola
    /// frequenza della forma, bigrammi e trigrammi del lessico locale. Il margine 3
    /// e' il braccio prudente del test sul development holdout.
    pub fn correggi_contesto(&self, testo: &str, continua_dalla_precedente: bool) -> Esito {
        if self.bigrammi.is_empty() && self.trigrammi.is_empty() {
            return Esito { testo: testo.to_owned(), correzioni: Vec::new() };
        }
        const MARGINE_MINIMO: f64 = 3.0;
        let formule: Vec<(usize, usize)> = self
            .formula
            .find_iter(testo)
            .map(|m| (m.start(), m.end()))
            .collect();
        let token: Vec<_> = self.parola.find_iter(testo).collect();
        let mut sostituzioni = Vec::new();
        for (indice, m) in token.iter().enumerate() {
            let originale = m.as_str();
            let parola = normalizza(originale);
            if parola.chars().count() < 5
                || parola.contains('\'')
                || originale.chars().next().is_some_and(char::is_uppercase)
                || originale
                    .chars()
                    .all(|c| !c.is_alphabetic() || c.is_uppercase())
                || formule
                    .iter()
                    .any(|&(inizio, fine)| m.start() < fine && m.end() > inizio)
                || sillabata_dopo(testo, m.end())
                || sillabata_prima(testo, m.start(), continua_dalla_precedente)
                || self.valida(&parola)
                || self.e_inglese(&parola)
            {
                continue;
            }
            let mut candidate: Vec<String> = self
                .candidati_distanza_uno(&parola)
                .into_iter()
                .filter(|c| self.frequenze_lessico.contains_key(c))
                .collect();
            if candidate.len() < 2 {
                continue;
            }
            let sinistra = indice
                .checked_sub(1)
                .and_then(|i| token.get(i))
                .map(|m| normalizza(m.as_str()))
                .unwrap_or_else(|| "<s>".into());
            let destra = token
                .get(indice + 1)
                .map(|m| normalizza(m.as_str()))
                .unwrap_or_else(|| "</s>".into());
            candidate.sort_by(|a, b| {
                self.punteggio_contesto(&sinistra, b, &destra)
                    .total_cmp(&self.punteggio_contesto(&sinistra, a, &destra))
                    .then_with(|| {
                        self.frequenza(b)
                            .cmp(&self.frequenza(a))
                            .then_with(|| a.cmp(b))
                    })
            });
            let primo = self.punteggio_contesto(&sinistra, &candidate[0], &destra);
            let secondo = self.punteggio_contesto(&sinistra, &candidate[1], &destra);
            if primo - secondo >= MARGINE_MINIMO {
                sostituzioni.push((m.start(), m.end(), candidate[0].clone(), "contesto"));
            }
        }

        costruisci_esito(testo, sostituzioni)
    }

    pub fn correggi_varianti(&self, testo: &str, continua_dalla_precedente: bool) -> Varianti {
        let dizionario = self.correggi(testo, continua_dalla_precedente);
        let contesto = self.correggi_contesto(testo, continua_dalla_precedente);
        let secondo_passaggio = self.correggi_contesto(&dizionario.testo, continua_dalla_precedente);
        let mut correzioni_dizionario = dizionario.correzioni.clone();
        let mut delta_precedente = 0isize;
        let spostamenti: Vec<(usize, isize)> = secondo_passaggio
            .correzioni
            .iter()
            .map(|c| {
                let inizio_ingresso = (c.inizio as isize - delta_precedente) as usize;
                let delta =
                    c.corretta.chars().count() as isize - c.originale.chars().count() as isize;
                delta_precedente += delta;
                (inizio_ingresso, delta)
            })
            .collect();
        for c in &mut correzioni_dizionario {
            let spostamento: isize = spostamenti
                .iter()
                .filter(|(inizio, _)| *inizio < c.inizio)
                .map(|(_, delta)| *delta)
                .sum();
            c.inizio = (c.inizio as isize + spostamento) as usize;
            c.fine = (c.fine as isize + spostamento) as usize;
        }
        correzioni_dizionario.extend(secondo_passaggio.correzioni.clone());
        correzioni_dizionario.sort_by_key(|c| c.inizio);
        Varianti {
            dizionario,
            contesto,
            entrambe: Esito {
                testo: secondo_passaggio.testo,
                correzioni: correzioni_dizionario,
            },
        }
    }

    fn frequenza(&self, parola: &str) -> u32 {
        self.frequenze_lessico.get(parola).copied().unwrap_or(0)
    }

    fn punteggio_contesto(&self, sinistra: &str, candidata: &str, destra: &str) -> f64 {
        let uni = self.frequenza(candidata) as f64;
        let bi_sx = self
            .bigrammi
            .get(&(sinistra.into(), candidata.into()))
            .copied()
            .unwrap_or(0) as f64;
        let bi_dx = self
            .bigrammi
            .get(&(candidata.into(), destra.into()))
            .copied()
            .unwrap_or(0) as f64;
        let tri = self
            .trigrammi
            .get(&(sinistra.into(), candidata.into(), destra.into()))
            .copied()
            .unwrap_or(0) as f64;
        uni.ln_1p()
            + 1.5 * (10.0 * bi_sx).ln_1p()
            + 1.5 * (10.0 * bi_dx).ln_1p()
            + 2.0 * (30.0 * tri).ln_1p()
    }

    fn valida(&self, parola: &str) -> bool {
        self.frequenze_lessico.contains_key(parola) || self.dizionario.check(parola)
    }

    /// Vera per una parola inglese che l'italiano non riconosce.
    ///
    /// Si consulta solo dopo `valida`, quindi le parole che esistono in
    /// entrambe le lingue (`note`, `piano`, `base`) non arrivano mai qui: sono
    /// gia' state lasciate stare come italiane. Resta il caso che conta, la
    /// parola inglese che in italiano sembra un errore di una lettera.
    ///
    /// Il prezzo e' una correzione italiana in meno quando l'errore OCR cade
    /// per caso su una parola inglese. Si accetta perche' il danno non e'
    /// simmetrico: non correggere lascia il testo com'e', correggere male lo
    /// altera in modo che nessuno rilegge piu'.
    fn e_inglese(&self, parola: &str) -> bool {
        self.inglese
            .as_ref()
            .is_some_and(|dizionario| dizionario.check(parola))
    }

    /// Enumera davvero tutte le modifiche a distanza 1. Questo e' piu'
    /// conservativo che prendere la prima proposta ordinata da Hunspell.
    fn candidati_distanza_uno(&self, parola: &str) -> Vec<String> {
        let caratteri: Vec<char> = parola.chars().collect();
        let alfabeto: Vec<char> = ALFABETO.chars().collect();
        let mut generati = HashSet::new();

        for i in 0..caratteri.len() {
            let mut v = caratteri.clone();
            v.remove(i);
            generati.insert(v.iter().collect::<String>());
        }
        for i in 0..caratteri.len() {
            for &c in &alfabeto {
                if c == caratteri[i] {
                    continue;
                }
                let mut v = caratteri.clone();
                v[i] = c;
                generati.insert(v.iter().collect::<String>());
            }
        }
        for i in 0..=caratteri.len() {
            for &c in &alfabeto {
                let mut v = caratteri.clone();
                v.insert(i, c);
                generati.insert(v.iter().collect::<String>());
            }
        }
        for i in 0..caratteri.len().saturating_sub(1) {
            if caratteri[i] != caratteri[i + 1] {
                let mut v = caratteri.clone();
                v.swap(i, i + 1);
                generati.insert(v.iter().collect::<String>());
            }
        }

        let mut valide: Vec<String> = generati.into_iter().filter(|p| self.valida(p)).collect();
        valide.sort();
        valide
    }
}

fn leggi_opzionale(percorso: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(percorso) {
        Ok(testo) => Ok(Some(testo)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("risorsa lessicale non leggibile: {e}")),
    }
}

fn costruisci_esito(testo: &str, sostituzioni: Vec<(usize, usize, String, &'static str)>) -> Esito {
    let mut fuori = String::with_capacity(testo.len());
    let mut correzioni = Vec::with_capacity(sostituzioni.len());
    let mut ultimo = 0;
    let mut caratteri_fuori = 0;
    for (inizio, fine, nuova, metodo) in sostituzioni {
        let prima = &testo[ultimo..inizio];
        fuori.push_str(prima);
        caratteri_fuori += prima.chars().count();
        let da = caratteri_fuori;
        fuori.push_str(&nuova);
        caratteri_fuori += nuova.chars().count();
        correzioni.push(Correzione {
            inizio: da,
            fine: caratteri_fuori,
            originale: testo[inizio..fine].to_string(),
            corretta: nuova,
            metodo: metodo.into(),
        });
        ultimo = fine;
    }
    fuori.push_str(&testo[ultimo..]);
    Esito {
        testo: fuori,
        correzioni,
    }
}

/// Vero se dopo la parola c'e' un trattino di sillabazione.
///
/// Non basta cercare `-\n`: quando la parola e' spezzata fra due **pagine**,
/// il testo della pagina finisce con il trattino e nient'altro, e la vecchia
/// condizione non scattava. Cosi' `repos-` a fondo pagina e il suo seguito
/// `itories` a inizio della successiva arrivavano scoperti al correttore, che
/// li vedeva come parole sbagliate: `itories` e' diventato `stories`.
fn sillabata_dopo(testo: &str, fine: usize) -> bool {
    let resto = &testo[fine..];
    resto.starts_with("-\n") || (resto.starts_with('-') && resto[1..].trim().is_empty())
}

/// Vero se prima della parola c'e' un trattino di sillabazione, oppure se la
/// parola apre un testo che continua una parola spezzata nella pagina prima.
fn sillabata_prima(testo: &str, inizio: usize, continua_dalla_precedente: bool) -> bool {
    let prima = &testo[..inizio];
    prima.ends_with("-\n") || (continua_dalla_precedente && prima.trim().is_empty())
}

/// Vero se quel testo lascia una parola a meta', cioe' finisce col trattino.
pub fn resta_spezzata(testo: &str) -> bool {
    testo.trim_end().ends_with('-')
}

fn normalizza(parola: &str) -> String {
    parola
        .replace('’', "'")
        .nfc()
        .flat_map(char::to_lowercase)
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    const AFF: &str = "SET UTF-8\nTRY abcdefghijklmnopqrstuvwxyzàèéìòù\n";
    const DIC: &str = "6\naffermazione\nitaliano\nitaliana\ncomputer\npoiché\nvalore\n";
    const TRAIN: &str = "affermazione\t2\nitaliano\t8\npoiché\t28\nvalore\t10\n";
    const AFF_EN: &str = "SET UTF-8\n";

    fn correttore() -> Correttore {
        Correttore::da_testi(AFF, DIC, TRAIN, "", None).unwrap()
    }

    fn dir_risorse(lingua: &str) -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../resources/dictionaries")
            .join(lingua)
    }

    #[test]
    fn corregge_lesempio_misurato() {
        let esito = correttore().correggi("una afferhazione italiana", false);
        assert_eq!(esito.testo, "una affermazione italiana");
        assert_eq!(esito.correzioni.len(), 1);
        assert_eq!(esito.correzioni[0].originale, "afferhazione");
    }

    #[test]
    fn protegge_maiuscole_formule_e_parole_non_viste() {
        let esito =
            correttore().correggi("Afferhazione $afferhazione$ computer afferhazione-\ncomputer", false);
        assert_eq!(
            esito.testo,
            "Afferhazione $afferhazione$ computer afferhazione-\ncomputer"
        );
        assert!(esito.correzioni.is_empty());
    }

    #[test]
    fn non_corregge_se_due_candidate_sono_valide() {
        let c = Correttore::da_testi(AFF, "3\ncane\npane\nsane\n", "cane\t5\npane\t5\n", "", None)
            .unwrap();
        assert_eq!(c.correggi("lane", false).testo, "lane");
    }

    #[test]
    fn il_contesto_disambigua_due_alternative_valide() {
        let c = Correttore::da_testi(
            AFF,
            "2\nbasta\npasta\n",
            "basta\t10\npasta\t10\n",
            "B\tla\tpasta\t4\nB\tpasta\tfresca\t4\nT\tla\tpasta\tfresca\t4\n",
            None,
        )
        .unwrap();
        let esito = c.correggi_contesto("la casta fresca", false);
        assert_eq!(esito.testo, "la pasta fresca");
        assert_eq!(esito.correzioni[0].metodo, "contesto");
    }

    /// Senza il dizionario inglese `notes` finisce in `note`; con, resta.
    /// Le due meta' del test condividono tutto tranne quello, cosi' la
    /// protezione e' l'unica spiegazione possibile della differenza.
    #[test]
    fn lascia_stare_una_parola_inglese_che_sembra_un_errore() {
        let dic_it = "2\nnote\nnota\n";
        let train_it = "note\t9\n";
        let senza = Correttore::da_testi(AFF, dic_it, train_it, "", None).unwrap();
        assert_eq!(senza.correggi("nelle notes finali", false).testo, "nelle note finali");

        let con = Correttore::da_testi(AFF, dic_it, train_it, "", Some((AFF_EN, "1\nnotes\n")))
            .unwrap();
        let esito = con.correggi("nelle notes finali", false);
        assert_eq!(esito.testo, "nelle notes finali");
        assert!(esito.correzioni.is_empty());
    }

    /// La protezione vale anche per la correzione contestuale: era il senso
    /// di «vale per tutto», non solo per il primo livello.
    #[test]
    fn la_protezione_vale_anche_per_il_contesto() {
        let dic_it = "2\nbasta\npasta\n";
        let train_it = "basta\t10\npasta\t10\n";
        let contesto = "B\tla\tpasta\t4\nB\tpasta\tfresca\t4\nT\tla\tpasta\tfresca\t4\n";
        let senza = Correttore::da_testi(AFF, dic_it, train_it, contesto, None).unwrap();
        assert_eq!(senza.correggi_contesto("la casta fresca", false).testo, "la pasta fresca");

        let con =
            Correttore::da_testi(AFF, dic_it, train_it, contesto, Some((AFF_EN, "1\ncasta\n")))
                .unwrap();
        let esito = con.correggi_contesto("la casta fresca", false);
        assert_eq!(esito.testo, "la casta fresca");
        assert!(esito.correzioni.is_empty());
    }

    /// Un dizionario inglese assente non deve rompere niente: si torna al
    /// comportamento di prima, che e' il caso dei pacchetti gia' distribuiti.
    #[test]
    fn senza_dizionario_inglese_corregge_come_prima() {
        let c = Correttore::carica(&dir_risorse("it_IT"), None).unwrap();
        assert_eq!(
            c.correggi("questa afferhazione resta verificabile", false).testo,
            "questa affermazione resta verificabile"
        );
    }

    #[test]
    fn le_risorse_spedite_correggono_afferhazione() {
        let dir = dir_risorse("it_IT");
        let c = Correttore::carica(&dir, None).unwrap();
        let esito = c.correggi("questa afferhazione resta verificabile", false);
        assert_eq!(esito.testo, "questa affermazione resta verificabile");
        assert_eq!(esito.correzioni.len(), 1);
        let contestuale = c.correggi_contesto("se perole sono diverse", false);
        assert_eq!(contestuale.testo, "se parole sono diverse");
        assert_eq!(contestuale.correzioni[0].metodo, "contesto");
    }

    /// Il caso vero, trovato passando testo inglese al correttore italiano coi
    /// dizionari che spediamo davvero: `values` ha un solo vicino italiano a
    /// distanza 1, `value`, che sta nel lessico locale. La prima meta' del test
    /// documenta il danno, la seconda che il dizionario inglese lo impedisce.
    #[test]
    fn le_risorse_spedite_non_traducono_linglese_in_italiano() {
        let frase = "we define the transfer function and compute the output values";

        let senza = Correttore::carica(&dir_risorse("it_IT"), None).unwrap();
        let rovinata = senza.correggi(frase, false);
        assert_eq!(rovinata.correzioni.len(), 1);
        assert_eq!(rovinata.correzioni[0].originale, "values");
        assert_eq!(rovinata.correzioni[0].corretta, "value");

        let con =
            Correttore::carica(&dir_risorse("it_IT"), Some(&dir_risorse("en_US"))).unwrap();
        let esito = con.correggi(frase, false);
        assert_eq!(esito.testo, frase);
        assert!(esito.correzioni.is_empty());
    }

    /// Il caso dello screenshot: `repos-` chiude una pagina e `itories` apre
    /// la successiva. Sono le due meta' di `repositories`, e il correttore
    /// non deve toccare ne' l'una ne' l'altra.
    #[test]
    fn non_tocca_le_parole_spezzate_fra_due_pagine() {
        // `itorie` esiste in italiano ed e' a distanza 1 da `itories`: senza
        // la protezione il frammento verrebbe "corretto".
        let dic = "2\nitorie\nrepose\n";
        let train = "itorie\t7\nrepose\t7\n";
        let c = Correttore::da_testi(AFF, dic, train, "", None).unwrap();

        // Fine pagina: il testo si chiude col trattino, senza newline.
        let coda = "among repos-";
        assert!(resta_spezzata(coda));
        assert_eq!(c.correggi(coda, false).testo, coda);

        // Inizio pagina successiva, sapendo come si era chiusa la precedente.
        let testa = "itories that declared";
        assert_eq!(c.correggi(testa, true).testo, testa);

        // Senza quel contesto il frammento resta scoperto: e' esattamente il
        // difetto, e va documentato perche' non torni.
        assert_ne!(c.correggi(testa, false).testo, testa);
    }

    /// La sillabazione a fine riga dentro la stessa pagina continua a valere.
    #[test]
    fn la_sillabazione_a_fine_riga_resta_protetta() {
        let dic = "2\nitorie\nrepose\n";
        let train = "itorie\t7\nrepose\t7\n";
        let c = Correttore::da_testi(AFF, dic, train, "", None).unwrap();
        let testo = "among repos-\nitories that";
        assert_eq!(c.correggi(testo, false).testo, testo);
    }

    /// L'italiano deve continuare a essere corretto anche col dizionario
    /// inglese caricato: la protezione non deve spegnere il correttore.
    #[test]
    fn litaliano_resta_corretto_col_dizionario_inglese_caricato() {
        let c = Correttore::carica(&dir_risorse("it_IT"), Some(&dir_risorse("en_US"))).unwrap();
        let esito = c.correggi("questa afferhazione resta verificabile", false);
        assert_eq!(esito.testo, "questa affermazione resta verificabile");
        assert_eq!(esito.correzioni.len(), 1);
    }
}
