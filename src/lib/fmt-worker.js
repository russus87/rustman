// Worker dedicato a formattare/compattare il corpo fuori dal thread principale.
// Su un corpo da molti MB `JSON.parse` + `JSON.stringify` bloccherebbero la
// finestra per secondi: qui girano su un thread a parte e la UI resta viva.
// Lo stesso vale per l'XML, che viene indentato con una scansione lineare.

// Riconosce il tipo dal primo carattere significativo: `{`/`[` JSON, `<` XML.
// Non è un parser, è un indizio: se poi il contenuto non è valido, chi
// trasforma fallisce e il testo resta com'è.
export function tipoDi(testo) {
  const c = testo.trimStart()[0];
  if (c === "{" || c === "[") return "json";
  if (c === "<") return "xml";
  return "ignoto";
}

// ---------------------------------------------------------------------------
// XML
//
// Non costruisce un albero: scorre il testo una volta sola separando i tag dal
// testo che sta in mezzo, e riscrive solo gli spazi *fra* i tag. Il contenuto
// (testo, attributi, commenti, CDATA) non viene mai toccato, e un elemento che
// contiene del testo resta su una riga sola: così non si cambia il significato
// di un contenuto misto (`<p>ciao <b>tu</b></p>`).
// ---------------------------------------------------------------------------

// Divide in pezzi: { t: "apre"|"chiude"|"solo"|"altro"|"testo", s, a, b }
// dove `a`/`b` sono gli estremi nel testo originale (servono a ricopiare un
// elemento identico a com'era).
function pezziXml(x) {
  const out = [];
  let i = 0;
  while (i < x.length) {
    const lt = x.indexOf("<", i);
    if (lt < 0) {
      out.push({ t: "testo", s: x.slice(i), a: i, b: x.length });
      break;
    }
    if (lt > i) out.push({ t: "testo", s: x.slice(i, lt), a: i, b: lt });

    // Commenti, CDATA e istruzioni: si copiano fino al loro terminatore, senza
    // guardarci dentro (possono contenere `<` e `>` a volontà).
    let fine;
    if (x.startsWith("<!--", lt)) {
      fine = x.indexOf("-->", lt);
      fine = fine < 0 ? x.length : fine + 3;
      out.push({ t: "altro", s: x.slice(lt, fine), a: lt, b: fine });
    } else if (x.startsWith("<![CDATA[", lt)) {
      fine = x.indexOf("]]>", lt);
      fine = fine < 0 ? x.length : fine + 3;
      // Un CDATA è contenuto a tutti gli effetti: vale come testo.
      out.push({ t: "testo", s: x.slice(lt, fine), a: lt, b: fine });
    } else if (x.startsWith("<?", lt)) {
      fine = x.indexOf("?>", lt);
      fine = fine < 0 ? x.length : fine + 2;
      out.push({ t: "altro", s: x.slice(lt, fine), a: lt, b: fine });
    } else {
      // Tag normale: il `>` va cercato fuori dalle virgolette, perché un
      // attributo può contenerlo (`titolo="a > b"`).
      let j = lt + 1;
      let apice = null;
      while (j < x.length) {
        const c = x[j];
        if (apice) {
          if (c === apice) apice = null;
        } else if (c === '"' || c === "'") {
          apice = c;
        } else if (c === ">") {
          break;
        }
        j++;
      }
      fine = Math.min(j + 1, x.length);
      const s = x.slice(lt, fine);
      const t = s.startsWith("</") ? "chiude" : s.endsWith("/>") || s.startsWith("<!") ? "solo" : "apre";
      out.push({ t, s, a: lt, b: fine });
    }
    i = fine;
  }
  return out;
}

export function formattaXml(x, unita = "  ") {
  const p = pezziXml(x);
  const out = [];
  let primo = true;
  const riga = (liv, s) => {
    out.push(primo ? s : "\n" + unita.repeat(Math.max(liv, 0)) + s);
    primo = false;
  };

  // Un giro solo per sapere, di ogni apertura, dove si chiude e se fra i suoi
  // figli diretti c'è del testo (contenuto misto: `<d>uno <e>due</e> tre</d>`,
  // dove gli spazi contano). Cercare le due cose elemento per elemento
  // costerebbe un tempo quadratico su un documento con centinaia di migliaia
  // di tag.
  const chiude = new Int32Array(p.length).fill(-1);
  const misto = new Uint8Array(p.length);
  const pila = [];
  for (let j = 0; j < p.length; j++) {
    const t = p[j].t;
    if (t === "apre") pila.push(j);
    else if (t === "chiude") {
      const i = pila.pop();
      if (i !== undefined) chiude[i] = j;
    } else if (t === "testo" && p[j].s.trim() && pila.length) {
      misto[pila[pila.length - 1]] = 1;
    }
  }

  function blocco(da, a, liv) {
    let i = da;
    while (i < a) {
      const t = p[i];
      if (t.t === "testo") {
        if (t.s.trim()) riga(liv, t.s.trim());
        i++;
        continue;
      }
      if (t.t !== "apre") {
        riga(liv, t.s);
        i++;
        continue;
      }
      const c = chiude[i];
      if (c < 0 || c >= a) {
        // Documento non bilanciato: meglio non indovinare dove finisce.
        riga(liv, t.s);
        i++;
      } else if (misto[i]) {
        // Ricopiato identico, spazi compresi: riscriverlo cambierebbe il
        // contenuto, non solo il suo aspetto.
        riga(liv, x.slice(t.a, p[c].b));
        i = c + 1;
      } else if (c === i + 1) {
        riga(liv, t.s + p[c].s); // elemento vuoto: resta su una riga
        i = c + 2;
      } else {
        riga(liv, t.s);
        blocco(i + 1, c, liv + 1);
        riga(liv, p[c].s);
        i = c + 1;
      }
    }
  }

  blocco(0, p.length, 0);
  return out.join("");
}

// Toglie solo gli spazi *fra* i tag: il testo dentro gli elementi resta identico.
export function compattaXml(x) {
  return pezziXml(x)
    .map((p) => (p.t === "testo" && !p.s.trim() ? "" : p.s))
    .join("");
}

// ---------------------------------------------------------------------------

// Trasforma `testo` secondo `azione` ("formatta" | "compatta"). `tipo` può
// essere "json", "xml" o "auto" (riconoscimento dal primo carattere).
// Lancia se il testo non è del tipo che dichiara di essere.
export function trasformaTesto(testo, azione = "formatta", tipo = "auto") {
  const t = tipo === "auto" ? tipoDi(testo) : tipo;
  if (t === "xml") {
    if (!testo.trimStart().startsWith("<")) throw new Error("non sembra XML");
    return azione === "compatta" ? compattaXml(testo) : formattaXml(testo);
  }
  const dato = JSON.parse(testo);
  return azione === "compatta" ? JSON.stringify(dato) : JSON.stringify(dato, null, 2);
}

// Il gestore si installa solo dentro un worker: questo modulo viene importato
// anche dai test in Node, e in una pagina `self` sarebbe la finestra.
if (typeof WorkerGlobalScope !== "undefined" && self instanceof WorkerGlobalScope) {
  self.onmessage = (e) => {
    const { id, azione, testo, tipo } = e.data;
    try {
      self.postMessage({ id, ok: true, testo: trasformaTesto(testo, azione, tipo) });
    } catch (err) {
      self.postMessage({ id, ok: false, errore: String(err?.message ?? err) });
    }
  };
}
