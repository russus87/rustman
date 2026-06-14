<script>
  // Console per connessioni WebSocket e Server-Sent Events (SSE).
  // Usa le API native della webview (WebSocket / EventSource): nessun backend.
  let { tab } = $props();

  let conn = null; // WebSocket | EventSource
  let stato = $state("chiuso"); // chiuso | connesso | errore
  let messaggi = $state([]); // { dir: "out"|"in"|"sys", testo, ora }
  let input = $state("");

  function log(dir, testo) {
    messaggi.push({ dir, testo, ora: new Date().toLocaleTimeString() });
    if (messaggi.length > 500) messaggi.shift();
  }

  function connetti() {
    if (conn) disconnetti();
    const url = (tab.url || "").trim();
    if (!url) return;
    try {
      if (tab.protocollo === "sse") {
        const es = new EventSource(url);
        es.onopen = () => { stato = "connesso"; log("sys", `SSE aperto: ${url}`); };
        es.onmessage = (e) => log("in", e.data);
        es.onerror = () => { stato = "errore"; log("sys", "errore SSE (connessione chiusa?)"); };
        conn = es;
      } else {
        const ws = new WebSocket(url);
        ws.onopen = () => { stato = "connesso"; log("sys", `WebSocket aperto: ${url}`); };
        ws.onmessage = (e) => log("in", typeof e.data === "string" ? e.data : "[binario]");
        ws.onclose = () => { stato = "chiuso"; log("sys", "WebSocket chiuso"); conn = null; };
        ws.onerror = () => { stato = "errore"; log("sys", "errore WebSocket"); };
        conn = ws;
      }
    } catch (e) {
      stato = "errore";
      log("sys", `connessione fallita: ${e}`);
    }
  }

  function disconnetti() {
    if (!conn) return;
    try { conn.close(); } catch { /* ignora */ }
    conn = null;
    stato = "chiuso";
    log("sys", "disconnesso");
  }

  function invia() {
    if (!conn || tab.protocollo === "sse" || stato !== "connesso") return;
    const t = input;
    if (!t) return;
    conn.send(t);
    log("out", t);
    input = "";
  }

  function suTasto(e) {
    if (e.key === "Enter" && !e.shiftKey) { e.preventDefault(); invia(); }
  }

  // Chiude la connessione quando il componente viene smontato (tab chiuso).
  $effect(() => () => disconnetti());
</script>

<div class="sock">
  <div class="bar">
    <span class="proto {tab.protocollo}">{tab.protocollo === "sse" ? "SSE" : "WS"}</span>
    <input
      class="url"
      placeholder={tab.protocollo === "sse" ? "http(s)://… (event stream)" : "ws://  o  wss://…"}
      bind:value={tab.url}
      disabled={stato === "connesso"}
    />
    {#if stato === "connesso"}
      <button class="btn rosso" onclick={disconnetti}>Disconnetti</button>
    {:else}
      <button class="btn verde" onclick={connetti}>Connetti</button>
    {/if}
    <span class="dot {stato}" title={stato}></span>
  </div>

  <div class="msgs">
    {#each messaggi as m}
      <div class="m {m.dir}">
        <span class="ora">{m.ora}</span>
        <span class="frec">{m.dir === "out" ? "↑" : m.dir === "in" ? "↓" : "•"}</span>
        <span class="txt">{m.testo}</span>
      </div>
    {/each}
    {#if messaggi.length === 0}
      <div class="vuoto">Nessun messaggio. {tab.protocollo === "sse" ? "Connetti per ricevere gli eventi." : "Connetti e invia un messaggio."}</div>
    {/if}
  </div>

  {#if tab.protocollo !== "sse"}
    <div class="invio">
      <textarea placeholder="Messaggio da inviare… (Invio per spedire)" bind:value={input} onkeydown={suTasto} disabled={stato !== "connesso"}></textarea>
      <button class="btn verde" onclick={invia} disabled={stato !== "connesso"}>Invia</button>
    </div>
  {/if}
</div>

<style>
  .sock { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  .bar { display: flex; align-items: center; gap: 8px; padding: 12px 16px; border-bottom: 1px solid var(--border); }
  .proto { font-family: var(--mono); font-weight: 700; font-size: 11px; padding: 3px 8px; border-radius: 6px; background: var(--panel-3); }
  .proto.sse { color: #e2b340; } .proto.ws { color: #4a9eff; }
  .url { flex: 1; background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px; padding: 8px 10px; color: var(--txt); font-family: var(--mono); font-size: 13px; outline: none; }
  .url:focus { border-color: var(--accent); }
  .btn { border: none; border-radius: 7px; padding: 8px 14px; font-size: 13px; cursor: pointer; color: #fff; }
  .btn.verde { background: linear-gradient(145deg,#3fb950,#2f9e41); }
  .btn.rosso { background: linear-gradient(145deg,#f85149,#c73a33); }
  .btn:disabled { opacity: .5; cursor: default; }
  .dot { width: 9px; height: 9px; border-radius: 50%; background: var(--txt-faint); }
  .dot.connesso { background: var(--green); box-shadow: 0 0 8px rgba(63,185,80,.6); }
  .dot.errore { background: var(--red); }
  .msgs { flex: 1; overflow-y: auto; padding: 8px 12px; font-family: var(--mono); font-size: 12.5px; min-height: 0; }
  .m { display: flex; gap: 8px; padding: 3px 0; align-items: baseline; }
  .m .ora { color: var(--txt-faint); font-size: 11px; }
  .m .frec { width: 12px; }
  .m.out .frec { color: #4a9eff; } .m.in .frec { color: var(--green); } .m.sys { color: var(--txt-faint); }
  .m .txt { white-space: pre-wrap; word-break: break-word; }
  .vuoto { color: var(--txt-faint); padding: 10px 0; }
  .invio { display: flex; gap: 8px; padding: 10px 12px; border-top: 1px solid var(--border); }
  .invio textarea { flex: 1; resize: none; height: 46px; background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px; padding: 8px 10px; color: var(--txt); font-family: var(--mono); font-size: 12.5px; outline: none; }
  .invio textarea:focus { border-color: var(--accent); }
</style>
