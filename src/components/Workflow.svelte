<script>
  // Canvas del workflow: editor visuale a nodi di una catena di chiamate.
  // I nodi sono richieste collegate da archi; il salvataggio persiste
  // posizioni + topologia (nodi/archi) e rigenera i `passi` per la CLI.
  import { SvelteFlow, Background, Controls } from "@xyflow/svelte";
  import "@xyflow/svelte/dist/style.css";
  import RequestNode from "./flow/WfRequestNode.svelte";
  import StartNode from "./flow/WfStartNode.svelte";
  import { t } from "../lib/i18n.svelte.js";
  import { indiciPassi, passoDelNodo } from "../lib/runner.js";

  let { catena, albero, onSalva, onEsegui } = $props();

  const nodeTypes = { request: RequestNode, start: StartNode };

  // ---- Elenco piatto delle richieste disponibili (per aggiungere nodi) ----
  function elenco(figli, acc) {
    for (const n of figli) {
      if (n.tipo === "richiesta") acc.push({ file: n.file, nome: n.richiesta.nome || n.file, metodo: n.richiesta.metodo });
      else if (n.tipo === "cartella") elenco(n.figli, acc);
    }
    return acc;
  }
  const richieste = $derived(albero.flatMap((c) => elenco(c.figli, [])));
  function info(file) { return richieste.find((r) => r.file === file); }
  function classeMetodo(m) {
    if (m === "GET") return "get"; if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put"; if (m === "DELETE") return "del";
    return "";
  }

  // ---- Conversione catena → grafo xyflow ----
  function datiNodo(file, label) {
    const r = info(file);
    return {
      file,
      label: label || r?.nome || file,
      metodo: r?.metodo || "",
      classe: classeMetodo(r?.metodo),
      mancante: !r,
    };
  }
  // Se la catena non ha ancora nodi, li sintetizza dai passi lineari (migrazione).
  function daPassi(passi) {
    const nodi = [{ id: "start", tipo: "start", file: "", label: "Start", x: 40, y: 150 }];
    const archi = [];
    let prec = "start";
    (passi || []).forEach((p, i) => {
      const id = `n${i}`;
      nodi.push({ id, tipo: "request", file: p.file, label: "", x: 240 + i * 220, y: 130 });
      archi.push({ da: prec, a: id, ramo: "" });
      prec = id;
    });
    return { nodi, archi };
  }
  function iniziale() {
    let nodi = catena.nodi, archi = catena.archi;
    if (!nodi || !nodi.length) ({ nodi, archi } = daPassi(catena.passi));
    if (!nodi.some((n) => n.tipo === "start")) nodi = [{ id: "start", tipo: "start", file: "", label: "Start", x: 40, y: 150 }, ...nodi];
    const nodes = nodi.map((n) => ({
      id: n.id,
      type: n.tipo === "start" ? "start" : "request",
      position: { x: n.x || 0, y: n.y || 0 },
      data: n.tipo === "start" ? { label: "Start" } : datiNodo(n.file, n.label),
      deletable: n.tipo !== "start",
    }));
    const edges = (archi || []).map((a, i) => ({
      id: `e${i}_${a.da}_${a.a}`, source: a.da, target: a.a,
      data: { ramo: a.ramo || "", condizione: a.condizione || null },
      label: etichettaCond(a.condizione), animated: true,
    }));
    return { nodes, edges };
  }

  // Etichetta breve mostrata sull'arco quando ha una condizione (branch).
  function etichettaCond(c) {
    if (!c) return undefined;
    const campo = c.tipo === "status" ? "status" : (c.campo || c.tipo);
    return `if ${campo} ${c.operatore} ${c.atteso}`;
  }

  const init = iniziale();
  let nodes = $state(init.nodes);
  let edges = $state(init.edges);
  let nome = $state(catena.nome);
  // Contatore id: oltre il massimo "nN" già presente.
  let contatore = Math.max(0, ...nodes.map((n) => Number(String(n.id).replace(/^n/, "")) || 0)) + 1;

  // Config di nodo (skip/catture/al_fallimento/loop), indicizzata per id di
  // nodo: due nodi che puntano alla stessa richiesta devono poter avere
  // impostazioni diverse. I flussi salvati prima l'abbinavano al file, quindi
  // in lettura si migra duplicandola — copie profonde, altrimenti i due nodi
  // continuerebbero a condividere gli stessi oggetti annidati.
  function cfgVuota() {
    return { condizione: null, catture: [], al_fallimento: "", ciclo: null };
  }
  function cfgDaPasso(p) {
    if (!p) return cfgVuota();
    return {
      condizione: p.condizione ? { ...p.condizione } : null,
      catture: (p.catture || []).map((c) => ({ ...c })),
      al_fallimento: p.al_fallimento || "",
      ciclo: p.ciclo
        ? { ...p.ciclo, esci_se: p.ciclo.esci_se ? { ...p.ciclo.esci_se } : null }
        : null,
    };
  }
  let cfg = $state({});
  {
    const indici = indiciPassi(catena.passi);
    for (const n of init.nodes) {
      if (n.type !== "request") continue;
      cfg[n.id] = cfgDaPasso(passoDelNodo(indici, { id: n.id, file: n.data.file }));
    }
  }
  function cfgDi(id) {
    if (!cfg[id]) cfg[id] = cfgVuota();
    return cfg[id];
  }
  let guidaAperta = $state(false);

  // ---- Conversione grafo xyflow → catena ----
  function verso() {
    const nodi = nodes.map((n) => ({
      id: n.id,
      tipo: n.type,
      file: n.data.file || "",
      label: n.type === "start" ? "Start" : (n.data.label || ""),
      x: Math.round(n.position.x),
      y: Math.round(n.position.y),
    }));
    const archi = edges.map((e) => ({ da: e.source, a: e.target, ramo: e.data?.ramo || "", condizione: e.data?.condizione || null }));
    // I passi portano la config per-nodo (catture/skip/al_fallimento) per la CLI.
    const passi = nodi
      .filter((n) => n.tipo === "request" && n.file)
      .map((n) => ({
        nodo: n.id,
        file: n.file,
        condizione: cfg[n.id]?.condizione || null,
        catture: cfg[n.id]?.catture || [],
        al_fallimento: cfg[n.id]?.al_fallimento || "",
        ciclo: cfg[n.id]?.ciclo || null,
      }));
    return { ...catena, nome, nodi, archi, passi };
  }

  // ---- Azioni ----
  function aggiungi(e) {
    const file = e.target.value;
    if (!file) return;
    const id = `n${contatore++}`;
    cfg[id] = cfgVuota();
    nodes = [...nodes, {
      id, type: "request",
      position: { x: 260 + (nodes.length % 4) * 40, y: 120 + nodes.length * 20 },
      data: datiNodo(file, ""), deletable: true,
    }];
    e.target.value = "";
  }
  // Crea un arco quando si trascina fra due handle.
  function connetti(conn) {
    if (!conn.source || !conn.target || conn.source === conn.target) return;
    if (edges.some((e) => e.source === conn.source && e.target === conn.target)) return;
    edges = [...edges, {
      id: `e_${conn.source}_${conn.target}_${edges.length}`,
      source: conn.source, target: conn.target,
      data: { ramo: "", condizione: null }, label: undefined, animated: true,
    }];
  }
  function salva() { onSalva(verso()); }
  function esegui() { onSalva(verso()); onEsegui(verso()); }

  // ---- Selezione / inspector ----
  const tipiCond = ["status", "json", "var"];
  const operatori = ["==", "!=", "<", ">", "contiene"];
  const fontiCattura = ["json", "header", "body"];
  let arcoSel = $state(null); // id dell'arco selezionato
  let nodoSel = $state(null); // id del nodo selezionato
  const arco = $derived(edges.find((e) => e.id === arcoSel) || null);
  const nodo = $derived(nodes.find((n) => n.id === nodoSel) || null);
  const nodoCfg = $derived(nodo && nodo.type === "request" ? (cfg[nodo.id] || null) : null);

  function deseleziona() { arcoSel = null; nodoSel = null; }
  // xyflow passa un oggetto { edge, event } / { node, event }.
  function selezionaArco({ edge }) { nodoSel = null; arcoSel = edge?.id ?? null; }
  function selezionaNodo({ node }) {
    arcoSel = null;
    if (node && node.type !== "start") { cfgDi(node.id); nodoSel = node.id; }
    else nodoSel = null;
  }
  function aggiornaArco(patch) {
    edges = edges.map((e) => {
      if (e.id !== arcoSel) return e;
      const cond = patch === null ? null : { ...(e.data?.condizione || { tipo: "status", campo: "", operatore: "==", atteso: "200" }), ...patch };
      return { ...e, data: { ...e.data, condizione: cond }, label: etichettaCond(cond) };
    });
  }
  function nomeNodo(id) {
    const n = nodes.find((x) => x.id === id);
    return n ? (n.type === "start" ? "Start" : n.data.label) : id;
  }
  // Catture del nodo selezionato.
  function aggiungiCattura() { nodoCfg.catture = [...nodoCfg.catture, { variabile: "", da: "json", campo: "" }]; }
  function rimuoviCattura(i) { nodoCfg.catture = nodoCfg.catture.filter((_, j) => j !== i); }
  function toggleSkip(on) {
    nodoCfg.condizione = on ? { tipo: "status", campo: "", operatore: "==", atteso: "200" } : null;
  }

  // ---- Ciclo (loop) sul nodo ----
  const CICLO_DEFAULT = {
    sorgente: "volte", volte: 5, lista: "", concorrenza: 1,
    ritardo_ms: 0, esci_se: null, al_fallimento: "",
  };
  function toggleCiclo(on) {
    nodoCfg.ciclo = on ? { ...CICLO_DEFAULT } : null;
  }
  function toggleEsciSe(on) {
    nodoCfg.ciclo.esci_se = on ? { tipo: "status", campo: "", operatore: "!=", atteso: "200" } : null;
  }
  function intero(v, min) {
    const n = Math.trunc(Number(v));
    return Number.isFinite(n) ? Math.max(min, n) : min;
  }
  // Badge mostrato sul nodo. Legge i singoli campi (non l'oggetto) così la
  // reattività scatta anche quando cambia solo il numero di giri.
  function etichettaCiclo(c) {
    if (!c) return "";
    const quanti = c.sorgente === "foreach" ? (c.lista || "lista") : String(c.volte ?? 0);
    const conc = Number(c.concorrenza) > 1 ? ` ×${c.concorrenza}` : "";
    return `↻ ${quanti}${conc}`;
  }
  // Riporta il badge nei dati dei nodi quando la configurazione cambia.
  $effect(() => {
    const attesi = nodes.map((n) => (n.type === "request" ? etichettaCiclo(cfg[n.id]?.ciclo) : ""));
    if (nodes.every((n, i) => (n.data.ciclo || "") === attesi[i])) return;
    nodes = nodes.map((n, i) => ({ ...n, data: { ...n.data, ciclo: attesi[i] } }));
  });
</script>

<div class="wf">
  <div class="wf-bar">
    <input class="wf-nome" bind:value={nome} spellcheck="false" />
    <select class="wf-add" onchange={aggiungi}>
      <option value="">+ {t("Aggiungi passo…") || "Aggiungi richiesta"}</option>
      {#each richieste as r}<option value={r.file}>{r.nome}</option>{/each}
    </select>
    <span class="wf-hint">Trascina per collegare · Canc per eliminare</span>
    <div class="wf-sp"></div>
    <button class="wf-info" title="Guida al canvas" aria-label="Guida" onclick={() => (guidaAperta = true)}>ⓘ</button>
    <button class="wf-btn ghost" onclick={salva}>Salva</button>
    <button class="wf-btn run" onclick={esegui} disabled={nodes.filter((n) => n.type === "request").length === 0}>▶ Esegui</button>
  </div>

  <div class="wf-canvas">
    <SvelteFlow bind:nodes bind:edges {nodeTypes} fitView deleteKey={["Backspace", "Delete"]}
      proOptions={{ hideAttribution: true }}
      onconnect={connetti}
      onedgeclick={selezionaArco}
      onnodeclick={selezionaNodo}
      onpaneclick={deseleziona}>
      <Background gap={18} />
      <Controls showLock={false} />
    </SvelteFlow>

    {#if arco}
      <div class="wf-insp">
        <div class="insp-h">
          Ramo <span class="insp-path">{nomeNodo(arco.source)} → {nomeNodo(arco.target)}</span>
          <span class="insp-x" onclick={() => (arcoSel = null)}>✕</span>
        </div>
        <label class="insp-ck">
          <input type="checkbox" checked={!!arco.data?.condizione}
            onchange={(e) => aggiornaArco(e.currentTarget.checked ? {} : null)} />
          Segui solo se…
        </label>
        {#if arco.data?.condizione}
          {@const c = arco.data.condizione}
          <div class="insp-row">
            <select value={c.tipo} onchange={(e) => aggiornaArco({ tipo: e.currentTarget.value })}>
              {#each tipiCond as tp}<option value={tp}>{tp}</option>{/each}
            </select>
            {#if c.tipo !== "status"}
              <input placeholder={c.tipo === "json" ? "data.ok" : "nome var"} value={c.campo}
                oninput={(e) => aggiornaArco({ campo: e.currentTarget.value })} />
            {/if}
          </div>
          <div class="insp-row">
            <select value={c.operatore} onchange={(e) => aggiornaArco({ operatore: e.currentTarget.value })}>
              {#each operatori as o}<option value={o}>{o}</option>{/each}
            </select>
            <input placeholder="atteso" value={c.atteso}
              oninput={(e) => aggiornaArco({ atteso: e.currentTarget.value })} />
          </div>
          <p class="insp-hint">La condizione è valutata sulla risposta del nodo sorgente.</p>
        {/if}
      </div>
    {/if}

    {#if nodo && nodoCfg}
      <div class="wf-insp">
        <div class="insp-h">
          Nodo <span class="insp-path">{nodo.data.label}</span>
          <span class="insp-x" onclick={() => (nodoSel = null)}>✕</span>
        </div>

        <div class="insp-sub">Cattura variabili</div>
        {#each nodoCfg.catture as c, j}
          <div class="insp-row">
            <input placeholder="variabile" value={c.variabile} oninput={(e) => (c.variabile = e.currentTarget.value)} />
            <select value={c.da} onchange={(e) => (c.da = e.currentTarget.value)}>
              {#each fontiCattura as f}<option value={f}>{f}</option>{/each}
            </select>
            {#if c.da !== "body"}
              <input placeholder={c.da === "json" ? "data.id" : "header"} value={c.campo} oninput={(e) => (c.campo = e.currentTarget.value)} />
            {/if}
            <span class="insp-x sm" onclick={() => rimuoviCattura(j)}>✕</span>
          </div>
        {/each}
        <button class="insp-add" onclick={aggiungiCattura}>+ cattura</button>

        <div class="insp-sub">Al fallimento</div>
        <div class="insp-row">
          <select value={nodoCfg.al_fallimento} onchange={(e) => (nodoCfg.al_fallimento = e.currentTarget.value)}>
            <option value="">Ferma il flusso</option>
            <option value="continua">Continua comunque</option>
          </select>
        </div>

        <div class="insp-sub">Loop</div>
        <label class="insp-ck">
          <input type="checkbox" checked={!!nodoCfg.ciclo} onchange={(e) => toggleCiclo(e.currentTarget.checked)} />
          Ripeti questo nodo
        </label>
        {#if nodoCfg.ciclo}
          {@const cl = nodoCfg.ciclo}
          <div class="insp-row">
            <select value={cl.sorgente} onchange={(e) => (cl.sorgente = e.currentTarget.value)}>
              <option value="volte">N volte</option>
              <option value="foreach">per ogni…</option>
            </select>
            {#if cl.sorgente === "foreach"}
              <input placeholder="data.items" value={cl.lista} oninput={(e) => (cl.lista = e.currentTarget.value)} />
            {:else}
              <input type="number" min="0" style="max-width:80px" value={cl.volte}
                oninput={(e) => (cl.volte = intero(e.currentTarget.value, 0))} />
              <span class="insp-u">giri</span>
            {/if}
          </div>
          <div class="insp-row">
            <select value={Number(cl.concorrenza) > 1 ? "par" : "seq"}
              onchange={(e) => (cl.concorrenza = e.currentTarget.value === "par" ? Math.max(2, Number(cl.concorrenza) || 2) : 1)}>
              <option value="seq">in sequenza</option>
              <option value="par">in concorrenza</option>
            </select>
            {#if Number(cl.concorrenza) > 1}
              <input type="number" min="2" style="max-width:70px" value={cl.concorrenza}
                oninput={(e) => (cl.concorrenza = intero(e.currentTarget.value, 2))} />
              <span class="insp-u">insieme</span>
            {:else}
              <input type="number" min="0" style="max-width:80px" value={cl.ritardo_ms}
                oninput={(e) => (cl.ritardo_ms = intero(e.currentTarget.value, 0))} />
              <span class="insp-u">ms di pausa</span>
            {/if}
          </div>
          <div class="insp-row">
            <select value={cl.al_fallimento} onchange={(e) => (cl.al_fallimento = e.currentTarget.value)}>
              <option value="">Se un giro fallisce: ferma</option>
              <option value="continua">Se un giro fallisce: continua</option>
            </select>
          </div>
          <label class="insp-ck">
            <input type="checkbox" checked={!!cl.esci_se} onchange={(e) => toggleEsciSe(e.currentTarget.checked)} />
            Esci dal loop se…
          </label>
          {#if cl.esci_se}
            {@const u = cl.esci_se}
            <div class="insp-row">
              <select value={u.tipo} onchange={(e) => (u.tipo = e.currentTarget.value)}>
                {#each tipiCond as tp}<option value={tp}>{tp}</option>{/each}
              </select>
              {#if u.tipo !== "status"}<input placeholder={u.tipo === "json" ? "data.fine" : "var"} value={u.campo} oninput={(e) => (u.campo = e.currentTarget.value)} />{/if}
            </div>
            <div class="insp-row">
              <select value={u.operatore} onchange={(e) => (u.operatore = e.currentTarget.value)}>{#each operatori as o}<option value={o}>{o}</option>{/each}</select>
              <input placeholder="atteso" value={u.atteso} oninput={(e) => (u.atteso = e.currentTarget.value)} />
            </div>
          {/if}
          <p class="insp-hint">
            Dentro il giro sono disponibili <code>{"{{$loopIndex}}"}</code> (da 0),
            <code>{"{{$loopCount}}"}</code>{#if cl.sorgente === "foreach"} e <code>{"{{$loopItem}}"}</code>{/if}
            in URL, header e corpo.
            {#if cl.sorgente === "foreach"}La lista è letta dalla risposta del nodo precedente.{/if}
            {#if Number(cl.concorrenza) > 1}In concorrenza ogni giro lavora su una copia delle variabili: le catture dell'ultimo giro vincono, e l'uscita anticipata non annulla i giri già partiti.{/if}
          </p>
        {/if}

        <label class="insp-ck" style="margin-top:4px">
          <input type="checkbox" checked={!!nodoCfg.condizione} onchange={(e) => toggleSkip(e.currentTarget.checked)} />
          Esegui solo se… (skip)
        </label>
        {#if nodoCfg.condizione}
          {@const c = nodoCfg.condizione}
          <div class="insp-row">
            <select value={c.tipo} onchange={(e) => (c.tipo = e.currentTarget.value)}>
              {#each tipiCond as tp}<option value={tp}>{tp}</option>{/each}
            </select>
            {#if c.tipo !== "status"}<input placeholder={c.tipo === "json" ? "data.ok" : "var"} value={c.campo} oninput={(e) => (c.campo = e.currentTarget.value)} />{/if}
          </div>
          <div class="insp-row">
            <select value={c.operatore} onchange={(e) => (c.operatore = e.currentTarget.value)}>{#each operatori as o}<option value={o}>{o}</option>{/each}</select>
            <input placeholder="atteso" value={c.atteso} oninput={(e) => (c.atteso = e.currentTarget.value)} />
          </div>
        {/if}
        <p class="insp-hint">La condizione di skip usa la risposta del nodo precedente.</p>
      </div>
    {/if}
  </div>
</div>

{#if guidaAperta}
  <div class="guida-overlay" onclick={() => (guidaAperta = false)}>
    <div class="guida" onclick={(e) => e.stopPropagation()}>
      <div class="guida-head">
        <h2>🕸 Guida al canvas dei flussi</h2>
        <span class="insp-x" onclick={() => (guidaAperta = false)}>✕</span>
      </div>
      <div class="guida-body">
        <p class="guida-intro">
          Un <b>flusso</b> è un test d'integrazione visuale: una catena di richieste collegate
          come un grafo. L'esecuzione parte da <b>Start</b> e segue gli archi; puoi creare
          <b>rami condizionali</b> e far <b>convergere</b> più strade su uno stesso nodo (merge).
        </p>

        <h3>1 · Aggiungere passi</h3>
        <ul>
          <li>Usa il menu <b>«+ Aggiungi richiesta»</b> in alto: il nodo appare sul canvas.</li>
          <li>Ogni nodo è una richiesta della collezione (metodo + nome). Trascinalo dove vuoi.</li>
          <li>Il nodo <b>Start</b> è il punto d'ingresso e non si elimina.</li>
        </ul>

        <h3>2 · Collegare i nodi</h3>
        <ul>
          <li>Trascina dal <b>pallino a destra</b> di un nodo al <b>pallino a sinistra</b> di un altro.</li>
          <li>Un nodo con <b>più archi in uscita</b> crea una biforcazione (branch).</li>
          <li>Un nodo con <b>più archi in entrata</b> è un merge: gira <b>una sola volta</b>.</li>
        </ul>

        <h3>3 · Rami condizionali (click su un arco)</h3>
        <ul>
          <li>Seleziona un arco e spunta <b>«Segui solo se…»</b>.</li>
          <li>La condizione si valuta sulla <b>risposta del nodo sorgente</b>:
            <br /><code>status</code> (codice HTTP), <code>json</code> (un campo del body, es. <code>data.ok</code>) o <code>var</code> (una variabile).</li>
          <li>Esempio: <code>status == 200</code> su un arco e <code>status != 200</code> sull'altro per gestire successo/errore.</li>
          <li>Un arco <b>senza condizione</b> viene sempre percorso.</li>
        </ul>

        <h3>4 · Configurare un nodo (click sul nodo)</h3>
        <ul>
          <li><b>Cattura variabili</b>: estrai un valore dalla risposta (da <code>json</code>/<code>header</code>/<code>body</code>) e salvalo in una variabile,
            riutilizzabile nei passi successivi con <code>{`{{nome}}`}</code> — es. cattura <code>id</code> dopo un POST.</li>
          <li><b>Al fallimento</b>: <i>Ferma il flusso</i> (default) oppure <i>Continua comunque</i>.</li>
          <li><b>Skip</b>: salta il nodo se una condizione (sulla risposta precedente) non è soddisfatta.</li>
        </ul>

        <h3>5 · Eseguire e salvare</h3>
        <ul>
          <li><b>▶ Esegui</b> salva e lancia il flusso: i risultati si aprono in un tab dedicato
            (ogni passo con status, tempo ed eventuali test; i rami non presi risultano <b>SKIP</b>).</li>
          <li><b>Salva</b> persiste posizioni, archi e configurazioni. Lo stesso flusso gira anche
            da riga di comando: <code>rustman run &lt;ws&gt; --chain "&lt;nome&gt;"</code>.</li>
          <li>Rinomina il flusso dal campo del titolo in alto a sinistra.</li>
        </ul>

        <h3>Scorciatoie</h3>
        <ul>
          <li><b>Canc / Backspace</b>: elimina il nodo o l'arco selezionato.</li>
          <li>Rotella per lo zoom, trascina lo sfondo per spostarti; i comandi in basso a sinistra fanno zoom/adatta.</li>
        </ul>
      </div>
    </div>
  </div>
{/if}

<style>
  .wf { display: flex; flex-direction: column; min-height: 0; height: 100%; }
  .wf-bar {
    display: flex; align-items: center; gap: 10px; padding: 9px 14px;
    border-bottom: 1px solid var(--border); background: var(--panel);
  }
  .wf-nome {
    background: transparent; border: 1px solid transparent; border-radius: 7px;
    padding: 5px 8px; color: var(--txt); font-weight: 600; font-size: 14px; outline: none; min-width: 120px;
  }
  .wf-nome:hover { border-color: var(--border); }
  .wf-nome:focus { border-color: var(--accent); background: var(--panel-2); }
  .wf-add {
    background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px;
    padding: 6px 10px; color: var(--txt); font-size: 12.5px; cursor: pointer; outline: none;
  }
  .wf-hint { color: var(--txt-faint); font-size: 11.5px; }
  .wf-sp { flex: 1; }
  .wf-btn { border: none; border-radius: var(--radius); padding: 8px 14px; font-size: 12.5px; font-weight: 600; cursor: pointer; }
  .wf-btn.ghost { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); }
  .wf-btn.ghost:hover { background: var(--panel-2); }
  .wf-btn.run { background: linear-gradient(145deg,var(--accent-2),var(--accent)); color: var(--accent-contrast); box-shadow: 0 4px 14px var(--accent-soft); }
  .wf-btn.run:disabled { opacity: .5; cursor: default; }
  .wf-canvas { flex: 1; min-height: 0; position: relative; }
  /* Adatta xyflow ai colori del tema. */
  .wf-canvas :global(.svelte-flow) { background: var(--bg); }
  .wf-canvas :global(.svelte-flow__edge-path) { stroke: var(--accent-line); stroke-width: 2; }
  .wf-canvas :global(.svelte-flow__edge.selected .svelte-flow__edge-path) { stroke: var(--accent); }
  .wf-canvas :global(.svelte-flow__controls) { box-shadow: var(--shadow-soft); border-radius: 8px; overflow: hidden; }
  .wf-canvas :global(.svelte-flow__controls-button) { background: var(--panel-2); border-bottom: 1px solid var(--border); color: var(--txt); fill: var(--txt); }
  .wf-canvas :global(.svelte-flow__controls-button:hover) { background: var(--panel-3); }
  .wf-canvas :global(.svelte-flow__minimap) { background: var(--panel); border: 1px solid var(--border); border-radius: 8px; }
  .wf-canvas :global(.svelte-flow__background) { color: var(--border); }
  .wf-canvas :global(.svelte-flow__edge-label) {
    background: var(--accent-soft); border: 1px solid var(--accent-line); color: var(--txt);
    font-size: 10.5px; font-family: var(--mono); padding: 2px 7px; border-radius: 999px; white-space: nowrap;
  }
  /* Inspector dell'arco (branch) */
  .wf-insp {
    position: absolute; top: 12px; right: 12px; width: 250px; z-index: 5;
    background: var(--panel-2); border: 1px solid var(--border-2); border-radius: var(--radius-lg);
    box-shadow: var(--shadow); padding: 12px; display: flex; flex-direction: column; gap: 8px;
  }
  .insp-h { display: flex; align-items: center; gap: 6px; font-size: 11px; font-weight: 600; letter-spacing: .5px; color: var(--txt-faint); text-transform: uppercase; }
  .insp-path { color: var(--txt-dim); font-weight: 500; text-transform: none; letter-spacing: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .insp-x { cursor: pointer; color: var(--txt-faint); }
  .insp-x:hover { color: var(--txt); }
  .insp-ck { display: flex; align-items: center; gap: 7px; font-size: 12px; color: var(--txt-dim); cursor: pointer; }
  .insp-row { display: flex; gap: 6px; }
  .insp-row select, .insp-row input {
    flex: 1; min-width: 0; background: var(--panel); border: 1px solid var(--border);
    border-radius: 6px; padding: 5px 7px; color: var(--txt); font-size: 12px; outline: none;
  }
  .insp-row select:focus, .insp-row input:focus { border-color: var(--accent); }
  .insp-hint { font-size: 10.5px; color: var(--txt-faint); line-height: 1.4; }
  .insp-hint code { font-family: var(--mono); color: var(--accent-2); }
  /* Unità di misura accanto ai campi numerici del loop. */
  .insp-u { font-size: 10.5px; color: var(--txt-faint); white-space: nowrap; }
  /* I select del loop hanno etichette lunghe: che non stringano gli input. */
  .insp-row select { min-width: 0; }
  .insp-x.sm { font-size: 11px; flex: none; align-self: center; }
  .insp-sub { font-size: 10px; text-transform: uppercase; letter-spacing: .06em; color: var(--txt-faint); margin-top: 4px; }
  .insp-add { align-self: flex-start; background: var(--panel-3); color: var(--txt-dim); border: 1px solid var(--border-2); border-radius: 6px; padding: 3px 9px; font-size: 11px; cursor: pointer; }
  .insp-add:hover { background: var(--panel); color: var(--txt); }
  .insp-row input, .insp-row select { min-width: 0; }

  /* Bottone info in toolbar */
  .wf-info { width: 32px; height: 32px; border-radius: 8px; border: 1px solid var(--border-2); background: var(--panel-3); color: var(--txt-dim); font-size: 15px; cursor: pointer; }
  .wf-info:hover { color: var(--accent-2); border-color: var(--accent-line); }

  /* Modale guida */
  .guida-overlay { position: fixed; inset: 0; z-index: 60; background: rgba(0,0,0,.55); backdrop-filter: blur(3px); display: grid; place-items: center; padding: 24px; }
  .guida { width: min(680px, 100%); max-height: 86vh; display: flex; flex-direction: column; background: var(--panel); border: 1px solid var(--border-2); border-radius: var(--radius-lg); box-shadow: var(--shadow); overflow: hidden; }
  .guida-head { display: flex; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--border); }
  .guida-head h2 { font-size: 16px; font-weight: 700; flex: 1; }
  .guida-body { padding: 8px 22px 24px; overflow-y: auto; }
  .guida-intro { color: var(--txt-dim); font-size: 13.5px; line-height: 1.65; margin: 14px 0 6px; }
  .guida-body h3 { font-size: 13px; font-weight: 700; color: var(--accent-2); margin: 20px 0 8px; }
  .guida-body ul { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 7px; }
  .guida-body li { color: var(--txt-dim); font-size: 13px; line-height: 1.55; }
  .guida-body b { color: var(--txt); }
  .guida-body code { font-family: var(--mono); font-size: 11.5px; background: var(--panel-3); border: 1px solid var(--border); border-radius: 5px; padding: 1px 5px; color: var(--accent-2); }
</style>
