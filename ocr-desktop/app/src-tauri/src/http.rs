//! Client HTTP/1.1 minimale verso llama-server su 127.0.0.1.
//!
//! Non si usa una crate general purpose per due motivi: il server e' sempre in
//! loopback senza TLS, e la cascata ha bisogno di **interrompere** una risposta
//! in streaming a meta'. Chiudere il socket e' esattamente il gesto che fa la
//! versione Python: llama.cpp controlla `should_stop` prima di scrivere il
//! chunk successivo e rilascia la slot.

use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpStream};
use std::time::Duration;

pub type Result<T> = std::result::Result<T, String>;

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

fn intestazioni(metodo: &str, percorso: &str, corpo: Option<&str>) -> String {
    let mut testa = format!(
        "{metodo} {percorso} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\
         Accept: application/json, text/event-stream\r\n"
    );
    if let Some(c) = corpo {
        testa.push_str("Content-Type: application/json\r\n");
        testa.push_str(&format!("Content-Length: {}\r\n", c.len()));
    }
    testa.push_str("\r\n");
    testa
}

/// Esito di una lettura non bloccante dallo stream.
pub enum Pezzo {
    /// Una riga completa del corpo (senza terminatore).
    Riga(String),
    /// Nessun dato entro il timeout: il chiamante puo' controllare la revoca.
    Attesa,
    /// Corpo terminato.
    Fine,
}

pub struct Risposta {
    socket: TcpStream,
    stato: u16,
    chunked: bool,
    /// Byte letti dal socket e non ancora sframmentati.
    grezzi: Vec<u8>,
    /// Byte di corpo gia' sframmentati e non ancora consegnati come righe.
    corpo: Vec<u8>,
    /// Byte che restano nel chunk corrente (o nel Content-Length).
    residuo: usize,
    /// Serve a consumare il CRLF che chiude un chunk.
    attesa_crlf: bool,
    finito: bool,
    lunghezza_nota: bool,
}

impl Risposta {
    pub fn stato(&self) -> u16 {
        self.stato
    }

    /// Chiude il socket in entrambe le direzioni. E' l'abort della cascata.
    pub fn interrompi(&mut self) {
        let _ = self.socket.shutdown(Shutdown::Both);
        self.finito = true;
    }

    fn riempi(&mut self) -> Result<bool> {
        let mut buf = [0u8; 16384];
        match self.socket.read(&mut buf) {
            Ok(0) => {
                self.finito = true;
                Ok(false)
            }
            Ok(n) => {
                self.grezzi.extend_from_slice(&buf[..n]);
                Ok(true)
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
                Ok(false)
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => Ok(false),
            Err(e) => Err(format!("lettura dal server interrotta: {e}")),
        }
    }

    /// Sposta byte da `grezzi` a `corpo` rispettando il framing HTTP.
    fn sframmenta(&mut self) -> Result<()> {
        if !self.chunked {
            let quanti = if self.lunghezza_nota {
                self.residuo.min(self.grezzi.len())
            } else {
                self.grezzi.len()
            };
            let coda = self.grezzi.split_off(quanti);
            self.corpo.append(&mut self.grezzi);
            self.grezzi = coda;
            if self.lunghezza_nota {
                self.residuo -= quanti;
                if self.residuo == 0 {
                    self.finito = true;
                }
            }
            return Ok(());
        }
        loop {
            if self.attesa_crlf {
                if self.grezzi.len() < 2 {
                    return Ok(());
                }
                self.grezzi.drain(..2);
                self.attesa_crlf = false;
                continue;
            }
            if self.residuo == 0 {
                let Some(fine) = trova(&self.grezzi, b"\r\n") else {
                    return Ok(());
                };
                let riga = String::from_utf8_lossy(&self.grezzi[..fine]).to_string();
                self.grezzi.drain(..fine + 2);
                let misura = riga.split(';').next().unwrap_or("").trim();
                let n = usize::from_str_radix(misura, 16)
                    .map_err(|_| format!("chunk size illeggibile: {riga:?}"))?;
                if n == 0 {
                    self.finito = true;
                    return Ok(());
                }
                self.residuo = n;
                continue;
            }
            if self.grezzi.is_empty() {
                return Ok(());
            }
            let quanti = self.residuo.min(self.grezzi.len());
            let coda = self.grezzi.split_off(quanti);
            self.corpo.extend_from_slice(&self.grezzi);
            self.grezzi = coda;
            self.residuo -= quanti;
            if self.residuo == 0 {
                self.attesa_crlf = true;
            }
        }
    }

    fn stacca_riga(&mut self) -> Option<String> {
        let posizione = self.corpo.iter().position(|b| *b == b'\n')?;
        let riga: Vec<u8> = self.corpo.drain(..=posizione).collect();
        let mut testo = String::from_utf8_lossy(&riga).to_string();
        while testo.ends_with('\n') || testo.ends_with('\r') {
            testo.pop();
        }
        Some(testo)
    }

    /// Una riga per chiamata; `Attesa` quando il socket e' silenzioso.
    pub fn prossima_riga(&mut self) -> Result<Pezzo> {
        loop {
            if let Some(r) = self.stacca_riga() {
                return Ok(Pezzo::Riga(r));
            }
            if self.finito {
                if self.corpo.is_empty() {
                    return Ok(Pezzo::Fine);
                }
                let resto = String::from_utf8_lossy(&self.corpo).to_string();
                self.corpo.clear();
                return Ok(Pezzo::Riga(resto));
            }
            if !self.riempi()? {
                self.sframmenta()?;
                if self.stacca_riga().is_none() && !self.finito {
                    return Ok(Pezzo::Attesa);
                }
                continue;
            }
            self.sframmenta()?;
        }
    }

    /// Legge tutto il corpo restante. Per le richieste non in streaming.
    pub fn corpo_intero(&mut self, tentativi_vuoti: usize) -> Result<String> {
        let mut vuoti = 0usize;
        let mut fuori = String::new();
        loop {
            match self.prossima_riga()? {
                Pezzo::Riga(r) => {
                    vuoti = 0;
                    fuori.push_str(&r);
                }
                Pezzo::Attesa => {
                    vuoti += 1;
                    if vuoti > tentativi_vuoti {
                        return Err("timeout in lettura dal server".into());
                    }
                }
                Pezzo::Fine => return Ok(fuori),
            }
        }
    }
}

fn trova(dati: &[u8], ago: &[u8]) -> Option<usize> {
    if ago.len() > dati.len() {
        return None;
    }
    (0..=dati.len() - ago.len()).find(|i| &dati[*i..*i + ago.len()] == ago)
}

/// Apre una richiesta e consuma le intestazioni, lasciando pronto il corpo.
pub fn apri(
    porta: u16,
    metodo: &str,
    percorso: &str,
    corpo: Option<&str>,
    attesa_connessione: Duration,
    attesa_lettura: Duration,
) -> Result<Risposta> {
    let socket = TcpStream::connect_timeout(&addr(porta), attesa_connessione)
        .map_err(|e| format!("connessione a 127.0.0.1:{porta} fallita: {e}"))?;
    socket.set_nodelay(true).ok();
    socket
        .set_read_timeout(Some(attesa_lettura))
        .map_err(|e| format!("timeout non impostabile: {e}"))?;
    socket
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| format!("timeout non impostabile: {e}"))?;
    let mut socket = socket;
    socket
        .write_all(intestazioni(metodo, percorso, corpo).as_bytes())
        .map_err(|e| format!("invio richiesta fallito: {e}"))?;
    if let Some(c) = corpo {
        socket
            .write_all(c.as_bytes())
            .map_err(|e| format!("invio corpo fallito: {e}"))?;
    }
    socket.flush().ok();

    let mut risposta = Risposta {
        socket,
        stato: 0,
        chunked: false,
        grezzi: Vec::new(),
        corpo: Vec::new(),
        residuo: 0,
        attesa_crlf: false,
        finito: false,
        lunghezza_nota: false,
    };
    // Le intestazioni si leggono a mano: finiscono al primo CRLFCRLF.
    let scadenza = std::time::Instant::now() + Duration::from_secs(120);
    let fine = loop {
        if let Some(p) = trova(&risposta.grezzi, b"\r\n\r\n") {
            break p;
        }
        if risposta.finito {
            return Err("il server ha chiuso prima delle intestazioni".into());
        }
        if std::time::Instant::now() > scadenza {
            return Err("intestazioni HTTP non arrivate".into());
        }
        risposta.riempi()?;
    };
    let testa = String::from_utf8_lossy(&risposta.grezzi[..fine]).to_string();
    risposta.grezzi.drain(..fine + 4);
    let mut righe = testa.split("\r\n");
    let stato = righe.next().unwrap_or("");
    risposta.stato = stato
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .ok_or_else(|| format!("stato HTTP illeggibile: {stato:?}"))?;
    for riga in righe {
        let Some((chiave, valore)) = riga.split_once(':') else {
            continue;
        };
        let chiave = chiave.trim().to_ascii_lowercase();
        let valore = valore.trim();
        if chiave == "transfer-encoding" && valore.to_ascii_lowercase().contains("chunked") {
            risposta.chunked = true;
        }
        if chiave == "content-length" {
            if let Ok(n) = valore.parse::<usize>() {
                risposta.residuo = n;
                risposta.lunghezza_nota = true;
                if n == 0 {
                    risposta.finito = true;
                }
            }
        }
    }
    Ok(risposta)
}

/// GET/POST che restituisce il corpo come JSON.
pub fn json(
    porta: u16,
    metodo: &str,
    percorso: &str,
    corpo: Option<&str>,
    attesa: Duration,
) -> Result<serde_json::Value> {
    let mut risposta = apri(
        porta,
        metodo,
        percorso,
        corpo,
        Duration::from_secs(5),
        Duration::from_millis(250),
    )?;
    let tentativi = (attesa.as_millis() / 250).max(1) as usize;
    let testo = risposta.corpo_intero(tentativi)?;
    if risposta.stato() >= 400 {
        return Err(format!("HTTP {} da {percorso}: {testo}", risposta.stato()));
    }
    serde_json::from_str(&testo).map_err(|e| format!("JSON non valido da {percorso}: {e}"))
}
