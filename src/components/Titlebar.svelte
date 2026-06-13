<script>
  // Barra del titolo: marchio + versione.
  import { VERSIONE } from "../lib/versione.js";
</script>

<div class="titlebar">
  <div class="brand"><span class="logo">R</span>Rustman</div>
  <span class="versione">v{VERSIONE}</span>
</div>

<style>
  .versione {
    color: var(--txt-faint);
    font-family: var(--mono);
    font-size: 11.5px;
    padding: 2px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
  }
</style>
