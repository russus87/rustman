<script>
  // Vista "Source Control": stato repo, remote, pull/push, commit, modifiche, cronologia.
  import * as api from "../lib/api.js";
  import { logga } from "../lib/log.svelte.js";

  let { segnale, onApriDiff, onCambiamento } = $props();

  let info = $state(null); // { branch, remote, ahead, behind }
  let modifiche = $state([]);
  let selezionati = $state({});
  let messaggio = $state("");
  let storia = $state([]);
  let remoteInput = $state("");
  let esito = $state("");
  let inAzione = $state(false);

  async function ricarica() {
    try {
      info = await api.gitInfo();
      remoteInput = info?.remote ?? "";
      modifiche = await api.gitStato();
      const sel = {};
      for (const m of modifiche) sel[m.file] = selezionati[m.file] ?? true;
      selezionati = sel;
      storia = await api.gitLog();
    } catch (e) {
      console.error("git ricarica:", e);
    }
  }

  $effect(() => {
    segnale;
    ricarica();
  });

  const nSelezionati = $derived(modifiche.filter((m) => selezionati[m.file]).length);

  function toggle(file) {
    selezionati[file] = !selezionati[file];
  }

  async function azione(fn) {
    inAzione = true;
    esito = "";
    try {
      esito = (await fn()) || "";
      if (esito) logga("ok", `Git: ${esito}`);
      await ricarica();
      onCambiamento?.();
    } catch (e) {
      esito = String(e);
      logga("errore", `Git: ${e}`);
    } finally {
      inAzione = false;
    }
  }

  const commit = () => {
    const files = modifiche.filter((m) => selezionati[m.file]).map((m) => m.file);
    if (files.length === 0 || !messaggio.trim()) return;
    return azione(async () => {
      const sha = await api.gitCommit(messaggio, files);
      messaggio = "";
      return `Commit ${sha}`;
    });
  };
  const impostaRemote = () => azione(async () => {
    await api.gitImpostaRemote(remoteInput.trim());
    return "Remote impostato.";
  });
  const pull = () => azione(api.gitPull);
  const push = () => azione(api.gitPush);
</script>

<div class="git-head">SOURCE CONTROL</div>

<div class="sezione">
  <div class="riga">
    <span class="branch">⎇ {info?.branch ?? "—"}</span>
    {#if info && (info.ahead || info.behind)}
      <span class="ab">↑{info.ahead} ↓{info.behind}</span>
    {/if}
  </div>
  <div class="remote-row">
    <input class="inp" placeholder="URL remote (origin)" bind:value={remoteInput} />
    <button class="mini" onclick={impostaRemote} disabled={inAzione}>Imposta</button>
  </div>
  <div class="riga">
    <button class="mini grow" onclick={pull} disabled={inAzione}>⬇ Pull</button>
    <button class="mini grow" onclick={push} disabled={inAzione}>⬆ Push</button>
  </div>
  {#if esito}<div class="esito">{esito}</div>{/if}
</div>

<div class="sezione">
  <textarea class="msg" placeholder="Messaggio di commit" bind:value={messaggio}></textarea>
  <button class="commit" onclick={commit} disabled={inAzione || nSelezionati === 0 || !messaggio.trim()}>
    ✓ Commit ({nSelezionati})
  </button>
</div>

<div class="lista-h">MODIFICHE ({modifiche.length})</div>
<div class="lista">
  {#if modifiche.length === 0}
    <div class="vuoto">Nessuna modifica</div>
  {/if}
  {#each modifiche as m}
    <div class="chg" onclick={() => onApriDiff(m.file)}>
      <input type="checkbox" checked={selezionati[m.file]} onclick={(e) => { e.stopPropagation(); toggle(m.file); }} />
      <span class="fn">{m.file}</span>
      <span class="st {m.stato.toLowerCase()}">{m.stato}</span>
    </div>
  {/each}
</div>

<div class="lista-h">CRONOLOGIA</div>
<div class="lista storia">
  {#if storia.length === 0}<div class="vuoto">Nessun commit</div>{/if}
  {#each storia as c}
    <div class="commit-riga">
      <span class="sha">{c.sha}</span> {c.messaggio}
      <div class="meta">{c.autore} · {c.quando}</div>
    </div>
  {/each}
</div>

<style>
  .git-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); }
  .sezione { padding: 8px 12px; border-bottom: 1px solid var(--border); display: flex; flex-direction: column; gap: 8px; }
  .riga { display: flex; align-items: center; gap: 8px; }
  .branch { font-family: var(--mono); font-size: 12.5px; color: var(--txt); }
  .ab { margin-left: auto; font-family: var(--mono); font-size: 12px; color: var(--txt-dim); }
  .remote-row { display: flex; gap: 6px; }
  .inp { flex: 1; background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 6px 8px; color: var(--txt); font-size: 12px; outline: none; }
  .inp:focus { border-color: var(--accent); }
  .mini { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); border-radius: 6px; padding: 6px 10px; font-size: 12px; cursor: pointer; }
  .mini:hover:not(:disabled) { background: #22222e; }
  .mini:disabled { opacity: .55; cursor: default; }
  .grow { flex: 1; }
  .esito { font-family: var(--mono); font-size: 11.5px; color: var(--txt-dim); white-space: pre-wrap; }
  .msg { background: var(--panel-2); border: 1px solid var(--border); border-radius: 7px; color: var(--txt); font-family: var(--sans); font-size: 12.5px; padding: 8px 10px; resize: none; height: 54px; outline: none; }
  .msg:focus { border-color: var(--accent); }
  .commit { background: linear-gradient(145deg,#8b6dff,#6c47ff); color: #fff; border: none; border-radius: 8px; padding: 9px; font-weight: 600; cursor: pointer; }
  .commit:disabled { opacity: .55; cursor: default; }
  .lista-h { padding: 10px 14px 4px; font-size: 10.5px; font-weight: 600; letter-spacing: .6px; color: var(--txt-faint); }
  .lista { overflow-y: auto; padding: 0 8px; }
  .lista.storia { max-height: 220px; }
  .vuoto { color: var(--txt-faint); padding: 8px; font-size: 12px; }
  .chg { display: flex; align-items: center; gap: 8px; padding: 5px 6px; border-radius: 6px; cursor: pointer; }
  .chg:hover { background: var(--panel-3); }
  .chg .fn { flex: 1; font-family: var(--mono); font-size: 12px; color: var(--txt); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .st { font-size: 11px; font-weight: 700; width: 14px; text-align: center; }
  .st.m { color: var(--orange); } .st.a { color: var(--green); } .st.d { color: var(--red); }
  .commit-riga { padding: 6px 6px; border-bottom: 1px solid var(--border); font-size: 12px; }
  .commit-riga .sha { font-family: var(--mono); color: var(--accent-2); }
  .commit-riga .meta { color: var(--txt-faint); font-size: 11px; margin-top: 2px; }
</style>
