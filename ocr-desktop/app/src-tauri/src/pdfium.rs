//! Binding minimale a libpdfium.so, caricata a runtime con dlopen.
//!
//! Servono quindici funzioni della C API stabile di PDFium: contare le pagine,
//! estrarre il text layer (fast path) e renderizzare a 150 DPI quando il text
//! layer non c'e'. Il documento viene passato in memoria, cosi' i percorsi con
//! spazi o nomi Unicode non attraversano mai una conversione di encoding.

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_ulong, c_ushort};
use std::path::Path;
use std::sync::{Mutex, OnceLock};

type FpdfDocument = *mut c_void;
type FpdfPage = *mut c_void;
type FpdfTextPage = *mut c_void;
type FpdfBitmap = *mut c_void;

struct Api {
    _lib: Library,
    init: unsafe extern "C" fn(),
    ultimo_errore: unsafe extern "C" fn() -> c_ulong,
    carica_memoria: unsafe extern "C" fn(*const c_void, c_int, *const c_char) -> FpdfDocument,
    chiudi_documento: unsafe extern "C" fn(FpdfDocument),
    conta_pagine: unsafe extern "C" fn(FpdfDocument) -> c_int,
    carica_pagina: unsafe extern "C" fn(FpdfDocument, c_int) -> FpdfPage,
    chiudi_pagina: unsafe extern "C" fn(FpdfPage),
    larghezza: unsafe extern "C" fn(FpdfPage) -> f32,
    altezza: unsafe extern "C" fn(FpdfPage) -> f32,
    bitmap_crea: unsafe extern "C" fn(c_int, c_int, c_int) -> FpdfBitmap,
    bitmap_riempi: unsafe extern "C" fn(FpdfBitmap, c_int, c_int, c_int, c_int, c_ulong) -> c_int,
    bitmap_buffer: unsafe extern "C" fn(FpdfBitmap) -> *mut c_void,
    bitmap_stride: unsafe extern "C" fn(FpdfBitmap) -> c_int,
    bitmap_distruggi: unsafe extern "C" fn(FpdfBitmap),
    renderizza: unsafe extern "C" fn(FpdfBitmap, FpdfPage, c_int, c_int, c_int, c_int, c_int, c_int),
    testo_carica: unsafe extern "C" fn(FpdfPage) -> FpdfTextPage,
    testo_chiudi: unsafe extern "C" fn(FpdfTextPage),
    testo_conta: unsafe extern "C" fn(FpdfTextPage) -> c_int,
    testo_leggi: unsafe extern "C" fn(FpdfTextPage, c_int, c_int, *mut c_ushort) -> c_int,
    testo_rettangoli: unsafe extern "C" fn(FpdfTextPage, c_int, c_int) -> c_int,
}

// PDFium non e' thread safe: tutte le chiamate passano da un solo mutex.
static PDFIUM: OnceLock<Mutex<Option<Api>>> = OnceLock::new();

macro_rules! simbolo {
    ($lib:expr, $nome:literal) => {{
        let s: Symbol<_> = unsafe { $lib.get(concat!($nome, "\0").as_bytes()) }
            .map_err(|e| format!("simbolo {} assente in libpdfium: {e}", $nome))?;
        *s
    }};
}

fn carica(percorso: &Path) -> Result<Api, String> {
    let lib = unsafe { Library::new(percorso) }
        .map_err(|e| format!("libpdfium non caricabile ({}): {e}", percorso.display()))?;
    let api = Api {
        init: simbolo!(lib, "FPDF_InitLibrary"),
        ultimo_errore: simbolo!(lib, "FPDF_GetLastError"),
        carica_memoria: simbolo!(lib, "FPDF_LoadMemDocument"),
        chiudi_documento: simbolo!(lib, "FPDF_CloseDocument"),
        conta_pagine: simbolo!(lib, "FPDF_GetPageCount"),
        carica_pagina: simbolo!(lib, "FPDF_LoadPage"),
        chiudi_pagina: simbolo!(lib, "FPDF_ClosePage"),
        larghezza: simbolo!(lib, "FPDF_GetPageWidthF"),
        altezza: simbolo!(lib, "FPDF_GetPageHeightF"),
        bitmap_crea: simbolo!(lib, "FPDFBitmap_Create"),
        bitmap_riempi: simbolo!(lib, "FPDFBitmap_FillRect"),
        bitmap_buffer: simbolo!(lib, "FPDFBitmap_GetBuffer"),
        bitmap_stride: simbolo!(lib, "FPDFBitmap_GetStride"),
        bitmap_distruggi: simbolo!(lib, "FPDFBitmap_Destroy"),
        renderizza: simbolo!(lib, "FPDF_RenderPageBitmap"),
        testo_carica: simbolo!(lib, "FPDFText_LoadPage"),
        testo_chiudi: simbolo!(lib, "FPDFText_ClosePage"),
        testo_conta: simbolo!(lib, "FPDFText_CountChars"),
        testo_leggi: simbolo!(lib, "FPDFText_GetText"),
        testo_rettangoli: simbolo!(lib, "FPDFText_CountRects"),
        _lib: lib,
    };
    unsafe { (api.init)() };
    Ok(api)
}

/// Carica la libreria una volta sola. Va chiamata prima di ogni uso.
pub fn inizializza(percorso: &Path) -> Result<(), String> {
    let cella = PDFIUM.get_or_init(|| Mutex::new(None));
    let mut guardia = cella.lock().map_err(|_| "PDFium in stato inconsistente")?;
    if guardia.is_none() {
        *guardia = Some(carica(percorso)?);
    }
    Ok(())
}

pub struct Pagina {
    pub numero: usize,
    /// Testo del layer nativo, gia' normalizzato.
    pub testo: String,
    /// Quanti caratteri stampabili ha il layer nativo.
    pub caratteri_utili: usize,
}

fn con_api<T>(f: impl FnOnce(&Api) -> Result<T, String>) -> Result<T, String> {
    let cella = PDFIUM.get().ok_or("PDFium non inizializzata")?;
    let guardia = cella.lock().map_err(|_| "PDFium in stato inconsistente")?;
    let api = guardia.as_ref().ok_or("PDFium non inizializzata")?;
    f(api)
}

struct Documento<'a> {
    api: &'a Api,
    handle: FpdfDocument,
}

impl Drop for Documento<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.chiudi_documento)(self.handle) };
    }
}

fn apri<'a>(api: &'a Api, dati: &[u8]) -> Result<Documento<'a>, String> {
    let handle = unsafe {
        (api.carica_memoria)(
            dati.as_ptr() as *const c_void,
            dati.len() as c_int,
            std::ptr::null(),
        )
    };
    if handle.is_null() {
        let codice = unsafe { (api.ultimo_errore)() };
        return Err(match codice {
            4 => "il PDF e' protetto da password".to_string(),
            3 => "il PDF e' cifrato e non apribile".to_string(),
            2 => "il file non e' un PDF valido".to_string(),
            altro => format!("PDF non apribile (codice PDFium {altro})"),
        });
    }
    Ok(Documento { api, handle })
}

fn testo_pagina(api: &Api, pagina: FpdfPage) -> String {
    let tp = unsafe { (api.testo_carica)(pagina) };
    if tp.is_null() {
        return String::new();
    }
    let n = unsafe { (api.testo_conta)(tp) };
    if n <= 0 {
        unsafe { (api.testo_chiudi)(tp) };
        return String::new();
    }
    // FPDFText_GetText vuole spazio anche per il terminatore.
    let mut buffer = vec![0u16; n as usize + 1];
    let letti = unsafe { (api.testo_leggi)(tp, 0, n, buffer.as_mut_ptr()) };
    unsafe { (api.testo_chiudi)(tp) };
    if letti <= 0 {
        return String::new();
    }
    let fine = (letti as usize).saturating_sub(1).min(buffer.len());
    String::from_utf16_lossy(&buffer[..fine])
        .replace('\u{0}', "")
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

/// Quante regioni di testo ha la pagina: distingue un layer vero da due
/// caratteri di intestazione appiccicati a una scansione.
fn rettangoli_testo(api: &Api, pagina: FpdfPage) -> i32 {
    let tp = unsafe { (api.testo_carica)(pagina) };
    if tp.is_null() {
        return 0;
    }
    // -1 = tutti i caratteri della pagina.
    let n = unsafe { (api.testo_rettangoli)(tp, 0, -1) };
    unsafe { (api.testo_chiudi)(tp) };
    n.max(0)
}

/// Apre il PDF e riporta, per ogni pagina, il testo nativo e la misura.
pub fn analizza(dati: &[u8]) -> Result<Vec<Pagina>, String> {
    con_api(|api| {
        let doc = apri(api, dati)?;
        let n = unsafe { (api.conta_pagine)(doc.handle) };
        if n <= 0 {
            return Err("il PDF non contiene pagine".into());
        }
        let mut fuori = Vec::with_capacity(n as usize);
        for indice in 0..n {
            let pagina = unsafe { (api.carica_pagina)(doc.handle, indice) };
            if pagina.is_null() {
                return Err(format!("pagina {} non apribile", indice + 1));
            }
            let testo = testo_pagina(api, pagina);
            let rettangoli = rettangoli_testo(api, pagina);
            unsafe { (api.chiudi_pagina)(pagina) };
            let utili = testo.chars().filter(|c| !c.is_whitespace()).count();
            fuori.push(Pagina {
                numero: indice as usize + 1,
                caratteri_utili: if rettangoli > 0 { utili } else { 0 },
                testo,
            });
        }
        Ok(fuori)
    })
}

/// Renderizza una pagina a `dpi` e restituisce (larghezza, altezza, RGB8).
pub fn renderizza(dati: &[u8], indice: usize, dpi: f32) -> Result<(u32, u32, Vec<u8>), String> {
    con_api(|api| {
        let doc = apri(api, dati)?;
        let pagina = unsafe { (api.carica_pagina)(doc.handle, indice as c_int) };
        if pagina.is_null() {
            return Err(format!("pagina {} non apribile", indice + 1));
        }
        let scala = dpi / 72.0;
        let larghezza = ((unsafe { (api.larghezza)(pagina) } * scala).round() as i32).max(1);
        let altezza = ((unsafe { (api.altezza)(pagina) } * scala).round() as i32).max(1);
        // Un limite prudente: oltre questa misura il canvas 960x1248 non guadagna
        // niente e la memoria esplode su PDF con pagine enormi.
        if larghezza > 20000 || altezza > 20000 {
            unsafe { (api.chiudi_pagina)(pagina) };
            return Err("pagina troppo grande da renderizzare".into());
        }
        // alpha = 0: PDFium alloca un buffer BGRx a 4 byte per pixel.
        let bitmap = unsafe { (api.bitmap_crea)(larghezza, altezza, 0) };
        if bitmap.is_null() {
            unsafe { (api.chiudi_pagina)(pagina) };
            return Err("bitmap PDFium non allocabile".into());
        }
        unsafe {
            (api.bitmap_riempi)(bitmap, 0, 0, larghezza, altezza, 0xFFFF_FFFF);
            // flag 0: niente annotazioni, come il rendering della pipeline.
            (api.renderizza)(bitmap, pagina, 0, 0, larghezza, altezza, 0, 0);
        }
        let stride = unsafe { (api.bitmap_stride)(bitmap) } as usize;
        let buffer = unsafe { (api.bitmap_buffer)(bitmap) } as *const u8;
        let mut rgb = vec![0u8; larghezza as usize * altezza as usize * 3];
        for y in 0..altezza as usize {
            let riga = unsafe { std::slice::from_raw_parts(buffer.add(y * stride), stride) };
            for x in 0..larghezza as usize {
                let p = x * 4;
                let d = (y * larghezza as usize + x) * 3;
                // BGRA -> RGB
                rgb[d] = riga[p + 2];
                rgb[d + 1] = riga[p + 1];
                rgb[d + 2] = riga[p];
            }
        }
        unsafe {
            (api.bitmap_distruggi)(bitmap);
            (api.chiudi_pagina)(pagina);
        }
        Ok((larghezza as u32, altezza as u32, rgb))
    })
}
