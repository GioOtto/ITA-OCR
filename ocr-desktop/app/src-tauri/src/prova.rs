//! Modalita' di verifica senza interfaccia: `ocr-ita-desktop --prova <file>`.
//!
//! Esercita esattamente la pipeline che usa la finestra: riconoscimento del
//! documento, fast path sul text layer, canvas 960x1248, cascata condizionale in
//! streaming, chiusura del motore, ma scrive su stdout invece che sulla
//! webview. Serve agli smoke test e a capire un problema senza aprire la GUI.

use crate::motore::{Backend, Motore};
use crate::{cascata, documenti, info, pdfium, risorse};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct Opzioni {
    pub percorsi: Vec<PathBuf>,
    pub backend: Option<Backend>,
    pub massimo_pagine: usize,
    pub annulla_dopo: Option<f64>,
    pub mostra_testo: bool,
}

pub fn analizza_argomenti(argomenti: &[String]) -> Result<Opzioni, String> {
    let mut opzioni = Opzioni {
        percorsi: Vec::new(),
        backend: None,
        massimo_pagine: usize::MAX,
        annulla_dopo: None,
        mostra_testo: false,
    };
    let mut indice = 0;
    while indice < argomenti.len() {
        match argomenti[indice].as_str() {
            "--backend" => {
                indice += 1;
                let valore = argomenti.get(indice).ok_or("--backend vuole un valore")?;
                opzioni.backend = match valore.as_str() {
                    "cuda" => Some(Backend::Cuda),
                    "vulkan" => Some(Backend::Vulkan),
                    "cpu" => Some(Backend::Cpu),
                    "auto" => None,
                    altro => return Err(format!("backend sconosciuto: {altro}")),
                };
            }
            "--pagine" => {
                indice += 1;
                opzioni.massimo_pagine = argomenti
                    .get(indice)
                    .and_then(|v| v.parse().ok())
                    .ok_or("--pagine vuole un numero")?;
            }
            "--annulla-dopo" => {
                indice += 1;
                opzioni.annulla_dopo = Some(
                    argomenti
                        .get(indice)
                        .and_then(|v| v.parse().ok())
                        .ok_or("--annulla-dopo vuole un numero di secondi")?,
                );
            }
            "--testo" => opzioni.mostra_testo = true,
            altro if altro.starts_with("--") => {
                return Err(format!("opzione sconosciuta: {altro}"))
            }
            percorso => opzioni.percorsi.push(PathBuf::from(percorso)),
        }
        indice += 1;
    }
    if opzioni.percorsi.is_empty() {
        return Err("serve almeno un file".into());
    }
    Ok(opzioni)
}

fn riga(etichetta: &str, valore: impl std::fmt::Display) {
    println!("{etichetta:<26} {valore}");
}

pub fn esegui(opzioni: Opzioni) -> i32 {
    let inizio_totale = Instant::now();
    match risorse::libreria_pdfium().and_then(|p| pdfium::inizializza(&p)) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("PDFium non disponibile: {e}");
            return 2;
        }
    }

    let mut documenti_aperti = Vec::new();
    for percorso in &opzioni.percorsi {
        match documenti::apri(percorso, false) {
            Ok(documento) => documenti_aperti.push(documento),
            Err(e) => {
                eprintln!("{}: {e}", percorso.display());
                return 2;
            }
        }
    }

    println!("=== documenti ===");
    for documento in &documenti_aperti {
        let con_testo = documento
            .pagine
            .iter()
            .filter(|p| p.testo_nativo.is_some())
            .count();
        riga(
            "documento",
            format!(
                "{} · {} · {} pagine · {} con text layer",
                documento.nome,
                match documento.tipo {
                    documenti::Tipo::Pdf => "PDF",
                    documenti::Tipo::Immagine => "immagine",
                },
                documento.pagine.len(),
                con_testo
            ),
        );
    }

    // Il motore si accende solo se serve davvero il modello.
    let servono_pagine_vlm = documenti_aperti.iter().any(|documento| {
        documento
            .pagine
            .iter()
            .take(opzioni.massimo_pagine)
            .any(|p| p.testo_nativo.is_none())
    });

    let annulla = Arc::new(AtomicBool::new(false));
    let mut motore: Option<Motore> = None;
    if servono_pagine_vlm {
        println!("\n=== motore ===");
        // Nessun ricordo: lo smoke test non deve dipendere da cosa aveva
        // funzionato in una sessione precedente dell'applicazione.
        let esito = Motore::avvia_con_fallback(opzioni.backend, None, &annulla);
        match esito {
            Ok((m, _)) => {
                riga("backend scelto", m.backend.etichetta());
                riga("dispositivo", &m.dispositivo);
                riga("porta locale", m.porta);
                riga("thread", m.thread);
                riga("caricamento", format!("{:.1} s", m.avvio_secondi));
                riga("pid llama-server", m.pid().unwrap_or(0));
                motore = Some(m);
            }
            Err(e) => {
                eprintln!("motore non avviabile: {e}");
                return 3;
            }
        }
    }

    if let Some(secondi) = opzioni.annulla_dopo {
        let bandiera = annulla.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs_f64(secondi));
            println!("\n[annullamento richiesto dopo {secondi:.1} s]");
            bandiera.store(true, Ordering::SeqCst);
        });
    }

    println!("\n=== pagine ===");
    let mut fatte = 0usize;
    let mut incomplete = 0usize;
    let mut fast_path = 0usize;
    let mut annullate = 0usize;
    let mut secondi_vlm = 0.0f64;

    'documenti: for documento in &documenti_aperti {
        for (indice, sorgente) in documento.pagine.iter().enumerate() {
            if indice >= opzioni.massimo_pagine {
                break;
            }
            if annulla.load(Ordering::Relaxed) {
                break 'documenti;
            }
            let etichetta = format!("{} p.{}", documento.nome, sorgente.numero);
            if let Some(testo) = &sorgente.testo_nativo {
                fast_path += 1;
                fatte += 1;
                println!(
                    "{etichetta:<44} text layer   {:>5} caratteri",
                    testo.chars().count()
                );
                if opzioni.mostra_testo {
                    println!("--- {etichetta} ---\n{}\n", testo.trim());
                }
                continue;
            }

            let Some(m) = motore.as_ref() else { continue };
            let inizio = Instant::now();
            let immagine = match documenti::immagine_per_ocr(documento, indice) {
                Ok(b64) => b64,
                Err(e) => {
                    println!("{etichetta:<44} ERRORE       {e}");
                    continue;
                }
            };
            let contesto = cascata::Contesto {
                porta: m.porta,
                prompt: &m.prompt,
                immagine_base64: &immagine,
                annulla: &annulla,
            };
            let mut caratteri = 0usize;
            let mut primo_token: Option<f64> = None;
            let mut su_evento = |evento: cascata::Evento| match evento {
                cascata::Evento::Stadio { indice, nome } => {
                    if indice > 0 {
                        println!("  → ciclo rilevato, ritento con {nome}");
                    }
                }
                cascata::Evento::Testo(testo) => {
                    if primo_token.is_none() {
                        primo_token = Some(inizio.elapsed().as_secs_f64());
                    }
                    caratteri += testo.chars().count();
                    if opzioni.mostra_testo {
                        print!("{testo}");
                        let _ = std::io::stdout().flush();
                    }
                }
            };
            match cascata::esegui(&contesto, &mut su_evento) {
                Ok(esito) => {
                    if opzioni.mostra_testo {
                        println!();
                    }
                    secondi_vlm += esito.secondi;
                    let stato = if esito.annullata {
                        annullate += 1;
                        "ANNULLATA"
                    } else if esito.completata {
                        fatte += 1;
                        "completata"
                    } else {
                        incomplete += 1;
                        "INCOMPLETA"
                    };
                    println!(
                        "{etichetta:<44} {stato:<12} {:>5.2} s · {:>4} token · \
                         {} tentativi · primo token {:.2} s · {} caratteri",
                        esito.secondi,
                        esito.token,
                        esito.tentativi.len(),
                        primo_token.unwrap_or(f64::NAN),
                        esito.testo.chars().count()
                    );
                    if esito.annullata {
                        break 'documenti;
                    }
                }
                Err(e) => println!("{etichetta:<44} ERRORE       {e}"),
            }
            let _ = caratteri;
        }
    }

    println!("\n=== riepilogo ===");
    riga("complete", fatte);
    riga("di cui fast path", fast_path);
    riga("incomplete", incomplete);
    riga("annullate", annullate);
    riga("secondi nel VLM", format!("{secondi_vlm:.2}"));
    riga("wall totale", format!("{:.2} s", inizio_totale.elapsed().as_secs_f64()));

    if let Some(mut m) = motore {
        let pid = m.pid();
        m.ferma();
        std::thread::sleep(Duration::from_millis(300));
        if let Some(pid) = pid {
            let vivo = crate::piattaforma::e_nostro_motore(pid, crate::piattaforma::ALIAS_MOTORE);
            riga(
                "llama-server residuo",
                if vivo { "SI (problema)" } else { "no" },
            );
            if vivo {
                return 4;
            }
        }
    }
    info!("verifica conclusa");
    0
}
