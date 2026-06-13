<script>
  // Vista "Collections": albero ricorsivo di collezioni, cartelle e richieste.
  import NodoAlbero from "../components/NodoAlbero.svelte";

  let {
    albero,
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
  } = $props();

  let nuovaColl = $state({ attiva: false, nome: "" });
  let fileInput;

  // Callback raccolti per i nodi ricorsivi.
  const azioni = {
    onApri,
    onNuovaCartella,
    onNuovaRichiesta,
    onRinomina,
    onEliminaCartella,
    onEliminaRichiesta,
    onEsporta,
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
    <input type="file" accept=".json,application/json" style="display:none" bind:this={fileInput} onchange={suFileImport} />
  </div>
</div>

{#if nuovaColl.attiva}
  <div class="search">
    <!-- svelte-ignore a11y_autofocus -->
    <input autofocus placeholder="Nome collezione, Invio"
      bind:value={nuovaColl.nome}
      onkeydown={(e) => e.key === "Enter" && confermaColl()}
      onblur={confermaColl} />
  </div>
{/if}

<div class="tree">
  {#if albero.length === 0}
    <div class="placeholder" style="height:auto;padding:24px 8px">
      <div>Nessuna collezione.</div>
      <div>Usa <b>+</b> qui sopra per crearne una.</div>
    </div>
  {/if}
  {#each albero as c}
    <NodoAlbero nodo={{ tipo: "cartella", nome: c.nome, dir: c.dir, figli: c.figli }} livello={0} {attivo} {azioni} />
  {/each}
</div>
