//! Scrittura atomica dei piccoli file di stato dell'applicazione.
//!
//! Il contenuto viene prima scritto e sincronizzato in un file temporaneo
//! nella stessa directory, poi sostituisce la destinazione con un rename.
//! In questo modo un arresto a meta' scrittura lascia intatta la copia prima.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static PROSSIMO_TEMPORANEO: AtomicU64 = AtomicU64::new(0);

fn percorso_temporaneo(percorso: &Path) -> io::Result<PathBuf> {
    let dir = percorso
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "percorso senza cartella"))?;
    let nome = percorso
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "nome file non valido"))?;
    let sequenza = PROSSIMO_TEMPORANEO.fetch_add(1, Ordering::Relaxed);
    Ok(dir.join(format!(".{nome}.{}.{}.tmp", std::process::id(), sequenza)))
}

#[cfg(not(windows))]
fn sostituisci(temporaneo: &Path, percorso: &Path) -> io::Result<()> {
    fs::rename(temporaneo, percorso)
}

#[cfg(windows)]
fn sostituisci(temporaneo: &Path, percorso: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    fn largo(percorso: &Path) -> io::Result<Vec<u16>> {
        let mut fuori: Vec<u16> = percorso.as_os_str().encode_wide().collect();
        if fuori.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "percorso con carattere nullo",
            ));
        }
        fuori.push(0);
        Ok(fuori)
    }

    let da = largo(temporaneo)?;
    let a = largo(percorso)?;
    let esito = unsafe {
        MoveFileExW(
            da.as_ptr(),
            a.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if esito == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

pub fn scrivi(percorso: &Path, dati: &[u8]) -> io::Result<()> {
    let dir = percorso
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "percorso senza cartella"))?;
    fs::create_dir_all(dir)?;

    // create_new impedisce che due salvataggi concorrenti condividano il file
    // temporaneo. La sequenza rende la collisione gia' improbabile; il ciclo
    // copre anche residui lasciati da un processo terminato male.
    for _ in 0..100 {
        let temporaneo = percorso_temporaneo(percorso)?;
        let apertura = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporaneo);
        let mut file = match apertura {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        };
        let esito = (|| {
            file.write_all(dati)?;
            file.sync_all()?;
            drop(file);
            sostituisci(&temporaneo, percorso)?;
            #[cfg(unix)]
            fs::File::open(dir)?.sync_all()?;
            Ok(())
        })();
        if esito.is_err() {
            let _ = fs::remove_file(&temporaneo);
        }
        return esito;
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "impossibile creare il file temporaneo",
    ))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn sostituisce_un_file_senza_lasciare_temporanei() {
        let dir = std::env::temp_dir().join(format!(
            "ita-ocr-atomico-{}-{}",
            std::process::id(),
            PROSSIMO_TEMPORANEO.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&dir).unwrap();
        let percorso = dir.join("stato.json");
        fs::write(&percorso, b"prima").unwrap();

        scrivi(&percorso, b"dopo").unwrap();

        assert_eq!(fs::read(&percorso).unwrap(), b"dopo");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }
}
