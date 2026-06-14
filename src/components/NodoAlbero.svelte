<script>
  // Nodo dell'albero delle collezioni: cartella (ricorsiva) o richiesta.
  import Self from "./NodoAlbero.svelte";

  let { nodo, livello = 0, attivo, azioni } = $props();

  let aperto = $state(true);
  // Modalità input inline: null | "req" | "folder" | "rinomina".
  let modo = $state(null);
  let testo = $state("");

  function classeMetodo(m) {
    if (m === "GET") return "get";
    if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put";
    if (m === "DELETE") return "del";
    return "";
  }

  function avvia(m) {
    modo = m;
    testo = m === "rinomina" ? nodo.nome : "";
    aperto = true;
  }

  function conferma() {
    const n = testo.trim();
    if (n) {
      if (modo === "req") azioni.onNuovaRichiesta(nodo.dir, n);
      else if (modo === "folder") azioni.onNuovaCartella(nodo.dir, n);
      else if (modo === "rinomina") azioni.onRinomina(nodo.dir, n);
    }
    modo = null;
    testo = "";
  }

  const indent = $derived(8 + livello * 14);
</script>

{#if nodo.tipo === "cartella"}
  <div class="folder" style="padding-left:{indent}px" onclick={() => (aperto = !aperto)}>
    <span class="chev" class:chiuso={!aperto}>
      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M9 6l6 6-6 6"/></svg>
    </span>
    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z"/></svg>
    {#if modo === "rinomina"}
      <!-- svelte-ignore a11y_autofocus -->
      <input class="inline" autofocus bind:value={testo} onclick={(e) => e.stopPropagation()}
        onkeydown={(e) => e.key === "Enter" && conferma()} onblur={conferma} />
    {:else}
      <span class="fname">{nodo.nome}</span>
      <span class="fact" title="Nuova richiesta" onclick={(e) => { e.stopPropagation(); avvia("req"); }}>＋</span>
      <span class="fact" title="Nuova cartella" onclick={(e) => { e.stopPropagation(); avvia("folder"); }}>🗀</span>
      <span class="fact" title="Rinomina" onclick={(e) => { e.stopPropagation(); avvia("rinomina"); }}>✎</span>
      <span class="fact" title="Esegui tutte (batch)" onclick={(e) => { e.stopPropagation(); azioni.onEseguiBatch(nodo.dir); }}>▶</span>
      <span class="fact" title="Header/auth ereditati" onclick={(e) => { e.stopPropagation(); azioni.onConfigCartella(nodo.dir, nodo.nome); }}>⚙</span>
      {#if livello === 0}
        <span class="fact" title="Esporta" onclick={(e) => { e.stopPropagation(); azioni.onEsporta(nodo.dir, nodo.nome); }}>⤓</span>
      {/if}
      <span class="fact fdel" title="Elimina" onclick={(e) => { e.stopPropagation(); azioni.onEliminaCartella(nodo.dir); }}>🗑</span>
    {/if}
  </div>

  {#if aperto}
    {#if modo === "req" || modo === "folder"}
      <div style="padding-left:{indent + 22}px;padding-right:8px">
        <!-- svelte-ignore a11y_autofocus -->
        <input class="inline" autofocus placeholder={modo === "req" ? "Nome richiesta" : "Nome cartella"}
          bind:value={testo} onkeydown={(e) => e.key === "Enter" && conferma()} onblur={conferma} />
      </div>
    {/if}
    {#each nodo.figli as figlio}
      <Self nodo={figlio} livello={livello + 1} {attivo} {azioni} />
    {/each}
  {/if}
{:else}
  <div class="req" class:active={nodo.file === attivo} style="padding-left:{indent + 16}px"
    onclick={() => azioni.onApri(nodo.file, nodo.richiesta)}>
    <span class="m {classeMetodo(nodo.richiesta.metodo)}">{nodo.richiesta.metodo}</span>
    <span class="rname">{nodo.richiesta.nome || "(senza nome)"}</span>
    <span class="del" title="Elimina" onclick={(e) => { e.stopPropagation(); azioni.onEliminaRichiesta(nodo.file); }}>✕</span>
  </div>
{/if}

<style>
  .chev {
    color: var(--txt-faint);
    transition: transform 0.15s;
  }
  .chev.chiuso {
    transform: rotate(-90deg);
  }
  .fname {
    flex: 1;
    font-weight: 500;
  }
  .fact {
    color: var(--txt-faint);
    opacity: 0;
    padding: 0 3px;
    border-radius: 4px;
    font-size: 12px;
    cursor: pointer;
  }
  .folder:hover .fact {
    opacity: 1;
  }
  .fact:hover {
    color: var(--txt);
    background: var(--panel-3);
  }
  .fact.fdel:hover {
    color: var(--red);
  }
  .del {
    margin-left: auto;
    color: var(--txt-faint);
    opacity: 0;
    padding: 0 4px;
    border-radius: 4px;
    cursor: pointer;
  }
  .req:hover .del {
    opacity: 1;
  }
  .del:hover {
    color: var(--red);
    background: var(--panel-3);
  }
  .inline {
    flex: 1;
    width: 100%;
    background: var(--panel-2);
    border: 1px solid var(--accent);
    border-radius: 6px;
    padding: 4px 8px;
    color: var(--txt);
    font-size: 12.5px;
    outline: none;
    margin: 2px 0;
  }
</style>
