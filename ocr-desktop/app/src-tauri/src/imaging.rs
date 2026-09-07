//! Normalizzazione delle pagine al canvas del protocollo e codifica PNG.
//!
//! Porta esatta di `canvas_pagina` in `train_glm_pagine_reali.py`: stessa
//! scala isotropa, stesso filtro Lanczos, stesso sfondo bianco, stesso
//! centraggio. Il canvas **non** si tocca: 840x1092 e 784x1008 risparmiano il
//! 5-7% degradando CER e WER, quindi restano 960x1248.

use image::imageops::FilterType;
use image::{DynamicImage, ImageEncoder, RgbImage};

pub const CANVAS: (u32, u32) = (960, 1248);
pub const DPI_RENDER: f32 = 150.0;

/// Adatta l'immagine al canvas senza deformarla, su fondo bianco.
pub fn canvas_pagina(immagine: &DynamicImage) -> RgbImage {
    let (larghezza, altezza) = CANVAS;
    let sorgente = immagine.to_rgb8();
    let scala =
        (larghezza as f32 / sorgente.width() as f32).min(altezza as f32 / sorgente.height() as f32);
    let nuova_larghezza = ((sorgente.width() as f32 * scala).round() as u32).max(1);
    let nuova_altezza = ((sorgente.height() as f32 * scala).round() as u32).max(1);
    let ridotta = image::imageops::resize(
        &sorgente,
        nuova_larghezza,
        nuova_altezza,
        FilterType::Lanczos3,
    );
    let mut fuori = RgbImage::from_pixel(larghezza, altezza, image::Rgb([255, 255, 255]));
    let x = (larghezza - nuova_larghezza) / 2;
    let y = (altezza - nuova_altezza) / 2;
    image::imageops::replace(&mut fuori, &ridotta, x as i64, y as i64);
    fuori
}

pub fn png(immagine: &RgbImage) -> Result<Vec<u8>, String> {
    let mut fuori = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut fuori,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::Adaptive,
    )
    .write_image(
        immagine.as_raw(),
        immagine.width(),
        immagine.height(),
        image::ExtendedColorType::Rgb8,
    )
    .map_err(|e| format!("codifica PNG fallita: {e}"))?;
    Ok(fuori)
}

/// Anteprima per la UI: lato lungo limitato, JPEG-free per non perdere tratto.
pub fn anteprima(immagine: &DynamicImage, lato_massimo: u32) -> Result<Vec<u8>, String> {
    let ridotta = if immagine.width().max(immagine.height()) > lato_massimo {
        immagine.resize(lato_massimo, lato_massimo, FilterType::Triangle)
    } else {
        immagine.clone()
    };
    png(&ridotta.to_rgb8())
}

const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 standard con padding: e' quello che si aspetta `multimodal_data`.
pub fn base64(dati: &[u8]) -> String {
    let mut fuori = String::with_capacity(dati.len().div_ceil(3) * 4);
    for blocco in dati.chunks(3) {
        let b0 = blocco[0] as u32;
        let b1 = *blocco.get(1).unwrap_or(&0) as u32;
        let b2 = *blocco.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        fuori.push(ALFABETO[(n >> 18) as usize & 63] as char);
        fuori.push(ALFABETO[(n >> 12) as usize & 63] as char);
        fuori.push(if blocco.len() > 1 {
            ALFABETO[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        fuori.push(if blocco.len() > 2 {
            ALFABETO[n as usize & 63] as char
        } else {
            '='
        });
    }
    fuori
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn base64_corrisponde_allo_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64(&[0xff, 0xfe, 0xfd]), "//79");
    }

    #[test]
    fn il_canvas_centra_senza_deformare() {
        let sorgente =
            DynamicImage::ImageRgb8(RgbImage::from_pixel(100, 50, image::Rgb([0, 0, 0])));
        let fuori = canvas_pagina(&sorgente);
        assert_eq!((fuori.width(), fuori.height()), CANVAS);
        // 100x50 su 960x1248 scala per larghezza: 960x480, centrato in verticale.
        assert_eq!(fuori.get_pixel(480, 624), &image::Rgb([0, 0, 0]));
        assert_eq!(fuori.get_pixel(480, 10), &image::Rgb([255, 255, 255]));
    }
}
