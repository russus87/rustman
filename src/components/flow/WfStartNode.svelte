<script>
  // Nodo "start" del canvas workflow: punto d'ingresso del flusso.
  import { Handle, Position } from "@xyflow/svelte";
</script>

<div class="wf-start">
  <span class="dot"></span> Start
  <Handle type="source" position={Position.Right} />
</div>

<style>
  .wf-start {
    display: inline-flex; align-items: center; gap: 8px;
    background: var(--accent-soft); border: 1px solid var(--accent-line);
    border-radius: 999px; padding: 8px 16px; color: var(--txt); font-weight: 600; font-size: 12.5px;
  }
  .wf-start .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--accent); box-shadow: 0 0 8px var(--accent); }
  .wf-start :global(.svelte-flow__handle) {
    width: 9px; height: 9px; background: var(--accent); border: 2px solid var(--panel);
  }
</style>
