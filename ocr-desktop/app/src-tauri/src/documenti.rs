//! Riconoscimento del tipo di documento, fast path sul text layer, rendering.
//!
//! Un PDF con un text layer valido non ha bisogno del VLM: il testo si legge
//! direttamente e la pagina costa millisecondi invece di secondi. Le pagine
//! senza layer vengono renderizzate a 150 DPI e normalizzate al canvas
//! 960x1248, cioe' esattamente il preprocessing con cui il modello e' stato
//! addestrato e valutato.

use crate::imaging;
use crate::pdfium;
use std::path::Path;

/// Sotto questa soglia di caratteri stampabili il "testo" di una pagina e'
/// rumore da intestazione o da scansione, non un layer utilizzabile.
pub const SOGLIA_TEXT_LAYER: usize = 24;

pub const ESTENSIONI_IMMAGINE: [&str; 10] = [
    "png", "jpg", "jpeg", "tif", "tiff", "bmp", "webp", "gif", "jpe", "jfif",
];

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Tipo {
    Pdf,
    Immagine,
}

pub struct PaginaSorgente {
    pub numero: usize,
    /// Testo del layer nativo quando la pagina ne ha uno valido.
    pub testo_nativo: Option<String>,
}

pub struct Documento {
    pub nome: String,
    /// Da dove e' stato aperto: serve all'archivio per ritrovare il sorgente e
    /// rigenerare le miniature senza doverle duplicare su disco.
    pub percorso: String,
    pub tipo: Tipo,
    /// Byte del PDF, tenuti in memoria: PDFium lavora sul buffer e i percorsi
    /// con spazi o nomi Unicode non attraversano nessuna conversione.
    pub dati: Vec<u8>,
    pub pagine: Vec<PaginaSorgente>,
}

fn estensione(percorso: &Path) -> String {
    percorso
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

pub fn e_supportato(percorso: &Path) -> bool {
    let ext = estensione(percorso);
    ext == "pdf" || ESTENSIONI_IMMAGINE.contains(&ext.as_str())
}

/// Riconosce il formato dal contenuto, non solo dall'estensione.
fn e_pdf(dati: &[u8]) -> bool {
    dati.len() > 5 && &dati[..5] == b"%PDF-"
}

pub fn apri(percorso: &Path, forza_ocr: bool) -> Result<Documento, String> {
    let dati = std::fs::read(percorso).map_err(|e| {
        format!(
            "impossibile leggere {}: {e}",
            percorso.file_name().unwrap_or_default().to_string_lossy()
        )
    })?;
    if dati.is_empty() {
        return Err("il file è vuoto".into());
    }
    let nome = percorso
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| percorso.display().to_string());

    if e_pdf(&dati) {
        let pagine_pdf = pdfium::analizza(&dati)?;
        let pagine = pagine_pdf
            .into_iter()
            .map(|p| PaginaSorgente {
                numero: p.numero,
                testo_nativo: if !forza_ocr && p.caratteri_utili >= SOGLIA_TEXT_LAYER {
                    Some(normalizza_testo(&p.testo))
                } else {
                    None
                },
            })
            .collect();
        return Ok(Documento {
            nome,
            percorso: percorso.to_string_lossy().into_owned(),
            tipo: Tipo::Pdf,
            dati,
            pagine,
        });
    }

    if !ESTENSIONI_IMMAGINE.contains(&estensione(percorso).as_str()) {
        return Err("formato non supportato: servono PDF, PNG, JPEG, TIFF, BMP o WebP".into());
    }
    // Si decodifica subito per dare l'errore adesso e non a meta' elaborazione.
    image::load_from_memory(&dati).map_err(|e| format!("immagine non leggibile: {e}"))?;
    Ok(Documento {
        nome,
        percorso: percorso.to_string_lossy().into_owned(),
        tipo: Tipo::Immagine,
        dati,
        pagine: vec![PaginaSorgente {
            numero: 1,
            testo_nativo: None,
        }],
    })
}

/// Il text layer di PDFium arriva con separatori incoerenti: si normalizzano
/// gli a capo e si tolgono le righe vuote in eccesso, senza toccare il resto.
fn normalizza_testo(grezzo: &str) -> String {
    let mut righe: Vec<String> = grezzo
        .replace('\u{feff}', "")
        .lines()
        .map(|r| r.trim_end().to_string())
        .collect();
    while righe.first().is_some_and(|r| r.trim().is_empty()) {
        righe.remove(0);
    }
    while righe.last().is_some_and(|r| r.trim().is_empty()) {
        righe.pop();
    }
    let mut fuori: Vec<String> = Vec::with_capacity(righe.len());
    for riga in righe {
        if riga.trim().is_empty() && fuori.last().is_some_and(|r| r.trim().is_empty()) {
            continue;
        }
        fuori.push(riga);
    }
    fuori.join("\n")
}

/// L'immagine della pagina pronta per il VLM: 150 DPI, canvas 960x1248, PNG.
pub fn immagine_per_ocr(documento: &Documento, indice: usize) -> Result<String, String> {
    let normalizzata = match documento.tipo {
        Tipo::Pdf => {
            let (larghezza, altezza, rgb) =
                pdfium::renderizza(&documento.dati, indice, imaging::DPI_RENDER)?;
            let buffer = image::RgbImage::from_raw(larghezza, altezza, rgb)
                .ok_or("bitmap della pagina incoerente")?;
            imaging::canvas_pagina(&image::DynamicImage::ImageRgb8(buffer))
        }
        Tipo::Immagine => {
            let immagine = image::load_from_memory(&documento.dati)
                .map_err(|e| format!("immagine non leggibile: {e}"))?;
            imaging::canvas_pagina(&immagine)
        }
    };
    Ok(imaging::base64(&imaging::png(&normalizzata)?))
}

/// Anteprima per la UI, in data URL. Non partecipa all'inferenza.
pub fn anteprima(documento: &Documento, indice: usize) -> Result<String, String> {
    let png = match documento.tipo {
        Tipo::Pdf => {
            let (larghezza, altezza, rgb) = pdfium::renderizza(&documento.dati, indice, 110.0)?;
            let buffer = image::RgbImage::from_raw(larghezza, altezza, rgb)
                .ok_or("bitmap della pagina incoerente")?;
            imaging::anteprima(&image::DynamicImage::ImageRgb8(buffer), 1400)?
        }
        Tipo::Immagine => {
            let immagine = image::load_from_memory(&documento.dati)
                .map_err(|e| format!("immagine non leggibile: {e}"))?;
            imaging::anteprima(&immagine, 1400)?
        }
    };
    Ok(format!("data:image/png;base64,{}", imaging::base64(&png)))
}
