<script>
  // Pannello dei log in basso al centro.
  import { logs, pulisciLog } from "../lib/log.svelte.js";

  let box; // riferimento per l'auto-scroll

  // Scrolla in fondo quando arriva un nuovo log.
  $effect(() => {
    logs.length;
    if (box) box.scrollTop = box.scrollHeight;
  });
</script>

<div class="log">
  <div class="log-head">
    <span class="t">LOG</span>
    <span class="n">{logs.length}</span>
    <span class="sp"></span>
    <span class="pulisci" onclick={pulisciLog} title="Svuota">Pulisci</span>
  </div>
  <div class="log-body" bind:this={box}>
    {#each logs as r}
      <div class="riga {r.livello}">
        <span class="ora">{r.ora}</span>
        <span class="msg">{r.testo}</span>
      </div>
    {/each}
    {#if logs.length === 0}
      <div class="vuoto">Nessun log.</div>
    {/if}
  </div>
</div>

<style>
  .log { display: flex; flex-direction: column; min-height: 0; height: 100%; background: var(--panel); border-top: 1px solid var(--border); }
  .log-head { display: flex; align-items: center; gap: 8px; padding: 6px 14px; border-bottom: 1px solid var(--border); }
  .log-head .t { font-size: 10.5px; font-weight: 600; letter-spacing: .6px; color: var(--txt-faint); }
  .log-head .n { font-size: 11px; color: var(--txt-faint); font-family: var(--mono); }
  .log-head .sp { flex: 1; }
  .log-head .pulisci { font-size: 11.5px; color: var(--txt-dim); cursor: pointer; }
  .log-head .pulisci:hover { color: var(--txt); }
  .log-body { flex: 1; overflow-y: auto; padding: 6px 14px; font-family: var(--mono); font-size: 11.5px; }
  .riga { display: flex; gap: 10px; padding: 1px 0; }
  .riga .ora { color: var(--txt-faint); flex-shrink: 0; }
  .riga .msg { white-space: pre-wrap; word-break: break-word; }
  .riga.info .msg { color: var(--txt-dim); }
  .riga.ok .msg { color: #56d364; }
  .riga.errore .msg { color: #f8918c; }
  .vuoto { color: var(--txt-faint); }
</style>
