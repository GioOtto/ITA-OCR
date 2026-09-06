//! La cascata condizionale, in streaming, identica a quella gia' valutata.
//!
//! Porta di `src/training/cascata_glm_vulkan.py`. Il rilevatore di ciclo non e'
//! una cura dell'EOS: serve a riconoscere presto che la generazione e' entrata
//! in loop e a far partire il retry senza spendere i 4096 token del tetto. La
//! soglia e' quella conservativa gia' usata offline (8 ripetizioni esatte,
//! periodo <= 256, span >= 16 token), non la regola aggressiva "4-gram ripetuto
//! 3 volte", che troncherebbe formule e liste legittime.

use crate::http::{self, Pezzo};
use crate::{attenzione, info};
use serde_json::json;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const N_PREDICT: u32 = 4096;
/// Nessun dato dal server per questo tempo: qualcosa si e' rotto.
const STALLO_MASSIMO: Duration = Duration::from_secs(600);

/// Gli stadi nell'ordine in cui vengono provati. I parametri sono quelli
/// misurati sul pannello da 48 pagine; `penalty_last_n` non viene passato di
/// proposito, resta il default 64 di llama.cpp.
pub fn stadi() -> Vec<(&'static str, serde_json::Value)> {
    vec![
        ("greedy", json!({})),
        (
            "discord_f35_p20",
            json!({"repeat_penalty": 1.0, "frequency_penalty": 0.35,
                   "presence_penalty": 0.20, "min_p": 0.05}),
        ),
        (
            "frequency35",
            json!({"repeat_penalty": 1.0, "frequency_penalty": 0.35,
                   "presence_penalty": 0.0, "min_p": 0.05}),
        ),
    ]
}

pub struct RilevatoreCiclo {
    min_ripetizioni: usize,
    periodo_massimo: usize,
    span_minimo: usize,
    token: Vec<i64>,
    corse: Vec<usize>,
    migliore_ripetizioni: usize,
    migliore_span: usize,
    migliore_periodo: usize,
    pub rilevato: bool,
    pub token_alla_scoperta: Option<usize>,
}

impl RilevatoreCiclo {
    pub fn nuovo() -> Self {
        Self::con_soglie(8, 256, 16)
    }

    pub fn con_soglie(min_ripetizioni: usize, periodo_massimo: usize, span_minimo: usize) -> Self {
        RilevatoreCiclo {
            min_ripetizioni,
            periodo_massimo,
            span_minimo,
            token: Vec::new(),
            corse: vec![0; periodo_massimo + 1],
            migliore_ripetizioni: 0,
            migliore_span: 0,
            migliore_periodo: 0,
            rilevato: false,
            token_alla_scoperta: None,
        }
    }

    /// Aggiunge un token; `true` se il ciclo e' appena scattato.
    pub fn aggiungi(&mut self, token: i64) -> bool {
        self.token.push(token);
        let indice = self.token.len() - 1;
        let limite = self.periodo_massimo.min(indice);
        for periodo in 1..=limite {
            // Al primo indice utile (indice == periodo) la corsa vale periodo,
            // esattamente come l'inizializzazione della versione offline.
            let corrente = if self.corse[periodo] == 0 {
                periodo
            } else {
                self.corse[periodo]
            };
            let corsa = if self.token[indice] == self.token[indice - periodo] {
                corrente + 1
            } else {
                periodo
            };
            self.corse[periodo] = corsa;
            let ripetizioni = corsa / periodo;
            if ripetizioni > self.migliore_ripetizioni
                || (ripetizioni == self.migliore_ripetizioni && corsa > self.migliore_span)
            {
                self.migliore_ripetizioni = ripetizioni;
                self.migliore_span = corsa;
                self.migliore_periodo = periodo;
            }
        }
        if !self.rilevato
            && self.migliore_ripetizioni >= self.min_ripetizioni
            && self.migliore_span >= self.span_minimo
        {
            self.rilevato = true;
            self.token_alla_scoperta = Some(self.token.len());
            return true;
        }
        false
    }

    pub fn descrizione(&self) -> String {
        format!(
            "periodo {} token, {} ripetizioni, span {}",
            self.migliore_periodo, self.migliore_ripetizioni, self.migliore_span
        )
    }
}

#[derive(Default)]
pub struct Esito {
    pub testo: String,
    pub testo_alla_scoperta: Option<String>,
    pub eos: bool,
    pub limite_raggiunto: bool,
    pub interrotto_per_ciclo: bool,
    pub annullato: bool,
    pub token: usize,
    pub secondi: f64,
    pub token_al_secondo: Option<f64>,
    pub token_prompt: Option<u64>,
    pub descrizione_ciclo: Option<String>,
}

/// Cosa la UI deve sapere mentre la generazione procede.
pub enum Evento<'a> {
    /// Nuovo stadio: la UI deve azzerare il testo mostrato.
    Stadio { indice: usize, nome: &'a str },
    /// Testo appena generato, da appendere.
    Testo(&'a str),
}

pub struct Contesto<'a> {
    pub porta: u16,
    pub prompt: &'a str,
    pub immagine_base64: &'a str,
    pub annulla: &'a Arc<AtomicBool>,
}

fn attendi_slot_libero(porta: u16) {
    // Dopo un abort la slot va liberata: senza attesa il retry si accoda.
    let scadenza = Instant::now() + Duration::from_secs(60);
    while Instant::now() < scadenza {
        match http::json(porta, "GET", "/slots", None, Duration::from_secs(5)) {
            Ok(valore) => {
                let libere = valore
                    .as_array()
                    .map(|slot| {
                        slot.iter()
                            .all(|s| s.get("is_processing").and_then(|p| p.as_bool()) == Some(false))
                    })
                    .unwrap_or(true);
                if libere {
                    return;
                }
            }
            // Endpoint non disponibile: non c'e' niente da aspettare.
            Err(_) => return,
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    attenzione!("slot ancora occupata dopo l'abort: proseguo comunque");
}

fn genera_stream(
    contesto: &Contesto,
    extra: &serde_json::Value,
    rilevatore: Option<&mut RilevatoreCiclo>,
    interrompi: bool,
    su_evento: &mut dyn FnMut(Evento),
) -> Result<Esito, String> {
    let mut corpo = json!({
        "prompt": {
            "prompt_string": contesto.prompt,
            "multimodal_data": [contesto.immagine_base64],
        },
        "n_predict": N_PREDICT,
        "temperature": 0,
        "seed": 0,
        "cache_prompt": false,
        "return_tokens": true,
        "stream": true,
    });
    if let (Some(destinazione), Some(sorgente)) = (corpo.as_object_mut(), extra.as_object()) {
        for (chiave, valore) in sorgente {
            destinazione.insert(chiave.clone(), valore.clone());
        }
    }
    let serializzato = corpo.to_string();

    let inizio = Instant::now();
    let mut risposta = http::apri(
        contesto.porta,
        "POST",
        "/completion",
        Some(&serializzato),
        Duration::from_secs(10),
        Duration::from_millis(200),
    )?;
    if risposta.stato() >= 400 {
        let messaggio = risposta.corpo_intero(50).unwrap_or_default();
        return Err(format!("il motore ha rifiutato la pagina: {messaggio}"));
    }

    let mut esito = Esito::default();
    let mut pezzi = String::new();
    let mut rilevatore = rilevatore;
    let mut ultimo_dato = Instant::now();
    let mut token_visti = 0usize;

    loop {
        if contesto.annulla.load(Ordering::Relaxed) {
            risposta.interrompi();
            esito.annullato = true;
            break;
        }
        match risposta.prossima_riga()? {
            Pezzo::Attesa => {
                if ultimo_dato.elapsed() > STALLO_MASSIMO {
                    risposta.interrompi();
                    return Err("il motore non risponde più".into());
                }
                continue;
            }
            Pezzo::Fine => break,
            Pezzo::Riga(riga) => {
                ultimo_dato = Instant::now();
                let riga = riga.trim();
                if !riga.starts_with("data:") {
                    continue;
                }
                let dato: serde_json::Value = serde_json::from_str(riga[5..].trim())
                    .map_err(|e| format!("risposta del motore illeggibile: {e}"))?;
                if let Some(errore) = dato.get("error") {
                    risposta.interrompi();
                    let messaggio = errore
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or(&errore.to_string())
                        .to_string();
                    return Err(format!("llama-server: {messaggio}"));
                }
                let contenuto = dato
                    .get("content")
                    .and_then(|c| c.as_str())
                    .unwrap_or_default();
                if !contenuto.is_empty() {
                    pezzi.push_str(contenuto);
                    su_evento(Evento::Testo(contenuto));
                }
                let nuovi: Vec<i64> = dato
                    .get("tokens")
                    .and_then(|t| t.as_array())
                    .map(|a| a.iter().filter_map(|v| v.as_i64()).collect())
                    .unwrap_or_default();
                token_visti += nuovi.len();

                if dato.get("stop").and_then(|s| s.as_bool()) == Some(true) {
                    let tipo = dato
                        .get("stop_type")
                        .and_then(|s| s.as_str())
                        .unwrap_or_default()
                        .to_string();
                    esito.eos = tipo == "eos";
                    esito.limite_raggiunto = tipo == "limit";
                    if let Some(timings) = dato.get("timings") {
                        esito.token = timings
                            .get("predicted_n")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(token_visti as u64) as usize;
                        esito.token_al_secondo =
                            timings.get("predicted_per_second").and_then(|v| v.as_f64());
                        esito.token_prompt = timings.get("prompt_n").and_then(|v| v.as_u64());
                    }
                    break;
                }
                if let Some(r) = rilevatore.as_deref_mut() {
                    let mut scattato = false;
                    for token in nuovi {
                        scattato |= r.aggiungi(token);
                    }
                    if scattato {
                        // Il prefisso prima del ciclo e' l'unica parte utile: va
                        // salvato anche quando si lascia finire la generazione.
                        esito.testo_alla_scoperta = Some(pezzi.trim().to_string());
                        esito.descrizione_ciclo = Some(r.descrizione());
                        if interrompi {
                            esito.interrotto_per_ciclo = true;
                            risposta.interrompi();
                            break;
                        }
                    }
                }
            }
        }
    }

    esito.testo = pezzi.trim().to_string();
    if esito.token == 0 {
        esito.token = token_visti;
    }
    esito.secondi = inizio.elapsed().as_secs_f64();
    if esito.interrotto_per_ciclo {
        attendi_slot_libero(contesto.porta);
    }
    Ok(esito)
}

pub struct Tentativo {
    pub stadio: String,
    pub eos: bool,
    pub interrotto_per_ciclo: bool,
    pub limite_raggiunto: bool,
    pub token: usize,
    pub secondi: f64,
    pub token_al_secondo: Option<f64>,
    pub descrizione_ciclo: Option<String>,
}

pub struct EsitoPagina {
    pub testo: String,
    pub completata: bool,
    pub annullata: bool,
    pub stadio_accettato: Option<String>,
    pub troncata_al_ciclo: bool,
    pub tentativi: Vec<Tentativo>,
    pub secondi: f64,
    pub token: usize,
    pub token_prompt: Option<u64>,
}

/// Greedy, poi retry con penalita' solo se il greedy non chiude.
///
/// L'abort anticipato serve a far partire prima lo stadio successivo: sull'
/// **ultimo** stadio non c'e' nessuno stadio dopo, quindi interrompere non fa
/// risparmiare niente e puo' solo buttare via una generazione che si sarebbe
/// fermata da sola.
pub fn esegui(
    contesto: &Contesto,
    su_evento: &mut dyn FnMut(Evento),
) -> Result<EsitoPagina, String> {
    let stadi = stadi();
    let mut tentativi = Vec::new();
    let mut ultimo = Esito::default();
    let mut annullata = false;

    for (indice, (nome, extra)) in stadi.iter().enumerate() {
        let ultimo_stadio = indice == stadi.len() - 1;
        su_evento(Evento::Stadio { indice, nome });
        let mut rilevatore = RilevatoreCiclo::nuovo();
        let esito = genera_stream(
            contesto,
            extra,
            Some(&mut rilevatore),
            !ultimo_stadio,
            su_evento,
        )?;
        if esito.interrotto_per_ciclo {
            info!(
                "stadio {nome}: ciclo rilevato ({}), passo allo stadio successivo",
                esito.descrizione_ciclo.clone().unwrap_or_default()
            );
        }
        tentativi.push(Tentativo {
            stadio: nome.to_string(),
            eos: esito.eos,
            interrotto_per_ciclo: esito.interrotto_per_ciclo,
            limite_raggiunto: esito.limite_raggiunto,
            token: esito.token,
            secondi: esito.secondi,
            token_al_secondo: esito.token_al_secondo,
            descrizione_ciclo: esito.descrizione_ciclo.clone(),
        });
        annullata = esito.annullato;
        let chiuso = esito.eos && !esito.interrotto_per_ciclo;
        ultimo = esito;
        if chiuso || annullata {
            break;
        }
    }

    let completata = ultimo.eos && !ultimo.interrotto_per_ciclo && !annullata;
    // Pagina persa: si restituisce il prefisso prima del ciclo, non la coda di
    // ripetizioni, e la si marca incompleta.
    let testo = if completata {
        ultimo.testo.clone()
    } else {
        ultimo
            .testo_alla_scoperta
            .clone()
            .unwrap_or_else(|| ultimo.testo.clone())
    };
    Ok(EsitoPagina {
        testo,
        completata,
        annullata,
        stadio_accettato: if completata {
            tentativi.last().map(|t| t.stadio.clone())
        } else {
            None
        },
        troncata_al_ciclo: !completata && ultimo.testo_alla_scoperta.is_some(),
        secondi: tentativi.iter().map(|t| t.secondi).sum(),
        token: tentativi.iter().map(|t| t.token).sum(),
        token_prompt: ultimo.token_prompt,
        tentativi,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn il_ciclo_scatta_solo_con_otto_ripetizioni() {
        let mut r = RilevatoreCiclo::nuovo();
        // Periodo 4, span 16 richiesto: servono 8 ripetizioni = 32 token.
        let motivo = [11i64, 22, 33, 44];
        let mut scattato_a = None;
        for i in 0..80 {
            if r.aggiungi(motivo[i % 4]) {
                scattato_a = Some(i + 1);
                break;
            }
        }
        assert_eq!(scattato_a, Some(32));
    }

    #[test]
    fn il_testo_normale_non_fa_scattare_il_rilevatore() {
        let mut r = RilevatoreCiclo::nuovo();
        let mut stato = 7u64;
        for _ in 0..3000 {
            stato = stato.wrapping_mul(6364136223846793005).wrapping_add(1);
            assert!(!r.aggiungi((stato >> 33) as i64 % 5000));
        }
        assert!(!r.rilevato);
    }

    #[test]
    fn una_ripetizione_breve_non_basta() {
        let mut r = RilevatoreCiclo::nuovo();
        // Periodo 1 ripetuto 8 volte: span 8 < 16, non deve scattare.
        for _ in 0..8 {
            assert!(!r.aggiungi(42));
        }
        // Alla sedicesima lo span raggiunge la soglia.
        let mut scattato = false;
        for _ in 0..8 {
            scattato |= r.aggiungi(42);
        }
        assert!(scattato);
    }
}
