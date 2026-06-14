<script>
  // Griglia degli esiti di un invio "batch" di un'intera cartella/collezione.
  let { titolo, righe = [], inCorso = false, ognis = 0, onPianifica, onFerma } = $props();
  let intervallo = $state(60);

  function classeMetodo(m) {
    if (m === "GET") return "get";
    if (m === "POST") return "post";
    if (m === "PUT" || m === "PATCH") return "put";
    if (m === "DELETE") return "del";
    return "";
  }
</script>

<div class="batch">
  <div class="bhead">
    <span class="bttl">{titolo}</span>
    {#if inCorso}<span class="binc">esecuzione…</span>{/if}
    <span class="bspc"></span>
    {#if ognis > 0}
      <span class="sched on">🔁 ogni {ognis}s</span>
      <button class="sched-btn ferma" onclick={() => onFerma?.()}>Ferma</button>
    {:else}
      <input class="sched-inp" type="number" min="2" max="3600" bind:value={intervallo} />
      <span class="sched-lbl">s</span>
      <button class="sched-btn" onclick={() => onPianifica?.(Number(intervallo))}>🔁 Ripeti</button>
    {/if}
    <span class="bcnt">{righe.length} richieste</span>
  </div>
  <table class="bgrid">
    <thead>
      <tr><th>Metodo</th><th>Richiesta</th><th>URL</th><th>Status</th><th>Tempo</th><th>Test</th></tr>
    </thead>
    <tbody>
      {#each righe as r}
        <tr>
          <td><span class="m {classeMetodo(r.metodo)}">{r.metodo}</span></td>
          <td class="nome">{r.nome}</td>
          <td class="url" title={r.url}>{r.url}</td>
          <td>
            {#if r.errore}<span class="st ko" title={r.errore}>ERR</span>
            {:else}<span class="st {r.status >= 400 ? 'ko' : 'ok'}">{r.status}</span>{/if}
          </td>
          <td class="num">{r.tempo} ms</td>
          <td>
            {#if r.tot > 0}<span class="st {r.ok === r.tot ? 'ok' : 'ko'}">{r.ok}/{r.tot}</span>
            {:else}<span class="muted">—</span>{/if}
          </td>
        </tr>
      {/each}
    </tbody>
  </table>
  {#if righe.length === 0 && !inCorso}
    <div class="vuoto">Nessuna richiesta nella cartella.</div>
  {/if}
</div>

<style>
  .batch { padding: 16px 20px; overflow: auto; height: 100%; }
  .bhead { display: flex; align-items: center; gap: 10px; margin-bottom: 14px; }
  .bttl { font-weight: 600; font-size: 14px; }
  .binc { color: var(--accent-2); font-size: 12px; }
  .bspc { flex: 1; }
  .bcnt { color: var(--txt-faint); font-size: 12px; margin-left: 10px; }
  .sched-inp { width: 56px; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 4px 6px; color: var(--txt); font-size: 12px; outline: none; }
  .sched-lbl { color: var(--txt-faint); font-size: 12px; }
  .sched-btn { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); border-radius: 6px; padding: 4px 10px; font-size: 12px; cursor: pointer; }
  .sched-btn.ferma { background: linear-gradient(145deg,#f85149,#c73a33); color: #fff; border: none; }
  .sched.on { color: var(--green); font-size: 12px; }
  .bgrid { width: 100%; border-collapse: collapse; font-size: 12.5px; }
  .bgrid th { text-align: left; color: var(--txt-faint); font-weight: 600; font-size: 11px; text-transform: uppercase; padding: 6px 10px; border-bottom: 1px solid var(--border); }
  .bgrid td { padding: 7px 10px; border-bottom: 1px solid var(--border); vertical-align: middle; }
  .m { font-family: var(--mono); font-weight: 700; font-size: 10.5px; }
  .m.get { color: var(--green); } .m.post { color: #e2b340; } .m.put { color: #4a9eff; } .m.del { color: var(--red); }
  .nome { font-weight: 500; }
  .url { color: var(--txt-dim); font-family: var(--mono); max-width: 320px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .num { font-family: var(--mono); color: var(--txt-dim); }
  .st { font-family: var(--mono); font-weight: 700; font-size: 11px; }
  .st.ok { color: var(--green); } .st.ko { color: var(--red); }
  .muted { color: var(--txt-faint); }
  .vuoto { color: var(--txt-faint); padding: 16px 0; }
</style>
