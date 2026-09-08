<script>
  // Editor di codice basato su CodeMirror 6.
  //
  // Perche' non una <textarea>: il browser fa il layout dell'intero contenuto,
  // quindi su un corpo da molti MB scroll, cursore e selezione diventano
  // inusabili. CodeMirror tiene il testo in una struttura ad albero e disegna
  // solo le righe visibili, percio' regge documenti da decine di MB.
  //
  // Due accorgimenti sono specifici per i corpi enormi:
  //  - il testo NON viene rispecchiato nello stato Svelte a ogni tasto:
  //    ricostruire la stringa dal documento costa quanto la sua dimensione,
  //    quindi la propagazione e' ritardata (`ritardo`) e forzata da flush();
  //  - sopra `SOGLIA_RICCA` le estensioni costose (evidenziazione, folding,
  //    parentesi, riga attiva) vengono spente: restano numeri di riga,
  //    ricerca, undo e a-capo, che non dipendono dalla dimensione.

  import { untrack } from "svelte";
  import { EditorState, Compartment } from "@codemirror/state";
  import {
    EditorView, keymap, lineNumbers, highlightActiveLine, highlightActiveLineGutter,
    drawSelection, dropCursor, rectangularSelection, crosshairCursor,
    highlightSpecialChars, placeholder as segnaposto,
  } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
  import { search, searchKeymap, highlightSelectionMatches } from "@codemirror/search";
  import {
    syntaxHighlighting, HighlightStyle, bracketMatching,
    foldGutter, foldKeymap, indentOnInput, indentUnit,
  } from "@codemirror/language";
  import { json } from "@codemirror/lang-json";
  import { tags as tg } from "@lezer/highlight";

  let {
    valore = "",
    onCambia,
    onStato,
    chiave = null,          // cambiandola l'editor ricarica da zero (nuovo tab)
    testo = "",             // testo del segnaposto
    soloLettura = false,
    aCapo = true,
    forzaRicca = false,     // riaccende l'evidenziazione oltre SOGLIA_RICCA
    ritardo = 350,          // ms di attesa prima di propagare il testo all'esterno
    scorciatoie = [],       // [{ key, run }] aggiunte in cima alla keymap
  } = $props();

  // Oltre questa dimensione l'editor passa in "modalita' leggera".
  // Misurato su questo componente: fino a ~14 MB con evidenziazione attiva una
  // battuta di tasto costa ~4 ms, oltre sale a ~23 ms (il parser non sta piu'
  // dietro). 4 MB lascia margine anche su webview piu' lente di Chromium;
  // `forzaRicca` permette di riaccenderla comunque.
  const SOGLIA_RICCA = 4 * 1024 * 1024;
  // Oltre questa lunghezza di riga l'a-capo viene spento da solo: per mandare a
  // capo CodeMirror deve calcolare le interruzioni su tutta la riga, e su un
  // JSON compattato da 13 MB (una riga sola) sono ~2 s di blocco al primo
  // disegno, contro ~50 ms senza a-capo. Torna attivo appena si formatta.
  const MAX_RIGA_ACAPO = 20000;

  let contenitore = $state(null);
  let vista = null;
  let ultimoEmesso = valore;   // ultimo testo propagato: evita di ricaricare il
                               // documento quando il cambio arriva da noi stessi
  let timer = null;
  let riccaAttiva = null;      // stato corrente della modalita' ricca
  let acapoAttivo = null;      // a-capo realmente applicato (vedi MAX_RIGA_ACAPO)
  let rigaLunga = false;       // c'e' almeno una riga oltre MAX_RIGA_ACAPO

  const compAcapo = new Compartment();
  const compRicca = new Compartment();
  const compSolaLettura = new Compartment();

  const evidenziazione = HighlightStyle.define([
    { tag: tg.propertyName, color: "var(--syn-key)" },
    { tag: [tg.string, tg.special(tg.string)], color: "var(--syn-str)" },
    { tag: tg.number, color: "var(--syn-num)" },
    { tag: [tg.bool, tg.null, tg.keyword], color: "var(--syn-bool)" },
    { tag: [tg.separator, tg.squareBracket, tg.brace, tg.punctuation], color: "var(--syn-punct)" },
    { tag: tg.invalid, color: "var(--red)" },
  ]);

  const tema = EditorView.theme({
    "&": { color: "var(--txt)", backgroundColor: "transparent", height: "100%", fontSize: "13px" },
    "&.cm-focused": { outline: "none" },
    ".cm-scroller": { fontFamily: "var(--mono)", lineHeight: "1.7", overflow: "auto" },
    ".cm-content": { padding: "12px 0", caretColor: "var(--accent)" },
    ".cm-gutters": {
      backgroundColor: "transparent", color: "var(--txt-faint)",
      border: "none", borderRight: "1px solid var(--border)", paddingRight: "4px",
    },
    ".cm-lineNumbers .cm-gutterElement": { padding: "0 8px 0 14px" },
    ".cm-activeLineGutter": { backgroundColor: "transparent", color: "var(--txt-dim)" },
    ".cm-activeLine": { backgroundColor: "var(--accent-soft)" },
    ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)", borderLeftWidth: "2px" },
    "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection":
      { backgroundColor: "var(--accent-line)" },
    ".cm-selectionMatch": { backgroundColor: "var(--accent-soft)" },
    ".cm-foldPlaceholder": {
      backgroundColor: "var(--panel-3)", border: "1px solid var(--border-2)",
      color: "var(--txt-dim)", borderRadius: "4px", padding: "0 6px",
    },
    ".cm-panels": { backgroundColor: "var(--panel)", color: "var(--txt)" },
    ".cm-panels.cm-panels-top": { borderBottom: "1px solid var(--border)" },
    ".cm-panels.cm-panels-bottom": { borderTop: "1px solid var(--border)" },
    ".cm-searchMatch": { backgroundColor: "var(--accent-soft)", outline: "1px solid var(--accent-line)" },
    ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "var(--accent-line)" },
    ".cm-textfield": {
      backgroundColor: "var(--panel-2)", color: "var(--txt)",
      border: "1px solid var(--border)", borderRadius: "6px", padding: "4px 7px",
    },
    ".cm-button": {
      backgroundColor: "var(--panel-3)", backgroundImage: "none", color: "var(--txt)",
      border: "1px solid var(--border-2)", borderRadius: "6px",
    },
    ".cm-tooltip": {
      backgroundColor: "var(--panel)", border: "1px solid var(--border-2)", color: "var(--txt)",
    },
  });

  // Estensioni accese solo sotto SOGLIA_RICCA: tutte fanno lavoro proporzionale
  // al documento (parsing, ricerca delle parentesi, calcolo dei fold).
  const estensioniRicche = [
    json(),
    syntaxHighlighting(evidenziazione),
    bracketMatching(),
    foldGutter(),
    indentOnInput(),
    highlightActiveLine(),
    highlightActiveLineGutter(),
    highlightSelectionMatches(),
  ];

  function ricca(lunghezza) {
    return forzaRicca || lunghezza <= SOGLIA_RICCA;
  }

  // Lunghezza della riga piu' lunga, stimata campionando posizioni sparse:
  // una riga enorme occupa una fetta grande del documento, quindi viene
  // pescata quasi certamente. `lineAt` costa O(log n), il campionamento e'
  // trascurabile anche a ogni battuta di tasto.
  function rigaMassima(doc) {
    let max = 0;
    for (let i = 0; i < 24; i++) {
      max = Math.max(max, doc.lineAt(Math.floor((doc.length * i) / 24)).length);
    }
    return max;
  }

  function acapoVoluto(doc) {
    rigaLunga = rigaMassima(doc) > MAX_RIGA_ACAPO;
    return aCapo && !rigaLunga;
  }

  // Stessa domanda ma su una stringa grezza, prima che esista un documento:
  // costruire un EditorState solo per misurare le righe raddoppierebbe il
  // lavoro di caricamento su un corpo da 19 MB.
  function rigaLungaTesto(t) {
    let inizio = 0;
    while (inizio < t.length) {
      const nl = t.indexOf("\n", inizio);
      if (nl < 0) return t.length - inizio > MAX_RIGA_ACAPO;
      if (nl - inizio > MAX_RIGA_ACAPO) return true;
      inizio = nl + 1;
    }
    return false;
  }

  function riportaStato() {
    if (!vista || !onStato) return;
    onStato({
      righe: vista.state.doc.lines,
      caratteri: vista.state.doc.length,
      ricca: riccaAttiva,
      aCapo: acapoAttivo,
      rigaLunga,
    });
  }

  // Propaga il testo verso il genitore. `subito` salta il debounce (invio,
  // salvataggio, perdita del focus): serve perche' il body dev'essere
  // aggiornato prima che qualcuno lo legga.
  function propaga(subito = false) {
    if (timer) { clearTimeout(timer); timer = null; }
    if (!vista) return;
    const invia = () => {
      timer = null;
      if (!vista) return;
      const t = vista.state.doc.toString();
      if (t === ultimoEmesso) return;
      ultimoEmesso = t;
      onCambia?.(t);
    };
    subito ? invia() : (timer = setTimeout(invia, ritardo));
  }

  export function flush() {
    propaga(true);
  }

  // Sostituisce il contenuto mantenendo la cronologia (l'operazione resta
  // annullabile con Ctrl+Z). Usata dopo "Formatta" / "Compatta".
  export function imposta(nuovo) {
    if (!vista || nuovo === vista.state.doc.toString()) return;
    vista.dispatch({
      changes: { from: 0, to: vista.state.doc.length, insert: nuovo },
      selection: { anchor: 0 },
      scrollIntoView: true,
    });
    propaga(true);
  }

  // Testo corrente senza passare dal debounce.
  export function contenuto() {
    return vista ? vista.state.doc.toString() : ultimoEmesso;
  }

  export function focus() {
    vista?.focus();
  }

  // Elenco delle estensioni: ricreato a ogni nuovo stato (i Compartment sono
  // solo chiavi, riusarli tra stati diversi e' corretto).
  function estensioni() {
    return [
      lineNumbers(),
      history(),
      drawSelection(),
      dropCursor(),
      highlightSpecialChars(),
      rectangularSelection(),
      crosshairCursor(),
      EditorState.allowMultipleSelections.of(true),
      EditorState.tabSize.of(2),
      indentUnit.of("  "),
      search({ top: true }),
      keymap.of([
        ...scorciatoie,
        ...searchKeymap,
        ...defaultKeymap,
        ...historyKeymap,
        ...foldKeymap,
        indentWithTab,
      ]),
      segnaposto(testo),
      compAcapo.of(acapoAttivo ? EditorView.lineWrapping : []),
      compRicca.of(riccaAttiva ? estensioniRicche : []),
      compSolaLettura.of(EditorState.readOnly.of(soloLettura)),
      tema,
      EditorView.domEventHandlers({ blur: () => { propaga(true); return false; } }),
      EditorView.updateListener.of((u) => {
        if (!u.docChanged) return;
        // Il passaggio sopra/sotto soglia riconfigura le estensioni pesanti
        // senza ricostruire lo stato (la cronologia resta intatta).
        const effetti = [];
        const ora = ricca(u.state.doc.length);
        if (ora !== riccaAttiva) {
          riccaAttiva = ora;
          effetti.push(compRicca.reconfigure(ora ? estensioniRicche : []));
        }
        const w = acapoVoluto(u.state.doc);
        if (w !== acapoAttivo) {
          acapoAttivo = w;
          effetti.push(compAcapo.reconfigure(w ? EditorView.lineWrapping : []));
        }
        if (effetti.length) u.view.dispatch({ effects: effetti });
        propaga();
        riportaStato();
      }),
    ];
  }

  function nuovoStato(doc) {
    riccaAttiva = ricca(doc.length);
    rigaLunga = rigaLungaTesto(doc);
    acapoAttivo = aCapo && !rigaLunga;
    return EditorState.create({ doc, extensions: estensioni() });
  }

  // Montaggio: una sola volta. Il testo iniziale viene letto senza tracciarlo,
  // altrimenti ogni modifica ricostruirebbe l'editor da capo.
  $effect(() => {
    if (!contenitore) return;
    const iniziale = untrack(() => valore ?? "");
    vista = new EditorView({ state: nuovoStato(iniziale), parent: contenitore });
    ultimoEmesso = iniziale;
    riportaStato();
    return () => {
      if (timer) { clearTimeout(timer); timer = null; }
      vista?.destroy();
      vista = null;
    };
  });

  // Cambio di tab: ricarica il documento e azzera la cronologia dell'undo,
  // altrimenti Ctrl+Z riporterebbe il testo di un'altra richiesta.
  let chiaveVista = chiave;
  $effect(() => {
    const k = chiave;
    if (!vista || k === chiaveVista) return;
    chiaveVista = k;
    const nuovo = untrack(() => valore ?? "");
    if (timer) { clearTimeout(timer); timer = null; }
    ultimoEmesso = nuovo;
    vista.setState(nuovoStato(nuovo));
    riportaStato();
  });

  // Valore cambiato dall'esterno (non da noi): allinea il documento.
  $effect(() => {
    const v = valore ?? "";
    if (!vista || v === ultimoEmesso) return;
    ultimoEmesso = v;
    vista.dispatch({ changes: { from: 0, to: vista.state.doc.length, insert: v } });
  });

  $effect(() => {
    void forzaRicca;
    if (!vista) return;
    const r = ricca(vista.state.doc.length);
    if (r === riccaAttiva) return;
    riccaAttiva = r;
    vista.dispatch({ effects: compRicca.reconfigure(r ? estensioniRicche : []) });
    riportaStato();
  });

  $effect(() => {
    void aCapo;
    if (!vista) return;
    const w = acapoVoluto(vista.state.doc);
    if (w === acapoAttivo) return;
    acapoAttivo = w;
    vista.dispatch({ effects: compAcapo.reconfigure(w ? EditorView.lineWrapping : []) });
    riportaStato();
  });

  $effect(() => {
    const r = soloLettura;
    vista?.dispatch({ effects: compSolaLettura.reconfigure(EditorState.readOnly.of(r)) });
  });
</script>

<div class="cm-host" bind:this={contenitore}></div>

<style>
  .cm-host {
    flex: 1;
    min-height: 0;
    height: 100%;
    overflow: hidden;
    background: var(--bg);
  }
</style>
