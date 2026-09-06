//! Log su file con rotazione, piu' un anello in memoria per il pannello avanzato.
//!
//! L'utente non deve aprire un terminale per capire cosa e' successo: il
//! diagnostico sta nella UI e il file su disco si apre con un pulsante.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const BYTE_MASSIMI: u64 = 2 * 1024 * 1024;
const COPIE: usize = 5;
const RIGHE_IN_MEMORIA: usize = 400;

struct Stato {
    file: PathBuf,
    anello: Vec<Riga>,
}

#[derive(Clone, serde::Serialize)]
pub struct Riga {
    pub istante: String,
    pub livello: String,
    pub testo: String,
}

static STATO: OnceLock<Mutex<Stato>> = OnceLock::new();

fn orario() -> String {
    // Niente crate di date: serve un timestamp ordinabile, non un calendario.
    let ora = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secondi = ora.as_secs();
    let giorni = secondi / 86400;
    let resto = secondi % 86400;
    let (mut anno, mut giorno) = (1970i64, giorni as i64);
    loop {
        let bisestile = (anno % 4 == 0 && anno % 100 != 0) || anno % 400 == 0;
        let lunghezza = if bisestile { 366 } else { 365 };
        if giorno < lunghezza {
            break;
        }
        giorno -= lunghezza;
        anno += 1;
    }
    let bisestile = (anno % 4 == 0 && anno % 100 != 0) || anno % 400 == 0;
    let mesi = [
        31,
        if bisestile { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut mese = 0;
    while mese < 12 && giorno >= mesi[mese] {
        giorno -= mesi[mese];
        mese += 1;
    }
    format!(
        "{anno:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        mese + 1,
        giorno + 1,
        resto / 3600,
        (resto % 3600) / 60,
        resto % 60
    )
}

pub fn inizializza(dir: PathBuf) {
    let _ = fs::create_dir_all(&dir);
    let _ = STATO.set(Mutex::new(Stato {
        file: dir.join("ocr-ita-desktop.log"),
        anello: Vec::new(),
    }));
}

fn ruota(file: &PathBuf) {
    let Ok(meta) = fs::metadata(file) else {
        return;
    };
    if meta.len() < BYTE_MASSIMI {
        return;
    }
    let _ = fs::remove_file(file.with_extension(format!("log.{COPIE}")));
    for indice in (1..COPIE).rev() {
        let da = if indice == 1 {
            file.clone()
        } else {
            file.with_extension(format!("log.{}", indice - 1))
        };
        let a = file.with_extension(format!("log.{indice}"));
        let _ = fs::rename(&da, &a);
    }
    let _ = fs::rename(file, file.with_extension("log.1"));
}

pub fn scrivi(livello: &str, testo: impl AsRef<str>) {
    let testo = testo.as_ref();
    let riga = Riga {
        istante: orario(),
        livello: livello.to_string(),
        testo: testo.to_string(),
    };
    let Some(cella) = STATO.get() else {
        eprintln!("[{livello}] {testo}");
        return;
    };
    let Ok(mut stato) = cella.lock() else {
        return;
    };
    ruota(&stato.file);
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&stato.file) {
        let _ = writeln!(f, "{} [{}] {}", riga.istante, riga.livello, riga.testo);
    }
    if stato.anello.len() >= RIGHE_IN_MEMORIA {
        stato.anello.remove(0);
    }
    stato.anello.push(riga);
}

pub fn recenti() -> Vec<Riga> {
    STATO
        .get()
        .and_then(|c| c.lock().ok().map(|s| s.anello.clone()))
        .unwrap_or_default()
}

pub fn file_corrente() -> Option<PathBuf> {
    STATO.get().and_then(|c| c.lock().ok().map(|s| s.file.clone()))
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => { $crate::log::scrivi("info", format!($($arg)*)) };
}

#[macro_export]
macro_rules! attenzione {
    ($($arg:tt)*) => { $crate::log::scrivi("warn", format!($($arg)*)) };
}

#[macro_export]
macro_rules! errore {
    ($($arg:tt)*) => { $crate::log::scrivi("error", format!($($arg)*)) };
}
