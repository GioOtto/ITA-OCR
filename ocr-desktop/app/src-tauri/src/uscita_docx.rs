//! Trascrizione in un documento Word vero.
//!
//! L'esportazione Markdown va bene per chi legge il testo, ma aperta in Word
//! lascia le tabelle come righe di barre verticali. Qui il Markdown prodotto da
//! `lavoro::esporta` viene riletto e ricostruito con gli oggetti di Word:
//! le tabelle diventano tabelle, i titoli diventano titoli, e il grassetto
//! resta grassetto invece di restare scritto con gli asterischi.
//!
//! Le formule LaTeX restano come sono, in carattere a spaziatura fissa: Word
//! usa OMML, e convertire LaTeX in OMML sarebbe un lavoro a parte.

use docx_rs::*;
use std::path::Path;

/// Larghezza utile di una pagina A4 con margini da un pollice, in twip.
const LARGHEZZA: usize = 9026;

pub fn scrivi(markdown: &str, percorso: &Path) -> Result<(), String> {
    let file = std::fs::File::create(percorso).map_err(|e| format!("file non creabile: {e}"))?;
    costruisci(markdown)
        .build()
        .pack(file)
        .map_err(|e| format!("documento non scrivibile: {e}"))?;
    Ok(())
}

/// Il documento in memoria: separato dalla scrittura per poterlo esaminare
/// nelle prove senza passare dal disco.
fn costruisci(markdown: &str) -> Docx {
    let righe: Vec<&str> = markdown.lines().collect();
    let mut documento = Docx::new();
    let mut i = 0;
    while i < righe.len() {
        let riga = righe[i];
        if riga.trim().is_empty() {
            i += 1;
            continue;
        }
        if e_tabella(riga) {
            if let Some((tabella, quante)) = leggi_tabella(&righe[i..]) {
                documento = documento.add_table(tabella).add_paragraph(Paragraph::new());
                i += quante;
                continue;
            }
        }
        documento = documento.add_paragraph(paragrafo(riga));
        i += 1;
    }
    documento
}

fn e_tabella(riga: &str) -> bool {
    riga.trim_start().starts_with('|')
}

/// Riga di separazione di una tabella Markdown: `| --- | :---: |`.
fn e_separatore(riga: &str) -> bool {
    let corpo = riga.trim();
    !corpo.is_empty()
        && corpo.chars().all(|c| matches!(c, '|' | '-' | ':' | ' '))
        && corpo.contains('-')
}

fn celle(riga: &str) -> Vec<String> {
    // Le barre protette (`\|`) appartengono al testo, non separano le celle.
    let protetta = riga.trim().replace("\\|", "\u{0}");
    let protetta = protetta.trim_start_matches('|').trim_end_matches('|');
    protetta
        .split('|')
        .map(|c| c.replace('\u{0}', "|").trim().to_string())
        .collect()
}

/// Legge il blocco di tabella che comincia alla prima riga, se e' una tabella
/// vera: intestazione, separatore e almeno una riga di dati.
fn leggi_tabella(blocco: &[&str]) -> Option<(Table, usize)> {
    if blocco.len() < 3 || !e_separatore(blocco[1]) {
        return None;
    }
    let intestazione = celle(blocco[0]);
    let colonne = intestazione.len();
    if colonne == 0 {
        return None;
    }
    let mut righe = vec![riga_tabella(&intestazione, colonne, true)];
    let mut quante = 2;
    for riga in &blocco[2..] {
        if !e_tabella(riga) {
            break;
        }
        righe.push(riga_tabella(&celle(riga), colonne, false));
        quante += 1;
    }
    let larghezza_colonna = LARGHEZZA / colonne;
    let tabella = Table::new(righe)
        .set_grid(vec![larghezza_colonna; colonne])
        .width(LARGHEZZA, WidthType::Dxa);
    Some((tabella, quante))
}

/// Una riga con esattamente `colonne` celle: le tabelle storte dell'OCR non
/// devono far uscire un documento che Word considera rotto.
fn riga_tabella(contenuto: &[String], colonne: usize, intestazione: bool) -> TableRow {
    let mut celle_riga = Vec::with_capacity(colonne);
    for indice in 0..colonne {
        let testo = contenuto.get(indice).map(String::as_str).unwrap_or("");
        let mut paragrafo = Paragraph::new();
        for pezzo in pezzi(testo) {
            paragrafo = paragrafo.add_run(pezzo.corsa(intestazione));
        }
        celle_riga.push(
            TableCell::new()
                .width(LARGHEZZA / colonne, WidthType::Dxa)
                .add_paragraph(paragrafo),
        );
    }
    TableRow::new(celle_riga)
}

fn paragrafo(riga: &str) -> Paragraph {
    let pulita = riga.trim_end();
    if let Some(testo) = pulita.strip_prefix("### ") {
        return titolo(testo, 24);
    }
    if let Some(testo) = pulita.strip_prefix("## ") {
        return titolo(testo, 28);
    }
    if let Some(testo) = pulita.strip_prefix("# ") {
        return titolo(testo, 34);
    }
    if let Some(testo) = pulita.strip_prefix("> ") {
        // Le note dell'esportazione (pagina incompleta, testo dal PDF).
        let mut p = Paragraph::new().indent(Some(360), None, None, None);
        for pezzo in pezzi(testo) {
            p = p.add_run(pezzo.corsa(false).italic().color("666666"));
        }
        return p;
    }
    for segno in ["- ", "* ", "\u{2022} "] {
        if let Some(testo) = pulita.strip_prefix(segno) {
            let mut p = Paragraph::new()
                .indent(Some(360), None, None, None)
                .add_run(Run::new().add_text("\u{2022}  "));
            for pezzo in pezzi(testo) {
                p = p.add_run(pezzo.corsa(false));
            }
            return p;
        }
    }
    let mut p = Paragraph::new();
    for pezzo in pezzi(pulita) {
        p = p.add_run(pezzo.corsa(false));
    }
    p
}

fn titolo(testo: &str, dimensione: usize) -> Paragraph {
    let mut p = Paragraph::new();
    for pezzo in pezzi(testo) {
        p = p.add_run(pezzo.corsa(true).size(dimensione));
    }
    p
}

/// Un tratto di testo con il suo stile inline.
struct Pezzo {
    testo: String,
    grassetto: bool,
    corsivo: bool,
    fisso: bool,
}

impl Pezzo {
    fn corsa(&self, grassetto_sempre: bool) -> Run {
        let mut corsa = Run::new().add_text(&self.testo);
        if self.grassetto || grassetto_sempre {
            corsa = corsa.bold();
        }
        if self.corsivo {
            corsa = corsa.italic();
        }
        if self.fisso {
            corsa = corsa.fonts(RunFonts::new().ascii("Consolas").hi_ansi("Consolas"));
        }
        corsa
    }
}

/// Spezza il testo su `**grassetto**`, `*corsivo*` e `` `codice` ``.
fn pezzi(testo: &str) -> Vec<Pezzo> {
    let caratteri: Vec<char> = testo.chars().collect();
    let mut fuori: Vec<Pezzo> = Vec::new();
    let mut corrente = String::new();
    let mut i = 0;
    while i < caratteri.len() {
        let (marcatore, lunghezza, grassetto, corsivo, fisso) = match caratteri[i] {
            '*' if caratteri.get(i + 1) == Some(&'*') => ("**", 2, true, false, false),
            '*' => ("*", 1, false, true, false),
            '`' => ("`", 1, false, false, true),
            _ => ("", 0, false, false, false),
        };
        if lunghezza > 0 {
            if let Some(fine) = trova(&caratteri, i + lunghezza, marcatore) {
                let dentro: String = caratteri[i + lunghezza..fine].iter().collect();
                if !dentro.is_empty() {
                    if !corrente.is_empty() {
                        fuori.push(Pezzo {
                            testo: std::mem::take(&mut corrente),
                            grassetto: false,
                            corsivo: false,
                            fisso: false,
                        });
                    }
                    fuori.push(Pezzo {
                        testo: dentro,
                        grassetto,
                        corsivo,
                        fisso,
                    });
                    i = fine + lunghezza;
                    continue;
                }
            }
        }
        corrente.push(caratteri[i]);
        i += 1;
    }
    if !corrente.is_empty() || fuori.is_empty() {
        fuori.push(Pezzo {
            testo: corrente,
            grassetto: false,
            corsivo: false,
            fisso: false,
        });
    }
    fuori
}

fn trova(caratteri: &[char], da: usize, marcatore: &str) -> Option<usize> {
    let segno: Vec<char> = marcatore.chars().collect();
    let mut i = da;
    while i + segno.len() <= caratteri.len() {
        if caratteri[i..i + segno.len()] == segno[..] {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod prove {
    use super::*;

    #[test]
    fn riconosce_una_tabella() {
        let blocco = ["| x | f(x) |", "| --- | --- |", "| -2 | 1 |", "", "altro"];
        let (_, quante) = leggi_tabella(&blocco).expect("tabella");
        assert_eq!(quante, 3);
    }

    #[test]
    fn le_barre_protette_restano_testo() {
        assert_eq!(celle(r"| a \| b | c |"), vec!["a | b", "c"]);
    }

    /// La prova che conta: nel documento ci deve essere una tabella di Word,
    /// non una riga di testo con le barre verticali. Il file resta in
    /// `target/prova-tabella.docx` per poterlo aprire davvero.
    #[test]
    fn la_tabella_diventa_una_tabella_di_word() {
        let markdown = "# Prova

## Pagina 1

Testo **in grassetto**.

                        | x | f(x) |
| --- | --- |
| -2 | 1 |
| -1 | 0 |

Fine.
";
        let xml = String::from_utf8(costruisci(markdown).build().document).expect("xml");
        assert!(xml.contains("<w:tbl>"), "manca la tabella: {xml}");
        assert_eq!(xml.matches("<w:tr>").count(), 3, "servono tre righe");
        assert!(!xml.contains("| x |"), "la tabella e' rimasta testo");
        assert!(xml.contains("<w:b "), "manca il grassetto");
        let percorso = std::path::Path::new("target/prova-tabella.docx");
        scrivi(markdown, percorso).expect("scrittura");
        assert!(percorso.exists());
    }

    #[test]
    fn separa_il_grassetto() {
        let pezzi = pezzi("prima **dopo** fine");
        assert_eq!(pezzi.len(), 3);
        assert!(pezzi[1].grassetto);
        assert_eq!(pezzi[1].testo, "dopo");
    }
}
