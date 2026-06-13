<script>
  // Barra attività stile VS Code: cambia la vista mostrata nella sidebar.
  let { vista, onCambiaVista } = $props();

  // Voci: chiave vista + tooltip + path SVG dell'icona.
  const voci = [
    { v: "collezioni", t: "Collections", d: "M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" },
    { v: "run", t: "Run", d: "M5 3l14 9-14 9z" },
    { v: "storia", t: "History", d: "M12 8v5l3 2M3.05 11a9 9 0 1 1 .5 4M3 4v5h5" },
    { v: "git", t: "Git", d: "M6 3v12M6 15a3 3 0 1 0 0 6 3 3 0 0 0 0-6zM6 3a3 3 0 1 0 0 0zM18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zm0 0v1a4 4 0 0 1-4 4H9" },
    { v: "ambienti", t: "Environments", d: "M8 3H7a2 2 0 0 0-2 2v4l-2 3 2 3v4a2 2 0 0 0 2 2h1M16 3h1a2 2 0 0 1 2 2v4l2 3-2 3v4a2 2 0 0 1-2 2h-1" },
    { v: "workspaces", t: "Workspaces", d: "M3 9l9-6 9 6v9a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z M9 21V12h6v9" },
    { v: "settings", t: "Settings", d: "M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19 12a7 7 0 0 0-.1-1.3l2-1.5-2-3.4-2.3 1a7 7 0 0 0-2.2-1.3l-.3-2.5H10l-.3 2.5a7 7 0 0 0-2.2 1.3l-2.3-1-2 3.4 2 1.5A7 7 0 0 0 5 12c0 .4 0 .9.1 1.3l-2 1.5 2 3.4 2.3-1a7 7 0 0 0 2.2 1.3l.3 2.5h4l.3-2.5a7 7 0 0 0 2.2-1.3l2.3 1 2-3.4-2-1.5z" },
    { v: "info", t: "Info", d: "M12 21a9 9 0 1 0 0-18 9 9 0 0 0 0 18zM12 11v5M12 8h.01" },
  ];
</script>

<div class="rail">
  <div class="avatar">R</div>
  {#each voci as voce}
    <div
      class="ri"
      class:active={vista === voce.v}
      title={voce.t}
      onclick={() => onCambiaVista(voce.v)}
    >
      <svg width="19" height="19" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
        <path d={voce.d} />
      </svg>
    </div>
  {/each}
  <div class="grow"></div>
</div>
