<script>
  // Vista "Settings": preferenze dell'app (autosalvataggio, tema, accento).
  import { settings, salvaSettings, applicaTema } from "../lib/settings.svelte.js";

  function aggiorna() {
    salvaSettings();
  }
  function aggiornaTema() {
    salvaSettings();
    applicaTema();
  }
  const accenti = ["#7c5cff", "#3fb950", "#f74c00", "#388bfd", "#39d0d8", "#e3a008", "#f85149"];
</script>

<div class="set-head">SETTINGS</div>
<div class="set-body">
  <div class="campo">
    <span>Tema</span>
    <select bind:value={settings.tema} onchange={aggiornaTema}>
      <option value="scuro">Scuro</option>
      <option value="chiaro">Chiaro</option>
    </select>
  </div>
  <div class="campo">
    <span>Accento</span>
    <div class="accenti">
      {#each accenti as c}
        <span class="pallino" class:on={settings.accento === c} style="background:{c}"
          onclick={() => { settings.accento = c; aggiornaTema(); }}></span>
      {/each}
    </div>
  </div>

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
  .campo select { background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; color: var(--txt); outline: none; }
  .accenti { display: flex; gap: 8px; }
  .pallino { width: 20px; height: 20px; border-radius: 50%; cursor: pointer; border: 2px solid transparent; }
  .pallino.on { border-color: var(--txt); }
</style>
