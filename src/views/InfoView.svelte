<script>
  // Vista "Info": versione corrente e changelog.
  import { VERSIONE, CHANGELOG } from "../lib/versione.js";
</script>

<div class="info-head">INFO</div>
<div class="info-body">
  <div class="logo-riga">
    <span class="logo">R</span>
    <div>
      <div class="nome">Rustman</div>
      <div class="ver">versione {VERSIONE}</div>
    </div>
  </div>

  <div class="cl-titolo">Changelog</div>
  {#each CHANGELOG as c, i}
    <div class="cl" class:corrente={i === 0}>
      <div class="cl-v">{c.versione}{#if i === 0} <span class="tag">attuale</span>{/if}</div>
      <ul>
        {#each c.voci as v}<li>{v}</li>{/each}
      </ul>
    </div>
  {/each}
</div>

<style>
  .info-head { padding: 14px 14px 8px; font-size: 11px; font-weight: 600; letter-spacing: .8px; color: var(--txt-faint); }
  .info-body { padding: 8px 14px; overflow-y: auto; }
  .logo-riga { display: flex; align-items: center; gap: 12px; margin-bottom: 18px; }
  .logo { width: 36px; height: 36px; border-radius: 9px; background: linear-gradient(145deg,#8b6dff,#5b3df0); display: grid; place-items: center; color: #fff; font-weight: 800; font-size: 18px; }
  .nome { font-weight: 600; font-size: 15px; }
  .ver { color: var(--txt-dim); font-size: 12.5px; font-family: var(--mono); }
  .cl-titolo { font-size: 11px; font-weight: 600; letter-spacing: .6px; color: var(--txt-faint); margin-bottom: 8px; }
  .cl { margin-bottom: 14px; padding-left: 10px; border-left: 2px solid var(--border); }
  .cl.corrente { border-left-color: var(--accent); }
  .cl-v { font-family: var(--mono); font-size: 12.5px; color: var(--txt); margin-bottom: 5px; }
  .tag { background: rgba(124,92,255,.18); color: var(--accent-2); font-size: 10px; padding: 1px 6px; border-radius: 5px; font-family: var(--sans); }
  ul { margin: 0 0 0 16px; padding: 0; }
  li { color: var(--txt-dim); font-size: 12px; margin-bottom: 3px; }
</style>
