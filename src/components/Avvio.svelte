<script>
  // Schermata di avvio, alla Postman: marchio al centro e una barra che si
  // riempie mentre workspace, collezioni e il resto vengono caricati.
  //
  // Prende il posto di quella statica in index.html (visibile prima che arrivi
  // il bundle) con lo stesso markup e le stesse classi globali .avvio*, quindi
  // il passaggio fra le due non si vede: cambia solo la barra, che da
  // "indefinita" diventa proporzionale ai passi completati.
  //
  // Decide anche quando chiudersi (onFine): la barra deve arrivare in fondo
  // prima di sparire, altrimenti su un caricamento veloce si vedrebbe partire e
  // svanire a metà.
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { VERSIONE } from "../lib/versione.js";

  // passi: [{ id, etichetta, testo, stato: "attesa"|"ok"|"errore", errore }]
  let { passi, onFine } = $props();

  // Durata del riempimento della barra: la chiusura aspetta che sia finito.
  const RIEMPIMENTO_MS = 450;
  // Sotto questa durata, contata dall'apertura della finestra e non dal
  // montaggio, la schermata sarebbe un lampo: su un workspace piccolo i dati
  // arrivano in poche decine di millisecondi.
  const MINIMO_MS = 800;

  const fatti = $derived(passi.filter((p) => p.stato !== "attesa").length);
  const falliti = $derived(passi.filter((p) => p.stato === "errore"));
  const finito = $derived(fatti === passi.length);
  const stato = $derived(
    !finito ? (passi.find((p) => p.stato === "attesa")?.testo ?? "")
      : falliti.length ? "Alcuni dati non sono stati caricati"
      : "Pronto",
  );

  onMount(() => {
    // Da qui in poi è questa a mostrarsi: quella statica si può togliere.
    document.getElementById("avvio")?.remove();
  });

  // Se qualcosa non è andato la schermata resta: chi apre l'app deve poter
  // leggere cosa manca prima di trovarsi davanti un albero vuoto. In quel caso
  // si chiude col pulsante Continua.
  $effect(() => {
    if (!finito || falliti.length) return;
    const attesa = Math.max(MINIMO_MS - performance.now(), RIEMPIMENTO_MS);
    const t = setTimeout(() => onFine?.(), attesa);
    return () => clearTimeout(t);
  });
</script>

<div class="avvio" out:fade={{ duration: 220 }} role="status" aria-live="polite">
  <div class="avvio-centro">
    <div class="avvio-logo">R</div>
    <div class="avvio-nome">Rustman</div>
    <div class="avvio-barra">
      <div class="avvio-riempi" class:ko={falliti.length}
        style="width:{(fatti / passi.length) * 100}%;transition:width {RIEMPIMENTO_MS}ms ease-out"></div>
    </div>
    <div class="avvio-stato">{stato}</div>

    {#if finito && falliti.length}
      <div class="errori">
        {#each falliti as p (p.id)}
          <div class="errore"><b>{p.etichetta}</b>: {p.errore}</div>
        {/each}
        <div class="nota">
          L'app si apre lo stesso, ma {falliti.length === 1 ? "quella sezione resta vuota" : "quelle sezioni restano vuote"}.
        </div>
        <button class="continua" onclick={() => onFine?.()}>Continua</button>
      </div>
    {/if}
  </div>
  <div class="avvio-ver">v{VERSIONE}</div>
</div>

<style>
  .avvio-riempi.ko { background: var(--red); }
  /* Sotto il blocco centrale, fuori dal flusso: il logo non si sposta. */
  .errori {
    position: absolute; top: calc(100% + 22px); left: 50%; transform: translateX(-50%);
    width: min(440px, 86vw);
    display: flex; flex-direction: column; align-items: center; gap: 8px;
    text-align: center;
  }
  .errore { color: var(--red); font-size: 12.5px; line-height: 1.45; }
  .errore b { font-weight: 600; }
  .nota { color: var(--txt-dim); font-size: 12px; }
  .continua {
    margin-top: 6px;
    background: linear-gradient(145deg, var(--accent-2), var(--accent));
    color: var(--accent-contrast); border: none; border-radius: 8px;
    padding: 8px 22px; font: inherit; font-weight: 600; cursor: pointer;
  }
  .continua:hover { filter: brightness(1.08); }
</style>
