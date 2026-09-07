// Interfaccia di ITA-OCR. Tutto lo stato vero sta in Rust: qui si disegna
// quello che arriva dagli eventi e si mandano i comandi.
//
// La finestra e' divisa in due: a sinistra le pagine originali, a destra il
// testo che il modello genera. Le due colonne scorrono agganciate, cosi'
// l'immagine mostra sempre la pagina che si sta leggendo.

const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const webview = window.__TAURI__.webview;

const el = (id) => document.getElementById(id);
const nodi = {
  documento: el("scorri-documento"),
  testo: el("testo-ocr"),
  zonaVuota: el("zona-vuota"),
  riquadroDrop: el("riquadro-drop"),
  areaLavoro: el("area-lavoro"),
  notaPagina: el("nota-pagina"),
  notaTesto: el("nota-testo"),
  riempimento: el("riempimento"),
  testoProgresso: el("testo-progresso"),
  etichettaMotore: el("etichetta-motore"),
  spiaMotore: el("spia-motore"),
  brindisi: el("brindisi"),
  log: el("log"),
  tempi: el("tempi"),
  dettagliMotore: el("dettagli-motore"),
  dettagliPipeline: el("dettagli-pipeline"),
  barraProgresso: el("barra-progresso"),
  etichettaAvvia: el("etichetta-avvia"),
};

/** Costruisce un'icona presa dal foglio di simboli in cima al documento. */
function icona(nome) {
  const NS = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(NS, "svg");
  svg.setAttribute("viewBox", "0 0 24 24");
  svg.setAttribute("aria-hidden", "true");
  const uso = document.createElementNS(NS, "use");
  uso.setAttribute("href", `#${nome}`);
  svg.append(uso);
  return svg;
}

let pagine = [];
let corrente = null;
let inCorso = false;
let importazioneInCorso = false;
/** Vero finche' l'utente non prende il comando dello scorrimento. */
let seguiIlFlusso = true;
/** Se vero il testo resta LaTeX grezzo; altrimenti le formule si compongono. */
let latexGrezzo = false;
/**
 * Il riquadro che l'utente sta muovendo davvero: solo quello puo' trascinare
 * l'altro. Quando non guida nessuno -- inseguimento della generazione,
 * anteprima che si carica -- non si propaga niente.
 */
let guidato = null;
let scadenzaGuida = 0;
/** Chi ha chiesto meno animazioni al sistema non deve vedere salti animati. */
const motoRidotta = window.matchMedia("(prefers-reduced-motion: reduce)");

/**
 * Registra che l'utente sta agendo su questo riquadro.
 *
 * Si chiama dagli eventi di input -- rotella, dito, puntatore, tastiera -- e
 * mai da `scroll`. E' il punto in cui tre tentativi precedenti hanno
 * sbagliato: `scroll` non dice chi lo ha causato, quindi ogni meccanismo che
 * partiva da li' finiva per scambiare l'eco di uno scorrimento nostro per un
 * gesto. Marcare l'atto non bastava, perche' una sola scrittura puo'
 * generare piu' eco e un marchio booleano ne assorbe una sola: era questo a
 * far "spammare" la stessa posizione quando si scorreva a lungo.
 */
let pulsantePremuto = false;

function segnalaGesto(riquadro) {
  guidato = riquadro;
  clearTimeout(scadenzaGuida);
  // Trascinando la barra, `pointerdown` arriva una volta sola e i movimenti
  // successivi vanno al documento: senza questa eccezione la guida scadeva a
  // meta' trascinamento e la colonna gemella smetteva di seguire.
  if (pulsantePremuto) return;
  // L'inerzia del trackpad continua a emettere `scroll` dopo l'ultimo evento
  // di input: la finestra tiene viva la guida per coprirla. Se scade a meta'
  // la sincronia si ferma, che e' innocuo -- non muove niente di sbagliato.
  scadenzaGuida = setTimeout(() => {
    guidato = null;
  }, 700);
}
const flusso = new Map();

const ETICHETTE = {
  attesa: "in attesa",
  corso: "in lettura…",
  completata: "completata",
  incompleta: "incompleta",
  errore: "errore",
  annullata: "annullata",
};

// GLM-OCR scrive la matematica in LaTeX: `$...$` in linea, `$$...$$` e `\[...\]`
// in display. Il `$` preceduto da backslash e' un dollaro vero, non un delimitatore.
const RE_FORMULA =
  /\$\$([\s\S]+?)\$\$|\\\[([\s\S]+?)\\\]|(?<!\\)\$((?:\\.|[^$\n])+?)(?<!\\)\$|\\\(([\s\S]+?)\\\)/g;

function avvisa(messaggio, male = false) {
  // Un avviso nuovo puo' arrivare mentre il precedente sta finendo la sua
  // animazione di chiusura. Quel timer non deve nascondere anche il nuovo.
  clearTimeout(chiudiAvviso.timer);
  const testo = document.createElement("span");
  testo.className = "testo-avviso";
  testo.textContent = messaggio;
  nodi.brindisi.replaceChildren(icona(male ? "i-allarme" : "i-info"), testo);
  nodi.brindisi.classList.toggle("male", male);
  nodi.brindisi.hidden = false;
  // Un giro di rendering fra `hidden = false` e la classe, altrimenti il
  // browser non ha uno stato di partenza da cui animare.
  clearTimeout(avvisa.timer);
  requestAnimationFrame(() => nodi.brindisi.classList.add("aperto"));
  avvisa.timer = setTimeout(chiudiAvviso, male ? 9000 : 4000);
}

/** Ritira l'avviso, lasciandogli il tempo di scivolare via. */
function chiudiAvviso() {
  clearTimeout(avvisa.timer);
  clearTimeout(chiudiAvviso.timer);
  nodi.brindisi.classList.remove("aperto");
  chiudiAvviso.timer = setTimeout(() => {
    nodi.brindisi.hidden = true;
  }, 240);
}

// Un avviso di troppo si toglie di mezzo con un clic.
nodi.brindisi.addEventListener("click", chiudiAvviso);

// Tooltip unico a livello finestra: non viene tagliato dalla colonna che
// scorre, come accadrebbe a un fumetto CSS interno al singolo <mark>.
const tooltipCorrezione = document.createElement("div");
tooltipCorrezione.id = "tooltip-correzione";
tooltipCorrezione.className = "tooltip-correzione";
tooltipCorrezione.setAttribute("role", "tooltip");
tooltipCorrezione.hidden = true;
document.body.append(tooltipCorrezione);

function mostraTooltipCorrezione(nodo) {
  tooltipCorrezione.textContent = nodo.dataset.tooltip || "";
  tooltipCorrezione.hidden = false;
  const bersaglio = nodo.getBoundingClientRect();
  const fumetto = tooltipCorrezione.getBoundingClientRect();
  const sinistra = Math.min(
    window.innerWidth - fumetto.width / 2 - 10,
    Math.max(fumetto.width / 2 + 10, bersaglio.left + bersaglio.width / 2)
  );
  const sopra = bersaglio.top >= fumetto.height + 14;
  tooltipCorrezione.style.left = `${sinistra}px`;
  tooltipCorrezione.style.top = `${
    sopra ? bersaglio.top - fumetto.height - 8 : bersaglio.bottom + 8
  }px`;
}

function nascondiTooltipCorrezione() {
  tooltipCorrezione.hidden = true;
}

const pagina = (id) => pagine.find((p) => p.id === id) || null;
const cartaDi = (id) => nodi.documento.querySelector(`.carta[data-id="${id}"]`);
const bloccoDi = (id) => nodi.testo.querySelector(`.blocco[data-id="${id}"]`);

// ---------------------------------------------------------------- formule

function indiceCaratteri(testo, indiceUtf16) {
  return Array.from(testo.slice(0, indiceUtf16)).length;
}

/** Aggiunge un intervallo di testo marcando le correzioni che contiene. */
function aggiungiAnnotato(corpo, testo, da, a, correzioni) {
  const caratteri = Array.from(testo);
  let corrente = da;
  for (const c of correzioni) {
    if (c.fine <= da || c.inizio >= a) continue;
    if (c.inizio < da || c.fine > a) continue;
    if (c.inizio > corrente) {
      corpo.append(document.createTextNode(caratteri.slice(corrente, c.inizio).join("")));
    }
    const evidenza = document.createElement("mark");
    evidenza.className = `correzione-automatica ${c.metodo || "dizionario"}`;
    evidenza.textContent = caratteri.slice(c.inizio, c.fine).join("");
    const metodo = c.metodo === "contesto" ? "Contesto" : "Dizionario";
    const spiegazione = `${metodo} · originale: ${c.originale} → ${c.corretta}`;
    evidenza.dataset.tooltip = spiegazione;
    evidenza.dataset.originale = c.originale;
    evidenza.tabIndex = 0;
    evidenza.setAttribute("aria-label", spiegazione);
    evidenza.setAttribute("aria-describedby", tooltipCorrezione.id);
    evidenza.addEventListener("mouseenter", () => mostraTooltipCorrezione(evidenza));
    evidenza.addEventListener("mouseleave", nascondiTooltipCorrezione);
    evidenza.addEventListener("focus", () => mostraTooltipCorrezione(evidenza));
    evidenza.addEventListener("blur", nascondiTooltipCorrezione);
    corpo.append(evidenza);
    corrente = c.fine;
  }
  if (corrente < a) {
    corpo.append(document.createTextNode(caratteri.slice(corrente, a).join("")));
  }
}

/** Aggiunge testo e formule nell'intervallo in caratteri [da, a). */
function aggiungiRicco(corpo, testo, da, a, correzioni) {
  const caratteri = Array.from(testo);
  const frammento = caratteri.slice(da, a).join("");
  if (
    latexGrezzo ||
    typeof katex === "undefined" ||
    (!frammento.includes("$") && !frammento.includes("\\[") && !frammento.includes("\\("))
  ) {
    aggiungiAnnotato(corpo, testo, da, a, correzioni);
    return;
  }
  let ultimoUtf16 = 0;
  let ultimoCarattere = da;
  RE_FORMULA.lastIndex = 0;
  let trovata;
  while ((trovata = RE_FORMULA.exec(frammento)) !== null) {
    const inizioCarattere = da + indiceCaratteri(frammento, trovata.index);
    if (trovata.index > ultimoUtf16) {
      aggiungiAnnotato(corpo, testo, ultimoCarattere, inizioCarattere, correzioni);
    }
    const display = trovata[1] !== undefined || trovata[2] !== undefined;
    const sorgente = trovata[1] ?? trovata[2] ?? trovata[3] ?? trovata[4] ?? "";
    const contenitore = document.createElement(display ? "div" : "span");
    try {
      katex.render(sorgente, contenitore, {
        displayMode: display,
        // Con `false` KaTeX inserisce da solo un errore rosso nel documento.
        // Il fallback dell'app e' piu' leggibile e non altera il layout.
        throwOnError: true,
        strict: "ignore",
        output: "html",
      });
    } catch (_) {
      contenitore.className = `formula-non-composta${display ? " display" : ""}`;
      contenitore.textContent = trovata[0];
      contenitore.title = "Formula LaTeX non componibile: mostrata senza modifiche";
    }
    corpo.append(contenitore);
    ultimoUtf16 = trovata.index + trovata[0].length;
    ultimoCarattere = da + indiceCaratteri(frammento, ultimoUtf16);
  }
  if (ultimoCarattere < a) {
    aggiungiAnnotato(corpo, testo, ultimoCarattere, a, correzioni);
  }
}

function celleMarkdown(riga, inizioRiga) {
  const caratteri = Array.from(riga);
  let da = caratteri[0] === "|" ? 1 : 0;
  const limite = caratteri[caratteri.length - 1] === "|"
    ? caratteri.length - 1
    : caratteri.length;
  const celle = [];
  let escape = false;
  for (let i = da; i <= limite; i += 1) {
    const c = caratteri[i];
    if (i === limite || (c === "|" && !escape)) {
      let sinistra = da;
      let destra = i;
      while (sinistra < destra && /\s/.test(caratteri[sinistra])) sinistra += 1;
      while (destra > sinistra && /\s/.test(caratteri[destra - 1])) destra -= 1;
      celle.push({
        testo: caratteri.slice(sinistra, destra).join(""),
        inizio: inizioRiga + sinistra,
        fine: inizioRiga + destra,
      });
      da = i + 1;
    }
    escape = c === "\\" && !escape;
    if (c !== "\\") escape = false;
  }
  return celle;
}

function eSeparatoreTabella(riga) {
  const celle = celleMarkdown(riga, 0);
  return celle.length >= 2 && celle.every((c) => /^:?-{3,}:?$/.test(c.testo));
}

function righeConPosizioni(testo) {
  const fuori = [];
  let inizio = 0;
  for (const riga of testo.split("\n")) {
    const lunghezza = Array.from(riga).length;
    fuori.push({ testo: riga, inizio, fine: inizio + lunghezza });
    inizio += lunghezza + 1;
  }
  return fuori;
}

function aggiungiTabella(corpo, testo, righe, indice, correzioni) {
  const tabella = document.createElement("table");
  tabella.className = "tabella-markdown";
  const testa = document.createElement("thead");
  const rigaTesta = document.createElement("tr");
  const intestazioni = celleMarkdown(righe[indice].testo, righe[indice].inizio);
  for (const cella of intestazioni) {
    const th = document.createElement("th");
    aggiungiRicco(th, testo, cella.inizio, cella.fine, correzioni);
    rigaTesta.append(th);
  }
  testa.append(rigaTesta);
  tabella.append(testa);

  const corpoTabella = document.createElement("tbody");
  let i = indice + 2;
  while (i < righe.length && righe[i].testo.includes("|") && righe[i].testo.trim()) {
    const tr = document.createElement("tr");
    const celle = celleMarkdown(righe[i].testo, righe[i].inizio);
    for (let colonna = 0; colonna < intestazioni.length; colonna += 1) {
      const td = document.createElement("td");
      const cella = celle[colonna];
      if (cella) aggiungiRicco(td, testo, cella.inizio, cella.fine, correzioni);
      tr.append(td);
    }
    corpoTabella.append(tr);
    i += 1;
  }
  tabella.append(corpoTabella);
  corpo.append(tabella);
  return i;
}

/** Compone formule e tabelle Markdown, conservando gli indicatori lessicali. */
function scriviTesto(corpo, testo, correzioni = []) {
  corpo.replaceChildren();
  const righe = righeConPosizioni(testo);
  let indice = 0;
  let inizioPiano = 0;
  while (indice < righe.length) {
    const tabella =
      indice + 1 < righe.length &&
      righe[indice].testo.includes("|") &&
      eSeparatoreTabella(righe[indice + 1].testo);
    if (!tabella) {
      indice += 1;
      continue;
    }
    if (righe[indice].inizio > inizioPiano) {
      aggiungiRicco(corpo, testo, inizioPiano, righe[indice].inizio, correzioni);
    }
    indice = aggiungiTabella(corpo, testo, righe, indice, correzioni);
    inizioPiano = indice < righe.length ? righe[indice].inizio : Array.from(testo).length;
  }
  const lunghezza = Array.from(testo).length;
  if (inizioPiano < lunghezza) {
    aggiungiRicco(corpo, testo, inizioPiano, lunghezza, correzioni);
  }
}

// ---------------------------------------------------------------- disegno

function etichettaPagina(p) {
  return p.pagine_documento > 1
    ? `${p.nome_documento} · pagina ${p.numero}`
    : p.nome_documento;
}

function statoPagina(p) {
  return p.fast_path && p.stato === "completata"
    ? "text layer"
    : ETICHETTE[p.stato] || p.stato;
}

function marchietti(p) {
  const fuori = [];
  if (p.fast_path && p.stato === "completata") {
    fuori.push(["veloce", "text layer", "Testo preso dal PDF senza eseguire il modello"]);
  }
  if (p.stato === "incompleta") {
    fuori.push(["attenta", "parziale", "Il modello non ha chiuso la pagina"]);
  }
  if (p.tentativi > 1 && p.stato !== "attesa") {
    fuori.push(["attenta", `${p.tentativi} tentativi`, "La cascata ha dovuto riprovare"]);
  }
  if ((p.correzioni_dizionario || 0) > 0) {
    fuori.push([
      "corretta",
      `${p.correzioni_dizionario} diz.`,
      `${p.correzioni_dizionario} correzioni da dizionario`,
    ]);
  }
  if ((p.correzioni_contesto || 0) > 0) {
    fuori.push([
      "corretta contesto",
      `${p.correzioni_contesto} cont.`,
      `${p.correzioni_contesto} correzioni contestuali`,
    ]);
  }
  return fuori;
}

function intestazione(nodo, p, conPunto) {
  nodo.replaceChildren();
  if (conPunto) {
    const punto = document.createElement("span");
    punto.className = `punto ${p.stato}`;
    nodo.append(punto);
  }
  const titolo = document.createElement("span");
  titolo.className = "titolo-pagina";
  titolo.textContent = new Set(pagine.map((pagina) => pagina.documento)).size > 1
    ? etichettaPagina(p) : `Pagina ${p.numero}`;
  titolo.title = etichettaPagina(p);
  const separatore = document.createElement("span");
  separatore.className = "separatore";
  const stato = document.createElement("span");
  stato.className = "stato-pagina";
  stato.textContent = statoPagina(p);
  nodo.append(titolo, separatore);
  for (const [classe, testo, spiegazione] of marchietti(p)) {
    if (testo === stato.textContent) continue;
    const m = document.createElement("span");
    m.className = `marchietto ${classe}`;
    m.textContent = testo;
    m.title = spiegazione;
    nodo.append(m);
  }
  nodo.append(stato);
}

/** Ricostruisce le due colonne mantenendo i nodi gia' presenti. */
function disegna() {
  const carteVecchie = new Map(
    [...nodi.documento.querySelectorAll(".carta")].map((c) => [c.dataset.id, c])
  );
  const blocchiVecchi = new Map(
    [...nodi.testo.querySelectorAll(".blocco")].map((b) => [b.dataset.id, b])
  );
  const carte = [];
  const blocchi = [];

  for (const p of pagine) {
    // Gli identificativi di pagina sono `d0p1`, `d0p2`... e ricominciano da
    // capo a ogni sessione: `d0p1` e' la prima pagina di *qualunque*
    // documento. Riusare la carta per solo identificativo faceva restare
    // l'anteprima del documento di prima, con l'intestazione giusta sopra --
    // perche' quella viene riscritta -- e l'immagine sbagliata sotto.
    // La firma dice di quale pagina si tratta davvero.
    // JSON evita separatori ambigui senza inserire byte NUL nel sorgente (che
    // faceva anche trattare questo file di testo come binario dagli strumenti).
    const firma = JSON.stringify([p.nome_documento, p.numero]);
    let carta = carteVecchie.get(p.id);
    if (carta && carta.dataset.firma !== firma) {
      const cornice = carta.querySelector(".cornice");
      const segnaposto = document.createElement("div");
      segnaposto.className = "segnaposto";
      cornice.replaceChildren(segnaposto);
      delete carta.dataset.anteprima;
      anteprimeInCoda.delete(carta);
      // Ri-osservare e' necessario: l'osservatore notifica i *cambi* di
      // stato, e una carta gia' dentro il campo visivo non ne ha nessuno da
      // segnalare. Senza questo, la carta appena azzerata non veniva mai
      // richiesta e restava sul segnaposto -- che pulsando fino a opacita'
      // 0.45 su cornice bianca sembra una pagina vuota.
      osservatore.unobserve(carta);
      osservatore.observe(carta);
    }
    if (!carta) {
      carta = document.createElement("article");
      carta.className = "carta";
      carta.dataset.id = p.id;
      const testa = document.createElement("header");
      testa.className = "testa-carta";
      const cornice = document.createElement("div");
      cornice.className = "cornice";
      const segnaposto = document.createElement("div");
      segnaposto.className = "segnaposto";
      cornice.append(segnaposto);
      carta.append(testa, cornice);
      carta.addEventListener("click", () => vaiA(p.id, "documento"));
      osservatore.observe(carta);
      carteVecchie.set(p.id, carta);
    }
    carta.dataset.firma = firma;
    intestazione(carta.querySelector(".testa-carta"), p, true);
    carte.push(carta);

    let blocco = blocchiVecchi.get(p.id);
    if (blocco && blocco.dataset.firma !== firma) {
      // Stessa collisione di identificativi delle carte. Qui si vede meno,
      // perche' il testo viene richiesto di nuovo per ogni pagina che ne ha,
      // ma una pagina ancora da leggere mostrerebbe la trascrizione vecchia.
      blocco.querySelector(".testo-blocco").replaceChildren();
    }
    if (!blocco) {
      blocco = document.createElement("section");
      blocco.className = "blocco";
      blocco.dataset.id = p.id;
      const testa = document.createElement("header");
      testa.className = "testa-blocco";
      testa.addEventListener("click", () => vaiA(p.id, "testo"));
      const avviso = document.createElement("div");
      avviso.className = "avviso";
      avviso.hidden = true;
      const corpo = document.createElement("div");
      corpo.className = "testo-blocco";
      blocco.append(testa, avviso, corpo);
      blocchiVecchi.set(p.id, blocco);
    }
    blocco.dataset.firma = firma;
    aggiornaBlocco(blocco, p);
    blocchi.push(blocco);
  }

  for (const [id, carta] of carteVecchie) {
    if (!pagine.some((p) => p.id === id)) {
      osservatore.unobserve(carta);
      carta.remove();
    }
  }
  nodi.documento.replaceChildren(...carte);
  nodi.testo.replaceChildren(...blocchi);

  const vuoto = pagine.length === 0;
  document.body.classList.toggle("vuoto", vuoto);
  nodi.zonaVuota.hidden = !vuoto;
  nodi.areaLavoro.hidden = vuoto;
  el("strumenti-lavoro").hidden = vuoto;
  if (vuoto) applicaVista("confronto");
  if (!sessioneCorrente) {
    const documenti = [...new Set(pagine.map((p) => p.nome_documento))];
    el("nome-sessione").textContent = documenti.length === 1
      ? documenti[0]
      : documenti.length > 1 ? `${documenti.length} documenti` : "Nuova trascrizione";
  }
  if (!vuoto && (!corrente || !pagina(corrente))) corrente = pagine[0].id;
  segnaCorrente();
  aggiornaAzioni();
}

function aggiornaBlocco(blocco, p) {
  intestazione(blocco.querySelector(".testa-blocco"), p, false);
  const avviso = blocco.querySelector(".avviso");
  if (p.messaggio) {
    avviso.hidden = false;
    avviso.classList.toggle("grave", p.stato === "errore");
    avviso.textContent = p.messaggio;
  } else {
    avviso.hidden = true;
  }
}

function segnaCorrente() {
  const corrette = pagine.reduce((totale, p) => totale + (p.correzioni_dizionario || 0), 0);
  el("conteggio-dizionario").textContent = corrette || "";
  for (const nodo of document.querySelectorAll(".carta, .blocco")) {
    nodo.classList.toggle("corrente", nodo.dataset.id === corrente);
  }
  const p = pagina(corrente);
  nodi.notaPagina.textContent = p
    ? `Pagina ${p.numero} di ${p.pagine_documento}`
    : "";
  nodi.notaPagina.title = p ? etichettaPagina(p) : "";
  const finite = pagine.filter((x) => x.caratteri > 0).length;
  nodi.notaTesto.textContent = finite
    ? `${finite} ${finite === 1 ? "pagina letta" : "pagine lette"}`
    : "";
  const indice = pagine.findIndex((p) => p.id === corrente);
  el("posizione-pagina").textContent = `${indice + 1} / ${pagine.length}`;
  el("pagina-precedente").disabled = indice <= 0;
  el("pagina-successiva").disabled = indice < 0 || indice >= pagine.length - 1;
}

function applicaVista(vista) {
  document.body.classList.toggle("solo-testo", vista === "testo");
  for (const bottone of el("scelta-vista").querySelectorAll("button")) {
    const attivo = bottone.dataset.vista === vista;
    bottone.classList.toggle("attivo", attivo);
    bottone.setAttribute("aria-pressed", String(attivo));
  }
  requestAnimationFrame(() => { if (corrente) vaiA(corrente, "vista"); });
}

el("scelta-vista").addEventListener("click", (evento) => {
  const bottone = evento.target.closest("button[data-vista]");
  if (bottone) applicaVista(bottone.dataset.vista);
});
for (const [id, passo] of [["pagina-precedente", -1], ["pagina-successiva", 1]]) {
  el(id).addEventListener("click", () => {
    const prossima = pagine[pagine.findIndex((p) => p.id === corrente) + passo];
    if (prossima) vaiA(prossima.id, "navigazione");
  });
}

async function riempiTesto(id) {
  const blocco = bloccoDi(id);
  if (!blocco) return;
  const contenuto = await invoke("testo_pagina", { id });
  const corpo = blocco.querySelector(".testo-blocco");
  corpo.contenutoPagina = contenuto;
  corpo.dataset.sorgente = contenuto.testo;
  scriviTesto(corpo, contenuto.testo, contenuto.correzioni || []);
  corpo.classList.remove("cursore-vivo");
}

function aggiornaAzioni() {
  const finite = pagine.filter((p) =>
    ["completata", "incompleta"].includes(p.stato)
  ).length;
  const daFare = pagine.filter((p) => ["attesa", "annullata"].includes(p.stato)).length;
  el("avvia").disabled = inCorso || importazioneInCorso || daFare === 0;
  nodi.etichettaAvvia.textContent = pagine.some((p) => p.stato === "annullata") || (finite > 0 && daFare > 0)
    ? "Riprendi"
    : "Trascrivi";
  el("avvia").hidden = inCorso || (pagine.length > 0 && daFare === 0);
  el("interrompi").hidden = !inCorso;
  el("copia").disabled = finite === 0;
  el("apri-esporta").disabled = finite === 0;
  if (finite === 0) apriMenuEsporta(false);
  // Con del testo pronto il comando principale passa dall'avvio alla copia.
  document.body.classList.toggle("esportabile", finite > 0);
  for (const id of ["scegli-file", "aggiungi-altri", "svuota", "nuovo-ocr"]) {
    el(id).disabled = inCorso || importazioneInCorso;
  }
}

/** Barra e testo del piede, ricavati dallo stato corrente delle pagine. */
function aggiornaPiede(in_corso = inCorso) {
  const fatte = pagine.filter((p) =>
    ["completata", "incompleta", "errore"].includes(p.stato)
  ).length;
  const totali = pagine.length;
  const percentuale = totali ? Math.round((fatte / totali) * 100) : 0;
  nodi.riempimento.style.width = `${percentuale}%`;
  nodi.barraProgresso.setAttribute("aria-valuenow", String(percentuale));
  // Prima di cominciare non c'e' avanzamento da mostrare: niente binario vuoto.
  nodi.barraProgresso.hidden = !fatte && !in_corso;
  document.body.classList.toggle("in-corso", Boolean(in_corso));
  document.body.classList.toggle("importazione", importazioneInCorso);
  el("stato-importazione").hidden = !importazioneInCorso;
  el("stato-importazione").textContent = importazioneInCorso ? "Importazione in corso…" : "";
  const documenti = new Set(pagine.map((p) => p.documento)).size;
  const unita = totali === 1 ? "pagina" : "pagine";
  const parziali = pagine.filter((p) => p.stato === "incompleta").length;
  const errori = pagine.filter((p) => p.stato === "errore").length;
  const problemi = [
    parziali ? `${parziali} ${parziali === 1 ? "parziale" : "parziali"}` : "",
    errori ? `${errori} ${errori === 1 ? "errore" : "errori"}` : "",
  ].filter(Boolean).join(" · ");
  nodi.testoProgresso.textContent = importazioneInCorso
    ? "Importazione dei documenti…"
    : !totali ? ""
      : fatte === 0 && !in_corso
        ? `${totali} ${unita} · ${documenti} document${documenti === 1 ? "o" : "i"}`
        : `${fatte} di ${totali} ${unita} · ${percentuale}%${in_corso ? " · in corso" : ""}${problemi ? ` · ${problemi}` : ""}`;
}

// ---------------------------------------------------------------- anteprime

/** Le anteprime si calcolano solo quando la pagina entra nel campo visivo. */
/**
 * Le carte in attesa di anteprima, e quante ne stiamo gia' chiedendo.
 *
 * Ogni anteprima e' un rendering PDF in Rust piu' un'immagine che attraversa
 * il ponte IPC: chiederle tutte insieme blocca l'interfaccia. Trascinando la
 * barra di scorrimento in fretta si passa sopra decine di carte in un attimo,
 * e prima ognuna faceva partire subito la sua richiesta -- comprese quelle
 * gia' superate, che nessuno avrebbe guardato. Da qui il blocco.
 */
const anteprimeInCoda = new Set();
let anteprimeAttive = 0;
let timerAnteprime = 0;
const ANTEPRIME_INSIEME = 2;

const osservatore = new IntersectionObserver(
  (voci) => {
    for (const voce of voci) {
      // Uscendo dal campo la carta lascia la coda: durante una scorsa veloce
      // si smette di volere quello che si e' gia' oltrepassato.
      if (voce.isIntersecting) anteprimeInCoda.add(voce.target);
      else anteprimeInCoda.delete(voce.target);
    }
    // Si aspetta che lo scorrimento si posi, cosi' si chiede solo cio' su cui
    // ci si e' davvero fermati.
    clearTimeout(timerAnteprime);
    timerAnteprime = setTimeout(smaltisciAnteprime, 150);
  },
  { root: nodi.documento, rootMargin: "600px 0px" }
);

function smaltisciAnteprime() {
  while (anteprimeAttive < ANTEPRIME_INSIEME) {
    const carta = anteprimeInCoda.values().next().value;
    if (!carta) return;
    anteprimeInCoda.delete(carta);
    if (carta.dataset.anteprima) continue;
    anteprimeAttive += 1;
    caricaAnteprima(carta).finally(() => {
      anteprimeAttive -= 1;
      smaltisciAnteprime();
    });
  }
}

async function caricaAnteprima(carta) {
  if (carta.dataset.anteprima) return;
  // La stessa carta DOM puo' essere riutilizzata quando si apre un nuovo
  // lavoro: una richiesta del documento precedente non deve vincere la gara
  // e rimettere la sua immagine nella carta appena azzerata.
  const firmaRichiesta = carta.dataset.firma;
  const idRichiesto = carta.dataset.id;
  carta.dataset.anteprima = "in corso";
  try {
    const dati = await invoke("anteprima", { id: idRichiesto });
    if (
      !carta.isConnected ||
      carta.dataset.firma !== firmaRichiesta ||
      carta.dataset.id !== idRichiesto
    ) {
      return;
    }
    const immagine = document.createElement("img");
    immagine.alt = `Originale: ${etichettaPagina(pagina(idRichiesto))}`;
    immagine.src = dati;
    // Si aspetta di conoscere le dimensioni prima di inserirla, altrimenti la
    // misura di quanto e' cresciuta la carta verrebbe presa a immagine ancora
    // vuota e la compensazione qui sotto sarebbe sbagliata.
    await immagine.decode();
    if (
      !carta.isConnected ||
      carta.dataset.firma !== firmaRichiesta ||
      carta.dataset.id !== idRichiesto
    ) {
      return;
    }

    const cornice = carta.querySelector(".cornice");
    const riquadro = nodi.documento.getBoundingClientRect();
    const prima = cornice.getBoundingClientRect();
    cornice.replaceChildren(immagine);
    const cresciuta = cornice.getBoundingClientRect().height - prima.height;

    // Le anteprime si caricano anche 600px sopra il campo visivo: una carta
    // che cresce lassu' spinge giu' tutto il resto e porta via la riga che si
    // stava leggendo. Era questo a far "saltare le pagine" e a riportare
    // all'inizio. Se la carta sta sopra, si recupera la differenza.
    if (cresciuta !== 0 && prima.bottom <= riquadro.top) {
      nodi.documento.scrollTop += cresciuta;
    }
    carta.dataset.anteprima = "fatta";
  } catch (e) {
    if (
      !carta.isConnected ||
      carta.dataset.firma !== firmaRichiesta ||
      carta.dataset.id !== idRichiesto
    ) {
      return;
    }
    const segnaposto = carta.querySelector(".cornice .segnaposto");
    if (segnaposto) {
      segnaposto.classList.add("rotto");
      segnaposto.textContent = `anteprima non disponibile: ${e}`;
    }
    delete carta.dataset.anteprima;
  }
}

// ---------------------------------------------------------------- sincronia

/** Porta entrambe le colonne sulla pagina `id`. */
function vaiA(id, origine) {
  corrente = id;
  segnaCorrente();
  // Qui saltare all'inizio della pagina e' voluto: e' una navigazione
  // esplicita, non un inseguimento. Le colonne le posiziona questa funzione,
  // quindi si toglie la guida: il clic che ha portato qui aveva marcato come
  // guidato il riquadro cliccato, che altrimenti riallineerebbe l'altro
  // sovrascrivendo l'allineamento appena fatto.
  guidato = null;
  clearTimeout(scadenzaGuida);
  // Il salto morbido si chiede qui, sull'atto singolo. Imporlo alla colonna
  // dal CSS lo applicava anche agli allineamenti continui, che invece devono
  // essere immediati per seguire il testo uno a uno.
  const modo = motoRidotta.matches ? "auto" : "smooth";
  if (origine !== "documento") {
    cartaDi(id)?.scrollIntoView({ behavior: modo, block: "start" });
  }
  if (origine !== "testo") {
    bloccoDi(id)?.scrollIntoView({ behavior: modo, block: "start" });
  }
}

/**
 * Dove si sta leggendo: quale pagina tocca il bordo superiore, e quanta parte
 * ne e' gia' passata sopra.
 *
 * La frazione e' il punto della correzione: allineare l'altra colonna
 * all'inizio della pagina buttava via la posizione dentro la pagina, e a meta'
 * di una facciata lunga la vista tornava indietro di colpo.
 */
function puntoDiLettura(contenitore, selettore) {
  const alto = contenitore.getBoundingClientRect().top;
  let ultima = null;
  for (const nodo of contenitore.querySelectorAll(selettore)) {
    const r = nodo.getBoundingClientRect();
    if (r.bottom <= alto) {
      ultima = nodo;
      continue;
    }
    // Nello spazio fra due blocchi si guarda al prossimo, dal suo inizio.
    // Restituire invece la fine di quello passato faceva scattare la colonna
    // a fondo pagina ogni volta che il bordo cadeva in un margine.
    if (r.top > alto) return { id: nodo.dataset.id, frazione: 0 };
    const frazione = r.height > 0 ? (alto - r.top) / r.height : 0;
    return { id: nodo.dataset.id, frazione: Math.min(Math.max(frazione, 0), 1) };
  }
  // Oltre l'ultima pagina: si resta appesi al suo fondo.
  return ultima ? { id: ultima.dataset.id, frazione: 1 } : null;
}

/** La pagina in cima, per chi non ha bisogno della frazione. */
function paginaInCima(contenitore, selettore) {
  return puntoDiLettura(contenitore, selettore)?.id ?? null;
}

/** Porta `destinazione` allo stesso punto di lettura, frazione compresa. */
function allinea(destinazione, selettore, id, frazione) {
  const nodo = destinazione.querySelector(`${selettore}[data-id="${CSS.escape(id)}"]`);
  if (!nodo) return;
  const alto = destinazione.getBoundingClientRect().top;
  const r = nodo.getBoundingClientRect();
  // L'immagine e il suo testo hanno altezze diverse: si trasporta la
  // proporzione, non i pixel.
  destinazione.scrollTop += r.top - alto + frazione * r.height;
}

function agganciaScorrimento(sorgente, selettoreSorgente, destinazione, selettoreDestinazione) {
  let programmata = false;
  sorgente.addEventListener(
    "scroll",
    () => {
      // Propaga solo il riquadro che l'utente sta muovendo: le eco dell'altro,
      // quante che siano, non comandano niente.
      if (guidato !== sorgente) return;
      // Vale anche per scrollbar e tastiera. Prima solo rotella e touch
      // fermavano l'auto-inseguimento: trascinando la barra, il delta OCR
      // successivo riportava immediatamente il testo in fondo.
      seguiIlFlusso = false;
      // Un allineamento per fotogramma, non per evento. Misurare i blocchi e
      // poi scrivere `scrollTop` costringe il browser a ricalcolare il layout
      // ogni volta; trascinando la barra gli eventi arrivano molto piu' fitti
      // dei fotogrammi, e farlo per ognuno satura il thread e blocca tutto.
      if (programmata) return;
      programmata = true;
      requestAnimationFrame(() => {
        programmata = false;
        if (guidato !== sorgente) return;
        const punto = puntoDiLettura(sorgente, selettoreSorgente);
        if (!punto) return;
        if (punto.id !== corrente) {
          corrente = punto.id;
          segnaCorrente();
        }
        allinea(destinazione, selettoreDestinazione, punto.id, punto.frazione);
      });
    },
    { passive: true }
  );
}

agganciaScorrimento(nodi.testo, ".blocco", nodi.documento, ".carta");
agganciaScorrimento(nodi.documento, ".carta", nodi.testo, ".blocco");

// Basta muovere la rotella sul testo per prendere il comando: si riprende a
// inseguire la generazione quando comincia una pagina nuova.
// Gli unici eventi che un utente puo' produrre e un nostro `scrollTop` no.
// Da qui si sa chi comanda; `scroll` da solo non lo direbbe mai.
for (const riquadro of [nodi.testo, nodi.documento]) {
  for (const evento of ["wheel", "touchstart", "touchmove", "pointerdown"]) {
    riquadro.addEventListener(
      evento,
      () => {
        if (evento === "pointerdown") pulsantePremuto = true;
        segnalaGesto(riquadro);
        // Muovere la vista significa prendere il comando anche
        // sull'inseguimento della generazione.
        if (evento === "wheel" || evento === "touchmove") seguiIlFlusso = false;
      },
      { passive: true }
    );
  }
  // Un tasto qualsiasi (per esempio Ctrl+C mentre si copia) non e' un gesto
  // di scorrimento. Marcare solo quelli che possono davvero muovere la vista
  // evita che un aggiornamento OCR programmato sembri un gesto dell'utente.
  riquadro.addEventListener(
    "keydown",
    (evento) => {
      if (
        ![
          "ArrowUp",
          "ArrowDown",
          "PageUp",
          "PageDown",
          "Home",
          "End",
          " ",
        ].includes(evento.key)
      ) {
        return;
      }
      segnalaGesto(riquadro);
    },
    { passive: true }
  );
}

// Il rilascio arriva ovunque sia finito il puntatore, anche fuori dal
// riquadro: va ascoltato in alto, non sulla colonna.
for (const evento of ["pointerup", "pointercancel"]) {
  window.addEventListener(
    evento,
    () => {
      if (!pulsantePremuto) return;
      pulsantePremuto = false;
      // Finito il trascinamento, la guida torna a scadere da sola.
      if (guidato) segnalaGesto(guidato);
    },
    { passive: true }
  );
}
// Se il puntatore viene rilasciato fuori dalla finestra non arriva pointerup:
// senza questo ripristino una colonna resterebbe guida per tutta la sessione.
window.addEventListener("blur", () => {
  pulsantePremuto = false;
  guidato = null;
  clearTimeout(scadenzaGuida);
});
nodi.testo.addEventListener("scroll", nascondiTooltipCorrezione, { passive: true });

/** Tiene visibile il fondo del blocco che sta crescendo. */
function inseguiBlocco(blocco) {
  const riquadro = nodi.testo.getBoundingClientRect();
  const suo = blocco.getBoundingClientRect();
  const eccedenza = suo.bottom - riquadro.bottom + 40;
  if (eccedenza > 0) {
    // Inseguire il testo che si genera non deve muovere il PDF. Il blocco si
    // sta allungando, quindi la frazione di pagina gia' letta *cala* anche
    // mentre si scende, e il documento tornava indietro di un pochino a ogni
    // riga nuova.
    nodi.testo.scrollTop += eccedenza;
  }
}

// ---------------------------------------------------------------- file

async function scegliFile() {
  if (inCorso || importazioneInCorso) return;
  try {
    const risposta = await invoke("plugin:dialog|open", {
      options: {
        multiple: true,
        directory: false,
        title: "Scegli i documenti da leggere",
        filters: [
          {
            name: "Documenti e immagini",
            extensions: ["pdf", "png", "jpg", "jpeg", "tif", "tiff", "bmp", "webp", "gif"],
          },
        ],
      },
    });
    const percorsi = normalizzaPercorsi(risposta);
    if (percorsi.length) await aggiungiPercorsi(percorsi);
  } catch (e) {
    avvisa(`Non riesco ad aprire la finestra dei file: ${e}`, true);
  }
}

function normalizzaPercorsi(risposta) {
  if (!risposta) return [];
  const lista = Array.isArray(risposta) ? risposta : [risposta];
  return lista
    .map((v) => (typeof v === "string" ? v : v && (v.path || v.file)))
    .filter(Boolean);
}

async function aggiungiPercorsi(percorsi) {
  if (inCorso || importazioneInCorso) {
    avvisa("Attendi la fine dell'operazione prima di aggiungere altri documenti.");
    return;
  }
  importazioneInCorso = true;
  nodi.riquadroDrop.setAttribute("aria-busy", "true");
  aggiornaAzioni();
  aggiornaPiede();
  try {
    const problemi = await invoke("apri_file", { percorsi });
    if (problemi.length) avvisa(problemi.join(" · "), true);
  } catch (e) {
    avvisa(`Importazione non riuscita: ${e}`, true);
  } finally {
    importazioneInCorso = false;
    nodi.riquadroDrop.removeAttribute("aria-busy");
    aggiornaAzioni();
    aggiornaPiede();
  }
}

el("scegli-file").addEventListener("click", scegliFile);
el("aggiungi-altri").addEventListener("click", scegliFile);
el("svuota").addEventListener("click", async () => {
  await nuovaSessione();
});

// ---------------------------------------------------------------- esecuzione

el("avvia").addEventListener("click", async () => {
  inCorso = true;
  seguiIlFlusso = true;
  aggiornaAzioni();
  aggiornaPiede();
  try {
    await invoke("avvia");
  } catch (e) {
    inCorso = false;
    aggiornaAzioni();
    aggiornaPiede();
    avvisa(`Trascrizione non avviata: ${e}`, true);
  }
});

el("interrompi").addEventListener("click", async () => {
  try {
    await invoke("annulla");
    avvisa("Interruzione richiesta: la pagina in corso viene chiusa.");
  } catch (e) {
    avvisa(`Interruzione non riuscita: ${e}`, true);
  }
});

// ---------------------------------------------------------------- uscite

async function copiaNegliAppunti(testo) {
  try {
    await navigator.clipboard.writeText(testo);
    return true;
  } catch (_) {
    /* la webview puo' negare l'API: si prova il plugin */
  }
  try {
    await invoke("plugin:clipboard-manager|write_text", {
      data: { plainText: { label: null, text: testo } },
    });
    return true;
  } catch (_) {
    /* ultimo tentativo: la vecchia strada */
  }
  const area = document.createElement("textarea");
  area.value = testo;
  area.style.position = "fixed";
  area.style.opacity = "0";
  document.body.append(area);
  area.select();
  const esito = document.execCommand("copy");
  area.remove();
  return esito;
}

el("copia").addEventListener("click", async () => {
  const selezione = String(window.getSelection() || "");
  const tutto = selezione.trim()
    ? selezione
    : await invoke("esporta");
  if (!tutto.trim()) {
    avvisa("Non c'è ancora niente da copiare.", true);
    return;
  }
  if (await copiaNegliAppunti(tutto)) {
    avvisa(selezione.trim() ? "Selezione copiata." : "Testo di tutte le pagine copiato.");
  } else {
    avvisa("Copia non riuscita.", true);
  }
});

const NOME_FORMATO = { txt: "Testo semplice", md: "Markdown", docx: "Documento Word" };

async function esporta(formato) {
  // Il .docx e' un file binario: lo compone Rust, che ha gia' il testo e puo'
  // ricostruire le tabelle. Gli altri due passano di qui come stringa.
  const binario = formato === "docx";
  const contenuto = binario ? "" : await invoke("esporta");
  if (!binario && !contenuto.trim()) {
    avvisa("Non c'è ancora niente da esportare.", true);
    return;
  }
  const primo = pagine[0];
  const base = (primo ? primo.nome_documento : "trascrizione").replace(/\.[^.]+$/, "");
  try {
    const percorso = await invoke("plugin:dialog|save", {
      options: {
        title: "Esporta la trascrizione",
        defaultPath: `${base}.${formato}`,
        filters: [{ name: NOME_FORMATO[formato], extensions: [formato] }],
      },
    });
    if (!percorso) return;
    const scelto = typeof percorso === "string" ? percorso : percorso.path;
    const salvato = binario
      ? await invoke("esporta_docx", { percorso: scelto })
      : await invoke("salva_file", { percorso: scelto, contenuto });
    avvisa(`Esportato in ${salvato}`);
  } catch (e) {
    avvisa(`Esportazione non riuscita: ${e}`, true);
  }
}

function apriMenuEsporta(apri) {
  const bottone = el("apri-esporta");
  const voci = el("voci-esporta");
  if (apri && bottone.disabled) return;
  voci.hidden = !apri;
  bottone.setAttribute("aria-expanded", String(apri));
  if (apri) voci.querySelector("button").focus();
}

el("apri-esporta").addEventListener("click", () => {
  apriMenuEsporta(el("voci-esporta").hidden);
});
// Un clic fuori, o il fuoco che se ne va, chiudono il menu come farebbe Windows.
document.addEventListener("pointerdown", (evento) => {
  if (!el("voci-esporta").hidden && !evento.target.closest(".menu-uscite")) apriMenuEsporta(false);
});
el("voci-esporta").addEventListener("focusout", (evento) => {
  if (!evento.relatedTarget?.closest(".menu-uscite")) apriMenuEsporta(false);
});

for (const [id, formato] of [["esporta-docx", "docx"], ["esporta-md", "md"], ["esporta-txt", "txt"]]) {
  el(id).addEventListener("click", () => {
    apriMenuEsporta(false);
    el("apri-esporta").focus();
    esporta(formato);
  });
}

// ---------------------------------------------------------------- avanzate

// `sezione` porta il cassetto direttamente su un riquadro: serve alla firma
// del pannello, che apre le impostazioni su "Informazioni".
function apriAvanzate(apri, sezione) {
  const pannello = el("pannello-avanzate");
  const velo = el("velo");
  const grilletto = el("apri-avanzate");
  clearTimeout(apriAvanzate.timer);
  grilletto.setAttribute("aria-expanded", String(apri));
  el("stato-motore").setAttribute("aria-expanded", String(apri));
  if (apri) {
    apriAvanzate.origine = document.activeElement;
    pannello.hidden = false;
    velo.hidden = false;
    document.querySelector(".guscio").inert = true;
    requestAnimationFrame(() => {
      pannello.classList.add("aperto");
      velo.classList.add("aperto");
    });
    // Lo scorrimento aspetta i dati: il cassetto si riempie dopo l'apertura, e
    // scorrere prima lo lascerebbe a meta' strada.
    aggiornaDiagnostica()
      .then(() => sezione && el(sezione).scrollIntoView({ block: "start" }))
      .catch((e) => avvisa(`Impostazioni non leggibili: ${e}`, true));
    el("chiudi-avanzate").focus();
  } else {
    document.querySelector(".guscio").inert = false;
    pannello.classList.remove("aperto");
    velo.classList.remove("aperto");
    // Si nasconde davvero solo a scivolata finita: `hidden` spegne le transizioni.
    apriAvanzate.timer = setTimeout(() => {
      pannello.hidden = true;
      velo.hidden = true;
    }, 240);
    if (document.activeElement === document.body || pannello.contains(document.activeElement)) {
      const origine = apriAvanzate.origine;
      (origine?.isConnected ? origine : grilletto).focus();
    }
  }
}

el("apri-avanzate").addEventListener("click", () => apriAvanzate(true));
el("stato-motore").addEventListener("click", () => apriAvanzate(true));
el("chiudi-avanzate").addEventListener("click", () => apriAvanzate(false));
el("velo").addEventListener("click", () => apriAvanzate(false));

el("scelta-backend").addEventListener("change", async (evento) => {
  try {
    await invoke("imposta_backend", { scelta: evento.target.value });
    avvisa("Backend cambiato: il motore si sta riavviando.");
  } catch (e) {
    avvisa(String(e), true);
  }
});

el("forza-ocr").addEventListener("change", async (evento) => {
  await invoke("imposta_forza_ocr", { valore: evento.target.checked });
  avvisa(
    evento.target.checked
      ? "I PDF con testo verranno comunque letti dal modello, dal prossimo caricamento."
      : "I PDF con testo torneranno al percorso veloce."
  );
});

el("correzione-automatica").addEventListener("change", async (evento) => {
  const attiva = evento.target.checked;
  try {
    await invoke("imposta_correzione_automatica", { valore: attiva });
    avvisa(
      attiva
        ? "Correzione automatica attiva: le sostituzioni sono evidenziate."
        : "Correzione automatica disattivata: viene mostrato il testo OCR originale."
    );
    segnaCorrente();
  } catch (e) {
    evento.target.checked = !attiva;
    avvisa(String(e), true);
  }
});

el("correzione-contestuale").addEventListener("change", async (evento) => {
  const attiva = evento.target.checked;
  try {
    const trovate = await invoke("imposta_correzione_contestuale", { valore: attiva });
    avvisa(
      attiva
        ? trovate > 0
          ? `Correzione contestuale attiva: ${trovate} modifiche evidenziate in viola.`
          : "Correzione contestuale attiva, ma nessuna parola di queste pagine supera la soglia prudente."
        : "Correzione contestuale disattivata."
    );
  } catch (e) {
    evento.target.checked = !attiva;
    avvisa(String(e), true);
  }
});

el("latex-grezzo").addEventListener("change", (evento) => {
  latexGrezzo = evento.target.checked;
  for (const corpo of document.querySelectorAll(".testo-blocco")) {
    if (corpo.contenutoPagina) {
      scriviTesto(
        corpo,
        corpo.contenutoPagina.testo,
        corpo.contenutoPagina.correzioni || []
      );
    }
  }
  avvisa(latexGrezzo ? "Le formule restano in LaTeX." : "Le formule vengono composte.");
});

// Il monospazio conserva gli allineamenti; Standard usa il carattere dell'app.
(() => {
  const gruppo = el("scelta-carattere");
  for (const bottone of gruppo.querySelectorAll("button")) {
    bottone.addEventListener("click", () => {
      const paginaVisibile = paginaInCima(nodi.testo, ".blocco") || corrente;
      for (const altro of gruppo.querySelectorAll("button")) {
        const scelto = altro === bottone;
        altro.classList.toggle("attivo", scelto);
        altro.setAttribute("aria-pressed", String(scelto));
      }
      nodi.testo.classList.toggle("lettura", bottone.dataset.carattere === "lettura");
      // Il cambio di font modifica tutte le altezze: riallinea entrambe le
      // colonne alla stessa pagina dopo che il browser ha rifatto il layout.
      requestAnimationFrame(() => {
        if (paginaVisibile) vaiA(paginaVisibile, "cambio-carattere");
      });
    });
  }
})();

el("riavvia-motore").addEventListener("click", async () => {
  try {
    await invoke("riavvia_motore");
    avvisa("Motore in riavvio.");
  } catch (e) {
    avvisa(String(e), true);
  }
});

// La firma nel pannello e' anche la porta di "Informazioni".
el("apri-informazioni").addEventListener("click", () => apriAvanzate(true, "informazioni"));

el("indirizzo-repo").addEventListener("click", async () => {
  try {
    await invoke("apri_repository");
  } catch (e) {
    avvisa(`Pagina non aperta: ${e}`, true);
  }
});

el("copia-repo").addEventListener("click", async () => {
  const indirizzo = el("indirizzo-repo").textContent.trim();
  if (await copiaNegliAppunti(indirizzo)) avvisa("Indirizzo copiato negli appunti.");
  else avvisa("Indirizzo non copiato: gli appunti non sono disponibili.", true);
});

el("apri-log").addEventListener("click", async () => {
  try {
    const dir = await invoke("apri_cartella_log");
    avvisa(`Cartella dei log: ${dir}`);
  } catch (e) {
    avvisa(String(e), true);
  }
});

function righeDettaglio(contenitore, coppie) {
  contenitore.replaceChildren();
  for (const [chiave, valore] of coppie) {
    if (valore === null || valore === undefined || valore === "") continue;
    const dt = document.createElement("dt");
    dt.textContent = chiave;
    const dd = document.createElement("dd");
    dd.textContent = String(valore);
    contenitore.append(dt, dd);
  }
}

async function aggiornaDiagnostica() {
  const d = await invoke("diagnostica");
  const m = d.motore;
  righeDettaglio(nodi.dettagliMotore, [
    ["backend", m.etichetta],
    ["dispositivo", m.dispositivo],
    ["stato", m.fase],
    ["porta locale", m.porta || "—"],
    ["thread", m.thread || "—"],
    ["caricamento", m.avvio_secondi ? `${m.avvio_secondi.toFixed(1)} s` : "—"],
    ["language model", m.modello ? m.modello.split("/").pop() : "—"],
    ["torre visiva", m.mmproj ? m.mmproj.split("/").pop() : "—"],
    ["problema", m.messaggio],
  ]);
  righeDettaglio(nodi.dettagliPipeline, [
    ["canvas", `${d.canvas[0]}×${d.canvas[1]}`],
    ["rendering PDF", `${d.dpi} DPI`],
    ["tetto token", d.n_predict],
    ["cascata", d.cascata.map((s) => s.stadio).join(" → ")],
    [
      "rilevatore ciclo",
      `${d.rilevatore.min_ripetizioni} ripetizioni, periodo ≤ ${d.rilevatore.periodo_massimo}, span ≥ ${d.rilevatore.span_minimo}`,
    ],
    ["prompt cache", "disattivata (--cache-ram 0)"],
  ]);
  el("scelta-backend").value = d.impostazioni.backend;
  el("forza-ocr").checked = d.impostazioni.forza_ocr;
  el("correzione-automatica").checked = d.impostazioni.correzione_automatica;
  el("correzione-contestuale").checked = d.impostazioni.correzione_contestuale;

  nodi.tempi.replaceChildren();
  const fatte = pagine.filter((p) => p.secondi > 0);
  if (!fatte.length) {
    const vuoto = document.createElement("p");
    vuoto.className = "minuta";
    vuoto.textContent = "Nessuna pagina ancora elaborata.";
    nodi.tempi.append(vuoto);
  } else {
    // In vista resta il riassunto: le righe pagina per pagina, che su un
    // documento lungo riempiono il pannello, stanno sotto un dettaglio.
    const totale = fatte.reduce((somma, p) => somma + p.secondi, 0);
    const riassunto = document.createElement("div");
    riassunto.className = "riga-tempo";
    const quante = document.createElement("span");
    quante.textContent = `${fatte.length} ${fatte.length === 1 ? "pagina letta" : "pagine lette"}`;
    const misura = document.createElement("span");
    misura.textContent = `${(totale / fatte.length).toFixed(2)} s a pagina · ${totale.toFixed(1)} s in tutto`;
    riassunto.append(quante, misura);
    const dettaglio = document.createElement("details");
    dettaglio.className = "details-tecnici";
    const titolo = document.createElement("summary");
    titolo.textContent = "Tempo di ogni pagina";
    dettaglio.append(titolo);
    for (const p of fatte) {
      const riga = document.createElement("div");
      riga.className = "riga-tempo";
      const sinistra = document.createElement("span");
      sinistra.textContent = `${p.nome_documento} · p. ${p.numero}${
        p.fast_path ? " (text layer)" : ""
      }`;
      const destra = document.createElement("span");
      const stadio = p.stadio && !p.fast_path ? ` · ${p.stadio}` : "";
      destra.textContent = `${p.secondi.toFixed(2)} s${stadio}`;
      riga.append(sinistra, destra);
      dettaglio.append(riga);
      // Stato dei retry: uno stadio della cascata per riga, con il suo esito.
      for (const t of p.dettaglio || []) {
        const passo = document.createElement("div");
        passo.className = "riga-tempo passo";
        const nome = document.createElement("span");
        nome.textContent = `↳ ${t.stadio}: ${t.esito}`;
        const misure = document.createElement("span");
        misure.textContent = `${t.secondi.toFixed(2)} s · ${t.token} token${
          t.token_al_secondo ? ` · ${t.token_al_secondo.toFixed(0)} tok/s` : ""
        }`;
        passo.append(nome, misure);
        dettaglio.append(passo);
      }
    }
    nodi.tempi.append(riassunto, dettaglio);
  }

  const righe = (d.righe_log || []).slice(-120);
  nodi.log.textContent = righe
    .map((r) => `${r.istante} [${r.livello}] ${r.testo}`)
    .join("\n");
  if (d.coda_log_motore) {
    nodi.log.textContent += `\n\n--- llama-server ---\n${d.coda_log_motore}`;
  }
  nodi.log.scrollTop = nodi.log.scrollHeight;
}

// ---------------------------------------------------------------- eventi

function aggiornaMotore(info) {
  nodi.etichettaMotore.textContent =
    info.fase === "pronto"
      ? info.etichetta
      : info.fase === "avvio"
        ? "avvio del motore…"
        : info.fase === "errore"
          ? "motore non disponibile"
          : "motore spento";
  nodi.spiaMotore.className = `spia ${info.fase}`;
  el("stato-motore").title =
    info.fase === "errore" && info.messaggio
      ? info.messaggio
      : info.dispositivo || "Stato del motore";
  if (info.fase === "errore" && info.messaggio) avvisa(info.messaggio, true);
}

listen("ocr://elenco", async (evento) => {
  pagine = evento.payload;
  disegna();
  aggiornaPiede();
  for (const p of pagine) {
    if (p.caratteri > 0) riempiTesto(p.id);
  }
});

listen("ocr://pagina", (evento) => {
  const aggiornata = evento.payload;
  const indice = pagine.findIndex((p) => p.id === aggiornata.id);
  if (indice < 0) return;
  pagine[indice] = aggiornata;
  const blocco = bloccoDi(aggiornata.id);
  const carta = cartaDi(aggiornata.id);
  if (blocco) aggiornaBlocco(blocco, aggiornata);
  if (carta) intestazione(carta.querySelector(".testa-carta"), aggiornata, true);
  if (blocco && aggiornata.stato !== "corso") {
    flusso.delete(aggiornata.id);
    riempiTesto(aggiornata.id);
  }
  segnaCorrente();
  aggiornaAzioni();
  if (!el("pannello-avanzate").hidden) aggiornaDiagnostica();
});

listen("ocr://delta", (evento) => {
  const { id, testo, sostituisci, stadio } = evento.payload;
  flusso.set(id, (sostituisci ? "" : flusso.get(id) || "") + testo);
  const blocco = bloccoDi(id);
  if (!blocco) return;
  const corpo = blocco.querySelector(".testo-blocco");
  corpo.classList.add("cursore-vivo");
  if (sostituisci) {
    corpo.contenutoPagina = null;
    corpo.dataset.sorgente = testo;
    corpo.textContent = testo;
    // Pagina nuova (o ritentata): le due colonne tornano a inseguire.
    seguiIlFlusso = true;
    vaiA(id, null);
    if (stadio && stadio !== "greedy" && stadio !== "text layer") {
      const avviso = blocco.querySelector(".avviso");
      avviso.hidden = false;
      avviso.classList.remove("grave");
      avviso.textContent = `Ripetizione rilevata: nuovo tentativo con ${stadio}.`;
    }
  } else {
    corpo.contenutoPagina = null;
    corpo.dataset.sorgente = (corpo.dataset.sorgente || "") + testo;
    corpo.append(document.createTextNode(testo));
  }
  if (seguiIlFlusso) inseguiBlocco(blocco);
});

listen("ocr://progresso", (evento) => {
  const { in_corso, id_corrente } = evento.payload;
  inCorso = in_corso;
  aggiornaPiede(in_corso);
  aggiornaAzioni();
  if (in_corso && id_corrente && seguiIlFlusso && id_corrente !== corrente) {
    vaiA(id_corrente, null);
  }
});

listen("ocr://motore", (evento) => {
  aggiornaMotore(evento.payload);
  if (!el("pannello-avanzate").hidden) aggiornaDiagnostica();
});

// ---------------------------------------------------------------- divisore

(() => {
  const divisore = el("divisore");
  const MINIMO = 20;
  const MASSIMO = 75;
  const META = 46;
  let attivo = false;
  let puntoTrascinamento = null;
  let puntoDaConservare = null;
  let riallineamento = 0;

  function puntoVisibile() {
    if (guidato === nodi.documento) {
      return puntoDiLettura(nodi.documento, ".carta");
    }
    return (
      puntoDiLettura(nodi.testo, ".blocco") ||
      puntoDiLettura(nodi.documento, ".carta")
    );
  }

  function riallineaDopoIlRiflusso(punto) {
    // Piu' movimenti possono arrivare nello stesso fotogramma. Si conserva
    // il punto misurato prima del primo cambio di larghezza: misurarlo di
    // nuovo dopo il riflusso significherebbe gia' accettare il disallineamento.
    if (!puntoDaConservare) puntoDaConservare = punto;
    if (!puntoDaConservare || riallineamento) return;
    riallineamento = requestAnimationFrame(() => {
      riallineamento = 0;
      const daRipristinare = puntoDaConservare;
      puntoDaConservare = null;
      // Gli scroll qui sotto sono nostri: una guida rimasta da un gesto
      // precedente non deve rimandarli avanti e indietro fra le colonne.
      guidato = null;
      clearTimeout(scadenzaGuida);
      allinea(
        nodi.documento,
        ".carta",
        daRipristinare.id,
        daRipristinare.frazione
      );
      allinea(
        nodi.testo,
        ".blocco",
        daRipristinare.id,
        daRipristinare.frazione
      );
    });
  }

  function imposta(frazione, punto = puntoVisibile()) {
    const valore = Math.min(MASSIMO, Math.max(MINIMO, frazione));
    document.documentElement.style.setProperty(
      "--larghezza-documento",
      `${valore}%`
    );
    divisore.setAttribute("aria-valuenow", String(Math.round(valore)));
    riallineaDopoIlRiflusso(punto);
  }

  const attuale = () => Number(divisore.getAttribute("aria-valuenow")) || META;

  divisore.addEventListener("mousedown", (e) => {
    attivo = true;
    puntoTrascinamento = puntoVisibile();
    divisore.classList.add("attivo");
    document.body.classList.add("ridimensiona");
    e.preventDefault();
  });

  window.addEventListener("mousemove", (e) => {
    if (!attivo) return;
    const area = nodi.areaLavoro.getBoundingClientRect();
    imposta(
      ((e.clientX - area.left) / area.width) * 100,
      puntoTrascinamento
    );
  });

  window.addEventListener("mouseup", () => {
    if (!attivo) return;
    attivo = false;
    puntoTrascinamento = null;
    divisore.classList.remove("attivo");
    document.body.classList.remove("ridimensiona");
  });

  // Doppio clic: le due colonne tornano in equilibrio.
  divisore.addEventListener("dblclick", () => imposta(META));

  // Da tastiera il divisore e' raggiungibile con Tab: le frecce lo muovono di
  // un punto, con Maiusc di cinque, Home rimette a meta'.
  divisore.addEventListener("keydown", (evento) => {
    const passo = evento.shiftKey ? 5 : 1;
    if (evento.key === "ArrowLeft") imposta(attuale() - passo);
    else if (evento.key === "ArrowRight") imposta(attuale() + passo);
    else if (evento.key === "Home") imposta(META);
    else return;
    evento.preventDefault();
  });

  imposta(META);
})();

// ---------------------------------------------------------------- tastiera

document.addEventListener("keydown", (evento) => {
  const pannello = el("pannello-avanzate");
  if (!pannello.hidden) {
    if (evento.key === "Escape") {
      evento.preventDefault();
      apriAvanzate(false);
    } else if (evento.key === "Tab") {
      const campi = [...pannello.querySelectorAll("button, input, select, summary, [tabindex='0']")]
        .filter((nodo) => !nodo.disabled && nodo.getClientRects().length > 0);
      const primo = campi[0];
      const ultimo = campi[campi.length - 1];
      if (evento.shiftKey && document.activeElement === primo) {
        evento.preventDefault(); ultimo?.focus();
      } else if (!evento.shiftKey && document.activeElement === ultimo) {
        evento.preventDefault(); primo?.focus();
      }
    }
    return;
  }
  const dentroUnCampo = ["INPUT", "SELECT", "TEXTAREA"].includes(
    document.activeElement?.tagName
  );
  if (evento.key === "Escape") {
    if (!el("voci-esporta").hidden) { apriMenuEsporta(false); el("apri-esporta").focus(); }
    else if (!el("pannello-avanzate").hidden) apriAvanzate(false);
    else if (inCorso) el("interrompi").click();
    return;
  }
  if (dentroUnCampo) return;
  if ((evento.ctrlKey && evento.key === "Enter") || evento.key === "F5") {
    evento.preventDefault();
    if (!el("avvia").disabled) el("avvia").click();
  } else if (evento.ctrlKey && evento.key.toLowerCase() === "o") {
    evento.preventDefault();
    scegliFile();
  }
});

// ---------------------------------------------------------------- tema

// Di partenza l'interfaccia e' chiara; la scelta dell'utente sopravvive alla
// chiusura, e "auto" toglie l'attributo e lascia decidere a prefers-color-scheme.
const TEMA_SALVATO = "ocr-ita:tema";

function applicaTema(scelta) {
  if (scelta === "auto") {
    document.documentElement.removeAttribute("data-tema");
  } else {
    document.documentElement.setAttribute("data-tema", scelta);
  }
  for (const b of el("scelta-tema").querySelectorAll("button")) {
    const suo = b.dataset.tema === scelta;
    b.classList.toggle("attivo", suo);
    b.setAttribute("aria-pressed", String(suo));
  }
  try {
    localStorage.setItem(TEMA_SALVATO, scelta);
  } catch (_) {
    /* webview senza storage: il tema vale per questa sessione e basta */
  }
}

for (const b of el("scelta-tema").querySelectorAll("button")) {
  b.addEventListener("click", () => applicaTema(b.dataset.tema));
}

(() => {
  let scelta = "chiaro";
  try {
    scelta = localStorage.getItem(TEMA_SALVATO) || "chiaro";
  } catch (_) {
    /* si resta sul chiaro */
  }
  applicaTema(["auto", "chiaro", "scuro"].includes(scelta) ? scelta : "chiaro");
})();

// ------------------------------------------------------- pannello laterale

const FIANCO_SALVATO = "ocr-ita:fianco";

function mostraFianco(aperto) {
  document.body.classList.toggle("fianco-chiuso", !aperto);
  document.body.classList.toggle("fianco-aperto", aperto);
  el("mostra-fianco").setAttribute("aria-expanded", String(aperto));
  el("fianco").inert = !aperto;
  try {
    localStorage.setItem(FIANCO_SALVATO, aperto ? "1" : "0");
  } catch (_) {
    /* non e' un dato che valga un errore */
  }
}

el("mostra-fianco").addEventListener("click", () => {
  mostraFianco(document.body.classList.contains("fianco-chiuso"));
});

(() => {
  let aperto = window.innerWidth > 800;
  try {
    const salvato = localStorage.getItem(FIANCO_SALVATO);
    if (salvato !== null) aperto = salvato !== "0";
  } catch (_) {
    /* aperto e' il valore giusto di partenza */
  }
  mostraFianco(aperto);
})();

// ---------------------------------------------------------------- archivio

let archivio = [];
/** Voce dell'archivio attualmente in finestra, per evidenziarla nell'elenco. */
let sessioneCorrente = null;
el("cerca-archivio").addEventListener("input", disegnaArchivio);

/** Giorni interi che separano una voce da oggi. */
function giorniFa(secondi) {
  const soloGiorno = (d) => new Date(d.getFullYear(), d.getMonth(), d.getDate());
  return (soloGiorno(new Date()) - soloGiorno(new Date(secondi * 1000))) / 86400000;
}

/** Titolo del gruppo: l'archivio lungo si legge per periodi, non a righe. */
function periodo(secondi) {
  const scarto = giorniFa(secondi);
  if (scarto <= 0) return "Oggi";
  if (scarto === 1) return "Ieri";
  if (scarto < 7) return "Ultimi 7 giorni";
  if (scarto < 30) return "Ultimi 30 giorni";
  return "Prima";
}

/** Ora o data breve: il giorno lo dice gia' il titolo del gruppo. */
function quando(secondi) {
  const data = new Date(secondi * 1000);
  const scarto = giorniFa(secondi);
  if (scarto <= 1) return data.toLocaleTimeString("it-IT", { hour: "2-digit", minute: "2-digit" });
  if (scarto < 7) return data.toLocaleDateString("it-IT", { weekday: "long" });
  const anno = data.getFullYear() === new Date().getFullYear() ? {} : { year: "numeric" };
  return data.toLocaleDateString("it-IT", { day: "numeric", month: "short", ...anno });
}

function disegnaArchivio() {
  const elenco = el("elenco-archivio");
  elenco.replaceChildren();
  el("archivio-vuoto").hidden = archivio.length > 0;
  el("conteggio-archivio").textContent = archivio.length || "";
  const normalizza = (testo) => testo.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toLocaleLowerCase("it");
  const ricerca = normalizza(el("cerca-archivio").value.trim());
  const voci = archivio.filter((voce) => normalizza(voce.nome).includes(ricerca));
  el("archivio-nessun-risultato").hidden = !archivio.length || voci.length > 0;

  let gruppoAperto = null;
  for (const voce of voci) {
    const gruppo = periodo(voce.istante);
    if (gruppo !== gruppoAperto) {
      gruppoAperto = gruppo;
      const titolo = document.createElement("li");
      titolo.className = "gruppo-archivio occhiello";
      titolo.textContent = gruppo;
      elenco.append(titolo);
    }
    const riga = document.createElement("li");
    riga.className = "voce-archivio";
    riga.dataset.id = voce.id;
    riga.classList.toggle("corrente", voce.id === sessioneCorrente);
    const apri = document.createElement("button");
    apri.type = "button";
    apri.className = "apri-voce";
    apri.title = voce.nome;
    if (voce.id === sessioneCorrente) apri.setAttribute("aria-current", "true");

    const nome = document.createElement("span");
    nome.className = "nome";
    nome.textContent = voce.nome;

    const meta = document.createElement("span");
    meta.className = "riga-meta";
    const parti = [quando(voce.istante), `${voce.pagine} pag.`];
    for (const [indice, testo] of parti.entries()) {
      if (indice) {
        const punto = document.createElement("span");
        punto.className = "punto-meta";
        meta.append(punto);
      }
      const s = document.createElement("span");
      s.textContent = testo;
      meta.append(s);
    }
    // Il sorgente sparito si dice: quella sessione si legge ma non si rifa'.
    if (!voce.completa) {
      const punto = document.createElement("span");
      punto.className = "punto-meta";
      const avviso = document.createElement("span");
      avviso.className = "scollegato";
      avviso.append(icona("i-scollegato"));
      avviso.append(document.createTextNode("originale assente"));
      avviso.title =
        "I file di partenza non sono più al loro posto: il testo resta, le immagini no.";
      meta.append(punto, avviso);
    }

    const comandi = document.createElement("div");
    comandi.className = "comandi-voce";
    const cancella = document.createElement("button");
    cancella.type = "button";
    cancella.title = "Elimina questa sessione";
    cancella.setAttribute("aria-label", `Elimina ${voce.nome}`);
    cancella.append(icona("i-cestino"));
    cancella.addEventListener("click", async (evento) => {
      evento.stopPropagation();
      await eliminaSessione(voce);
    });
    comandi.append(cancella);

    apri.append(nome, meta);
    apri.addEventListener("click", () => apriSessione(voce));
    riga.append(apri, comandi);
    elenco.append(riga);
  }
}

async function aggiornaArchivio() {
  try {
    archivio = await invoke("archivio_elenco");
    disegnaArchivio();
  } catch (e) {
    avvisa(`Archivio non leggibile: ${e}`, true);
  }
}

async function apriSessione(voce) {
  if (inCorso || importazioneInCorso) {
    avvisa("C'è un'elaborazione in corso: interrompila prima di aprire un'altra sessione.", true);
    return;
  }
  try {
    const mancanti = await invoke("archivio_carica", { id: voce.id });
    sessioneCorrente = voce.id;
    corrente = pagine[0]?.id || null;
    segnaCorrente();
    flusso.clear();
    el("nome-sessione").textContent = voce.nome;
    disegnaArchivio();
    avvisa(
      mancanti
        ? `"${voce.nome}" ripresa. ${mancanti} file di partenza non sono più al loro posto: resta il testo, non le immagini.`
        : `"${voce.nome}" ripresa.`,
      mancanti > 0
    );
  } catch (e) {
    avvisa(String(e), true);
  }
}

async function eliminaSessione(voce) {
  try {
    await invoke("archivio_elimina", { id: voce.id });
    if (sessioneCorrente === voce.id) sessioneCorrente = null;
    avvisa(`"${voce.nome}" eliminata dall'archivio.`);
  } catch (e) {
    avvisa(String(e), true);
  }
}

// `svuota` archivia la trascrizione prima di liberare l'area di lavoro.
async function nuovaSessione() {
  if (inCorso || importazioneInCorso) {
    avvisa("Interrompi l'elaborazione in corso prima di cominciarne una nuova.", true);
    return;
  }
  try {
    await invoke("svuota");
    corrente = null;
    flusso.clear();
    sessioneCorrente = null;
    el("nome-sessione").textContent = "Nuova trascrizione";
    disegnaArchivio();
    el("scegli-file").focus();
  } catch (e) {
    avvisa(`Nuova trascrizione non aperta: ${e}`, true);
  }
}
el("nuovo-ocr").addEventListener("click", nuovaSessione);

listen("ocr://archivio", (evento) => {
  archivio = evento.payload;
  disegnaArchivio();
});

// ---------------------------------------------------------------- drag & drop

webview.getCurrentWebview().onDragDropEvent(async (evento) => {
  const tipo = evento.payload.type;
  if (tipo === "over" || tipo === "enter") {
    nodi.riquadroDrop.classList.add("sopra");
    document.body.classList.add("trascinamento");
  } else if (tipo === "drop") {
    nodi.riquadroDrop.classList.remove("sopra");
    document.body.classList.remove("trascinamento");
    const percorsi = normalizzaPercorsi(evento.payload.paths);
    if (percorsi.length && el("pannello-avanzate").hidden) await aggiungiPercorsi(percorsi);
  } else {
    nodi.riquadroDrop.classList.remove("sopra");
    document.body.classList.remove("trascinamento");
  }
});

// ---------------------------------------------------------------- avvio

(async () => {
  // Se una di queste chiamate fallisce non si deve restare con una finestra
  // vuota e muta: senza `catch` l'errore diventava una promessa rifiutata che
  // nessuno legge, e l'interfaccia non veniva nemmeno disegnata.
  try {
    aggiornaMotore(await invoke("info_motore"));
  } catch (e) {
    avvisa(`Stato del motore non leggibile: ${e}`, true);
  }
  try {
    pagine = await invoke("elenco");
  } catch (e) {
    pagine = [];
    avvisa(`Elenco delle pagine non leggibile: ${e}`, true);
  }
  disegna();
  aggiornaPiede();
  for (const p of pagine) {
    if (p.caratteri > 0) riempiTesto(p.id);
  }
  await aggiornaArchivio();
})();
