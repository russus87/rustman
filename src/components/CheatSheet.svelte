<script>
  // Modale con le scorciatoie da tastiera.
  let { onChiudi } = $props();
  const scorciatoie = [
    ["Ctrl/Cmd + K", "Command Palette"],
    ["Ctrl/Cmd + Invio", "Invia la richiesta"],
    ["Ctrl/Cmd + S", "Salva la richiesta"],
    ["{{", "Autocomplete delle variabili nell'URL"],
    ["Esc", "Chiudi palette / questo pannello"],
  ];
</script>

<div class="ov" onclick={onChiudi}>
  <div class="card" onclick={(e) => e.stopPropagation()}>
    <div class="h">⌨ Scorciatoie da tastiera</div>
    <table>
      {#each scorciatoie as [k, d]}
        <tr><td class="k"><kbd>{k}</kbd></td><td class="d">{d}</td></tr>
      {/each}
    </table>
  </div>
</div>

<style>
  .ov { position: fixed; inset: 0; background: rgba(0,0,0,.45); display: flex; align-items: center; justify-content: center; z-index: 1000; }
  .card { width: 460px; max-width: 92vw; background: var(--panel); border: 1px solid var(--border-2); border-radius: 12px; box-shadow: var(--shadow); padding: 20px 22px; }
  .h { font-weight: 600; font-size: 15px; margin-bottom: 14px; }
  table { width: 100%; border-collapse: collapse; }
  td { padding: 8px 4px; border-bottom: 1px solid var(--border); }
  td.d { color: var(--txt-dim); font-size: 13px; }
  kbd { font-family: var(--mono); font-size: 12px; background: var(--panel-3); border: 1px solid var(--border-2); border-radius: 5px; padding: 2px 7px; color: var(--txt); }
</style>
