<script>
  // Vista "Environments": gestione ambienti e variabili, con scelta dell'ambiente attivo.
  let { environments, ambienteAttivo, onSalva, onElimina, onImpostaAttivo } = $props();

  // Copia locale modificabile (rinfrescata a ogni montaggio della vista).
  let envs = $state($state.snapshot(environments));
  let sel = $state(envs.length ? 0 : -1);
  const corrente = $derived(sel >= 0 ? envs[sel] : null);

  function nuovo() {
    envs.push({ file: null, environment: { nome: "Nuovo ambiente", variabili: [] } });
    sel = envs.length - 1;
  }
  function aggiungiVar() {
    corrente.environment.variabili.push({ chiave: "", valore: "" });
  }
  function rimuoviVar(i) {
    corrente.environment.variabili.splice(i, 1);
  }
  async function salva() {
    if (!corrente) return;
    corrente.file = await onSalva(corrente.file, $state.snapshot(corrente.environment));
  }
  async function elimina() {
    if (!corrente) return;
    if (corrente.file) await onElimina(corrente.file);
    envs.splice(sel, 1);
    sel = Math.min(sel, envs.length - 1);
  }
  async function attiva() {
    if (!corrente) return;
    if (!corrente.file) await salva(); // serve un file salvato per poterlo attivare
    onImpostaAttivo(corrente.file);
  }
</script>

<div class="env-head">
  ENVIRONMENTS
  <span class="add" title="Nuovo ambiente" onclick={nuovo}>+</span>
</div>

<div class="env-list">
  {#each envs as e, i}
    <div class="env-item" class:active={i === sel} onclick={() => (sel = i)}>
      <span class="dot" class:on={e.file && e.file === ambienteAttivo}></span>
      <span class="nm">{e.environment.nome || "(senza nome)"}</span>
    </div>
  {/each}
  {#if envs.length === 0}<div class="vuoto">Nessun ambiente.</div>{/if}
</div>

{#if corrente}
  <div class="dettaglio">
    <label class="campo"><span>Nome</span><input class="inp" bind:value={corrente.environment.nome} /></label>

    <div class="vars-h">Variabili</div>
    {#each corrente.environment.variabili as v, i}
      <div class="var-riga">
        <input class="inp" placeholder="chiave" bind:value={v.chiave} />
        <input class="inp" placeholder="valore" bind:value={v.valore} />
        <span class="rm" onclick={() => rimuoviVar(i)} title="Rimuovi">✕</span>
      </div>
    {/each}
    <button class="mini" onclick={aggiungiVar}>+ Variabile</button>

    <div class="azioni">
      <button class="mini primario" onclick={salva}>Salva</button>
      <button class="mini" onclick={attiva} disabled={corrente.file === ambienteAttivo}>
        {corrente.file === ambienteAttivo ? "Attivo" : "Attiva"}
      </button>
      <button class="mini" onclick={elimina}>Elimina</button>
    </div>
    <div class="hint">Usa le variabili nelle richieste con <code>{"{{nome}}"}</code>.</div>
  </div>
{/if}

<style>
  .env-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); display: flex; align-items: center; }
  .env-head .add { margin-left: auto; cursor: pointer; color: var(--txt-dim); font-size: 16px; line-height: 1; padding: 0 4px; border-radius: 4px; }
  .env-head .add:hover { background: var(--panel-3); color: var(--txt); }
  .env-list { max-height: 180px; overflow-y: auto; padding: 0 8px; border-bottom: 1px solid var(--border); }
  .env-item { display: flex; align-items: center; gap: 8px; padding: 7px 8px; border-radius: 6px; cursor: pointer; color: var(--txt-dim); }
  .env-item:hover { background: var(--panel-3); }
  .env-item.active { background: rgba(124,92,255,.13); color: var(--txt); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--txt-faint); }
  .dot.on { background: var(--green); box-shadow: 0 0 8px rgba(63,185,80,.6); }
  .nm { font-size: 12.5px; }
  .vuoto { color: var(--txt-faint); padding: 8px; font-size: 12px; }
  .dettaglio { padding: 12px 14px; overflow-y: auto; }
  .campo { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; }
  .campo span { width: 48px; color: var(--txt-dim); font-size: 12.5px; }
  .vars-h { font-size: 11px; font-weight: 600; color: var(--txt-faint); margin-bottom: 6px; }
  .var-riga { display: flex; gap: 6px; margin-bottom: 6px; align-items: center; }
  .inp { flex: 1; width: 100%; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; color: var(--txt); font-size: 12px; outline: none; }
  .inp:focus { border-color: var(--accent); }
  .rm { color: var(--txt-faint); cursor: pointer; padding: 0 4px; }
  .rm:hover { color: var(--red); }
  .mini { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); border-radius: 6px; padding: 6px 10px; font-size: 12px; cursor: pointer; margin-top: 6px; }
  .mini:hover:not(:disabled) { background: #22222e; }
  .mini:disabled { opacity: .55; cursor: default; }
  .mini.primario { background: linear-gradient(145deg,#8b6dff,#6c47ff); border: none; color: #fff; }
  .azioni { display: flex; gap: 6px; margin-top: 12px; }
  .hint { margin-top: 12px; color: var(--txt-faint); font-size: 11.5px; }
  .hint code { font-family: var(--mono); color: var(--accent-2); }
</style>
