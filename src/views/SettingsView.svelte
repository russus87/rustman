<script>
  // Vista "Settings": preferenze dell'app (autosalvataggio, tema, accento).
  import { settings, salvaSettings, applicaTema } from "../lib/settings.svelte.js";
  import { t } from "../lib/i18n.svelte.js";

  function aggiorna() {
    salvaSettings();
  }
  function aggiornaTema() {
    salvaSettings();
    applicaTema();
  }
  // Preset "brand" candidati (mostrati con etichetta per il confronto).
  const brand = [
    { nome: "Rust", col: "#f74c00" },
    { nome: "Verde", col: "#3fb950" },
    { nome: "Viola", col: "#7c5cff" },
  ];
  // Altri accenti rapidi.
  const accenti = ["#388bfd", "#39d0d8", "#e3a008", "#f85149"];
  function scegli(c) {
    settings.accento = c;
    aggiornaTema();
  }
</script>

<div class="set-head">{t("Settings").toUpperCase()}</div>
<div class="set-body">
  <div class="campo">
    <span>{t("Lingua")}</span>
    <select bind:value={settings.lingua} onchange={aggiorna}>
      <option value="it">Italiano</option>
      <option value="en">English</option>
    </select>
  </div>
  <div class="campo">
    <span>{t("Tema")}</span>
    <select bind:value={settings.tema} onchange={aggiornaTema}>
      <option value="scuro">{t("Scuro")}</option>
      <option value="chiaro">{t("Chiaro")}</option>
      <option value="sistema">{t("Sistema")}</option>
    </select>
  </div>
  <div class="campo">
    <span>{t("Dimensione")}</span>
    <select bind:value={settings.scala} onchange={aggiornaTema}>
      <option value={0.9}>Piccola</option>
      <option value={1}>Normale</option>
      <option value={1.1}>Grande</option>
      <option value={1.25}>Molto grande</option>
    </select>
  </div>
  <div class="campo col">
    <span>Accento</span>
    <div class="brand-row">
      {#each brand as b}
        <button type="button" class="brand-sw" class:on={settings.accento === b.col} onclick={() => scegli(b.col)}>
          <span class="chip" style="background:{b.col}"></span>
          {b.nome}
        </button>
      {/each}
    </div>
    <div class="accenti">
      {#each accenti as c}
        <span class="pallino" class:on={settings.accento === c} style="background:{c}"
          title={c} onclick={() => scegli(c)}></span>
      {/each}
      <label class="custom" title="Colore personalizzato">
        <input type="color" value={settings.accento} oninput={(e) => scegli(e.currentTarget.value)} />
        <span class="plus">+</span>
      </label>
    </div>
  </div>

  <label class="opzione">
    <input type="checkbox" bind:checked={settings.autosave} onchange={aggiorna} />
    <span>
      <b>{t("Autosalvataggio")}</b>
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
  .campo.col { flex-direction: column; align-items: stretch; gap: 8px; }
  .campo.col > span { display: flex; align-items: center; gap: 6px; }
  .campo.col small { color: var(--txt-faint); font-size: 10px; text-transform: uppercase; letter-spacing: .06em; }
  .brand-row { display: flex; gap: 8px; }
  .brand-sw {
    flex: 1; display: flex; align-items: center; gap: 8px; cursor: pointer;
    background: var(--panel-2); border: 1px solid var(--border); border-radius: 8px;
    padding: 8px 10px; color: var(--txt-dim); font-size: 12px; transition: border-color .12s, background .12s;
  }
  .brand-sw:hover { background: var(--panel-3); }
  .brand-sw.on { border-color: var(--accent); background: var(--accent-soft); color: var(--txt); }
  .brand-sw .chip { width: 16px; height: 16px; border-radius: 5px; flex: none; box-shadow: 0 0 0 1px rgba(255,255,255,.12) inset; }
  .accenti { display: flex; gap: 8px; align-items: center; }
  .pallino { width: 20px; height: 20px; border-radius: 50%; cursor: pointer; border: 2px solid transparent; }
  .pallino.on { border-color: var(--txt); }
  .custom { position: relative; width: 20px; height: 20px; border-radius: 50%; cursor: pointer; display: grid; place-items: center; border: 1px dashed var(--border-2); }
  .custom input { position: absolute; inset: 0; opacity: 0; cursor: pointer; }
  .custom .plus { color: var(--txt-faint); font-size: 14px; line-height: 1; pointer-events: none; }
</style>
