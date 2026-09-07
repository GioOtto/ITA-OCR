// L'app e' una GUI: niente console su nessuna piattaforma.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archivio;
mod cascata;
mod correzione;
mod documenti;
mod file_atomico;
mod gguf;
mod http;
mod imaging;
mod lavoro;
mod log;
mod motore;
mod pdfium;
mod piattaforma;
mod prova;
mod risorse;
mod uscita_docx;

use lavoro::Stato;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, Manager, State};

type Esito<T> = Result<T, String>;

#[tauri::command]
fn apri_file(app: AppHandle, stato: State<'_, Arc<Stato>>, percorsi: Vec<String>) -> Vec<String> {
    let percorsi: Vec<PathBuf> = percorsi.into_iter().map(PathBuf::from).collect();
    lavoro::aggiungi(&app, &stato, percorsi)
}

#[tauri::command]
fn svuota(app: AppHandle, stato: State<'_, Arc<Stato>>) {
    if stato.in_corso.load(Ordering::Relaxed) {
        stato.annulla.store(true, Ordering::SeqCst);
    }
    lavoro::svuota(&app, &stato);
}

#[tauri::command]
fn avvia(app: AppHandle, stato: State<'_, Arc<Stato>>) {
    let stato = stato.inner().clone();
    if stato.in_corso.load(Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(move || lavoro::elabora(app, stato));
}

#[tauri::command]
fn annulla(stato: State<'_, Arc<Stato>>) {
    stato.annulla.store(true, Ordering::SeqCst);
    info!("elaborazione interrotta dall'utente");
}

#[tauri::command]
fn elenco(stato: State<'_, Arc<Stato>>) -> Vec<lavoro::Pagina> {
    lavoro::elenco(&stato)
}

#[tauri::command]
fn testo_pagina(stato: State<'_, Arc<Stato>>, id: String) -> lavoro::TestoPagina {
    lavoro::testo_pagina(&stato, &id)
}

#[tauri::command]
fn anteprima(stato: State<'_, Arc<Stato>>, id: String) -> Esito<String> {
    lavoro::anteprima(&stato, &id)
}

#[tauri::command]
fn esporta(stato: State<'_, Arc<Stato>>) -> String {
    lavoro::esporta(&stato)
}

#[tauri::command]
fn salva_file(percorso: String, contenuto: String) -> Esito<String> {
    let percorso = PathBuf::from(percorso);
    if let Some(dir) = percorso.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cartella non creabile: {e}"))?;
    }
    std::fs::write(&percorso, contenuto).map_err(|e| format!("salvataggio fallito: {e}"))?;
    info!("esportato in {}", percorso.display());
    Ok(percorso.display().to_string())
}

/// Word ha bisogno di un file binario: lo scrive Rust, senza passare dal
/// ponte, cosi' le tabelle restano tabelle e non righe di barre verticali.
#[tauri::command]
fn esporta_docx(stato: State<'_, Arc<Stato>>, percorso: String) -> Esito<String> {
    let testo = lavoro::esporta(&stato);
    if testo.trim().is_empty() {
        return Err("non c'è ancora niente da esportare".into());
    }
    let percorso = PathBuf::from(percorso);
    if let Some(dir) = percorso.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cartella non creabile: {e}"))?;
    }
    uscita_docx::scrivi(&testo, &percorso)?;
    info!("esportato in {}", percorso.display());
    Ok(percorso.display().to_string())
}

#[tauri::command]
fn diagnostica(stato: State<'_, Arc<Stato>>) -> serde_json::Value {
    lavoro::diagnostica(&stato)
}

#[tauri::command]
fn riepilogo(stato: State<'_, Arc<Stato>>) -> std::collections::HashMap<String, usize> {
    lavoro::riepilogo(&stato)
}

#[tauri::command]
fn info_motore(stato: State<'_, Arc<Stato>>) -> lavoro::InfoMotore {
    stato
        .info_motore
        .lock()
        .map(|i| i.clone())
        .unwrap_or_default()
}

#[tauri::command]
fn imposta_backend(app: AppHandle, stato: State<'_, Arc<Stato>>, scelta: String) -> Esito<()> {
    if !["auto", "cuda", "vulkan", "cpu"].contains(&scelta.as_str()) {
        return Err(format!("backend sconosciuto: {scelta}"));
    }
    if stato.in_corso.load(Ordering::Relaxed) {
        return Err("elaborazione in corso: fermarla prima di cambiare backend".into());
    }
    {
        let mut impostazioni = stato
            .impostazioni
            .lock()
            .map_err(|_| "stato inconsistente")?;
        impostazioni.backend = scelta.clone();
        // `backend_funzionante` non si tocca qui: e' la memoria di cosa ha
        // funzionato da solo in automatico, e una scelta manuale non e' una
        // scoperta. Scriverla qui rendeva definitivo un giro in CPU imposto a
        // mano: tornati su "automatico" la catena partiva da CPU, CPU parte
        // sempre, e la GPU non veniva piu' provata.
        impostazioni.salva();
    }
    info!("backend richiesto: {scelta}");
    let stato = stato.inner().clone();
    lavoro::ferma_motore(&app, &stato);
    std::thread::spawn(move || {
        let _ = lavoro::assicura_motore(&app, &stato);
    });
    Ok(())
}

#[tauri::command]
fn imposta_forza_ocr(stato: State<'_, Arc<Stato>>, valore: bool) -> Esito<()> {
    let mut impostazioni = stato
        .impostazioni
        .lock()
        .map_err(|_| "stato inconsistente")?;
    impostazioni.forza_ocr = valore;
    impostazioni.salva();
    Ok(())
}

#[tauri::command]
fn imposta_correzione_automatica(
    app: AppHandle,
    stato: State<'_, Arc<Stato>>,
    valore: bool,
) -> Esito<()> {
    lavoro::imposta_correzione_automatica(&app, &stato, valore)
}

#[tauri::command]
fn imposta_correzione_contestuale(
    app: AppHandle,
    stato: State<'_, Arc<Stato>>,
    valore: bool,
) -> Esito<usize> {
    lavoro::imposta_correzione_contestuale(&app, &stato, valore)
}

#[tauri::command]
fn riavvia_motore(app: AppHandle, stato: State<'_, Arc<Stato>>) -> Esito<()> {
    if stato.in_corso.load(Ordering::Relaxed) {
        return Err("elaborazione in corso: fermarla prima di riavviare il motore".into());
    }
    let stato = stato.inner().clone();
    lavoro::ferma_motore(&app, &stato);
    std::thread::spawn(move || {
        let _ = lavoro::assicura_motore(&app, &stato);
    });
    Ok(())
}

// ---------------------------------------------------------------- archivio

#[tauri::command]
fn archivio_elenco() -> Vec<archivio::Voce> {
    archivio::elenco()
}

/// Salva a richiesta la sessione corrente. Restituisce l'id, oppure None se
/// non c'era ancora niente di trascritto da archiviare.
#[tauri::command]
fn archivio_salva(
    app: AppHandle,
    stato: State<'_, Arc<Stato>>,
    nome: Option<String>,
) -> Esito<Option<String>> {
    let id = archivio::salva(&stato, nome)?;
    let _ = app.emit("ocr://archivio", archivio::elenco());
    Ok(id)
}

/// Rimette in finestra una sessione archiviata. Il numero restituito dice
/// quanti sorgenti non si sono potuti riaprire: con zero e' rielaborabile.
#[tauri::command]
fn archivio_carica(app: AppHandle, stato: State<'_, Arc<Stato>>, id: String) -> Esito<usize> {
    if stato.in_corso.load(Ordering::SeqCst) {
        return Err(
            "c'è un'elaborazione in corso: interrompila prima di aprire un'altra sessione".into(),
        );
    }
    let mancanti = archivio::carica(&stato, &id)?;
    // Si sta lavorando su una voce che nell'archivio c'e': non ha piu' senso
    // trattenersi dal salvarla.
    stato.scartata.store(false, Ordering::Relaxed);
    let _ = app.emit("ocr://elenco", lavoro::elenco(&stato));
    let _ = app.emit("ocr://archivio", archivio::elenco());
    Ok(mancanti)
}

#[tauri::command]
fn archivio_elimina(app: AppHandle, stato: State<'_, Arc<Stato>>, id: String) -> Esito<()> {
    archivio::elimina(&id)?;
    // Se si e' appena eliminata la sessione aperta, va dimenticato anche il
    // suo identificativo: `salva` riusa quello memorizzato, quindi il primo
    // salvataggio automatico successivo -- a fine elaborazione, senza che
    // nessuno lo chieda -- riscriveva il file appena cancellato con lo stesso
    // nome. Dall'esterno sembrava che l'eliminazione non funzionasse.
    let era_la_corrente = stato
        .sessione
        .lock()
        .map(|mut corrente| {
            let combacia = corrente.as_deref() == Some(id.as_str());
            if combacia {
                *corrente = None;
            }
            combacia
        })
        .unwrap_or(false);
    if era_la_corrente {
        // Il contenuto resta in finestra, ma non deve piu' rientrare
        // nell'archivio da solo: e' quello che l'utente ha appena buttato.
        stato.scartata.store(true, Ordering::Relaxed);
    }
    let _ = app.emit("ocr://archivio", archivio::elenco());
    Ok(())
}

#[tauri::command]
fn archivio_rinomina(app: AppHandle, id: String, nome: String) -> Esito<()> {
    archivio::rinomina(&id, &nome)?;
    let _ = app.emit("ocr://archivio", archivio::elenco());
    Ok(())
}

/// L'indirizzo del codice sorgente sta qui, non nel frontend: il comando che
/// apre il browser non accetta un URL qualunque dalla webview, apre questo e
/// basta. La pagina lo mostra scritto, e una prova controlla che sia lo stesso.
const URL_REPOSITORY: &str = "https://github.com/GioOtto/ITA-OCR";

#[tauri::command]
fn apri_repository() -> Esito<String> {
    piattaforma::apri_collegamento(URL_REPOSITORY)?;
    Ok(URL_REPOSITORY.to_string())
}

#[cfg(test)]
mod prove_repository {
    /// L'indirizzo compare in due posti: qui e nella pagina delle
    /// informazioni. Se divergono, il bottone porta da una parte e il testo
    /// sotto le dita ne dice un'altra.
    #[test]
    fn la_pagina_mostra_lindirizzo_che_si_apre() {
        let html = include_str!("../../ui/index.html");
        assert!(
            html.contains(super::URL_REPOSITORY),
            "index.html non cita {}",
            super::URL_REPOSITORY
        );
    }
}

#[tauri::command]
fn apri_cartella_log() -> Esito<String> {
    let dir = risorse::dir_log();
    std::fs::create_dir_all(&dir).map_err(|e| format!("cartella log non creabile: {e}"))?;
    // Il gestore file cambia da sistema a sistema: la scelta sta in
    // piattaforma.rs, non qui. Con `xdg-open` cablato, su Windows questo
    // comando falliva sempre.
    piattaforma::apri_cartella(&dir)?;
    Ok(dir.display().to_string())
}

fn main() {
    // Modalita' di verifica senza finestra, per gli smoke test.
    let argomenti: Vec<String> = std::env::args().skip(1).collect();
    let modalita_prova = argomenti.first().map(String::as_str) == Some("--prova");
    if modalita_prova && std::env::var_os("OCR_ITA_DATA").is_none() {
        // Una prova puo' girare mentre la GUI e' aperta. PID file e log devono
        // essere distinti, altrimenti `ripulisci_orfani` termina il server
        // della GUI e il log della prova sostituisce quello diagnostico.
        let dir = std::env::temp_dir().join(format!("ocr-ita-prova-{}", std::process::id()));
        std::env::set_var("OCR_ITA_DATA", dir);
    }
    log::inizializza(risorse::dir_log());
    info!("avvio di ITA-OCR desktop {}", env!("CARGO_PKG_VERSION"));

    if modalita_prova {
        // Anche la modalita' senza GUI deve avere la garanzia kill-on-close
        // del job object nel caso venga terminata durante il caricamento.
        piattaforma::arma_chiusura_figli();
        motore::ripulisci_orfani();
        match prova::analizza_argomenti(&argomenti[1..]) {
            Ok(opzioni) => std::process::exit(prova::esegui(opzioni)),
            Err(e) => {
                eprintln!("uso: ocr-ita-desktop --prova [--backend auto|vulkan|cpu] \\\n                           \t[--pagine N] [--annulla-dopo SECONDI] [--testo] FILE...\n{e}");
                std::process::exit(64);
            }
        }
    }
    // Un server rimasto vivo da una sessione chiusa male tiene occupata la VRAM.
    motore::ripulisci_orfani();
    piattaforma::arma_chiusura_figli();

    let stato = Arc::new(Stato::nuovo());
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(stato.clone())
        .setup({
            let stato = stato.clone();
            move |app| {
                match risorse::libreria_pdfium() {
                    Ok(percorso) => {
                        if let Err(e) = pdfium::inizializza(&percorso) {
                            errore!("PDFium non disponibile: {e}");
                        } else {
                            info!("PDFium caricata da {}", percorso.display());
                        }
                    }
                    Err(e) => errore!("{e}"),
                }
                // I file passati sulla riga di comando (o da "apri con" del
                // gestore file) entrano subito nella coda.
                let iniziali: Vec<PathBuf> = std::env::args()
                    .skip(1)
                    .filter(|a| !a.starts_with('-'))
                    .map(PathBuf::from)
                    .collect();
                if !iniziali.is_empty() {
                    let handle = app.handle().clone();
                    let stato = stato.clone();
                    std::thread::spawn(move || {
                        let problemi = lavoro::aggiungi(&handle, &stato, iniziali);
                        for problema in problemi {
                            attenzione!("{problema}");
                        }
                    });
                }
                // Il motore parte subito, in un thread suo: il caricamento dei
                // pesi non deve bloccare la finestra.
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    let _ = lavoro::assicura_motore(&handle, &stato);
                });
                Ok(())
            }
        })
        .invoke_handler(tauri::generate_handler![
            apri_file,
            svuota,
            avvia,
            annulla,
            elenco,
            testo_pagina,
            anteprima,
            esporta,
            esporta_docx,
            salva_file,
            diagnostica,
            riepilogo,
            info_motore,
            imposta_backend,
            imposta_forza_ocr,
            imposta_correzione_automatica,
            imposta_correzione_contestuale,
            riavvia_motore,
            apri_cartella_log,
            apri_repository,
            archivio_elenco,
            archivio_salva,
            archivio_carica,
            archivio_elimina,
            archivio_rinomina,
        ])
        .build(tauri::generate_context!())
        .expect("finestra non creabile")
        .run(move |handle, evento| {
            if let tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit = evento {
                // Il figlio non sopravvive mai al padre.
                if let Some(riferimento) = handle.try_state::<Arc<Stato>>() {
                    let stato = riferimento.inner().clone();
                    stato.annulla.store(true, Ordering::SeqCst);
                    stato.annulla_avvio.store(true, Ordering::SeqCst);
                    let da_fermare = stato.motore.lock().ok().and_then(|mut s| s.take());
                    if let Some(mut motore) = da_fermare {
                        motore.ferma();
                    }
                }
            }
        });
}
