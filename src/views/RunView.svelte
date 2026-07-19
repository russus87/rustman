<script>
  // Vista "Run": elenco dei flussi. Cliccarne uno lo apre nel canvas (vista
  // di default). "+ Nuovo flusso" crea un flusso vuoto e ne apre il canvas.
  import * as api from "../lib/api.js";

  let { onApriCanvas, segnale = 0 } = $props();

  let flussi = $state([]);
  async function ricarica() {
    try { flussi = await api.caricaCatene(); } catch (e) { console.error(e); }
  }
  // Ricarica al primo render e ogni volta che App segnala un cambiamento.
  $effect(() => { segnale; ricarica(); });

  async function nuovo() {
    const file = await api.salvaCatena(null, { nome: "Nuovo flusso", passi: [], nodi: [], archi: [] });
    await ricarica();
    const cs = flussi.find((c) => c.file === file);
    if (cs) onApriCanvas?.(cs);
  }
  async function elimina(cs, e) {
    e.stopPropagation();
    if (!confirm(`Eliminare il flusso "${cs.catena.nome}"?`)) return;
    await api.eliminaCatena(cs.file);
    await ricarica();
  }
  function conteggio(c) {
    const req = (c.catena.nodi || []).filter((n) => n.tipo !== "start").length;
    return req || (c.catena.passi?.length || 0);
  }
</script>

<div class="run-head">
  FLUSSI
  <span class="add" title="Nuovo flusso" onclick={nuovo}>+</span>
</div>

<div class="run-list">
  {#each flussi as c}
    <div class="run-item" onclick={() => onApriCanvas?.(c)} title="Apri nel canvas">
      <span class="ico">🕸</span>
      <span class="nm">{c.catena.nome}</span>
      <span class="cnt">{conteggio(c)}</span>
      <span class="rm" title="Elimina" onclick={(e) => elimina(c, e)}>✕</span>
    </div>
  {/each}
  {#if flussi.length === 0}
    <div class="vuoto">Nessun flusso ancora.<br />Crea il primo con <b>+</b>.</div>
  {/if}
</div>

<div class="run-foot">
  <button class="nuovo" onclick={nuovo}>+ Nuovo flusso</button>
  <p class="tip">Clicca un flusso per aprirlo nel canvas visuale.</p>
</div>

<style>
  .run-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); display: flex; align-items: center; }
  .run-head .add { margin-left: auto; cursor: pointer; color: var(--txt-dim); font-size: 16px; padding: 0 4px; border-radius: 4px; }
  .run-head .add:hover { background: var(--panel-3); color: var(--txt); }
  .run-list { flex: 1; overflow-y: auto; padding: 2px 8px; }
  .run-item { display: flex; align-items: center; gap: 9px; padding: 8px 8px; border-radius: 7px; cursor: pointer; color: var(--txt-dim); }
  .run-item:hover { background: var(--panel-3); color: var(--txt); }
  .run-item .ico { font-size: 13px; opacity: .8; }
  .run-item .nm { flex: 1; font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .run-item .cnt { font-family: var(--mono); font-size: 10.5px; color: var(--txt-faint); background: var(--panel-2); border: 1px solid var(--border); border-radius: 999px; padding: 1px 7px; }
  .run-item .rm { color: var(--txt-faint); opacity: 0; padding: 0 3px; }
  .run-item:hover .rm { opacity: 1; }
  .run-item .rm:hover { color: var(--red); }
  .vuoto { color: var(--txt-faint); padding: 16px 10px; font-size: 12.5px; line-height: 1.6; text-align: center; }
  .run-foot { border-top: 1px solid var(--border); padding: 12px; }
  .nuovo { width: 100%; background: linear-gradient(145deg,var(--accent-2),var(--accent)); color: var(--accent-contrast); border: none; border-radius: var(--radius); padding: 9px; font-weight: 600; font-size: 12.5px; cursor: pointer; }
  .nuovo:hover { filter: brightness(1.06); }
  .tip { margin-top: 10px; color: var(--txt-faint); font-size: 11px; line-height: 1.5; text-align: center; }
</style>
