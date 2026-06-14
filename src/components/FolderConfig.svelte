<script>
  // Editor della configurazione ereditabile di una cartella/collezione:
  // header e auth applicati a tutte le richieste figlie.
  let { dir, nome, config, onSalva } = $props();

  let cfg = $state($state.snapshot(config));
  if (!Array.isArray(cfg.headers)) cfg.headers = [];
  if (!Array.isArray(cfg.variabili)) cfg.variabili = [];
  if (!cfg.auth) cfg.auth = { tipo: "none", token: "", utente: "", password: "", oauth2: null };

  function aggiungiHeader() {
    cfg.headers.push({ chiave: "", valore: "", attivo: true });
  }
  function rimuoviHeader(i) {
    cfg.headers.splice(i, 1);
  }
  function aggiungiVar() {
    cfg.variabili.push({ chiave: "", valore: "", segreto: false });
  }
  function rimuoviVar(i) {
    cfg.variabili.splice(i, 1);
  }
  async function salva() {
    await onSalva(dir, $state.snapshot(cfg));
  }
</script>

<div class="fc">
  <div class="fc-head">
    <span class="fc-ico">📁</span>
    <span class="fc-nome">{nome}</span>
    <span class="fc-sub">— header e auth ereditati dalle richieste figlie</span>
  </div>

  <h4>Header ereditati</h4>
  <table class="fc-kv">
    <tbody>
      {#each cfg.headers as h, i}
        <tr>
          <td><input type="checkbox" bind:checked={h.attivo} /></td>
          <td><input class="fc-inp" placeholder="chiave" bind:value={h.chiave} /></td>
          <td><input class="fc-inp" placeholder="valore" bind:value={h.valore} /></td>
          <td><span class="fc-rm" onclick={() => rimuoviHeader(i)}>✕</span></td>
        </tr>
      {/each}
    </tbody>
  </table>
  <button class="fc-mini" onclick={aggiungiHeader}>+ Header</button>

  <h4>Variabili di collezione/cartella</h4>
  <table class="fc-kv">
    <tbody>
      {#each cfg.variabili as v, i}
        <tr>
          <td><input class="fc-inp" placeholder="chiave" bind:value={v.chiave} /></td>
          <td><input class="fc-inp" placeholder="valore" bind:value={v.valore} /></td>
          <td><span class="fc-rm" onclick={() => rimuoviVar(i)}>✕</span></td>
        </tr>
      {/each}
    </tbody>
  </table>
  <button class="fc-mini" onclick={aggiungiVar}>+ Variabile</button>

  <h4>Auth ereditata</h4>
  <div class="fc-row">
    <label>Tipo</label>
    <select class="fc-inp" bind:value={cfg.auth.tipo}>
      <option value="none">Nessuna</option>
      <option value="bearer">Bearer Token</option>
      <option value="basic">Basic Auth</option>
    </select>
  </div>
  {#if cfg.auth.tipo === "bearer"}
    <div class="fc-row"><label>Token</label><input class="fc-inp" bind:value={cfg.auth.token} /></div>
  {:else if cfg.auth.tipo === "basic"}
    <div class="fc-row"><label>Utente</label><input class="fc-inp" bind:value={cfg.auth.utente} /></div>
    <div class="fc-row"><label>Password</label><input class="fc-inp" type="password" bind:value={cfg.auth.password} /></div>
  {/if}

  <div class="fc-actions">
    <button class="fc-save" onclick={salva}>Salva configurazione</button>
  </div>
  <p class="fc-hint">L'header proprio di una richiesta ha la precedenza su quello ereditato; l'auth ereditata si applica solo se la richiesta non ne ha una.</p>
</div>

<style>
  .fc { padding: 18px 22px; overflow-y: auto; }
  .fc-head { display: flex; align-items: center; gap: 8px; margin-bottom: 18px; }
  .fc-nome { font-weight: 600; font-size: 15px; }
  .fc-sub { color: var(--txt-faint); font-size: 12px; }
  h4 { font-size: 11px; text-transform: uppercase; letter-spacing: .05em; color: var(--txt-faint); margin: 16px 0 6px; }
  .fc-kv { width: 100%; border-collapse: collapse; }
  .fc-kv td { padding: 3px 4px; }
  .fc-row { display: flex; align-items: center; gap: 10px; margin: 6px 0; }
  .fc-row label { width: 70px; color: var(--txt-dim); font-size: 12.5px; }
  .fc-inp { flex: 1; width: 100%; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; color: var(--txt); font-size: 12.5px; outline: none; }
  .fc-inp:focus { border-color: var(--accent); }
  .fc-rm { color: var(--txt-faint); cursor: pointer; padding: 0 4px; }
  .fc-rm:hover { color: var(--red); }
  .fc-mini { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); border-radius: 6px; padding: 6px 10px; font-size: 12px; cursor: pointer; margin-top: 6px; }
  .fc-actions { margin-top: 18px; }
  .fc-save { background: linear-gradient(145deg,#8b6dff,#6c47ff); border: none; color: #fff; border-radius: 6px; padding: 8px 16px; font-size: 13px; cursor: pointer; }
  .fc-hint { margin-top: 12px; color: var(--txt-faint); font-size: 11.5px; max-width: 560px; }
</style>
