//! Log su file con rotazione, piu' un anello in memoria per il pannello avanzato.
//!
//! L'utente non deve aprire un terminale per capire cosa e' successo: il
//! diagnostico sta nella UI e il file su disco si apre con un pulsante.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
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

fn ruota_con_limite(file: &PathBuf, limite: u64) -> io::Result<()> {
    let meta = match fs::metadata(file) {
        Ok(meta) => meta,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if meta.len() < limite {
        return Ok(());
    }
    match fs::remove_file(file.with_extension(format!("log.{COPIE}"))) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    for indice in (1..COPIE).rev() {
        let da = file.with_extension(format!("log.{indice}"));
        let a = file.with_extension(format!("log.{}", indice + 1));
        match fs::rename(da, a) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    fs::rename(file, file.with_extension("log.1"))
}

fn ruota(file: &PathBuf) -> io::Result<()> {
    ruota_con_limite(file, BYTE_MASSIMI)
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
    if let Err(e) = ruota(&stato.file) {
        eprintln!("[warn] rotazione del log fallita: {e}");
    }
    if let Ok(mut f) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stato.file)
    {
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
    STATO
        .get()
        .and_then(|c| c.lock().ok().map(|s| s.file.clone()))
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn conserva_le_cinque_copie_nell_ordine_corretto() {
        let dir = std::env::temp_dir().join(format!(
            "ita-ocr-log-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("ocr-ita-desktop.log");
        fs::write(&file, "corrente").unwrap();
        for indice in 1..=COPIE {
            fs::write(
                file.with_extension(format!("log.{indice}")),
                format!("copia {indice}"),
            )
            .unwrap();
        }

        ruota_con_limite(&file, 0).unwrap();

        assert!(!file.exists());
        assert_eq!(
            fs::read_to_string(file.with_extension("log.1")).unwrap(),
            "corrente"
        );
        for indice in 2..=COPIE {
            assert_eq!(
                fs::read_to_string(file.with_extension(format!("log.{indice}"))).unwrap(),
                format!("copia {}", indice - 1)
            );
        }
        fs::remove_dir_all(dir).unwrap();
    }
}
