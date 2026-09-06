//! Ciclo di vita di `llama-server`: scelta del backend, avvio, salute, chiusura.
//!
//! Il protocollo e' quello gia' misurato: `-c 8192 -np 1 --no-cache-prompt
//! --cache-ram 0`, override dell'`eot` per rimettere in gioco entrambi gli stop
//! ufficiali di GLM-OCR. I backend GPU sono CUDA (NVIDIA) e Vulkan (AMD e
//! Intel), scelti in quest'ordine quando la scelta e' automatica, con la CPU
//! come rete di sicurezza. **ROCm non viene mai usato**: non e' nemmeno
//! compilato dentro il motore che spediamo.

use crate::{attenzione, gguf, http, info, piattaforma, risorse};
use std::io::{BufRead, BufReader};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    Cuda,
    Vulkan,
    Cpu,
}

impl Backend {
    pub fn etichetta(self) -> &'static str {
        match self {
            Backend::Cuda => "GPU · CUDA",
            Backend::Vulkan => "GPU · Vulkan",
            Backend::Cpu => "CPU",
        }
    }
    pub fn chiave(self) -> &'static str {
        match self {
            Backend::Cuda => "cuda",
            Backend::Vulkan => "vulkan",
            Backend::Cpu => "cpu",
        }
    }
}

pub struct Dispositivo {
    pub identificativo: String,
    pub descrizione: String,
}

/// Elenco dei device che il motore vede davvero, chiedendolo al binario.
pub fn dispositivi(binario: &Path, dir: &Path) -> Vec<Dispositivo> {
    let mut comando = Command::new(binario);
    piattaforma::nascondi_console(&mut comando);
    let uscita = comando
        .arg("--list-devices")
        .env(piattaforma::VARIABILE_LIBRERIE, piattaforma::percorso_librerie(dir))
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(uscita) = uscita else {
        return Vec::new();
    };
    String::from_utf8_lossy(&uscita.stdout)
        .lines()
        .filter_map(|riga| {
            let riga = riga.trim();
            let (nome, resto) = riga.split_once(':')?;
            let nome = nome.trim();
            // llama.cpp elenca i device come "Vulkan0:" o "CUDA0:": una parola
            // sola col numero attaccato. Le righe di intestazione hanno spazi.
            if nome.contains(' ') {
                return None;
            }
            if !nome.starts_with("Vulkan") && !nome.starts_with("CUDA") {
                return None;
            }
            Some(Dispositivo {
                identificativo: nome.to_string(),
                descrizione: resto.trim().to_string(),
            })
        })
        .collect()
}

/// Il prefisso con cui llama.cpp nomina i device di quel backend.
fn prefisso_device(backend: Backend) -> Option<&'static str> {
    match backend {
        Backend::Cuda => Some("CUDA"),
        Backend::Vulkan => Some("Vulkan"),
        Backend::Cpu => None,
    }
}

/// Il primo device del backend richiesto che non sia un rasterizzatore
/// software: llvmpipe e lavapipe girano su CPU spacciandosi per GPU, e
/// sceglierli vorrebbe dire fare inferenza in software credendo il contrario.
pub fn dispositivo_per(backend: Backend, binario: &Path, dir: &Path) -> Option<Dispositivo> {
    let prefisso = prefisso_device(backend)?;
    dispositivi(binario, dir).into_iter().find(|d| {
        if !d.identificativo.starts_with(prefisso) {
            return false;
        }
        let minuscolo = d.descrizione.to_ascii_lowercase();
        !minuscolo.contains("llvmpipe") && !minuscolo.contains("lavapipe")
    })
}

fn porta_libera() -> Result<u16, String> {
    let ascolto = TcpListener::bind(("127.0.0.1", 0))
        .map_err(|e| format!("nessuna porta locale disponibile: {e}"))?;
    let porta = ascolto
        .local_addr()
        .map_err(|e| format!("porta non leggibile: {e}"))?
        .port();
    drop(ascolto);
    Ok(porta)
}

fn thread_consigliati() -> usize {
    let disponibili = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8);
    disponibili.saturating_sub(2).clamp(4, 32)
}

pub struct Motore {
    processo: Option<Child>,
    pub porta: u16,
    pub backend: Backend,
    pub dispositivo: String,
    pub prompt: String,
    pub thread: usize,
    pub log_server: PathBuf,
    pub modello: PathBuf,
    pub mmproj: PathBuf,
    pub avvio_secondi: f64,
}

/// Prompt canonico di GLM-OCR. Nessun prompt custom, nessun system message.
fn prompt_renderizzato(marcatore: &str) -> String {
    format!("[gMASK]<sop><|user|>\n{marcatore}Text Recognition:<|assistant|>\n")
}

/// Prende in carico il processo appena nato: da qui in poi il sistema
/// garantisce che non sopravviva a questa applicazione.
fn scrivi_pidfile(processo: &std::process::Child) {
    let dir = risorse::dir_dati();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("llama-server.pid"), processo.id().to_string());
    piattaforma::adotta(processo);
}

fn cancella_pidfile() {
    piattaforma::dimentica();
    let _ = std::fs::remove_file(risorse::dir_dati().join("llama-server.pid"));
}

/// Uccide un eventuale server rimasto orfano da una sessione precedente.
pub fn ripulisci_orfani() {
    let percorso = risorse::dir_dati().join("llama-server.pid");
    let Ok(contenuto) = std::fs::read_to_string(&percorso) else {
        return;
    };
    let Ok(pid) = contenuto.trim().parse::<u32>() else {
        let _ = std::fs::remove_file(&percorso);
        return;
    };
    // Si uccide solo se e' davvero un nostro llama-server: un PID riciclato da
    // un altro programma non viene toccato.
    if piattaforma::e_nostro_motore(pid, piattaforma::ALIAS_MOTORE) {
        attenzione!("trovato llama-server orfano (pid {pid}): lo chiudo");
        piattaforma::termina(pid, false);
        std::thread::sleep(Duration::from_millis(500));
        piattaforma::termina(pid, true);
    }
    let _ = std::fs::remove_file(&percorso);
}

impl Motore {
    pub fn avvia(backend: Backend, annulla: &Arc<AtomicBool>) -> Result<Motore, String> {
        let inizio = Instant::now();
        let dir = risorse::dir_llama()?;
        let binario = risorse::binario_llama()?;
        let (modello, mmproj) = risorse::modelli()?;
        let porta = porta_libera()?;
        let thread = thread_consigliati();
        let dir_log = risorse::dir_log();
        std::fs::create_dir_all(&dir_log).ok();
        let log_server = dir_log.join("llama-server.log");
        // Il log del server non si accumula fra sessioni: la sessione precedente
        // resta come .1 e basta, cosi' la diagnostica e' sempre leggibile.
        if log_server.is_file() {
            let _ = std::fs::rename(&log_server, dir_log.join("llama-server.1.log"));
        }

        let mut comando = Command::new(&binario);
        comando
            .arg("-m")
            .arg(&modello)
            .arg("--mmproj")
            .arg(&mmproj)
            .args(["-c", "8192"])
            .args(["-np", "1"])
            .args(["-t", &thread.to_string()])
            .args(["-tb", &thread.to_string()])
            .arg("--no-cache-prompt")
            // 8192 MiB di prompt cache in RAM host che nel nostro carico non
            // fanno mai centro: ogni pagina e' un'immagine diversa.
            .args(["--cache-ram", "0"])
            .arg("--no-webui")
            .args(["-a", piattaforma::ALIAS_MOTORE])
            .args(["--host", "127.0.0.1"])
            .args(["--port", &porta.to_string()]);

        let descrizione_dispositivo;
        match backend {
            // Dal punto di vista degli argomenti CUDA e Vulkan sono la stessa
            // cosa: tutti i layer sulla GPU e un device scelto per nome. Cambia
            // solo quale libreria ggml carica sotto.
            Backend::Cuda | Backend::Vulkan => {
                let scelto = dispositivo_per(backend, &binario, &dir).ok_or_else(|| {
                    format!("nessun dispositivo {} utilizzabile", backend.chiave())
                })?;
                descrizione_dispositivo = scelto.descrizione.clone();
                comando
                    .args(["-ngl", "999"])
                    .arg("--device")
                    .arg(&scelto.identificativo)
                    // `--device` vale solo per il modello linguistico: la torre
                    // visiva sceglie il backend per conto suo, prendendo il
                    // primo dispositivo di tipo GPU che ggml ha registrato. Su
                    // una macchina NVIDIA quello e' sempre CUDA, anche quando
                    // abbiamo chiesto Vulkan -- e se CUDA li' non funziona il
                    // motore parte, dice "pronto", e poi muore alla prima
                    // pagina dentro IM2COL, che e' la convoluzione della torre
                    // visiva. Qui la inchiodiamo allo stesso dispositivo.
                    .env("MTMD_BACKEND_DEVICE", &scelto.identificativo);
            }
            Backend::Cpu => {
                descrizione_dispositivo = "dispatch automatico llama.cpp".into();
                comando
                    .args(["-ngl", "0"])
                    .args(["--device", "none"])
                    // `--device none` non basta: la torre visiva ha una selezione
                    // di device sua e resterebbe sulla GPU.
                    .arg("--no-mmproj-offload");
            }
        }
        if let Some(override_kv) = gguf::override_eot(&modello) {
            info!("override stop GLM-OCR: {override_kv}");
            comando.arg("--override-kv").arg(&override_kv);
        } else {
            attenzione!("nessun override eot applicato: il GGUF dichiara un eos inatteso");
        }

        let uscita = std::fs::File::create(&log_server)
            .map_err(|e| format!("log del motore non scrivibile: {e}"))?;
        let errori = uscita
            .try_clone()
            .map_err(|e| format!("log del motore non duplicabile: {e}"))?;
        comando
            .env(piattaforma::VARIABILE_LIBRERIE, piattaforma::percorso_librerie(&dir))
            .stdin(Stdio::null())
            .stdout(Stdio::from(uscita))
            .stderr(Stdio::from(errori));
        // Il driver Vulkan si pinna a quello AMD quando c'e': senza, il loader
        // puo' scegliere lavapipe e l'inferenza va in software.
        if backend == Backend::Vulkan && std::env::var_os("VK_DRIVER_FILES").is_none() {
            if let Some(icd) = piattaforma::icd_vulkan_da_forzare() {
                comando.env("VK_DRIVER_FILES", icd);
            }
        }

        info!(
            "avvio motore: backend {} ({}), porta {porta}, {thread} thread",
            backend.chiave(),
            descrizione_dispositivo
        );
        piattaforma::nascondi_console(&mut comando);
        let mut processo = comando
            .spawn()
            .map_err(|e| format!("llama-server non avviabile: {e}"))?;
        scrivi_pidfile(&processo);

        // Il caricamento di 1,2 GB di pesi piu' la torre visiva puo' prendere
        // qualche decina di secondi a cache fredda.
        let scadenza = Instant::now() + Duration::from_secs(180);
        loop {
            if annulla.load(Ordering::Relaxed) {
                let _ = processo.kill();
                let _ = processo.wait();
                cancella_pidfile();
                return Err("avvio annullato".into());
            }
            if let Ok(Some(stato)) = processo.try_wait() {
                let coda = coda_log(&log_server, 40);
                cancella_pidfile();
                return Err(format!(
                    "il motore si e' chiuso durante il caricamento ({stato}).\n{coda}"
                ));
            }
            if let Ok(salute) = http::json(porta, "GET", "/health", None, Duration::from_secs(3)) {
                if salute.get("status").and_then(|s| s.as_str()) == Some("ok") {
                    break;
                }
            }
            if Instant::now() > scadenza {
                let _ = processo.kill();
                let _ = processo.wait();
                cancella_pidfile();
                return Err(format!(
                    "il motore non e' diventato pronto entro 180 s.\n{}",
                    coda_log(&log_server, 40)
                ));
            }
            std::thread::sleep(Duration::from_millis(200));
        }

        // Da qui il processo e' gia' vivo e registrato nel pidfile: ogni
        // errore deve chiuderlo esplicitamente. Lasciare che `Child` esca di
        // scope non termina il figlio e produce un server orfano.
        let proprieta = match http::json(porta, "GET", "/props", None, Duration::from_secs(30)) {
            Ok(proprieta) => proprieta,
            Err(e) => {
                let _ = processo.kill();
                let _ = processo.wait();
                cancella_pidfile();
                return Err(format!("proprieta' del motore non leggibili: {e}"));
            }
        };
        let marcatore = proprieta
            .get("media_marker")
            .and_then(|m| m.as_str())
            .filter(|m| !m.is_empty())
            .map(str::to_string);
        let Some(marcatore) = marcatore else {
            let _ = processo.kill();
            let _ = processo.wait();
            cancella_pidfile();
            return Err("llama-server senza media_marker multimodale".into());
        };
        if annulla.load(Ordering::Relaxed) {
            let _ = processo.kill();
            let _ = processo.wait();
            cancella_pidfile();
            return Err("avvio annullato".into());
        }

        let avvio_secondi = inizio.elapsed().as_secs_f64();
        info!(
            "motore pronto in {avvio_secondi:.1} s su 127.0.0.1:{porta} ({})",
            backend.chiave()
        );
        Ok(Motore {
            processo: Some(processo),
            porta,
            backend,
            dispositivo: descrizione_dispositivo,
            prompt: prompt_renderizzato(&marcatore),
            thread,
            log_server,
            modello,
            mmproj,
            avvio_secondi,
        })
    }

    /// Prova la catena richiesta. La scelta che funziona viene restituita al
    /// chiamante, che in automatico la memorizza per la prossima sessione.
    pub fn avvia_con_fallback(
        esplicito: Option<Backend>,
        ricordato: Option<Backend>,
        annulla: &Arc<AtomicBool>,
    ) -> Result<(Motore, Vec<String>), String> {
        let mut note = Vec::new();
        let ordine: Vec<Backend> = match esplicito {
            Some(Backend::Cpu) => vec![Backend::Cpu],
            Some(Backend::Cuda) => vec![Backend::Cuda, Backend::Cpu],
            Some(Backend::Vulkan) => vec![Backend::Vulkan, Backend::Cpu],
            // In automatico si prova prima CUDA (le NVIDIA vanno meglio con la
            // loro libreria che col Vulkan generico), poi Vulkan per AMD e
            // Intel, e la CPU come rete di sicurezza. Quello che aveva
            // funzionato l'ultima volta passa in testa, ma gli altri restano
            // dietro: e' un ordine, non un'esclusione, altrimenti un ripiego
            // occasionale sulla CPU diventerebbe definitivo.
            None => {
                let mut catena = vec![Backend::Cuda, Backend::Vulkan, Backend::Cpu];
                if let Some(primo) = ricordato {
                    catena.retain(|b| *b != primo);
                    catena.insert(0, primo);
                }
                catena
            }
        };
        let mut ultimo = String::from("nessun backend provato");
        for backend in ordine {
            if annulla.load(Ordering::Relaxed) {
                return Err("avvio annullato".into());
            }
            match Motore::avvia(backend, annulla) {
                Ok(motore) => return Ok((motore, note)),
                Err(e) => {
                    attenzione!("backend {} non utilizzabile: {e}", backend.chiave());
                    note.push(format!("{}: {e}", backend.chiave()));
                    ultimo = e;
                }
            }
        }
        Err(ultimo)
    }

    pub fn pid(&self) -> Option<u32> {
        self.processo.as_ref().map(|p| p.id())
    }

    pub fn ferma(&mut self) {
        let Some(mut processo) = self.processo.take() else {
            return;
        };
        let pid = processo.id();
        info!("chiusura del motore (pid {pid})");
        piattaforma::termina(pid, false);
        let scadenza = Instant::now() + Duration::from_secs(15);
        loop {
            match processo.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < scadenza => {
                    std::thread::sleep(Duration::from_millis(100));
                }
                _ => {
                    attenzione!("il motore non ha risposto alla chiusura: lo termino");
                    let _ = processo.kill();
                    let _ = processo.wait();
                    break;
                }
            }
        }
        cancella_pidfile();
    }
}

impl Drop for Motore {
    fn drop(&mut self) {
        self.ferma();
    }
}

pub fn coda_log(percorso: &Path, righe: usize) -> String {
    let Ok(file) = std::fs::File::open(percorso) else {
        return String::new();
    };
    let tutte: Vec<String> = BufReader::new(file).lines().map_while(Result::ok).collect();
    let da = tutte.len().saturating_sub(righe);
    tutte[da..].join("\n")
}
