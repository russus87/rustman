<script>
  // Vista "Settings": preferenze dell'app (per ora l'autosalvataggio).
  import { settings, salvaSettings } from "../lib/settings.svelte.js";

  function aggiorna() {
    salvaSettings();
  }
</script>

<div class="set-head">SETTINGS</div>
<div class="set-body">
  <label class="opzione">
    <input type="checkbox" bind:checked={settings.autosave} onchange={aggiorna} />
    <span>
      <b>Autosalvataggio</b>
      <small>Salva la richiesta da sola dopo una breve pausa, così non perdi le modifiche.</small>
    </span>
  </label>

  {#if settings.autosave}
    <label class="campo">
      <span>Ritardo (ms)</span>
      <input type="number" min="200" max="10000" step="100" bind:value={settings.autosaveMs} onchange={aggiorna} />
    </label>
  {/if}
</div>

<style>
  .set-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); }
  .set-body { padding: 8px 14px; display: flex; flex-direction: column; gap: 16px; }
  .opzione { display: flex; gap: 10px; align-items: flex-start; cursor: pointer; }
  .opzione input { margin-top: 2px; }
  .opzione span { display: flex; flex-direction: column; gap: 3px; }
  .opzione small { color: var(--txt-faint); font-size: 11.5px; }
  .campo { display: flex; align-items: center; gap: 10px; color: var(--txt-dim); font-size: 12.5px; }
  .campo input { width: 90px; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; color: var(--txt); font-family: var(--mono); outline: none; }
  .campo input:focus { border-color: var(--accent); }
</style>
