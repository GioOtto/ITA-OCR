//! Lettura dei soli metadati che servono dall'intestazione di un GGUF.
//!
//! Porta di `_eos_dichiarato` in `diagnose_glm_checkpoints_vulkan.py`: GLM-OCR
//! dichiara **due** stop ufficiali (`eos_token_id = [59246, 59253]`), ma il GGUF
//! ne registra uno solo come `eos`. Senza rimettere in gioco l'altro come `eot`
//! il modello non si ferma dove si fermava nelle valutazioni.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

pub const STOP_UFFICIALI: [i64; 2] = [59246, 59253];

fn leggi<const N: usize>(f: &mut impl Read) -> Result<[u8; N], String> {
    let mut buf = [0u8; N];
    f.read_exact(&mut buf)
        .map_err(|e| format!("GGUF troncato: {e}"))?;
    Ok(buf)
}

fn u32le(f: &mut impl Read) -> Result<u32, String> {
    Ok(u32::from_le_bytes(leggi::<4>(f)?))
}

fn u64le(f: &mut impl Read) -> Result<u64, String> {
    Ok(u64::from_le_bytes(leggi::<8>(f)?))
}

fn stringa(f: &mut impl Read) -> Result<String, String> {
    let n = u64le(f)? as usize;
    if n > 64 * 1024 * 1024 {
        return Err("stringa GGUF assurda".into());
    }
    let mut buf = vec![0u8; n];
    f.read_exact(&mut buf)
        .map_err(|e| format!("GGUF troncato: {e}"))?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

/// Legge un valore e lo restituisce come intero quando ha senso.
fn valore(f: &mut impl Read, tipo: u32) -> Result<Option<i64>, String> {
    Ok(match tipo {
        0 => Some(i64::from(leggi::<1>(f)?[0])),
        1 => Some(i64::from(leggi::<1>(f)?[0] as i8)),
        2 => Some(i64::from(u16::from_le_bytes(leggi::<2>(f)?))),
        3 => Some(i64::from(i16::from_le_bytes(leggi::<2>(f)?))),
        4 => Some(i64::from(u32le(f)?)),
        5 => Some(i64::from(i32::from_le_bytes(leggi::<4>(f)?))),
        6 => {
            leggi::<4>(f)?;
            None
        }
        7 => Some(i64::from(leggi::<1>(f)?[0])),
        8 => {
            stringa(f)?;
            None
        }
        9 => {
            let elemento = u32le(f)?;
            let n = u64le(f)?;
            for _ in 0..n {
                valore(f, elemento)?;
            }
            None
        }
        10 => Some(u64le(f)? as i64),
        11 => Some(i64::from_le_bytes(leggi::<8>(f)?)),
        12 => {
            leggi::<8>(f)?;
            None
        }
        altro => return Err(format!("tipo GGUF sconosciuto: {altro}")),
    })
}

/// `tokenizer.ggml.eos_token_id` dichiarato dal file, se presente.
pub fn eos_dichiarato(percorso: &Path) -> Result<Option<i64>, String> {
    let file = File::open(percorso).map_err(|e| format!("{}: {e}", percorso.display()))?;
    let mut f = BufReader::with_capacity(1 << 20, file);
    if &leggi::<4>(&mut f)? != b"GGUF" {
        return Err(format!("{} non è un GGUF", percorso.display()));
    }
    u32le(&mut f)?; // versione
    u64le(&mut f)?; // numero di tensori
    let nkv = u64le(&mut f)?;
    for _ in 0..nkv {
        let chiave = stringa(&mut f)?;
        let tipo = u32le(&mut f)?;
        let letto = valore(&mut f, tipo)?;
        if chiave == "tokenizer.ggml.eos_token_id" {
            return Ok(letto);
        }
    }
    Ok(None)
}

/// L'override da passare a llama-server perche' **entrambi** gli stop ufficiali
/// fermino la generazione. Semantica identica a `ServerVulkan.__enter__`.
pub fn override_eot(percorso: &Path) -> Option<String> {
    let eos = eos_dichiarato(percorso).ok().flatten()?;
    if !STOP_UFFICIALI.contains(&eos) {
        return None;
    }
    let altro = STOP_UFFICIALI.iter().find(|x| **x != eos)?;
    Some(format!("tokenizer.ggml.eot_token_id=int:{altro}"))
}
