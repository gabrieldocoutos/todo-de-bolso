<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { X, Globe, AppWindow, LayoutGrid, List } from "lucide-svelte";

  let {
    domains,
    apps,
    onSaveDomains,
    onSaveApps,
    focusActive,
  }: {
    domains: string[];
    apps: string[];
    onSaveDomains: (domains: string[]) => Promise<void>;
    onSaveApps: (apps: string[]) => Promise<void>;
    focusActive: boolean;
  } = $props();

  let entryType = $state<"domain" | "app">("domain");
  let newEntry = $state("");
  let localDomains = $state<string[]>([]);
  let localApps = $state<string[]>([]);
  let installedApps = $state<string[]>([]);
  let error = $state<string | null>(null);
  let showSuggestions = $state(false);
  let viewMode = $state<"grid" | "list">("grid");
  let showAll = $state(false);

  $effect(() => {
    localDomains = [...domains];
  });

  $effect(() => {
    localApps = [...apps];
  });

  $effect(() => {
    invoke<string[]>("list_installed_apps").then((list) => {
      installedApps = list;
    });
  });

  type RestrictedItem = { name: string; type: "domain" | "app" };

  let allItems = $derived<RestrictedItem[]>([
    ...localDomains.map((d) => ({ name: d, type: "domain" as const })),
    ...localApps.map((a) => ({ name: a, type: "app" as const })),
  ]);

  let visibleItems = $derived(showAll ? allItems : allItems.slice(0, 4));

  let filtered = $derived(
    entryType === "app" && newEntry.trim()
      ? installedApps
          .filter(
            (a) =>
              a.toLowerCase().includes(newEntry.trim().toLowerCase()) &&
              !localApps.includes(a),
          )
          .slice(0, 8)
      : [],
  );

  async function add(name?: string) {
    const value = (name ?? newEntry).trim();
    if (!value) return;
    error = null;

    try {
      if (entryType === "domain") {
        const domain = value.toLowerCase();
        if (localDomains.includes(domain)) return;
        localDomains = [...localDomains, domain];
        await onSaveDomains(localDomains);
      } else {
        if (localApps.includes(value)) return;
        localApps = [...localApps, value];
        await onSaveApps(localApps);
      }
    } catch (e: unknown) {
      error = "Could not save: " + String(e);
    }

    newEntry = "";
    showSuggestions = false;
  }

  async function remove(item: RestrictedItem) {
    error = null;
    try {
      if (item.type === "domain") {
        localDomains = localDomains.filter((d) => d !== item.name);
        await onSaveDomains(localDomains);
      } else {
        localApps = localApps.filter((a) => a !== item.name);
        await onSaveApps(localApps);
      }
    } catch (e: unknown) {
      error = "Could not save: " + String(e);
    }
  }

  function onInputKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") add();
  }
</script>

<div class="focus-restrictions">
  <div class="header">
    <h1 class="title">Focus Restrictions</h1>
    <p class="subtitle">
      Enhance focus by restricting access to distracting domains and desktop
      applications. Your deep work session is currently
      <strong class:status-active={focusActive} class:status-inactive={!focusActive}>
        {focusActive ? "Active" : "Inactive"}</strong
      >.
    </p>
  </div>

  <div class="content">
    <!-- Left: Add form -->
    <div class="form-panel">
      <h2 class="panel-label">Restrict New Entry</h2>

      <div class="field">
        <span class="field-label">Entry Type</span>
        <div class="type-toggle">
          <button
            class="toggle-btn"
            class:toggle-active={entryType === "domain"}
            onclick={() => { entryType = "domain"; newEntry = ""; }}
          >Domain</button>
          <button
            class="toggle-btn"
            class:toggle-active={entryType === "app"}
            onclick={() => { entryType = "app"; newEntry = ""; }}
          >App</button>
        </div>
      </div>

      <div class="field">
        <span class="field-label">
          {entryType === "domain" ? "Domain URL" : "App Name"}
        </span>
        <div class="input-wrap">
          <input
            bind:value={newEntry}
            onkeydown={onInputKeyDown}
            onfocus={() => (showSuggestions = true)}
            onblur={() => setTimeout(() => (showSuggestions = false), 150)}
            placeholder={entryType === "domain" ? "e.g. twitter.com" : "Enter app name..."}
            spellcheck="false"
            autocomplete="off"
          />
          {#if showSuggestions && filtered.length > 0}
            <ul class="suggestions">
              {#each filtered as suggestion (suggestion)}
                <li>
                  <button
                    class="suggestion-btn"
                    onmousedown={() => add(suggestion)}
                  >
                    {suggestion}
                  </button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>
      </div>

      <button
        class="cta"
        onclick={() => add()}
        disabled={!newEntry.trim()}
      >Add to Blocklist</button>

      {#if error}
        <p class="error">{error}</p>
      {/if}
    </div>

    <!-- Right: Restricted items -->
    <div class="items-panel">
      <div class="items-header">
        <h2 class="panel-label">Currently Restricted</h2>
        <span class="count-chip">{allItems.length} items</span>
        <div class="view-toggle">
          <button
            class="icon-btn"
            class:icon-active={viewMode === "grid"}
            onclick={() => (viewMode = "grid")}
            title="Grid view"
          ><LayoutGrid size={16} /></button>
          <button
            class="icon-btn"
            class:icon-active={viewMode === "list"}
            onclick={() => (viewMode = "list")}
            title="List view"
          ><List size={16} /></button>
        </div>
      </div>

      {#if allItems.length === 0}
        <p class="empty">No restricted entries yet.</p>
      {:else if viewMode === "grid"}
        <div class="card-grid">
          {#each visibleItems as item (item.type + ":" + item.name)}
            <div class="card">
              <button class="card-remove" onclick={() => remove(item)}>
                <X size={14} />
              </button>
              <div class="card-icon">
                {#if item.type === "domain"}
                  <Globe size={20} />
                {:else}
                  <AppWindow size={20} />
                {/if}
              </div>
              <h3 class="card-name">{item.name}</h3>
              <div class="card-chips">
                <span class="chip">{item.type === "domain" ? "Domain" : "App"}</span>
                <span class="chip">Hard Block</span>
              </div>
            </div>
          {/each}
        </div>
      {:else}
        <div class="list-view">
          {#each visibleItems as item (item.type + ":" + item.name)}
            <div class="list-item">
              <div class="list-icon">
                {#if item.type === "domain"}
                  <Globe size={16} />
                {:else}
                  <AppWindow size={16} />
                {/if}
              </div>
              <span class="list-name">{item.name}</span>
              <div class="list-chips">
                <span class="chip chip-sm">{item.type === "domain" ? "Domain" : "App"}</span>
                <span class="chip chip-sm">Hard Block</span>
              </div>
              <button class="list-remove" onclick={() => remove(item)}>
                <X size={14} />
              </button>
            </div>
          {/each}
        </div>
      {/if}

      {#if allItems.length > 4 && !showAll}
        <button class="load-all" onclick={() => (showAll = true)}>
          Load all restricted entries
        </button>
      {/if}
    </div>
  </div>
</div>

<style>
  .focus-restrictions {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #131313;
    color: #e5e2e1;
    overflow-y: auto;
    font-family: "Inter", "Menlo", "Monaco", "Courier New", sans-serif;
  }

  .header {
    padding: 28px 32px 0;
  }

  .title {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 1.5rem;
    font-weight: 700;
    color: #e5e2e1;
    margin: 0 0 8px;
  }

  .subtitle {
    font-size: 0.8125rem;
    color: #bccac4;
    margin: 0;
    line-height: 1.5;
  }

  .status-active {
    color: #6de5cb;
    font-weight: 600;
  }

  .status-inactive {
    color: #86948f;
    font-weight: 600;
  }

  .content {
    display: flex;
    gap: 24px;
    padding: 24px 32px 32px;
    flex: 1;
    min-height: 0;
  }

  /* ---- Form panel (left) ---- */
  .form-panel {
    background: #1b1b1c;
    border-radius: 0.375rem;
    padding: 24px;
    width: 280px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 20px;
    align-self: flex-start;
  }

  .panel-label {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: #6de5cb;
    margin: 0;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .field-label {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.6875rem;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: #bccac4;
  }

  .type-toggle {
    display: flex;
    gap: 0;
    border-radius: 0.375rem;
    overflow: hidden;
  }

  .toggle-btn {
    flex: 1;
    background: #202020;
    color: #bccac4;
    border: none;
    border-radius: 0;
    padding: 8px 16px;
    font-size: 0.8125rem;
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, color 0.15s;
  }

  .toggle-btn:hover:not(.toggle-active) {
    background: #2a2a2a;
  }

  .toggle-active {
    background: #4ec9b0;
    color: #00382e;
  }

  .input-wrap {
    position: relative;
  }

  input {
    width: 100%;
    background: #0e0e0e;
    color: #e5e2e1;
    border: 1px solid rgba(61, 73, 70, 0.15);
    border-radius: 0.375rem;
    padding: 10px 12px;
    font-size: 0.8125rem;
    font-family: inherit;
    outline: none;
    box-sizing: border-box;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  input:focus {
    border-color: #6de5cb;
    box-shadow: 0 0 0 4px rgba(109, 229, 203, 0.12);
  }

  input::placeholder {
    color: #86948f;
  }

  .suggestions {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    background: #202020;
    border: 1px solid rgba(61, 73, 70, 0.15);
    border-top: none;
    border-radius: 0 0 0.375rem 0.375rem;
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 200px;
    overflow-y: auto;
    z-index: 10;
  }

  .suggestion-btn {
    display: block;
    width: 100%;
    text-align: left;
    background: transparent;
    border: none;
    border-radius: 0;
    color: #e5e2e1;
    padding: 8px 12px;
    font-size: 0.8125rem;
    font-family: inherit;
    cursor: pointer;
  }

  .suggestion-btn:hover {
    background: #1b1b1c;
    color: #6de5cb;
  }

  .cta {
    background: linear-gradient(135deg, #6de5cb, #4ec9b0);
    color: #00382e;
    border: none;
    border-radius: 0.375rem;
    padding: 12px 20px;
    font-size: 0.8125rem;
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-weight: 600;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    transition: opacity 0.15s;
  }

  .cta:hover:not(:disabled) {
    opacity: 0.9;
  }

  .cta:disabled {
    opacity: 0.35;
    cursor: default;
  }

  .error {
    font-size: 0.75rem;
    color: #ffb4ab;
    margin: 0;
  }

  /* ---- Items panel (right) ---- */
  .items-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .items-header {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 16px;
  }

  .count-chip {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.6875rem;
    background: #353535;
    color: #80f7dc;
    padding: 3px 10px;
    border-radius: 9999px;
    font-weight: 500;
  }

  .view-toggle {
    margin-left: auto;
    display: flex;
    gap: 4px;
  }

  .icon-btn {
    background: transparent;
    border: none;
    color: #86948f;
    padding: 6px;
    border-radius: 0.375rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: color 0.15s, background 0.15s;
  }

  .icon-btn:hover {
    background: #202020;
    color: #e5e2e1;
  }

  .icon-active {
    color: #e5e2e1;
    background: #202020;
  }

  .empty {
    font-size: 0.8125rem;
    color: #86948f;
    margin: 24px 0;
  }

  /* ---- Grid view ---- */
  .card-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 12px;
    overflow-y: auto;
    flex: 1;
    align-content: start;
  }

  .card {
    background: #2a2a2a;
    border-radius: 0.375rem;
    padding: 20px;
    position: relative;
    display: flex;
    flex-direction: column;
    gap: 10px;
    transition: background 0.15s;
  }

  .card:hover {
    background: #303030;
  }

  .card-remove {
    position: absolute;
    top: 12px;
    right: 12px;
    background: transparent;
    border: none;
    color: #86948f;
    padding: 4px;
    border-radius: 0.25rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    transition: color 0.15s, background 0.15s;
  }

  .card-remove:hover {
    color: #ffb4ab;
    background: rgba(147, 0, 10, 0.15);
  }

  .card-icon {
    color: #6de5cb;
    display: flex;
    align-items: center;
  }

  .card-name {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.9375rem;
    font-weight: 600;
    color: #e5e2e1;
    margin: 0;
    word-break: break-all;
  }

  .card-chips {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }

  .chip {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.625rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    background: #353535;
    color: #80f7dc;
    padding: 3px 8px;
    border-radius: 9999px;
  }

  /* ---- List view ---- */
  .list-view {
    display: flex;
    flex-direction: column;
    overflow-y: auto;
    flex: 1;
  }

  .list-item {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 16px;
    border-radius: 0.375rem;
    transition: background 0.15s;
  }

  .list-item:hover {
    background: #1b1b1c;
  }

  .list-icon {
    color: #6de5cb;
    display: flex;
    align-items: center;
    flex-shrink: 0;
  }

  .list-name {
    font-size: 0.8125rem;
    font-weight: 500;
    color: #e5e2e1;
    flex: 1;
    word-break: break-all;
  }

  .list-chips {
    display: flex;
    gap: 6px;
    flex-shrink: 0;
  }

  .chip-sm {
    font-size: 0.5625rem;
    padding: 2px 6px;
  }

  .list-remove {
    background: transparent;
    border: none;
    color: #86948f;
    padding: 4px;
    border-radius: 0.25rem;
    cursor: pointer;
    display: flex;
    align-items: center;
    flex-shrink: 0;
    transition: color 0.15s, background 0.15s;
  }

  .list-remove:hover {
    color: #ffb4ab;
    background: rgba(147, 0, 10, 0.15);
  }

  .load-all {
    background: transparent;
    border: none;
    color: #86948f;
    font-size: 0.75rem;
    font-family: "Space Grotesk", "Inter", sans-serif;
    cursor: pointer;
    padding: 16px 0;
    text-align: center;
    letter-spacing: 0.04em;
    text-transform: uppercase;
    transition: color 0.15s;
  }

  .load-all:hover {
    color: #6de5cb;
  }

  /* Scrollbar */
  .card-grid::-webkit-scrollbar,
  .list-view::-webkit-scrollbar {
    width: 4px;
  }

  .card-grid::-webkit-scrollbar-track,
  .list-view::-webkit-scrollbar-track {
    background: #131313;
  }

  .card-grid::-webkit-scrollbar-thumb,
  .list-view::-webkit-scrollbar-thumb {
    background: #353535;
    border-radius: 10px;
  }
</style>
