<script>
  // Vista "Run": crea ed esegue catene di chiamate (integration test).
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  let { albero, onEsegui } = $props();

  let catene = $state([]); // CatenaSuDisco[]
  let sel = $state(-1);
  const corrente = $derived(sel >= 0 ? catene[sel] : null);

  // Elenco piatto delle richieste disponibili (per il picker dei passi).
  function elenco(figli, acc) {
    for (const n of figli) {
      if (n.tipo === "richiesta") acc.push({ file: n.file, label: n.richiesta.nome || n.file });
      else if (n.tipo === "cartella") elenco(n.figli, acc);
    }
    return acc;
  }
  const richieste = $derived(albero.flatMap((c) => elenco(c.figli, [])));

  function etichetta(file) {
    return richieste.find((r) => r.file === file)?.label ?? file;
  }

  async function ricarica() {
    catene = await api.caricaCatene();
  }
  onMount(ricarica);

  async function persist() {
    if (!corrente) return;
    const nuovoFile = await api.salvaCatena(corrente.file, $state.snapshot(corrente.catena));
    catene = await api.caricaCatene();
    sel = catene.findIndex((c) => c.file === nuovoFile);
  }

  async function nuova() {
    const file = await api.salvaCatena(null, { nome: "Nuova catena", passi: [] });
    await ricarica();
    sel = catene.findIndex((c) => c.file === file);
  }
  async function elimina() {
    if (!corrente) return;
    await api.eliminaCatena(corrente.file);
    await ricarica();
    sel = -1;
  }

  function aggiungiPasso(e) {
    const file = e.target.value;
    if (!file) return;
    corrente.catena.passi.push({ file });
    e.target.value = "";
    persist();
  }
  function rimuoviPasso(i) {
    corrente.catena.passi.splice(i, 1);
    persist();
  }
  function muovi(i, d) {
    const p = corrente.catena.passi;
    const j = i + d;
    if (j < 0 || j >= p.length) return;
    [p[i], p[j]] = [p[j], p[i]];
    persist();
  }

  // Editor del flusso: condizione, catture, comportamento al fallimento.
  let espanso = $state(-1);
  const tipiCond = ["status", "json", "var"];
  const operatori = ["==", "!=", "<", ">", "contiene"];
  function toggleCond(p) {
    p.condizione = p.condizione ? null : { tipo: "status", campo: "", operatore: "==", atteso: "200" };
    persist();
  }
  function aggiungiCattura(p) {
    if (!p.catture) p.catture = [];
    p.catture.push({ variabile: "", da: "json", campo: "" });
  }
  function rimuoviCattura(p, i) {
    p.catture.splice(i, 1);
    persist();
  }
</script>

<div class="run-head">
  RUN
  <span class="add" title="Nuova catena" onclick={nuova}>+</span>
</div>

<div class="run-list">
  {#each catene as c, i}
    <div class="run-item" class:active={i === sel} onclick={() => (sel = i)}>{c.catena.nome}</div>
  {/each}
  {#if catene.length === 0}<div class="vuoto">Nessuna catena. Crea con +</div>{/if}
</div>

{#if corrente}
  <div class="dettaglio">
    <input class="inp nome" bind:value={corrente.catena.nome} onblur={persist} />

    <div class="passi-h">Passi</div>
    {#each corrente.catena.passi as p, i}
      <div class="passo">
        <span class="num">{i + 1}</span>
        <span class="lbl" title={p.file}>
          {etichetta(p.file)}
          {#if p.condizione}<span class="cbadge" title="Condizionale">if</span>{/if}
          {#if p.catture?.length}<span class="cbadge cap">⇲{p.catture.length}</span>{/if}
        </span>
        <span class="a" class:on={espanso === i} onclick={() => (espanso = espanso === i ? -1 : i)} title="Configura flusso">⚙</span>
        <span class="a" onclick={() => muovi(i, -1)} title="Su">↑</span>
        <span class="a" onclick={() => muovi(i, 1)} title="Giù">↓</span>
        <span class="a rm" onclick={() => rimuoviPasso(i)} title="Rimuovi">✕</span>
      </div>
      {#if espanso === i}
        <div class="cfg">
          <label class="ck"><input type="checkbox" checked={!!p.condizione} onchange={() => toggleCond(p)} /> Esegui solo se…</label>
          {#if p.condizione}
            <div class="cond">
              <select class="s" bind:value={p.condizione.tipo} onchange={persist}>{#each tipiCond as t}<option>{t}</option>{/each}</select>
              {#if p.condizione.tipo !== "status"}<input class="s" placeholder={p.condizione.tipo === "json" ? "data.ok" : "nome var"} bind:value={p.condizione.campo} onblur={persist} />{/if}
              <select class="s" bind:value={p.condizione.operatore} onchange={persist}>{#each operatori as o}<option>{o}</option>{/each}</select>
              <input class="s" placeholder="atteso" bind:value={p.condizione.atteso} onblur={persist} />
            </div>
          {/if}

          <div class="cap-h">Cattura variabili</div>
          {#each p.catture ?? [] as c, j}
            <div class="cond">
              <input class="s" placeholder="variabile" bind:value={c.variabile} onblur={persist} />
              <select class="s" bind:value={c.da} onchange={persist}><option value="json">json</option><option value="header">header</option><option value="body">body</option></select>
              {#if c.da !== "body"}<input class="s" placeholder={c.da === "json" ? "data.token" : "nome header"} bind:value={c.campo} onblur={persist} />{/if}
              <span class="a rm" onclick={() => rimuoviCattura(p, j)}>✕</span>
            </div>
          {/each}
          <button class="addv" onclick={() => aggiungiCattura(p)}>+ cattura</button>

          <label class="ck" style="margin-top:8px">Al fallimento:
            <select class="s" bind:value={p.al_fallimento} onchange={persist}><option value="">stop</option><option value="continua">continua</option></select>
          </label>
        </div>
      {/if}
    {/each}
    {#if corrente.catena.passi.length === 0}<div class="vuoto">Nessun passo.</div>{/if}

    <select class="inp" onchange={aggiungiPasso}>
      <option value="">+ Aggiungi passo…</option>
      {#each richieste as r}<option value={r.file}>{r.label}</option>{/each}
    </select>

    <div class="azioni">
      <button class="esegui" onclick={() => onEsegui($state.snapshot(corrente.catena))} disabled={corrente.catena.passi.length === 0}>▶ Esegui</button>
      <button class="mini" onclick={elimina}>Elimina</button>
    </div>
  </div>
{/if}

<style>
  .run-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); display: flex; align-items: center; }
  .run-head .add { margin-left: auto; cursor: pointer; color: var(--txt-dim); font-size: 16px; padding: 0 4px; border-radius: 4px; }
  .run-head .add:hover { background: var(--panel-3); color: var(--txt); }
  .run-list { max-height: 160px; overflow-y: auto; padding: 0 8px; border-bottom: 1px solid var(--border); }
  .run-item { padding: 7px 8px; border-radius: 6px; cursor: pointer; color: var(--txt-dim); font-size: 12.5px; }
  .run-item:hover { background: var(--panel-3); }
  .run-item.active { background: rgba(124,92,255,.13); color: var(--txt); }
  .vuoto { color: var(--txt-faint); padding: 8px; font-size: 12px; }
  .dettaglio { padding: 12px 14px; overflow-y: auto; }
  .inp { width: 100%; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 7px 9px; color: var(--txt); font-size: 12.5px; outline: none; }
  .inp:focus { border-color: var(--accent); }
  .nome { font-weight: 600; margin-bottom: 12px; }
  .passi-h { font-size: 11px; font-weight: 600; color: var(--txt-faint); margin-bottom: 6px; }
  .passo { display: flex; align-items: center; gap: 6px; padding: 5px 4px; border-radius: 6px; }
  .passo:hover { background: var(--panel-3); }
  .num { width: 16px; color: var(--txt-faint); font-family: var(--mono); font-size: 11px; }
  .lbl { flex: 1; font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .a { color: var(--txt-faint); cursor: pointer; padding: 0 3px; }
  .a:hover { color: var(--txt); }
  .a.rm:hover { color: var(--red); }
  .a.on { color: var(--accent); }
  .cbadge { font-family: var(--mono); font-size: 9px; padding: 1px 4px; border-radius: 4px; background: var(--panel-3); color: var(--accent-2); margin-left: 4px; }
  .cbadge.cap { color: var(--green); }
  .cfg { margin: 2px 0 8px 22px; padding: 8px; border-left: 2px solid var(--border); display: flex; flex-direction: column; gap: 6px; }
  .ck { font-size: 11.5px; color: var(--txt-dim); display: flex; align-items: center; gap: 6px; }
  .cond { display: flex; gap: 4px; align-items: center; }
  .cap-h { font-size: 10px; text-transform: uppercase; letter-spacing: .05em; color: var(--txt-faint); margin-top: 4px; }
  .s { flex: 1; min-width: 0; background: var(--panel-2); border: 1px solid var(--border); border-radius: 5px; padding: 4px 6px; color: var(--txt); font-size: 11.5px; outline: none; }
  .s:focus { border-color: var(--accent); }
  .addv { align-self: flex-start; background: var(--panel-3); color: var(--txt-dim); border: 1px solid var(--border-2); border-radius: 5px; padding: 3px 8px; font-size: 11px; cursor: pointer; }
  .azioni { display: flex; gap: 8px; margin-top: 14px; }
  .esegui { flex: 1; background: linear-gradient(145deg,#8b6dff,#6c47ff); color: #fff; border: none; border-radius: 8px; padding: 9px; font-weight: 600; cursor: pointer; }
  .esegui:disabled { opacity: .55; cursor: default; }
  .mini { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); border-radius: 8px; padding: 9px 12px; cursor: pointer; font-size: 12px; }
  .mini:hover { background: #22222e; }
</style>
