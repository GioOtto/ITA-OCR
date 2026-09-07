//! Stato dell'applicazione e coda di elaborazione.
//!
//! Un documento e una pagina alla volta: il motore gira con `-np 1` e una sola
//! slot, quindi parallelizzare qui non farebbe che accodare richieste. Il lavoro
//! gia' fatto non si perde mai: ogni pagina viene chiusa e pubblicata prima che
//! parta la successiva, e un errore su una pagina non tocca le precedenti.

use crate::documenti::{self, Documento};
use crate::motore::{Backend, Motore};
use crate::{attenzione, cascata, correzione, errore, file_atomico, imaging, info, risorse};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StatoPagina {
    Attesa,
    Corso,
    Completata,
    Incompleta,
    Errore,
    Annullata,
}

/// Come e' andato un singolo stadio della cascata, per il pannello avanzato.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DettaglioTentativo {
    pub stadio: String,
    pub esito: String,
    pub token: usize,
    pub secondi: f64,
    pub token_al_secondo: Option<f64>,
}

#[derive(Clone, serde::Serialize)]
pub struct Pagina {
    pub id: String,
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
    pub dettaglio: Vec<DettaglioTentativo>,
    pub token_prompt: Option<u64>,
    /// Numero di parole effettivamente corrette e mostrate in questa pagina.
    pub correzioni: usize,
    /// Conteggi separati delle sole correzioni attualmente mostrate.
    pub correzioni_dizionario: usize,
    pub correzioni_contesto: usize,
    #[serde(skip)]
    pub testo: String,
    #[serde(skip)]
    pub testo_originale: String,
    #[serde(skip)]
    pub testo_corretto: String,
    #[serde(skip)]
    pub testo_contestuale: String,
    #[serde(skip)]
    pub testo_completo: String,
    #[serde(skip)]
    pub dettagli_correzione: Vec<correzione::Correzione>,
    #[serde(skip)]
    pub dettagli_contestuali: Vec<correzione::Correzione>,
    #[serde(skip)]
    pub dettagli_completi: Vec<correzione::Correzione>,
}

#[derive(Clone, serde::Serialize)]
pub struct TestoPagina {
    pub testo: String,
    pub originale: String,
    pub correzioni: Vec<correzione::Correzione>,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum FaseMotore {
    Spento,
    Avvio,
    Pronto,
    Errore,
}

#[derive(Clone, serde::Serialize)]
pub struct InfoMotore {
    pub fase: FaseMotore,
    pub backend: Option<String>,
    pub etichetta: String,
    pub dispositivo: String,
    pub messaggio: Option<String>,
    pub thread: usize,
    pub porta: u16,
    pub avvio_secondi: f64,
    pub modello: String,
    pub mmproj: String,
    pub tentativi_falliti: Vec<String>,
}

impl Default for InfoMotore {
    fn default() -> Self {
        InfoMotore {
            fase: FaseMotore::Spento,
            backend: None,
            etichetta: "motore non avviato".into(),
            dispositivo: String::new(),
            messaggio: None,
            thread: 0,
            porta: 0,
            avvio_secondi: 0.0,
            modello: String::new(),
            mmproj: String::new(),
            tentativi_falliti: Vec::new(),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Impostazioni {
    /// `auto`, `cuda`, `vulkan` o `cpu`. La scelta manuale sta nel pannello avanzato.
    #[serde(default = "preferenza_predefinita")]
    pub backend: String,
    /// L'ultimo backend che si e' davvero caricato: si riparte da li'.
    #[serde(default)]
    pub backend_funzionante: Option<String>,
    #[serde(default)]
    pub forza_ocr: bool,
    /// La policy conservativa misurata sul development holdout e' attiva per
    /// impostazione predefinita. Il testo grezzo viene comunque conservato.
    #[serde(default = "vero")]
    pub correzione_automatica: bool,
    #[serde(default)]
    pub correzione_contestuale: bool,
}

fn vero() -> bool {
    true
}

fn preferenza_predefinita() -> String {
    "auto".into()
}

impl Default for Impostazioni {
    fn default() -> Self {
        Impostazioni {
            backend: preferenza_predefinita(),
            backend_funzionante: None,
            forza_ocr: false,
            correzione_automatica: true,
            correzione_contestuale: false,
        }
    }
}

impl Impostazioni {
    fn percorso() -> PathBuf {
        risorse::dir_config().join("impostazioni.json")
    }

    pub fn carica() -> Impostazioni {
        std::fs::read_to_string(Self::percorso())
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn salva(&self) {
        let percorso = Self::percorso();
        if let Ok(testo) = serde_json::to_string_pretty(self) {
            if let Err(e) = file_atomico::scrivi(&percorso, testo.as_bytes()) {
                attenzione!("impostazioni non salvate: {e}");
            }
        }
    }

    /// Il backend imposto dall'utente, oppure `None` in automatico.
    ///
    /// Una scelta esplicita si rispetta e basta: se qualcuno chiede solo CPU
    /// non si va a cercare la GPU, e se chiede CUDA si prova CUDA anche se
    /// l'ultima volta era fallito.
    pub fn scelta_esplicita(&self) -> Option<Backend> {
        match self.backend.as_str() {
            "cuda" => Some(Backend::Cuda),
            "vulkan" => Some(Backend::Vulkan),
            "cpu" => Some(Backend::Cpu),
            _ => None,
        }
    }

    /// Quello che in automatico aveva funzionato l'ultima volta, da provare
    /// per primo per non rifare ogni volta tutta la trafila.
    ///
    /// E' solo un suggerimento sull'ordine, mai una restrizione: se non parte
    /// si scende comunque lungo il resto della catena. Va tenuto distinto
    /// dalla scelta esplicita, perche' confonderli significa che un giro in
    /// CPU imposto a mano si ricorda come "la GPU non funziona".
    pub fn ricordato(&self) -> Option<Backend> {
        match self.backend_funzionante.as_deref() {
            Some("cpu") => Some(Backend::Cpu),
            Some("cuda") => Some(Backend::Cuda),
            Some("vulkan") => Some(Backend::Vulkan),
            _ => None,
        }
    }

    pub fn in_automatico(&self) -> bool {
        self.scelta_esplicita().is_none()
    }
}

pub struct Stato {
    pub documenti: Mutex<Vec<Arc<Documento>>>,
    pub pagine: Mutex<Vec<Pagina>>,
    pub motore: Mutex<Option<Motore>>,
    pub info_motore: Mutex<InfoMotore>,
    pub impostazioni: Mutex<Impostazioni>,
    pub correttore: Mutex<Option<correzione::Correttore>>,
    pub annulla: Arc<AtomicBool>,
    /// Annullamento separato per un modello ancora in caricamento. Usare la
    /// bandiera del lavoro mescolerebbe "ferma pagina" e "cambia backend".
    pub annulla_avvio: Arc<AtomicBool>,
    pub in_corso: AtomicBool,
    /// Voce dell'archivio che questa sessione sta modificando. Finche' e'
    /// valorizzata si riscrive quella, invece di accumulare duplicati a ogni
    /// rielaborazione.
    pub sessione: Mutex<Option<String>>,
    /// Vero quando l'utente ha eliminato dall'archivio proprio il lavoro che
    /// ha ancora in finestra.
    ///
    /// Serve perche' l'archiviazione e' automatica in due punti -- a fine coda
    /// e prima di "Nuovo OCR" -- e senza questo segno il contenuto appena
    /// cancellato tornava da solo nell'elenco, con un identificativo nuovo e
    /// lo stesso nome: da fuori, una cancellazione che non cancella.
    /// Si spegne appena arriva lavoro nuovo, che invece va conservato.
    pub scartata: AtomicBool,
    /// Serializza gli avvii del motore: due thread non devono caricarlo due volte.
    pub avvio: Mutex<()>,
}

impl Stato {
    pub fn nuovo() -> Stato {
        Stato {
            documenti: Mutex::new(Vec::new()),
            pagine: Mutex::new(Vec::new()),
            motore: Mutex::new(None),
            info_motore: Mutex::new(InfoMotore::default()),
            impostazioni: Mutex::new(Impostazioni::carica()),
            correttore: Mutex::new(None),
            annulla: Arc::new(AtomicBool::new(false)),
            annulla_avvio: Arc::new(AtomicBool::new(false)),
            in_corso: AtomicBool::new(false),
            sessione: Mutex::new(None),
            scartata: AtomicBool::new(false),
            avvio: Mutex::new(()),
        }
    }
}

pub fn elenco(stato: &Stato) -> Vec<Pagina> {
    stato.pagine.lock().map(|p| p.clone()).unwrap_or_default()
}

fn pubblica_elenco(app: &AppHandle, stato: &Stato) {
    let _ = app.emit("ocr://elenco", elenco(stato));
}

fn pubblica_pagina(app: &AppHandle, pagina: &Pagina) {
    let _ = app.emit("ocr://pagina", pagina.clone());
}

fn pubblica_motore(app: &AppHandle, info: &InfoMotore) {
    let _ = app.emit("ocr://motore", info.clone());
}

fn aggiorna_info(app: &AppHandle, stato: &Stato, f: impl FnOnce(&mut InfoMotore)) {
    let copia = {
        let Ok(mut info) = stato.info_motore.lock() else {
            return;
        };
        f(&mut info);
        info.clone()
    };
    pubblica_motore(app, &copia);
}

/// Aggiunge documenti alla coda. I file illeggibili non fermano gli altri.
pub fn aggiungi(app: &AppHandle, stato: &Stato, percorsi: Vec<PathBuf>) -> Vec<String> {
    // Arrivano documenti nuovi: quello che c'era e' stato scartato, ma da qui
    // in avanti c'e' di nuovo qualcosa che vale la pena conservare.
    stato.scartata.store(false, Ordering::Relaxed);
    let forza = stato
        .impostazioni
        .lock()
        .map(|i| i.forza_ocr)
        .unwrap_or(false);
    let mut problemi = Vec::new();
    for percorso in percorsi {
        if !documenti::e_supportato(&percorso) {
            problemi.push(format!("{}: formato non supportato", nome_breve(&percorso)));
            continue;
        }
        match documenti::apri(&percorso, forza) {
            Ok(documento) => {
                let indice_documento = {
                    let Ok(mut elenco) = stato.documenti.lock() else {
                        continue;
                    };
                    elenco.push(Arc::new(documento));
                    elenco.len() - 1
                };
                let documento = {
                    let Ok(elenco) = stato.documenti.lock() else {
                        continue;
                    };
                    elenco[indice_documento].clone()
                };
                let totale = documento.pagine.len();
                if let Ok(mut pagine) = stato.pagine.lock() {
                    for (indice, sorgente) in documento.pagine.iter().enumerate() {
                        pagine.push(Pagina {
                            id: format!("d{indice_documento}p{indice}"),
                            documento: indice_documento,
                            nome_documento: documento.nome.clone(),
                            numero: sorgente.numero,
                            pagine_documento: totale,
                            stato: StatoPagina::Attesa,
                            fast_path: sorgente.testo_nativo.is_some(),
                            secondi: 0.0,
                            stadio: None,
                            tentativi: 0,
                            messaggio: None,
                            caratteri: 0,
                            dettaglio: Vec::new(),
                            token_prompt: None,
                            correzioni: 0,
                            correzioni_dizionario: 0,
                            correzioni_contesto: 0,
                            testo: String::new(),
                            testo_originale: String::new(),
                            testo_corretto: String::new(),
                            testo_contestuale: String::new(),
                            testo_completo: String::new(),
                            dettagli_correzione: Vec::new(),
                            dettagli_contestuali: Vec::new(),
                            dettagli_completi: Vec::new(),
                        });
                    }
                }
                info!(
                    "aggiunto {} ({} pagine, {} con text layer)",
                    documento.nome,
                    totale,
                    documento
                        .pagine
                        .iter()
                        .filter(|p| p.testo_nativo.is_some())
                        .count()
                );
            }
            Err(e) => {
                attenzione!("{}: {e}", nome_breve(&percorso));
                problemi.push(format!("{}: {e}", nome_breve(&percorso)));
            }
        }
    }
    pubblica_elenco(app, stato);
    pubblica_progresso(app, stato, None);
    problemi
}

fn nome_breve(percorso: &Path) -> String {
    percorso
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| percorso.display().to_string())
}

pub fn svuota(app: &AppHandle, stato: &Stato) {
    // Prima di buttare via, si mette al sicuro: "Nuovo OCR" non deve mai
    // significare "perdi quello che avevi trascritto". L'unica eccezione e'
    // il lavoro che l'utente ha appena eliminato dall'archivio: li' salvarlo
    // di nuovo lo farebbe ricomparire, e sarebbe il contrario di quanto
    // aveva chiesto.
    if !stato.scartata.load(Ordering::Relaxed) {
        crate::archivio::salva_in_silenzio(stato);
    }
    stato.scartata.store(false, Ordering::Relaxed);
    if let Ok(mut s) = stato.sessione.lock() {
        *s = None;
    }
    if let Ok(mut d) = stato.documenti.lock() {
        d.clear();
    }
    if let Ok(mut p) = stato.pagine.lock() {
        p.clear();
    }
    pubblica_elenco(app, stato);
    pubblica_progresso(app, stato, None);
    let _ = app.emit("ocr://archivio", crate::archivio::elenco());
}

/// Avvia il motore se non gira gia', rispettando scelta e catena di fallback.
pub fn assicura_motore(app: &AppHandle, stato: &Arc<Stato>) -> Result<(u16, String), String> {
    let _guardia = stato.avvio.lock().map_err(|_| "stato inconsistente")?;
    if let Ok(motore) = stato.motore.lock() {
        if let Some(m) = motore.as_ref() {
            return Ok((m.porta, m.prompt.clone()));
        }
    }
    // Un eventuale avvio precedente e' ormai uscito e ha rilasciato la
    // guardia. Questa nuova generazione ha una bandiera pulita.
    stato.annulla_avvio.store(false, Ordering::SeqCst);
    aggiorna_info(app, stato, |info| {
        info.fase = FaseMotore::Avvio;
        info.etichetta = "avvio del motore…".into();
        info.messaggio = None;
        info.tentativi_falliti.clear();
    });

    let (esplicito, ricordato, automatico) = stato
        .impostazioni
        .lock()
        .ok()
        .map(|i| (i.scelta_esplicita(), i.ricordato(), i.in_automatico()))
        .unwrap_or((None, None, true));
    let annulla = stato.annulla_avvio.clone();
    match Motore::avvia_con_fallback(esplicito, ricordato, &annulla) {
        Ok((motore, tentativi_falliti)) => {
            let (porta, prompt) = (motore.porta, motore.prompt.clone());
            let backend = motore.backend;
            let descrizione = InfoMotore {
                fase: FaseMotore::Pronto,
                backend: Some(backend.chiave().to_string()),
                etichetta: backend.etichetta().to_string(),
                dispositivo: motore.dispositivo.clone(),
                messaggio: None,
                thread: motore.thread,
                porta: motore.porta,
                avvio_secondi: motore.avvio_secondi,
                modello: motore.modello.display().to_string(),
                mmproj: motore.mmproj.display().to_string(),
                tentativi_falliti,
            };
            if let Ok(mut slot) = stato.motore.lock() {
                *slot = Some(motore);
            }
            // Si ricorda solo cio' che la catena automatica ha scoperto da
            // sola. Un avvio andato a buon fine perche' l'utente aveva
            // imposto un backend non dice niente su cosa converrebbe provare
            // per primo la prossima volta in automatico.
            if automatico {
                if let Ok(mut impostazioni) = stato.impostazioni.lock() {
                    impostazioni.backend_funzionante = Some(backend.chiave().to_string());
                    impostazioni.salva();
                }
            }
            aggiorna_info(app, stato, |info| *info = descrizione);
            Ok((porta, prompt))
        }
        Err(e) => {
            errore!("motore non avviabile: {e}");
            let messaggio = e.clone();
            aggiorna_info(app, stato, |info| {
                info.fase = FaseMotore::Errore;
                info.etichetta = "motore non disponibile".into();
                info.messaggio = Some(messaggio);
            });
            Err(e)
        }
    }
}

pub fn ferma_motore(app: &AppHandle, stato: &Arc<Stato>) {
    // Funziona anche quando il motore non e' ancora entrato nello slot: il
    // ciclo di health interrompe il figlio e libera la guardia d'avvio.
    stato.annulla_avvio.store(true, Ordering::SeqCst);
    if let Ok(mut slot) = stato.motore.lock() {
        if let Some(mut motore) = slot.take() {
            motore.ferma();
        }
    }
    aggiorna_info(app, stato, |info| {
        *info = InfoMotore::default();
    });
}

fn indice_per_id(pagine: &[Pagina], id: &str) -> Option<usize> {
    pagine.iter().position(|p| p.id == id)
}

fn modifica_pagina(
    app: &AppHandle,
    stato: &Stato,
    id: &str,
    f: impl FnOnce(&mut Pagina),
) -> Option<Pagina> {
    let copia = {
        let mut pagine = stato.pagine.lock().ok()?;
        let indice = indice_per_id(&pagine, id)?;
        f(&mut pagine[indice]);
        pagine[indice].caratteri = pagine[indice].testo.chars().count();
        pagine[indice].clone()
    };
    pubblica_pagina(app, &copia);
    Some(copia)
}

fn preferenze_correzione(stato: &Stato) -> (bool, bool) {
    stato
        .impostazioni
        .lock()
        .map(|i| (i.correzione_automatica, i.correzione_contestuale))
        .unwrap_or((true, false))
}

fn applica_preferenze_correzione(pagina: &mut Pagina, dizionario: bool, contesto: bool) {
    let (testo, dettagli) = if pagina.fast_path || (!dizionario && !contesto) {
        (&pagina.testo_originale, None)
    } else if dizionario && contesto {
        (&pagina.testo_completo, Some(&pagina.dettagli_completi))
    } else if dizionario {
        (&pagina.testo_corretto, Some(&pagina.dettagli_correzione))
    } else {
        (
            &pagina.testo_contestuale,
            Some(&pagina.dettagli_contestuali),
        )
    };
    pagina.testo = testo.clone();
    pagina.correzioni = dettagli.map_or(0, |d| d.len());
    pagina.correzioni_dizionario =
        dettagli.map_or(0, |d| d.iter().filter(|c| c.metodo == "dizionario").count());
    pagina.correzioni_contesto =
        dettagli.map_or(0, |d| d.iter().filter(|c| c.metodo == "contesto").count());
    pagina.caratteri = pagina.testo.chars().count();
}

fn postcorreggi(
    stato: &Stato,
    testo: &str,
    continua_dalla_precedente: bool,
) -> Result<correzione::Varianti, String> {
    let mut slot = stato
        .correttore
        .lock()
        .map_err(|_| "stato del correttore inconsistente".to_string())?;
    if slot.is_none() {
        let dir = risorse::dir_dizionario()?;
        info!("caricamento del dizionario italiano da {}", dir.display());
        let dir_inglese = risorse::dir_dizionario_inglese();
        match &dir_inglese {
            Some(d) => info!(
                "dizionario inglese da {} (solo riconoscimento)",
                d.display()
            ),
            None => {
                attenzione!("dizionario inglese assente: le parole inglesi non saranno protette")
            }
        }
        *slot = Some(correzione::Correttore::carica(
            &dir,
            dir_inglese.as_deref(),
        )?);
    }
    Ok(slot
        .as_ref()
        .expect("correttore inizializzato")
        .correggi_varianti(testo, continua_dalla_precedente))
}

/// Cambia la vista e l'esportazione anche per le pagine gia' elaborate: non
/// serve rilanciare il modello e il testo OCR originale non viene mai perso.
pub fn imposta_correzione_automatica(
    app: &AppHandle,
    stato: &Stato,
    valore: bool,
) -> Result<(), String> {
    {
        let mut impostazioni = stato
            .impostazioni
            .lock()
            .map_err(|_| "stato inconsistente")?;
        impostazioni.correzione_automatica = valore;
        impostazioni.salva();
    }
    if let Ok(mut pagine) = stato.pagine.lock() {
        for pagina in pagine.iter_mut() {
            if !pagina.testo_originale.is_empty() {
                applica_preferenze_correzione(
                    pagina,
                    valore,
                    stato
                        .impostazioni
                        .lock()
                        .map(|i| i.correzione_contestuale)
                        .unwrap_or(false),
                );
            }
        }
    }
    pubblica_elenco(app, stato);
    Ok(())
}

pub fn imposta_correzione_contestuale(
    app: &AppHandle,
    stato: &Stato,
    valore: bool,
) -> Result<usize, String> {
    if valore && !risorse::dir_dizionario()?.join("contesto.txt").is_file() {
        return Err("La correzione contestuale richiede un lessico locale aggiuntivo, non incluso nel pacchetto.".into());
    }
    let dizionario = {
        let mut impostazioni = stato
            .impostazioni
            .lock()
            .map_err(|_| "stato inconsistente")?;
        impostazioni.correzione_contestuale = valore;
        let dizionario = impostazioni.correzione_automatica;
        impostazioni.salva();
        dizionario
    };
    if let Ok(mut pagine) = stato.pagine.lock() {
        for pagina in pagine.iter_mut() {
            if !pagina.testo_originale.is_empty() {
                applica_preferenze_correzione(pagina, dizionario, valore);
            }
        }
    }
    let trovate = stato
        .pagine
        .lock()
        .map(|pagine| pagine.iter().map(|p| p.correzioni_contesto).sum())
        .unwrap_or(0);
    pubblica_elenco(app, stato);
    Ok(trovate)
}

#[derive(Clone, serde::Serialize)]
struct Progresso {
    fatte: usize,
    totali: usize,
    in_corso: bool,
    id_corrente: Option<String>,
}

fn pubblica_progresso(app: &AppHandle, stato: &Stato, corrente: Option<&str>) {
    let (fatte, totali) = {
        let Ok(pagine) = stato.pagine.lock() else {
            return;
        };
        (
            pagine
                .iter()
                .filter(|p| {
                    matches!(
                        p.stato,
                        StatoPagina::Completata | StatoPagina::Incompleta | StatoPagina::Errore
                    )
                })
                .count(),
            pagine.len(),
        )
    };
    let _ = app.emit(
        "ocr://progresso",
        Progresso {
            fatte,
            totali,
            in_corso: stato.in_corso.load(Ordering::Relaxed),
            id_corrente: corrente.map(str::to_string),
        },
    );
}

#[derive(Clone, serde::Serialize)]
struct Delta {
    id: String,
    testo: String,
    sostituisci: bool,
    stadio: Option<String>,
}

/// Il ciclo di elaborazione, su un thread suo.
pub fn elabora(app: AppHandle, stato: Arc<Stato>) {
    if stato
        .in_corso
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    stato.annulla.store(false, Ordering::SeqCst);
    // Rilanciare l'OCR e' lavoro nuovo: torna a meritare l'archiviazione
    // automatica, anche se il giro precedente era stato buttato.
    stato.scartata.store(false, Ordering::Relaxed);
    pubblica_progresso(&app, &stato, None);

    let da_fare: Vec<(String, usize, usize)> = {
        let Ok(pagine) = stato.pagine.lock() else {
            stato.in_corso.store(false, Ordering::SeqCst);
            return;
        };
        pagine
            .iter()
            .enumerate()
            .filter(|(_, p)| matches!(p.stato, StatoPagina::Attesa | StatoPagina::Annullata))
            .map(|(i, p)| (p.id.clone(), p.documento, i))
            .collect()
    };

    let mut motore_pronto: Option<(u16, String)> = None;

    for (id, indice_documento, indice_elenco) in da_fare {
        if stato.annulla.load(Ordering::Relaxed) {
            break;
        }
        let Some(documento) = stato
            .documenti
            .lock()
            .ok()
            .and_then(|d| d.get(indice_documento).cloned())
        else {
            continue;
        };
        let indice_pagina = {
            let Ok(pagine) = stato.pagine.lock() else {
                break;
            };
            let Some(i) = indice_per_id(&pagine, &id) else {
                continue;
            };
            pagine[i].numero - 1
        };

        modifica_pagina(&app, &stato, &id, |p| {
            p.stato = StatoPagina::Corso;
            p.messaggio = None;
            p.testo.clear();
            p.testo_originale.clear();
            p.testo_corretto.clear();
            p.testo_contestuale.clear();
            p.testo_completo.clear();
            p.dettagli_correzione.clear();
            p.dettagli_contestuali.clear();
            p.dettagli_completi.clear();
            p.correzioni = 0;
            p.correzioni_dizionario = 0;
            p.correzioni_contesto = 0;
        });
        pubblica_progresso(&app, &stato, Some(&id));

        // Fast path: il text layer nativo non ha bisogno del VLM.
        if let Some(testo) = documento
            .pagine
            .get(indice_pagina)
            .and_then(|p| p.testo_nativo.clone())
        {
            let inizio = std::time::Instant::now();
            let _ = app.emit(
                "ocr://delta",
                Delta {
                    id: id.clone(),
                    testo: testo.clone(),
                    sostituisci: true,
                    stadio: Some("text layer".into()),
                },
            );
            modifica_pagina(&app, &stato, &id, |p| {
                p.testo = testo.clone();
                p.testo_originale = testo.clone();
                p.testo_corretto = testo.clone();
                p.testo_contestuale = testo.clone();
                p.testo_completo = testo;
                p.dettagli_correzione.clear();
                p.dettagli_contestuali.clear();
                p.dettagli_completi.clear();
                p.correzioni = 0;
                p.correzioni_dizionario = 0;
                p.correzioni_contesto = 0;
                p.stato = StatoPagina::Completata;
                p.fast_path = true;
                p.stadio = Some("text layer".into());
                p.dettaglio = Vec::new();
                p.secondi = inizio.elapsed().as_secs_f64();
            });
            pubblica_progresso(&app, &stato, Some(&id));
            continue;
        }

        if motore_pronto.is_none() {
            match assicura_motore(&app, &stato) {
                Ok(coordinate) => motore_pronto = Some(coordinate),
                Err(e) => {
                    modifica_pagina(&app, &stato, &id, |p| {
                        p.stato = StatoPagina::Errore;
                        p.messaggio = Some(e.clone());
                    });
                    pubblica_progresso(&app, &stato, None);
                    break;
                }
            }
        }
        let Some((porta, prompt)) = motore_pronto.clone() else {
            break;
        };

        let immagine = match documenti::immagine_per_ocr(&documento, indice_pagina) {
            Ok(b64) => b64,
            Err(e) => {
                attenzione!("pagina {id}: {e}");
                modifica_pagina(&app, &stato, &id, |p| {
                    p.stato = StatoPagina::Errore;
                    p.messaggio = Some(e);
                });
                pubblica_progresso(&app, &stato, Some(&id));
                continue;
            }
        };

        let contesto = cascata::Contesto {
            porta,
            prompt: &prompt,
            immagine_base64: &immagine,
            annulla: &stato.annulla,
        };
        let app_evento = app.clone();
        let id_evento = id.clone();
        let mut ultimo_invio = std::time::Instant::now();
        let mut accumulato = String::new();
        let mut stadio_corrente = String::from("greedy");
        let mut su_evento = |evento: cascata::Evento| match evento {
            cascata::Evento::Stadio { indice, nome } => {
                stadio_corrente = nome.to_string();
                accumulato.clear();
                let _ = app_evento.emit(
                    "ocr://delta",
                    Delta {
                        id: id_evento.clone(),
                        testo: String::new(),
                        sostituisci: true,
                        stadio: Some(if indice == 0 {
                            nome.to_string()
                        } else {
                            format!("{nome} (tentativo {})", indice + 1)
                        }),
                    },
                );
            }
            cascata::Evento::Testo(testo) => {
                accumulato.push_str(testo);
                // Un evento per token intasa la webview: si raggruppa a 50 ms.
                if ultimo_invio.elapsed() > std::time::Duration::from_millis(50) {
                    let _ = app_evento.emit(
                        "ocr://delta",
                        Delta {
                            id: id_evento.clone(),
                            testo: std::mem::take(&mut accumulato),
                            sostituisci: false,
                            stadio: None,
                        },
                    );
                    ultimo_invio = std::time::Instant::now();
                }
            }
        };

        let esito = cascata::esegui(&contesto, &mut su_evento);
        if !accumulato.is_empty() {
            let _ = app.emit(
                "ocr://delta",
                Delta {
                    id: id.clone(),
                    testo: accumulato.clone(),
                    sostituisci: false,
                    stadio: None,
                },
            );
        }

        match esito {
            Ok(esito) => {
                let annullata = esito.annullata;
                let riepilogo = format!(
                    "pagina {id}: {} in {:.1} s, {} token, {} tentativi",
                    if esito.completata {
                        "completata"
                    } else if annullata {
                        "annullata"
                    } else {
                        "incompleta"
                    },
                    esito.secondi,
                    esito.token,
                    esito.tentativi.len()
                );
                info!("{riepilogo}");
                let ciclo = esito
                    .tentativi
                    .iter()
                    .filter_map(|t| t.descrizione_ciclo.clone())
                    .next_back();
                let dettaglio: Vec<DettaglioTentativo> = esito
                    .tentativi
                    .iter()
                    .map(|t| DettaglioTentativo {
                        stadio: t.stadio.clone(),
                        esito: if t.eos && !t.interrotto_per_ciclo {
                            "chiusa con stop".into()
                        } else if t.interrotto_per_ciclo {
                            format!(
                                "ciclo ({})",
                                t.descrizione_ciclo.clone().unwrap_or_default()
                            )
                        } else if t.limite_raggiunto {
                            "tetto di 4096 token".into()
                        } else {
                            "interrotta".into()
                        },
                        token: t.token,
                        secondi: t.secondi,
                        token_al_secondo: t.token_al_secondo,
                    })
                    .collect();
                let token_prompt = esito.token_prompt;
                let originale = esito.testo.clone();
                // Se la pagina prima -- dello stesso documento -- si chiudeva
                // con un trattino, la prima parola di questa e' la seconda
                // meta' di una parola spezzata e non va corretta: e' cosi' che
                // `itories`, seguito di `repos-`, diventava `stories`.
                let continua_dalla_precedente = indice_elenco
                    .checked_sub(1)
                    .and_then(|i| {
                        let pagine = stato.pagine.lock().ok()?;
                        let prima = pagine.get(i)?;
                        (prima.documento == indice_documento)
                            .then(|| correzione::resta_spezzata(&prima.testo_originale))
                    })
                    .unwrap_or(false);
                let varianti = match postcorreggi(&stato, &originale, continua_dalla_precedente) {
                    Ok(c) => c,
                    Err(e) => {
                        attenzione!("pagina {id}: post-correzione non disponibile: {e}");
                        correzione::Varianti {
                            dizionario: correzione::Esito {
                                testo: originale.clone(),
                                correzioni: Vec::new(),
                            },
                            contesto: correzione::Esito {
                                testo: originale.clone(),
                                correzioni: Vec::new(),
                            },
                            entrambe: correzione::Esito {
                                testo: originale.clone(),
                                correzioni: Vec::new(),
                            },
                        }
                    }
                };
                let preferenze = preferenze_correzione(&stato);
                if !varianti.entrambe.correzioni.is_empty() {
                    info!(
                        "pagina {id}: {} correzioni da dizionario, {} contestuali",
                        varianti.dizionario.correzioni.len(),
                        varianti
                            .entrambe
                            .correzioni
                            .len()
                            .saturating_sub(varianti.dizionario.correzioni.len())
                    );
                }
                modifica_pagina(&app, &stato, &id, |p| {
                    p.testo_originale = originale.clone();
                    p.testo_corretto = varianti.dizionario.testo.clone();
                    p.testo_contestuale = varianti.contesto.testo.clone();
                    p.testo_completo = varianti.entrambe.testo.clone();
                    p.dettagli_correzione = varianti.dizionario.correzioni.clone();
                    p.dettagli_contestuali = varianti.contesto.correzioni.clone();
                    p.dettagli_completi = varianti.entrambe.correzioni.clone();
                    applica_preferenze_correzione(p, preferenze.0, preferenze.1);
                    p.secondi = esito.secondi;
                    p.tentativi = esito.tentativi.len();
                    p.dettaglio = dettaglio.clone();
                    p.token_prompt = token_prompt;
                    p.stadio = esito
                        .stadio_accettato
                        .clone()
                        .or_else(|| esito.tentativi.last().map(|t| t.stadio.clone()));
                    p.stato = if annullata {
                        StatoPagina::Annullata
                    } else if esito.completata {
                        StatoPagina::Completata
                    } else {
                        StatoPagina::Incompleta
                    };
                    p.messaggio = if annullata {
                        Some("interrotta dall'utente".into())
                    } else if !esito.completata {
                        Some(match (esito.troncata_al_ciclo, ciclo) {
                            (true, Some(d)) => format!(
                                "il modello e' entrato in ripetizione ({d}): \
                                 restituito il testo utile prima del ciclo"
                            ),
                            (true, None) => "il modello e' entrato in ripetizione: \
                                 restituito il testo utile prima del ciclo"
                                .into(),
                            _ => "il modello non ha chiuso la pagina entro 4096 token".into(),
                        })
                    } else {
                        None
                    };
                });
                if annullata {
                    pubblica_progresso(&app, &stato, None);
                    break;
                }
            }
            Err(e) => {
                errore!("pagina {id}: {e}");
                modifica_pagina(&app, &stato, &id, |p| {
                    p.stato = StatoPagina::Errore;
                    p.messaggio = Some(e);
                });
            }
        }
        pubblica_progresso(&app, &stato, Some(&id));
    }

    stato.in_corso.store(false, Ordering::SeqCst);
    stato.annulla.store(false, Ordering::SeqCst);
    pubblica_progresso(&app, &stato, None);
    pubblica_elenco(&app, &stato);
    // Il lavoro fatto non deve dipendere dal fatto che l'utente si ricordi di
    // salvare: a fine coda la sessione finisce da sola nell'archivio.
    crate::archivio::salva_in_silenzio(&stato);
    let _ = app.emit("ocr://archivio", crate::archivio::elenco());
}

pub fn testo_pagina(stato: &Stato, id: &str) -> TestoPagina {
    stato
        .pagine
        .lock()
        .ok()
        .and_then(|pagine| {
            pagine.iter().find(|p| p.id == id).map(|p| TestoPagina {
                testo: p.testo.clone(),
                originale: p.testo_originale.clone(),
                correzioni: if p.correzioni == 0 {
                    Vec::new()
                } else {
                    let (dizionario, contesto) = preferenze_correzione(stato);
                    if dizionario && contesto {
                        p.dettagli_completi.clone()
                    } else if dizionario {
                        p.dettagli_correzione.clone()
                    } else if contesto {
                        p.dettagli_contestuali.clone()
                    } else {
                        Vec::new()
                    }
                },
            })
        })
        .unwrap_or(TestoPagina {
            testo: String::new(),
            originale: String::new(),
            correzioni: Vec::new(),
        })
}

/// Esporta tutto il lavoro fatto. Le pagine incomplete restano, marcate.
/// Il testo trascritto e basta: niente intestazioni di pagina, niente nome del
/// documento, niente marcatori di stato. La stessa stringa serve agli appunti,
/// ai file .txt e .md e al documento Word, che la rilegge per ricostruire le
/// tabelle. Lo stato delle pagine si legge nella finestra, non nell'esportato.
pub fn esporta(stato: &Stato) -> String {
    let Ok(pagine) = stato.pagine.lock() else {
        return String::new();
    };
    let mut fuori = String::new();
    for pagina in pagine.iter() {
        let testo = pagina.testo.trim();
        if testo.is_empty() {
            continue;
        }
        fuori.push_str(testo);
        fuori.push_str("\n\n");
    }
    fuori
}

/// Anteprima della pagina, calcolata su richiesta: non partecipa all'OCR.
pub fn anteprima(stato: &Stato, id: &str) -> Result<String, String> {
    let (indice_documento, numero) = {
        let pagine = stato.pagine.lock().map_err(|_| "stato inconsistente")?;
        let pagina = pagine
            .iter()
            .find(|p| p.id == id)
            .ok_or("pagina non trovata")?;
        (pagina.documento, pagina.numero)
    };
    let documento = stato
        .documenti
        .lock()
        .map_err(|_| "stato inconsistente")?
        .get(indice_documento)
        .cloned()
        .ok_or("documento non trovato")?;
    documenti::anteprima(&documento, numero - 1)
}

pub fn diagnostica(stato: &Stato) -> serde_json::Value {
    let info = stato
        .info_motore
        .lock()
        .map(|i| i.clone())
        .unwrap_or_default();
    let impostazioni = stato
        .impostazioni
        .lock()
        .map(|i| i.clone())
        .unwrap_or_default();
    let log_motore = stato
        .motore
        .lock()
        .ok()
        .and_then(|m| m.as_ref().map(|m| m.log_server.clone()));
    serde_json::json!({
        "motore": info,
        "impostazioni": impostazioni,
        "canvas": [imaging::CANVAS.0, imaging::CANVAS.1],
        "dpi": imaging::DPI_RENDER,
        "n_predict": cascata::N_PREDICT,
        "cascata": cascata::stadi()
            .iter()
            .map(|(nome, parametri)| serde_json::json!({"stadio": nome, "parametri": parametri}))
            .collect::<Vec<_>>(),
        "rilevatore": {
            "min_ripetizioni": 8,
            "periodo_massimo": 256,
            "span_minimo": 16,
            "interrompe_ultimo_stadio": false,
        },
        "log_applicazione": crate::log::file_corrente()
            .map(|p| p.display().to_string()),
        "log_motore": log_motore.map(|p| p.display().to_string()),
        "coda_log_motore": stato
            .motore
            .lock()
            .ok()
            .and_then(|m| m.as_ref().map(|m| crate::motore::coda_log(&m.log_server, 60)))
            .unwrap_or_default(),
        "righe_log": crate::log::recenti(),
    })
}

/// Riepilogo per la barra di stato: quante pagine e in che condizione.
pub fn riepilogo(stato: &Stato) -> HashMap<String, usize> {
    let mut fuori = HashMap::new();
    if let Ok(pagine) = stato.pagine.lock() {
        for pagina in pagine.iter() {
            let chiave = match pagina.stato {
                StatoPagina::Attesa => "attesa",
                StatoPagina::Corso => "corso",
                StatoPagina::Completata => "completata",
                StatoPagina::Incompleta => "incompleta",
                StatoPagina::Errore => "errore",
                StatoPagina::Annullata => "annullata",
            };
            *fuori.entry(chiave.to_string()).or_insert(0) += 1;
            if pagina.fast_path && pagina.stato == StatoPagina::Completata {
                *fuori.entry("fast_path".to_string()).or_insert(0) += 1;
            }
        }
    }
    fuori
}

#[cfg(test)]
mod test {
    use super::*;

    fn pagina_finta(id: &str, numero: usize, stato: StatoPagina, testo: &str) -> Pagina {
        Pagina {
            id: id.into(),
            documento: 0,
            nome_documento: "appunti.pdf".into(),
            numero,
            pagine_documento: 3,
            stato,
            fast_path: false,
            secondi: 1.0,
            stadio: Some("greedy".into()),
            tentativi: 1,
            messaggio: None,
            caratteri: testo.chars().count(),
            dettaglio: Vec::new(),
            token_prompt: None,
            correzioni: 0,
            correzioni_dizionario: 0,
            correzioni_contesto: 0,
            testo: testo.into(),
            testo_originale: testo.into(),
            testo_corretto: testo.into(),
            testo_contestuale: testo.into(),
            testo_completo: testo.into(),
            dettagli_correzione: Vec::new(),
            dettagli_contestuali: Vec::new(),
            dettagli_completi: Vec::new(),
        }
    }

    fn stato_con(pagine: Vec<Pagina>) -> Stato {
        let stato = Stato::nuovo();
        *stato.pagine.lock().unwrap() = pagine;
        stato
    }

    #[test]
    fn lesportazione_e_solo_il_testo_trascritto() {
        let stato = stato_con(vec![
            pagina_finta("a", 1, StatoPagina::Completata, "prima pagina"),
            pagina_finta("b", 2, StatoPagina::Completata, "seconda pagina"),
        ]);
        assert_eq!(esporta(&stato), "prima pagina\n\nseconda pagina\n\n");
    }

    /// Nell'esportato non deve comparire niente che non venga dalle pagine:
    /// ne' il numero di pagina, ne' il nome del file, ne' lo stato.
    #[test]
    fn nessuna_intestazione_di_pagina_nellesportato() {
        let mut incompleta = pagina_finta("b", 2, StatoPagina::Incompleta, "meta' pagina");
        incompleta.messaggio = Some("il modello e' entrato in ripetizione".into());
        let mut veloce = pagina_finta("a", 1, StatoPagina::Completata, "prima pagina");
        veloce.fast_path = true;
        let stato = stato_con(vec![veloce, incompleta]);
        let fuori = esporta(&stato);
        assert_eq!(fuori, "prima pagina\n\nmeta' pagina\n\n");
        for intruso in [
            "Pagina",
            "PAGINA",
            "appunti.pdf",
            "#",
            "=====",
            "ripetizione",
        ] {
            assert!(!fuori.contains(intruso), "trovato {intruso} in {fuori:?}");
        }
    }

    #[test]
    fn le_pagine_non_ancora_lette_non_finiscono_nellesportazione() {
        let stato = stato_con(vec![
            pagina_finta("a", 1, StatoPagina::Completata, "letta"),
            pagina_finta("b", 2, StatoPagina::Attesa, ""),
        ]);
        assert_eq!(esporta(&stato), "letta\n\n");
    }
}
