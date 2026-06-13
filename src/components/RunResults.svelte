<script>
  // Mostra l'esito di una catena di run (un passo per riga, con test e log).
  let { titolo, risultati = [] } = $props();

  const passati = $derived(risultati.filter((r) => r.ok).length);
</script>

<div class="rr">
  <div class="rr-head">
    <span class="t">{titolo}</span>
    <span class="riepilogo {passati === risultati.length ? 'ok' : 'ko'}">
      {passati}/{risultati.length} passi ok
    </span>
  </div>

  <div class="rr-body">
    {#each risultati as r, i}
      <div class="passo" class:ko={!r.ok}>
        <div class="riga1">
          <span class="badge {r.ok ? 'ok' : 'ko'}">{r.ok ? "OK" : "FAIL"}</span>
          <span class="num">#{i + 1}</span>
          <span class="nome">{r.nome}</span>
          <span class="sp"></span>
          {#if r.status}<span class="meta">{r.status}</span>{/if}
          {#if r.tempo != null}<span class="meta">{r.tempo} ms</span>{/if}
        </div>
        {#if r.errore}<div class="errore">{r.errore}</div>{/if}
        {#each r.tests as t}
          <div class="test">
            <span class="te {t.passato ? 'ok' : 'ko'}">{t.passato ? "PASS" : "FAIL"}</span>
            <span>{t.descrizione}</span>
            {#if t.dettaglio}<span class="det">{t.dettaglio}</span>{/if}
          </div>
        {/each}
        {#if r.logs?.length}
          <div class="logs">{r.logs.join("\n")}</div>
        {/if}
      </div>
    {/each}
    {#if risultati.length === 0}
      <div class="placeholder"><div>Nessun risultato.</div></div>
    {/if}
  </div>
</div>

<style>
  .rr { flex: 1; display: flex; flex-direction: column; min-height: 0; }
  .rr-head { display: flex; align-items: center; gap: 12px; padding: 12px 16px; border-bottom: 1px solid var(--border); }
  .rr-head .t { font-family: var(--mono); font-size: 12.5px; }
  .riepilogo { margin-left: auto; font-weight: 700; font-size: 12px; padding: 3px 9px; border-radius: 6px; }
  .riepilogo.ok { color: #56d364; background: rgba(63,185,80,.15); }
  .riepilogo.ko { color: #f8918c; background: rgba(248,81,73,.15); }
  .rr-body { flex: 1; overflow: auto; padding: 12px 16px; }
  .passo { border: 1px solid var(--border); border-radius: 9px; padding: 10px 12px; margin-bottom: 10px; background: var(--panel-2); }
  .passo.ko { border-color: rgba(248,81,73,.4); }
  .riga1 { display: flex; align-items: center; gap: 10px; }
  .badge { font-family: var(--mono); font-weight: 700; font-size: 11px; padding: 2px 8px; border-radius: 5px; }
  .badge.ok { color: #56d364; background: rgba(63,185,80,.15); }
  .badge.ko { color: #f8918c; background: rgba(248,81,73,.15); }
  .num { color: var(--txt-faint); font-family: var(--mono); font-size: 11px; }
  .nome { font-weight: 600; }
  .sp { flex: 1; }
  .meta { color: var(--txt-dim); font-family: var(--mono); font-size: 12px; }
  .errore { color: #f8918c; font-family: var(--mono); font-size: 12px; margin-top: 6px; }
  .test { display: flex; align-items: center; gap: 8px; font-size: 12px; margin-top: 6px; }
  .te { font-family: var(--mono); font-weight: 700; font-size: 10.5px; padding: 1px 6px; border-radius: 4px; }
  .te.ok { color: #56d364; background: rgba(63,185,80,.15); }
  .te.ko { color: #f8918c; background: rgba(248,81,73,.15); }
  .det { color: var(--txt-faint); font-family: var(--mono); }
  .logs { margin-top: 8px; padding: 8px 10px; background: var(--bg); border-radius: 6px; font-family: var(--mono); font-size: 11.5px; color: var(--txt-dim); white-space: pre-wrap; }
</style>
