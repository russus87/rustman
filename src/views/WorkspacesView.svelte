<script>
  // Vista "Workspaces": elenca e cambia la cartella di lavoro attiva.
  import { onMount } from "svelte";
  import * as api from "../lib/api.js";

  let { onCambiato } = $props();

  let cfg = $state({ workspaces: [], attivo: null });
  let pathInput = $state("");

  async function ricarica() {
    cfg = await api.listaWorkspaces();
  }
  onMount(ricarica);

  async function attiva(p) {
    await api.impostaWorkspaceAttivo(p);
    await ricarica();
    onCambiato?.();
  }
  async function rimuovi(p) {
    await api.rimuoviWorkspace(p);
    await ricarica();
    onCambiato?.();
  }

  async function aggiungi() {
    if (api.inTauri) {
      // Folder picker nativo di Tauri.
      const { open } = await import("@tauri-apps/plugin-dialog");
      const scelta = await open({ directory: true, title: "Scegli la cartella del workspace" });
      if (scelta) await attiva(scelta);
    } else {
      const p = pathInput.trim();
      if (p) {
        pathInput = "";
        await attiva(p);
      }
    }
  }
</script>

<div class="ws-head">WORKSPACES</div>
<div class="ws-list">
  {#each cfg.workspaces as p}
    <div class="ws-item" class:active={p === cfg.attivo} onclick={() => attiva(p)}>
      <span class="dot" class:on={p === cfg.attivo}></span>
      <span class="path" title={p}>{p}</span>
      <span class="rm" title="Rimuovi dall'elenco" onclick={(e) => { e.stopPropagation(); rimuovi(p); }}>✕</span>
    </div>
  {/each}
  {#if cfg.workspaces.length === 0}
    <div class="vuoto">Nessun workspace.</div>
  {/if}
</div>

<div class="ws-add">
  {#if !api.inTauri}
    <input class="inp" placeholder="Percorso cartella (lato server)" bind:value={pathInput}
      onkeydown={(e) => e.key === "Enter" && aggiungi()} />
  {/if}
  <button class="btn-add" onclick={aggiungi}>+ Aggiungi workspace</button>
</div>

<style>
  .ws-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); }
  .ws-list { flex: 1; overflow-y: auto; padding: 0 8px; }
  .ws-item { display: flex; align-items: center; gap: 8px; padding: 8px 8px; border-radius: 6px; cursor: pointer; color: var(--txt-dim); }
  .ws-item:hover { background: var(--panel-3); }
  .ws-item.active { background: rgba(124,92,255,.13); color: var(--txt); }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--txt-faint); flex-shrink: 0; }
  .dot.on { background: var(--green); box-shadow: 0 0 8px rgba(63,185,80,.6); }
  .path { flex: 1; font-family: var(--mono); font-size: 11.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .rm { color: var(--txt-faint); opacity: 0; padding: 0 4px; border-radius: 4px; }
  .ws-item:hover .rm { opacity: 1; }
  .rm:hover { color: var(--red); }
  .vuoto { color: var(--txt-faint); padding: 10px 8px; font-size: 12px; }
  .ws-add { padding: 10px 12px; border-top: 1px solid var(--border); display: flex; flex-direction: column; gap: 8px; }
  .inp { background: var(--panel-2); border: 1px solid var(--border); border-radius: 6px; padding: 7px 9px; color: var(--txt); font-size: 12px; outline: none; }
  .inp:focus { border-color: var(--accent); }
  .btn-add { background: var(--panel-3); color: var(--txt); border: 1px solid var(--border-2); border-radius: 7px; padding: 8px; cursor: pointer; font-size: 12.5px; }
  .btn-add:hover { background: #22222e; }
</style>
