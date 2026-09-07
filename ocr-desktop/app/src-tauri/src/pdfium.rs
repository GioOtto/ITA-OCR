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

const LATO_MASSIMO: i32 = 20_000;
const PIXEL_MASSIMI: u64 = 50_000_000;
const CARATTERI_MASSIMI: usize = 10_000_000;
const PAGINE_MASSIME: usize = 100_000;

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
    renderizza:
        unsafe extern "C" fn(FpdfBitmap, FpdfPage, c_int, c_int, c_int, c_int, c_int, c_int),
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

struct PaginaAperta<'a> {
    api: &'a Api,
    handle: FpdfPage,
}

impl Drop for PaginaAperta<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.chiudi_pagina)(self.handle) };
    }
}

struct Bitmap<'a> {
    api: &'a Api,
    handle: FpdfBitmap,
}

impl Drop for Bitmap<'_> {
    fn drop(&mut self) {
        unsafe { (self.api.bitmap_distruggi)(self.handle) };
    }
}

fn apri<'a>(api: &'a Api, dati: &[u8]) -> Result<Documento<'a>, String> {
    let lunghezza: c_int = dati
        .len()
        .try_into()
        .map_err(|_| "PDF troppo grande da aprire")?;
    let handle = unsafe {
        (api.carica_memoria)(dati.as_ptr() as *const c_void, lunghezza, std::ptr::null())
    };
    if handle.is_null() {
        let codice = unsafe { (api.ultimo_errore)() };
        return Err(match codice {
            4 => "il PDF è protetto da password".to_string(),
            3 => "il PDF è cifrato e non apribile".to_string(),
            2 => "il file non è un PDF valido".to_string(),
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
    let lunghezza = n as usize;
    if lunghezza > CARATTERI_MASSIMI {
        unsafe { (api.testo_chiudi)(tp) };
        return String::new();
    }
    let mut buffer = Vec::new();
    if buffer.try_reserve_exact(lunghezza + 1).is_err() {
        unsafe { (api.testo_chiudi)(tp) };
        return String::new();
    }
    buffer.resize(lunghezza + 1, 0u16);
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
        let numero_pagine = n as usize;
        if numero_pagine > PAGINE_MASSIME {
            return Err("il PDF contiene troppe pagine".into());
        }
        let mut fuori = Vec::new();
        fuori
            .try_reserve_exact(numero_pagine)
            .map_err(|_| "memoria insufficiente per analizzare il PDF")?;
        for indice in 0..n {
            let handle = unsafe { (api.carica_pagina)(doc.handle, indice) };
            if handle.is_null() {
                return Err(format!("pagina {} non apribile", indice + 1));
            }
            let pagina = PaginaAperta { api, handle };
            let testo = testo_pagina(api, pagina.handle);
            let rettangoli = rettangoli_testo(api, pagina.handle);
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

fn dimensioni_render(punti_x: f32, punti_y: f32, dpi: f32) -> Result<(i32, i32, usize), String> {
    if !dpi.is_finite() || dpi <= 0.0 || !punti_x.is_finite() || !punti_y.is_finite() {
        return Err("dimensioni pagina non valide".into());
    }
    let scala = dpi as f64 / 72.0;
    let x = punti_x as f64 * scala;
    let y = punti_y as f64 * scala;
    if x <= 0.0 || y <= 0.0 || x > c_int::MAX as f64 || y > c_int::MAX as f64 {
        return Err("dimensioni pagina non valide".into());
    }
    let larghezza = (x.round() as i32).max(1);
    let altezza = (y.round() as i32).max(1);
    let pixel = (larghezza as u64)
        .checked_mul(altezza as u64)
        .ok_or("dimensioni pagina non valide")?;
    if larghezza > LATO_MASSIMO || altezza > LATO_MASSIMO || pixel > PIXEL_MASSIMI {
        return Err("pagina troppo grande da renderizzare".into());
    }
    let byte_rgb = usize::try_from(pixel)
        .ok()
        .and_then(|n| n.checked_mul(3))
        .ok_or("dimensioni pagina non valide")?;
    Ok((larghezza, altezza, byte_rgb))
}

fn valida_bitmap(
    buffer: *const u8,
    stride: c_int,
    larghezza: usize,
    altezza: usize,
) -> Result<usize, String> {
    if buffer.is_null() || stride <= 0 {
        return Err("bitmap PDFium non valida".into());
    }
    let stride = stride as usize;
    let byte_riga = larghezza.checked_mul(4).ok_or("bitmap PDFium non valida")?;
    if stride < byte_riga || stride.checked_mul(altezza).is_none() {
        return Err("bitmap PDFium non valida".into());
    }
    Ok(stride)
}

/// Renderizza una pagina a `dpi` e restituisce (larghezza, altezza, RGB8).
pub fn renderizza(dati: &[u8], indice: usize, dpi: f32) -> Result<(u32, u32, Vec<u8>), String> {
    con_api(|api| {
        let doc = apri(api, dati)?;
        let indice_pdfium: c_int = indice.try_into().map_err(|_| "indice pagina non valido")?;
        let handle = unsafe { (api.carica_pagina)(doc.handle, indice_pdfium) };
        if handle.is_null() {
            return Err(format!("pagina {} non apribile", indice + 1));
        }
        let pagina = PaginaAperta { api, handle };
        let (larghezza, altezza, byte_rgb) = dimensioni_render(
            unsafe { (api.larghezza)(pagina.handle) },
            unsafe { (api.altezza)(pagina.handle) },
            dpi,
        )?;
        // alpha = 0: PDFium alloca un buffer BGRx a 4 byte per pixel.
        let handle = unsafe { (api.bitmap_crea)(larghezza, altezza, 0) };
        if handle.is_null() {
            return Err("bitmap PDFium non allocabile".into());
        }
        let bitmap = Bitmap { api, handle };
        let riempita =
            unsafe { (api.bitmap_riempi)(bitmap.handle, 0, 0, larghezza, altezza, 0xFFFF_FFFF) };
        if riempita == 0 {
            return Err("bitmap PDFium non inizializzabile".into());
        }
        unsafe {
            // flag 0: niente annotazioni, come il rendering della pipeline.
            (api.renderizza)(bitmap.handle, pagina.handle, 0, 0, larghezza, altezza, 0, 0);
        }
        let buffer = unsafe { (api.bitmap_buffer)(bitmap.handle) } as *const u8;
        let stride = valida_bitmap(
            buffer,
            unsafe { (api.bitmap_stride)(bitmap.handle) },
            larghezza as usize,
            altezza as usize,
        )?;
        let byte_riga = larghezza as usize * 4;
        let mut rgb = Vec::new();
        rgb.try_reserve_exact(byte_rgb)
            .map_err(|_| "memoria insufficiente per renderizzare il PDF")?;
        rgb.resize(byte_rgb, 0u8);
        for y in 0..altezza as usize {
            let riga = unsafe { std::slice::from_raw_parts(buffer.add(y * stride), byte_riga) };
            for x in 0..larghezza as usize {
                let p = x * 4;
                let d = (y * larghezza as usize + x) * 3;
                // BGRA -> RGB
                rgb[d] = riga[p + 2];
                rgb[d + 1] = riga[p + 1];
                rgb[d + 2] = riga[p];
            }
        }
        Ok((larghezza as u32, altezza as u32, rgb))
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn limita_anche_il_numero_totale_di_pixel() {
        assert!(dimensioni_render(612.0, 792.0, 150.0).is_ok());
        let errore = dimensioni_render(14_400.0, 14_400.0, 72.0).unwrap_err();
        assert_eq!(errore, "pagina troppo grande da renderizzare");
        assert!(dimensioni_render(f32::NAN, 792.0, 150.0).is_err());
        assert!(dimensioni_render(612.0, 792.0, 0.0).is_err());
    }

    #[test]
    fn rifiuta_buffer_null_e_stride_corti() {
        assert!(valida_bitmap(std::ptr::null(), 400, 100, 100).is_err());
        let buffer = std::ptr::NonNull::<u8>::dangling().as_ptr();
        assert!(valida_bitmap(buffer, 0, 100, 100).is_err());
        assert!(valida_bitmap(buffer, 399, 100, 100).is_err());
        assert_eq!(valida_bitmap(buffer, 400, 100, 100).unwrap(), 400);
    }
}
