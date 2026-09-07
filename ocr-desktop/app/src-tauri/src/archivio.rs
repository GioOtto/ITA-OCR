//! Archivio delle trascrizioni gia' fatte.
//!
//! Di una sessione si salva il testo (nelle sue varianti: grezzo, corretto,
//! contestuale) con le correzioni, piu' il **percorso** dei file sorgente. Le
//! miniature non si duplicano: si rigenerano dal sorgente come fa `anteprima`,
//! e questo tiene l'archivio leggero (qualche decina di KB per sessione invece
//! di svariati MB).
//!
//! Il percorso ha un secondo vantaggio: se il file e' ancora al suo posto, la
//! sessione ricaricata e' di nuovo lavorabile e ci si puo' rifare l'OCR sopra.
//! Se non c'e' piu', il testo resta comunque leggibile ed esportabile: si perde
//! solo l'immagine.

use crate::documenti;
use crate::lavoro::{DettaglioTentativo, Pagina, Stato, StatoPagina};
use crate::{attenzione, correzione, file_atomico, info, risorse};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

/// Una pagina come finisce su disco: i campi che il modello ha prodotto, senza
/// nulla che dipenda dallo stato in memoria.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct PaginaSalvata {
    pub documento: usize,
    pub nome_documento: String,
    pub numero: usize,
    pub pagine_documento: usize,
    pub stato: StatoPagina,
    pub fast_path: bool,
    pub secondi: f64,
    pub stadio: Option<String>,
    pub tentativi: usize,
    pub messaggio: Option<String>,
    pub caratteri: usize,
    #[serde(default)]
    pub dettaglio: Vec<DettaglioTentativo>,
    pub token_prompt: Option<u64>,
    #[serde(default)]
    pub correzioni: usize,
    #[serde(default)]
    pub correzioni_dizionario: usize,
    #[serde(default)]
    pub correzioni_contesto: usize,
    #[serde(default)]
    pub testo: String,
    #[serde(default)]
    pub testo_originale: String,
    #[serde(default)]
    pub testo_corretto: String,
    #[serde(default)]
    pub testo_contestuale: String,
    #[serde(default)]
    pub testo_completo: String,
    #[serde(default)]
    pub dettagli_correzione: Vec<correzione::Correzione>,
    #[serde(default)]
    pub dettagli_contestuali: Vec<correzione::Correzione>,
    #[serde(default)]
    pub dettagli_completi: Vec<correzione::Correzione>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DocumentoSalvato {
    pub nome: String,
    /// Vuoto quando il documento e' stato aperto da una sessione in cui il
    /// sorgente era gia' sparito: si conserva comunque il nome.
    pub percorso: String,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Sessione {
    pub id: String,
    pub nome: String,
    /// Secondi da epoch: basta a ordinare e a mostrare una data.
    pub istante: i64,
    pub documenti: Vec<DocumentoSalvato>,
    pub pagine: Vec<PaginaSalvata>,
}

/// La riga dell'elenco: quanto serve al pannello laterale, senza caricare i testi.
#[derive(Clone, serde::Serialize)]
pub struct Voce {
    pub id: String,
    pub nome: String,
    pub istante: i64,
    pub pagine: usize,
    pub pagine_lette: usize,
    pub caratteri: usize,
    pub documenti: Vec<String>,
    /// Vero se tutti i sorgenti sono ancora al loro posto: solo allora la
    /// sessione si puo' rielaborare.
    pub completa: bool,
}

fn cartella() -> PathBuf {
    risorse::dir_dati().join("archivio")
}

fn id_valido(id: &str) -> bool {
    let Some(cifre) = id.strip_prefix('s') else {
        return false;
    };
    (10..=20).contains(&cifre.len()) && cifre.bytes().all(|c| c.is_ascii_digit())
}

fn file_sessione(id: &str) -> Result<PathBuf, String> {
    if !id_valido(id) {
        return Err("identificativo di sessione non valido".into());
    }
    Ok(cartella().join(format!("{id}.json")))
}

fn adesso() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Un identificativo che ordina cronologicamente e non collide fra sessioni
/// salvate nello stesso secondo.
fn nuovo_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("s{millis}")
}

/// Il nome da mostrare: il documento se e' uno solo, altrimenti quanti sono.
fn nome_predefinito(documenti: &[DocumentoSalvato]) -> String {
    match documenti.len() {
        0 => "sessione vuota".into(),
        1 => documenti[0].nome.clone(),
        n => format!("{} e altri {} documenti", documenti[0].nome, n - 1),
    }
}

// ------------------------------------------------------------------ lettura

pub fn elenco() -> Vec<Voce> {
    let mut voci: Vec<Voce> = Vec::new();
    let Ok(contenuto) = std::fs::read_dir(cartella()) else {
        return voci;
    };
    for voce in contenuto.flatten() {
        let percorso = voce.path();
        if percorso.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Some(sessione) = leggi(&percorso) else {
            continue;
        };
        // Non basta fidarsi dell'id dentro il JSON: deve essere valido e deve
        // corrispondere al nome del file da cui e' stato letto.
        if !id_valido(&sessione.id)
            || percorso.file_stem().and_then(|n| n.to_str()) != Some(sessione.id.as_str())
        {
            attenzione!(
                "sessione con identificativo non valido: {}",
                percorso.display()
            );
            continue;
        }
        let pagine_lette = sessione.pagine.iter().filter(|p| p.caratteri > 0).count();
        voci.push(Voce {
            id: sessione.id.clone(),
            nome: sessione.nome.clone(),
            istante: sessione.istante,
            pagine: sessione.pagine.len(),
            pagine_lette,
            caratteri: sessione.pagine.iter().map(|p| p.caratteri).sum(),
            documenti: sessione.documenti.iter().map(|d| d.nome.clone()).collect(),
            completa: sessione
                .documenti
                .iter()
                .all(|d| !d.percorso.is_empty() && Path::new(&d.percorso).is_file()),
        });
    }
    // Le piu' recenti in cima.
    voci.sort_by_key(|voce| std::cmp::Reverse(voce.istante));
    voci
}

fn leggi(percorso: &Path) -> Option<Sessione> {
    let testo = std::fs::read_to_string(percorso).ok()?;
    match serde_json::from_str::<Sessione>(&testo) {
        Ok(s) => Some(s),
        Err(e) => {
            attenzione!("sessione illeggibile {}: {e}", percorso.display());
            None
        }
    }
}

// ------------------------------------------------------------ scrittura

/// Scrive la sessione corrente. Restituisce l'id, oppure None se non c'era
/// niente che valesse la pena salvare.
pub fn salva(stato: &Stato, nome: Option<String>) -> Result<Option<String>, String> {
    // Si legge subito, e si rilascia: piu' avanti servono altri lucchetti e
    // tenerne due aperti insieme e' il modo classico per incastrarsi.
    let precedente = stato.sessione.lock().ok().and_then(|s| s.clone());
    let pagine = stato.pagine.lock().map_err(|_| "stato inconsistente")?;
    // Una sessione senza una riga di testo non merita una voce nell'archivio.
    if pagine.iter().all(|p| p.caratteri == 0) {
        return Ok(None);
    }
    let documenti = stato.documenti.lock().map_err(|_| "stato inconsistente")?;
    let salvati: Vec<DocumentoSalvato> = documenti
        .iter()
        .map(|d| DocumentoSalvato {
            nome: d.nome.clone(),
            percorso: d.percorso.clone(),
        })
        .collect();

    // Un nome gia' dato dall'utente non si sovrascrive a ogni rielaborazione.
    let nome = match (nome, precedente.as_deref().and_then(nome_esistente)) {
        (Some(scelto), _) => scelto,
        (None, Some(gia_dato)) => gia_dato,
        (None, None) => nome_predefinito(&salvati),
    };
    let sessione = Sessione {
        id: precedente.unwrap_or_else(nuovo_id),
        nome,
        istante: adesso(),
        documenti: salvati,
        pagine: pagine.iter().map(da_pagina).collect(),
    };
    scrivi(&sessione)?;
    if let Ok(mut corrente) = stato.sessione.lock() {
        *corrente = Some(sessione.id.clone());
    }
    info!(
        "sessione archiviata: {} ({} pagine)",
        sessione.nome,
        sessione.pagine.len()
    );
    Ok(Some(sessione.id))
}

/// Il nome gia' salvato per quella voce, se la voce esiste ancora.
fn nome_esistente(id: &str) -> Option<String> {
    file_sessione(id)
        .ok()
        .and_then(|percorso| leggi(&percorso))
        .map(|s| s.nome)
}

fn scrivi(sessione: &Sessione) -> Result<(), String> {
    let dir = cartella();
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("cartella dell'archivio non creabile: {e}"))?;
    let testo = serde_json::to_string_pretty(sessione)
        .map_err(|e| format!("sessione non serializzabile: {e}"))?;
    let percorso = file_sessione(&sessione.id)?;
    file_atomico::scrivi(&percorso, testo.as_bytes())
        .map_err(|e| format!("sessione non scrivibile: {e}"))
}

/// Salvataggio di cortesia a fine elaborazione: non deve mai far fallire il
/// lavoro vero, quindi un errore si annota e basta.
pub fn salva_in_silenzio(stato: &Stato) {
    match salva(stato, None) {
        Ok(_) => {}
        Err(e) => attenzione!("sessione non archiviata: {e}"),
    }
}

pub fn elimina(id: &str) -> Result<(), String> {
    std::fs::remove_file(file_sessione(id)?).map_err(|e| format!("sessione non eliminabile: {e}"))
}

pub fn rinomina(id: &str, nome: &str) -> Result<(), String> {
    let percorso = file_sessione(id)?;
    let mut sessione = leggi(&percorso).ok_or("sessione non trovata")?;
    sessione.nome = nome.trim().to_string();
    if sessione.nome.is_empty() {
        sessione.nome = nome_predefinito(&sessione.documenti);
    }
    scrivi(&sessione)
}

// ------------------------------------------------------------- caricamento

/// Rimette una sessione dentro lo stato, al posto di quella corrente.
///
/// I sorgenti ancora presenti vengono riaperti, cosi' la sessione torna
/// pienamente lavorabile; quelli spariti lasciano le pagine senza immagine.
/// Restituisce quanti documenti non si sono potuti riaprire.
pub fn carica(stato: &Stato, id: &str) -> Result<usize, String> {
    let sessione = leggi(&file_sessione(id)?).ok_or("sessione non trovata")?;
    let forza = stato
        .impostazioni
        .lock()
        .map(|i| i.forza_ocr)
        .unwrap_or(false);

    // Si riaprono i sorgenti disponibili tenendo la corrispondenza fra il
    // vecchio indice salvato e quello nuovo nello stato.
    let mut riaperti: Vec<Arc<crate::documenti::Documento>> = Vec::new();
    let mut mappa: Vec<Option<usize>> = Vec::with_capacity(sessione.documenti.len());
    let mut mancanti = 0usize;
    for salvato in &sessione.documenti {
        let percorso = PathBuf::from(&salvato.percorso);
        if salvato.percorso.is_empty() || !percorso.is_file() {
            mappa.push(None);
            mancanti += 1;
            continue;
        }
        match documenti::apri(&percorso, forza) {
            Ok(documento) => {
                riaperti.push(Arc::new(documento));
                mappa.push(Some(riaperti.len() - 1));
            }
            Err(e) => {
                attenzione!("{}: {e}", salvato.nome);
                mappa.push(None);
                mancanti += 1;
            }
        }
    }

    let pagine: Vec<Pagina> = sessione
        .pagine
        .iter()
        .enumerate()
        .map(|(posizione, salvata)| {
            // Un indice fuori range fa fallire `anteprima` con grazia: la
            // pagina resta leggibile, l'immagine no.
            let indice = mappa
                .get(salvata.documento)
                .copied()
                .flatten()
                .unwrap_or(usize::MAX);
            a_pagina(salvata, indice, posizione)
        })
        .collect();

    {
        let mut d = stato.documenti.lock().map_err(|_| "stato inconsistente")?;
        *d = riaperti;
    }
    {
        let mut p = stato.pagine.lock().map_err(|_| "stato inconsistente")?;
        *p = pagine;
    }
    // Da qui in poi si lavora su questa voce: rielaborare aggiorna lei.
    if let Ok(mut corrente) = stato.sessione.lock() {
        *corrente = Some(sessione.id.clone());
    }
    info!(
        "sessione ripresa: {} ({} pagine, {mancanti} sorgenti non più disponibili)",
        sessione.nome,
        sessione.pagine.len()
    );
    Ok(mancanti)
}

// ---------------------------------------------------------- conversioni

fn da_pagina(p: &Pagina) -> PaginaSalvata {
    PaginaSalvata {
        documento: p.documento,
        nome_documento: p.nome_documento.clone(),
        numero: p.numero,
        pagine_documento: p.pagine_documento,
        stato: p.stato,
        fast_path: p.fast_path,
        secondi: p.secondi,
        stadio: p.stadio.clone(),
        tentativi: p.tentativi,
        messaggio: p.messaggio.clone(),
        caratteri: p.caratteri,
        dettaglio: p.dettaglio.clone(),
        token_prompt: p.token_prompt,
        correzioni: p.correzioni,
        correzioni_dizionario: p.correzioni_dizionario,
        correzioni_contesto: p.correzioni_contesto,
        testo: p.testo.clone(),
        testo_originale: p.testo_originale.clone(),
        testo_corretto: p.testo_corretto.clone(),
        testo_contestuale: p.testo_contestuale.clone(),
        testo_completo: p.testo_completo.clone(),
        dettagli_correzione: p.dettagli_correzione.clone(),
        dettagli_contestuali: p.dettagli_contestuali.clone(),
        dettagli_completi: p.dettagli_completi.clone(),
    }
}

fn a_pagina(s: &PaginaSalvata, indice_documento: usize, posizione: usize) -> Pagina {
    Pagina {
        // L'id si rigenera sulla posizione: quello vecchio faceva riferimento a
        // indici di documento che dopo il riordino non valgono piu'.
        id: format!("d{indice_documento}p{posizione}"),
        documento: indice_documento,
        nome_documento: s.nome_documento.clone(),
        numero: s.numero,
        pagine_documento: s.pagine_documento,
        stato: s.stato,
        fast_path: s.fast_path,
        secondi: s.secondi,
        stadio: s.stadio.clone(),
        tentativi: s.tentativi,
        messaggio: s.messaggio.clone(),
        caratteri: s.caratteri,
        dettaglio: s.dettaglio.clone(),
        token_prompt: s.token_prompt,
        correzioni: s.correzioni,
        correzioni_dizionario: s.correzioni_dizionario,
        correzioni_contesto: s.correzioni_contesto,
        testo: s.testo.clone(),
        testo_originale: s.testo_originale.clone(),
        testo_corretto: s.testo_corretto.clone(),
        testo_contestuale: s.testo_contestuale.clone(),
        testo_completo: s.testo_completo.clone(),
        dettagli_correzione: s.dettagli_correzione.clone(),
        dettagli_contestuali: s.dettagli_contestuali.clone(),
        dettagli_completi: s.dettagli_completi.clone(),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn accetta_solo_identificativi_generati_dall_app() {
        assert!(id_valido("s1725638400123"));
        assert!(id_valido("s12345678901234567890"));
        for id in [
            "../../impostazioni",
            "s123.json/../altro",
            "x1725638400123",
            "s123456789",
            "s123456789012345678901",
            "s172563840O123",
        ] {
            assert!(!id_valido(id), "accettato {id:?}");
        }
    }
}
