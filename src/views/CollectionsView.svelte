<script>
  // Vista "Collections": albero ricorsivo di collezioni, cartelle e richieste.
  import NodoAlbero from "../components/NodoAlbero.svelte";

  let {
    albero,
    percorsoWs = "",
    attivo,
    onApri,
    onNuovaCollezione,
    onNuovaCartella,
    onNuovaRichiesta,
    onRinomina,
    onEliminaCartella,
    onEliminaRichiesta,
    onEsporta,
    onImporta,
    onGeneraDoc,
    onEsportaOpenapi,
    onTrovaSostituisci,
    onDrift,
    onDiffColl,
    onConfigCartella,
    onCoverage,
    onLint,
    onEseguiBatch,
    onEsportaWs,
    onImportaWs,
  } = $props();

  let nuovaColl = $state({ attiva: false, nome: "" });
  let cerca = $state("");

  // Chiavi localStorage per-workspace (filtri salvati e preferiti).
  const kFiltri = $derived(`rustman_filtri:${percorsoWs}`);
  const kPin = $derived(`rustman_pin:${percorsoWs}`);
  let filtri = $state([]);
  let pinnati = $state([]);
  $effect(() => {
    // Ricarica quando cambia il workspace.
    filtri = JSON.parse(localStorage.getItem(kFiltri) || "[]");
    pinnati = JSON.parse(localStorage.getItem(kPin) || "[]");
  });
  function salvaFiltro() {
    const q = cerca.trim();
    if (!q || filtri.includes(q)) return;
    filtri = [...filtri, q];
    localStorage.setItem(kFiltri, JSON.stringify(filtri));
  }
  function rimuoviFiltro(f) {
    filtri = filtri.filter((x) => x !== f);
    localStorage.setItem(kFiltri, JSON.stringify(filtri));
  }
  // Preferiti: pin/unpin di una richiesta (per percorso file).
  function togglePin(file) {
    pinnati = pinnati.includes(file) ? pinnati.filter((x) => x !== file) : [...pinnati, file];
    localStorage.setItem(kPin, JSON.stringify(pinnati));
  }
  // Risolve i file pinnati in {file, richiesta} dall'albero.
  function trovaRichiesta(figli, file) {
    for (const n of figli) {
      if (n.tipo === "cartella") { const r = trovaRichiesta(n.figli, file); if (r) return r; }
      else if (n.file === file) return n;
    }
    return null;
  }
  const preferiti = $derived.by(() =>
    pinnati.map((f) => { for (const c of albero) { const r = trovaRichiesta(c.figli, f); if (r) return r; } return null; }).filter(Boolean)
  );

  // True se una richiesta combacia col testo cercato (nome/url/metodo/tag).
  function combacia(r, q) {
    return (
      (r.nome || "").toLowerCase().includes(q) ||
      (r.url || "").toLowerCase().includes(q) ||
      (r.metodo || "").toLowerCase().includes(q) ||
      (r.tags || []).some((t) => t.toLowerCase().includes(q))
    );
  }
  // Filtra ricorsivamente i nodi: tiene le richieste che combaciano e le cartelle
  // che hanno discendenti corrispondenti.
  function filtra(figli, q) {
    const out = [];
    for (const n of figli) {
      if (n.tipo === "cartella") {
        const sub = filtra(n.figli, q);
        if (sub.length) out.push({ ...n, figli: sub });
      } else if (combacia(n.richiesta, q)) {
        out.push(n);
      }
    }
    return out;
  }
  const alberoFiltrato = $derived.by(() => {
    const q = cerca.trim().toLowerCase();
    if (!q) return albero;
    return albero
      .map((c) => ({ ...c, figli: filtra(c.figli, q) }))
      .filter((c) => c.figli.length || c.nome.toLowerCase().includes(q));
  });
  let fileInput;
  let driftInput;
  let covInput;
  let collDiffInput;
  let wsInput;
  let lintInput;

  async function suLintFile(e) {
    const f = e.target.files?.[0];
    if (f) await onLint?.(await f.text(), f.name);
    e.target.value = "";
  }

  async function suWsFile(e) {
    const f = e.target.files?.[0];
    if (f) await onImportaWs?.(await f.text());
    e.target.value = "";
  }
  async function suCollDiffFiles(e) {
    const files = [...(e.target.files || [])];
    if (files.length === 2) {
      const [a, b] = await Promise.all(files.map((f) => f.text()));
      await onDiffColl?.(a, b, files[0].name, files[1].name);
    }
    e.target.value = "";
  }

  async function suCovFile(e) {
    const f = e.target.files?.[0];
    if (f) await onCoverage?.(await f.text(), f.name);
    e.target.value = "";
  }
  let fr = $state({ aperto: false, cerca: "", con: "" });

  async function applicaFr() {
    if (!fr.cerca) return;
    await onTrovaSostituisci?.(fr.cerca, fr.con);
    fr = { aperto: false, cerca: "", con: "" };
  }
  async function suDriftFiles(e) {
    const files = [...(e.target.files || [])];
    if (files.length === 2) {
      const [a, b] = await Promise.all(files.map((f) => f.text()));
      await onDrift?.(a, b, files[0].name, files[1].name);
    }
    e.target.value = "";
  }

  // Callback raccolti per i nodi ricorsivi.
  const azioni = {
    onApri,
    onNuovaCartella,
    onNuovaRichiesta,
    onRinomina,
    onEliminaCartella,
    onEliminaRichiesta,
    onEsporta,
    onConfigCartella,
    onEseguiBatch,
    onPin: togglePin,
  };

  function confermaColl() {
    const n = nuovaColl.nome.trim();
    if (n) onNuovaCollezione(n);
    nuovaColl = { attiva: false, nome: "" };
  }

  async function suFileImport(e) {
    const f = e.target.files?.[0];
    if (!f) return;
    await onImporta(await f.text());
    e.target.value = "";
  }
</script>

<div class="col-head" style="padding-top:14px">
  <span class="side-title">COLLECTIONS</span>
  <div class="col-tools">
    <span class="side-add" title="Nuova collezione" onclick={() => (nuovaColl.attiva = true)}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12h14"/></svg>
    </span>
    <span class="side-add" title="Importa (Rustman o Postman: collection / environment)" onclick={() => fileInput.click()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 3v12M7 10l5 5 5-5M5 21h14"/></svg>
    </span>
    <span class="side-add" title="Genera documentazione HTML" onclick={() => onGeneraDoc?.()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><path d="M14 2v6h6M9 13h6M9 17h6"/></svg>
    </span>
    <span class="side-add" title="Cerca e sostituisci nelle richieste" onclick={() => (fr.aperto = !fr.aperto)}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="7"/><path d="M21 21l-4.3-4.3"/></svg>
    </span>
    <span class="side-add" title="Confronto OpenAPI (drift): scegli 2 file" onclick={() => driftInput.click()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M8 7h12M8 7l3-3M8 7l3 3M16 17H4M16 17l-3-3M16 17l-3 3"/></svg>
    </span>
    <span class="side-add" title="Esporta in OpenAPI" onclick={() => onEsportaOpenapi?.()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 21V9M8 13l4-4 4 4M5 3h14"/></svg>
    </span>
    <span class="side-add" title="Coverage delle API (scegli uno spec OpenAPI)" onclick={() => covInput.click()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 11l3 3L22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/></svg>
    </span>
    <input type="file" accept=".json,application/json" style="display:none" bind:this={fileInput} onchange={suFileImport} />
    <input type="file" accept=".json,.yaml,.yml" multiple style="display:none" bind:this={driftInput} onchange={suDriftFiles} />
    <span class="side-add" title="Diff di due collezioni (.rustman.json): scegli 2 file" onclick={() => collDiffInput.click()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 3v18M15 3v18M4 8h5M15 8h5M4 16h5M15 16h5"/></svg>
    </span>
    <span class="side-add" title="Lint di uno spec OpenAPI" onclick={() => lintInput.click()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M9 11l3 3 8-8M20 12v7a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h9"/></svg>
    </span>
    <input type="file" accept=".json,.yaml,.yml" style="display:none" bind:this={covInput} onchange={suCovFile} />
    <input type="file" accept=".json,.yaml,.yml" style="display:none" bind:this={lintInput} onchange={suLintFile} />
    <span class="side-add" title="Esporta tutto il workspace (bundle)" onclick={() => onEsportaWs?.()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 8v13H3V8M1 3h22v5H1zM10 12h4"/></svg>
    </span>
    <span class="side-add" title="Importa un bundle di workspace" onclick={() => wsInput.click()}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 8v13h18V8M12 3v12M8 11l4 4 4-4"/></svg>
    </span>
    <input type="file" accept=".json" multiple style="display:none" bind:this={collDiffInput} onchange={suCollDiffFiles} />
    <input type="file" accept=".json" style="display:none" bind:this={wsInput} onchange={suWsFile} />
  </div>
</div>

{#if fr.aperto}
  <div style="display:flex;flex-direction:column;gap:6px;padding:8px 12px;border-bottom:1px solid var(--border)">
    <input style="background:var(--panel-2);border:1px solid var(--border);border-radius:6px;padding:6px 8px;color:var(--txt);font-size:12px;outline:none" placeholder="cerca…" bind:value={fr.cerca} />
    <input style="background:var(--panel-2);border:1px solid var(--border);border-radius:6px;padding:6px 8px;color:var(--txt);font-size:12px;outline:none" placeholder="sostituisci con…" bind:value={fr.con} />
    <button style="background:linear-gradient(145deg,#8b6dff,#6c47ff);border:none;color:#fff;border-radius:6px;padding:6px 10px;font-size:12px;cursor:pointer" onclick={applicaFr}>Applica a tutte</button>
  </div>
{/if}

{#if nuovaColl.attiva}
  <div class="search">
    <!-- svelte-ignore a11y_autofocus -->
    <input autofocus placeholder="Nome collezione, Invio"
      bind:value={nuovaColl.nome}
      onkeydown={(e) => e.key === "Enter" && confermaColl()}
      onblur={confermaColl} />
  </div>
{/if}

{#if albero.length > 0}
  <div style="padding:6px 12px">
    <div style="display:flex;gap:6px;align-items:center">
      <input style="flex:1;background:var(--panel-2);border:1px solid var(--border);border-radius:6px;padding:6px 9px;color:var(--txt);font-size:12px;outline:none"
        placeholder="🔍 filtra (nome, url, metodo, tag)…" bind:value={cerca} />
      {#if cerca.trim()}
        <span style="cursor:pointer;color:var(--txt-faint)" title="Salva come filtro" onclick={salvaFiltro}>💾</span>
      {/if}
    </div>
    {#if filtri.length}
      <div style="display:flex;flex-wrap:wrap;gap:5px;margin-top:6px">
        {#each filtri as f}
          <span style="display:inline-flex;align-items:center;gap:4px;background:var(--panel-3);border:1px solid var(--border);border-radius:999px;padding:2px 8px;font-size:11px;cursor:pointer" onclick={() => (cerca = f)}>
            🏷 {f}<span style="color:var(--txt-faint)" onclick={(e) => { e.stopPropagation(); rimuoviFiltro(f); }}>✕</span>
          </span>
        {/each}
      </div>
    {/if}
  </div>
{/if}

<div class="tree">
  {#if albero.length === 0}
    <div class="placeholder" style="height:auto;padding:24px 8px">
      <div>Nessuna collezione.</div>
      <div>Usa <b>+</b> qui sopra per crearne una.</div>
    </div>
  {/if}
  {#if preferiti.length && !cerca.trim()}
    <div style="padding:6px 14px 2px;font-size:10.5px;font-weight:600;letter-spacing:.6px;color:var(--txt-faint)">★ PREFERITI</div>
    {#each preferiti as n}
      <NodoAlbero nodo={n} livello={0} {attivo} {azioni} {pinnati} />
    {/each}
    <div style="height:1px;background:var(--border);margin:6px 8px"></div>
  {/if}
  {#each alberoFiltrato as c}
    <NodoAlbero nodo={{ tipo: "cartella", nome: c.nome, dir: c.dir, figli: c.figli }} livello={0} {attivo} {azioni} {pinnati} />
  {/each}
  {#if cerca.trim() && alberoFiltrato.length === 0}
    <div class="placeholder" style="height:auto;padding:16px 8px"><div>Nessun risultato.</div></div>
  {/if}
</div>
