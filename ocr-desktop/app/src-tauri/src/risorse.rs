//! Dove stanno motore, libreria PDF, modelli, log e impostazioni.
//!
//! Il pacchetto e' relocabile: tutto si risolve a partire dall'eseguibile o da
//! `$APPDIR`, mai da percorsi assoluti compilati dentro. I due GGUF possono
//! stare nel pacchetto oppure in una directory risorse esterna, perche' pesano
//! 1,2 GB e non e' detto che si vogliano dentro l'AppImage.

use std::path::{Path, PathBuf};

pub const NOME_MODELLO: &str = "glm-ocr-ocr-ita-v8-native-3ep-clean-q8_0.gguf";
pub const NOME_MMPROJ: &str = "glm-ocr-base-mmproj-q8_0.gguf";
pub const NOME_APP: &str = "ocr-ita-desktop";

pub fn dir_eseguibile() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

/// La directory dell'AppImage, quando si gira dentro una.
pub fn dir_appimage() -> Option<PathBuf> {
    std::env::var_os("APPIMAGE")
        .map(PathBuf::from)
        .and_then(|p| p.parent().map(Path::to_path_buf))
}

fn candidati_risorse() -> Vec<PathBuf> {
    let mut fuori = Vec::new();
    if let Some(p) = std::env::var_os("OCR_ITA_RESOURCES") {
        fuori.push(PathBuf::from(p));
    }
    let exe = dir_eseguibile();
    if let Some(appdir) = std::env::var_os("APPDIR") {
        fuori.push(PathBuf::from(&appdir).join("usr/lib").join(NOME_APP));
    }
    fuori.push(exe.join("../lib").join(NOME_APP));
    fuori.push(exe.join("resources"));
    fuori.push(exe.clone());
    fuori
}

/// Radice delle risorse: contiene `llama/` e `libpdfium.so`.
pub fn radice_risorse() -> Result<PathBuf, String> {
    for candidato in candidati_risorse() {
        if candidato.join("llama").join(crate::piattaforma::NOME_MOTORE).is_file() {
            return Ok(candidato
                .canonicalize()
                .unwrap_or(candidato));
        }
    }
    Err("motore llama-server non trovato accanto all'applicazione. \
         Impostare OCR_ITA_RESOURCES sulla directory delle risorse."
        .into())
}

pub fn dir_llama() -> Result<PathBuf, String> {
    Ok(radice_risorse()?.join("llama"))
}

pub fn binario_llama() -> Result<PathBuf, String> {
    Ok(dir_llama()?.join(crate::piattaforma::NOME_MOTORE))
}

pub fn libreria_pdfium() -> Result<PathBuf, String> {
    let radice = radice_risorse()?;
    for nome in crate::piattaforma::NOMI_PDFIUM {
        for candidato in [radice.join(nome), radice.join("pdfium").join(nome)] {
            if candidato.is_file() {
                return Ok(candidato);
            }
        }
    }
    Err(format!(
        "{} non trovata nelle risorse dell'applicazione",
        crate::piattaforma::NOMI_PDFIUM[0]
    ))
}

pub fn dir_dizionario() -> Result<PathBuf, String> {
    let dir = radice_risorse()?.join("dictionaries/it_IT");
    if dir.join("it_IT.aff").is_file()
        && dir.join("it_IT.dic").is_file()
    {
        Ok(dir)
    } else {
        Err("dizionario italiano o lessico del training v8 non presenti nelle risorse".into())
    }
}

/// Il dizionario inglese, se spedito. Restituisce `Option` e non `Result`
/// perche' la sua assenza non impedisce di correggere l'italiano: un pacchetto
/// costruito prima che esistesse deve continuare a funzionare.
pub fn dir_dizionario_inglese() -> Option<PathBuf> {
    let dir = radice_risorse().ok()?.join("dictionaries/en_US");
    (dir.join("en_US.aff").is_file() && dir.join("en_US.dic").is_file()).then_some(dir)
}

fn candidati_modelli() -> Vec<PathBuf> {
    let mut fuori = Vec::new();
    if let Some(p) = std::env::var_os("OCR_ITA_MODELS") {
        fuori.push(PathBuf::from(p));
    }
    if let Ok(radice) = radice_risorse() {
        fuori.push(radice.join("models"));
    }
    fuori.push(dir_eseguibile().join("models"));
    if let Some(p) = dir_appimage() {
        fuori.push(p.join("models"));
        fuori.push(p.join(format!("{NOME_APP}-models")));
    }
    fuori.push(dir_dati().join("models"));
    fuori
}

/// La coppia (language model, torre visiva) da spedire: Q8_0 + Q8_0.
pub fn modelli() -> Result<(PathBuf, PathBuf), String> {
    let mut visti = Vec::new();
    for dir in candidati_modelli() {
        let lm = dir.join(NOME_MODELLO);
        let mm = dir.join(NOME_MMPROJ);
        if lm.is_file() && mm.is_file() {
            return Ok((lm, mm));
        }
        visti.push(dir.display().to_string());
    }
    Err(format!(
        "modelli non trovati. Servono {NOME_MODELLO} e {NOME_MMPROJ} in una di \
         queste directory, oppure in quella indicata da OCR_ITA_MODELS:\n  {}",
        visti.join("\n  ")
    ))
}

fn casa() -> PathBuf {
    // Su Windows la home sta in USERPROFILE: HOME di solito non esiste e il
    // vecchio ripiego su /tmp non vorrebbe dire niente.
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Dati dell'applicazione: log, pidfile, archivio delle sessioni.
#[cfg(windows)]
pub fn dir_dati() -> PathBuf {
    // Gli smoke test e gli ambienti gestiti possono isolare log, pidfile e
    // archivio. Senza questo override una verifica `--prova` avviata mentre
    // la GUI era aperta scambiava il suo server attivo per un orfano.
    if let Some(dir) = std::env::var_os("OCR_ITA_DATA") {
        return PathBuf::from(dir);
    }
    // La convenzione di Windows e' %APPDATA%, non una cartella nascosta nella
    // home: mettere qui un ".local/share" sarebbe fuori posto.
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| casa().join("AppData/Roaming"))
        .join(NOME_APP)
}

#[cfg(not(windows))]
pub fn dir_dati() -> PathBuf {
    if let Some(dir) = std::env::var_os("OCR_ITA_DATA") {
        return PathBuf::from(dir);
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| casa().join(".local/share"))
        .join(NOME_APP)
}

/// Impostazioni. Su Windows vivono accanto ai dati, come vuole la piattaforma.
#[cfg(windows)]
pub fn dir_config() -> PathBuf {
    dir_dati()
}

#[cfg(not(windows))]
pub fn dir_config() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| casa().join(".config"))
        .join(NOME_APP)
}

pub fn dir_log() -> PathBuf {
    dir_dati().join("logs")
}
