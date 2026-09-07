//! Le differenze fra sistemi operativi, tutte in un posto solo.
//!
//! Il resto del programma non deve sapere se sta girando su Linux o su Windows:
//! chiede qui il nome di un eseguibile, come si passa una directory di librerie
//! al figlio, e come si fa in modo che `llama-server` non sopravviva mai al
//! processo che lo ha avviato.
//!
//! Quest'ultimo punto e' quello che cambia davvero. Su Unix si intercettano
//! SIGTERM/SIGINT/SIGHUP e si uccide il figlio a mano, perche' il distruttore
//! non viene eseguito quando arriva un segnale. Su Windows non esistono i
//! segnali: si mette il figlio in un *job object* con kill-on-close, e il
//! sistema lo chiude da solo quando il padre muore, anche se il padre e' stato
//! terminato di forza. La garanzia e' la stessa, il meccanismo no.

/// L'alias con cui si riconosce un nostro `llama-server` fra i processi.
pub const ALIAS_MOTORE: &str = "ocr-ita-desktop-engine";

// ============================================================ nomi dei file

#[cfg(windows)]
pub const NOME_MOTORE: &str = "llama-server.exe";
#[cfg(not(windows))]
pub const NOME_MOTORE: &str = "llama-server";

/// I nomi possibili della libreria PDFium, in ordine di preferenza.
#[cfg(windows)]
pub const NOMI_PDFIUM: &[&str] = &["pdfium.dll"];
#[cfg(target_os = "macos")]
pub const NOMI_PDFIUM: &[&str] = &["libpdfium.dylib"];
#[cfg(all(not(windows), not(target_os = "macos")))]
pub const NOMI_PDFIUM: &[&str] = &["libpdfium.so"];

// ================================================= ricerca delle librerie

/// La variabile con cui si dice al figlio dove stanno le sue librerie.
/// Su Windows il caricatore usa `PATH`, su Unix `LD_LIBRARY_PATH`.
#[cfg(windows)]
pub const VARIABILE_LIBRERIE: &str = "PATH";
#[cfg(not(windows))]
pub const VARIABILE_LIBRERIE: &str = "LD_LIBRARY_PATH";

#[cfg(windows)]
const SEPARATORE: char = ';';
#[cfg(not(windows))]
const SEPARATORE: char = ':';

/// Antepone `dir` al contenuto attuale della variabile delle librerie.
pub fn percorso_librerie(dir: &std::path::Path) -> String {
    match std::env::var(VARIABILE_LIBRERIE) {
        Ok(esistente) if !esistente.is_empty() => {
            format!("{}{SEPARATORE}{esistente}", dir.display())
        }
        _ => dir.display().to_string(),
    }
}

// ======================================================== vita dei figli

#[cfg(unix)]
mod interno {
    use std::sync::atomic::{AtomicI32, Ordering};

    /// PID del server in corso, letto anche dal gestore di segnale.
    static PID_MOTORE: AtomicI32 = AtomicI32::new(0);

    /// Gestore di SIGTERM/SIGINT/SIGHUP: chiude il figlio e poi esce.
    ///
    /// Senza questo, un `pkill` o la chiusura della sessione lascerebbero
    /// `llama-server` vivo con la VRAM occupata: il gestore della finestra non
    /// viene eseguito quando il processo riceve un segnale.
    extern "C" fn su_segnale(segnale: libc::c_int) {
        let pid = PID_MOTORE.load(Ordering::SeqCst);
        if pid > 0 {
            // kill() e _exit() sono le uniche cose lecite qui dentro.
            unsafe { libc::kill(pid, libc::SIGKILL) };
        }
        unsafe { libc::_exit(128 + segnale) };
    }

    pub fn arma_chiusura_figli() {
        for segnale in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
            unsafe {
                libc::signal(segnale, su_segnale as *const () as libc::sighandler_t);
            }
        }
    }

    pub fn adotta(processo: &std::process::Child) {
        PID_MOTORE.store(processo.id() as i32, Ordering::SeqCst);
    }

    pub fn dimentica() {
        PID_MOTORE.store(0, Ordering::SeqCst);
    }

    pub fn termina(pid: u32, forza: bool) {
        let segnale = if forza { libc::SIGKILL } else { libc::SIGTERM };
        unsafe { libc::kill(pid as i32, segnale) };
    }

    pub fn apri_cartella(dir: &std::path::Path) -> Result<(), String> {
        std::process::Command::new("xdg-open")
            .arg(dir)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("non riesco ad aprire il gestore file: {e}"))
    }

    pub fn apri_collegamento(url: &str) -> Result<(), String> {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("non riesco ad aprire il browser: {e}"))
    }

    /// Vero solo se quel PID e' davvero un nostro `llama-server`: un PID
    /// riciclato da un altro programma non deve essere toccato.
    pub fn e_nostro_motore(pid: u32, alias: &str) -> bool {
        let cmdline = std::fs::read(format!("/proc/{pid}/cmdline")).unwrap_or_default();
        String::from_utf8_lossy(&cmdline)
            .replace('\0', " ")
            .contains(alias)
    }
}

#[cfg(windows)]
mod interno {
    use std::os::windows::io::AsRawHandle;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, MAX_PATH};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, TerminateProcess,
        PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    };

    /// Il job a cui si appende il motore. Windows lo chiude quando l'ultimo
    /// riferimento sparisce, cioe' quando questo processo muore comunque sia.
    static JOB: AtomicIsize = AtomicIsize::new(0);

    pub fn arma_chiusura_figli() {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return;
            }
            let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let esito = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const std::ffi::c_void,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            );
            if esito == 0 {
                CloseHandle(job);
                return;
            }
            JOB.store(job as isize, Ordering::SeqCst);
        }
    }

    pub fn adotta(processo: &std::process::Child) {
        let job = JOB.load(Ordering::SeqCst);
        if job == 0 {
            return;
        }
        unsafe {
            AssignProcessToJobObject(job as HANDLE, processo.as_raw_handle() as HANDLE);
        }
    }

    /// Il job resta armato per tutta la vita del programma: non c'e' niente da
    /// dimenticare quando un singolo motore si chiude.
    pub fn dimentica() {}

    pub fn termina(pid: u32, _forza: bool) {
        // Windows non distingue fra chiusura garbata e forzata per un processo
        // senza finestre: TerminateProcess e' l'unica strada.
        unsafe {
            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid);
            if !handle.is_null() {
                TerminateProcess(handle, 1);
                CloseHandle(handle);
            }
        }
    }

    /// Su Windows il gestore file e' `explorer`, e non esiste `xdg-open`.
    ///
    /// `explorer.exe` restituisce 1 anche quando la finestra si apre, percio'
    /// non si guarda il codice di uscita: basta essere riusciti a lanciarlo.
    pub fn apri_cartella(dir: &std::path::Path) -> Result<(), String> {
        let mut comando = std::process::Command::new("explorer.exe");
        comando.arg(dir);
        super::nascondi_console(&mut comando);
        comando
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("non riesco ad aprire il gestore file: {e}"))
    }

    /// `explorer.exe <url>` apre il browser predefinito, come farebbe un
    /// doppio clic su un collegamento: non serve `cmd /c start`, che
    /// mostrerebbe una console per un istante.
    pub fn apri_collegamento(url: &str) -> Result<(), String> {
        let mut comando = std::process::Command::new("explorer.exe");
        comando.arg(url);
        super::nascondi_console(&mut comando);
        comando
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("non riesco ad aprire il browser: {e}"))
    }

    /// Su Windows la riga di comando di un altro processo non si legge senza
    /// WMI, ma il percorso completo dell'eseguibile si': deve coincidere col
    /// `llama-server` del nostro pacchetto. Controllare il solo nome rischia di
    /// terminare il server di un'altra applicazione dopo il riciclo di un PID.
    pub fn e_nostro_motore(pid: u32, _alias: &str) -> bool {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return false;
            }
            let mut buffer = [0u16; MAX_PATH as usize];
            let mut lunghezza = buffer.len() as u32;
            let esito = QueryFullProcessImageNameW(handle, 0, buffer.as_mut_ptr(), &mut lunghezza);
            CloseHandle(handle);
            if esito == 0 {
                return false;
            }
            let visto =
                std::path::PathBuf::from(String::from_utf16_lossy(&buffer[..lunghezza as usize]));
            let Ok(atteso) = crate::risorse::binario_llama() else {
                return false;
            };
            let visto = visto.canonicalize().unwrap_or(visto);
            let atteso = atteso.canonicalize().unwrap_or(atteso);
            visto
                .to_string_lossy()
                .eq_ignore_ascii_case(&atteso.to_string_lossy())
        }
    }
}

pub use interno::{
    adotta, apri_cartella, apri_collegamento, arma_chiusura_figli, dimentica, e_nostro_motore,
    termina,
};

/// Impedisce che il figlio apra una finestra di console.
///
/// Su Windows un processo lanciato da un'applicazione grafica si porta dietro
/// una console nera: `llama-server` ne aprirebbe una a ogni avvio. Altrove non
/// c'e' niente da nascondere.
#[cfg(windows)]
pub fn nascondi_console(comando: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    comando.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn nascondi_console(_comando: &mut std::process::Command) {}

// ================================================ dettagli solo per Linux

/// Su Linux il loader Vulkan puo' scegliere lavapipe (software) quando c'e' un
/// ICD AMD utilizzabile: si punta esplicitamente a quello. Altrove non serve.
#[cfg(target_os = "linux")]
pub fn icd_vulkan_da_forzare() -> Option<std::path::PathBuf> {
    let icd = std::path::PathBuf::from("/usr/share/vulkan/icd.d/radeon_icd.json");
    icd.is_file().then_some(icd)
}

#[cfg(not(target_os = "linux"))]
pub fn icd_vulkan_da_forzare() -> Option<std::path::PathBuf> {
    None
}
