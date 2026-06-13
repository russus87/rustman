<script>
  // Divisorio trascinabile. direction="col" → barra verticale (ridimensiona larghezza);
  // direction="row" → barra orizzontale (ridimensiona altezza).
  // `onResize(deltaPx)` viene chiamata durante il trascinamento.
  let { direction = "col", onResize } = $props();

  function giu(e) {
    e.preventDefault();
    let ultimo = direction === "col" ? e.clientX : e.clientY;
    const muovi = (ev) => {
      const cur = direction === "col" ? ev.clientX : ev.clientY;
      onResize(cur - ultimo);
      ultimo = cur;
    };
    const su = () => {
      window.removeEventListener("mousemove", muovi);
      window.removeEventListener("mouseup", su);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
    window.addEventListener("mousemove", muovi);
    window.addEventListener("mouseup", su);
    document.body.style.cursor = direction === "col" ? "col-resize" : "row-resize";
    document.body.style.userSelect = "none";
  }
</script>

<div class="splitter {direction}" onmousedown={giu}></div>

<style>
  .splitter {
    background: transparent;
    flex-shrink: 0;
    z-index: 5;
  }
  .splitter.col {
    width: 5px;
    cursor: col-resize;
  }
  .splitter.row {
    height: 5px;
    cursor: row-resize;
  }
  .splitter:hover {
    background: var(--accent);
  }
</style>
