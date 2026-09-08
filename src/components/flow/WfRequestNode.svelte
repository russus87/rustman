<script>
  // Nodo "richiesta" del canvas workflow: metodo + nome, con handle in/out.
  import { Handle, Position } from "@xyflow/svelte";
  let { data } = $props();
</script>

<div class="wf-node" class:mancante={data.mancante}>
  <Handle type="target" position={Position.Left} />
  <span class="m {data.classe}">{data.metodo || "?"}</span>
  <span class="nm">{data.label}</span>
  {#if data.ciclo}<span class="ciclo" title="Questo nodo viene ripetuto">{data.ciclo}</span>{/if}
  <Handle type="source" position={Position.Right} />
</div>

<style>
  .wf-node {
    display: flex; align-items: center; gap: 9px;
    background: var(--panel-2); border: 1px solid var(--border-2);
    border-radius: var(--radius); padding: 10px 14px; min-width: 150px; max-width: 230px;
    box-shadow: var(--shadow-soft); color: var(--txt); font-size: 12.5px;
  }
  .wf-node.mancante { border-color: var(--red); }
  .wf-node :global(.svelte-flow__handle) {
    width: 9px; height: 9px; background: var(--accent); border: 2px solid var(--panel);
  }
  .m { font-size: 10.5px; font-weight: 700; letter-spacing: .3px; }
  .m.get { color: var(--green); } .m.post { color: #d9a441; }
  .m.put { color: var(--blue); } .m.del { color: var(--red); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  /* Badge del ciclo: quante ripetizioni e con quale concorrenza. */
  .ciclo {
    margin-left: auto; flex-shrink: 0; font-size: 10.5px; font-family: var(--mono);
    color: var(--accent-2); background: var(--accent-soft);
    border: 1px solid var(--accent-line); border-radius: 5px; padding: 1px 5px;
  }
</style>
