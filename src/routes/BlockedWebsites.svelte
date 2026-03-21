<script lang="ts">
  let {
    domains,
    onSave,
  }: {
    domains: string[];
    onSave: (domains: string[]) => Promise<void>;
  } = $props();

  let local = $state<string[]>([]);
  let newDomain = $state("");
  let saving = $state(false);
  let error = $state<string | null>(null);

  // Sync local copy when parent domains change (including initial load)
  $effect(() => {
    local = [...domains];
  });

  function add() {
    const domain = newDomain.trim().toLowerCase();
    if (!domain || local.includes(domain)) return;
    local = [...local, domain];
    newDomain = "";
  }

  function remove(domain: string) {
    local = local.filter((d) => d !== domain);
  }

  async function save() {
    saving = true;
    error = null;
    try {
      await onSave(local);
    } catch (e: unknown) {
      const msg = String(e);
      if (msg.includes("Cancelled")) {
        error = "Save cancelled.";
      } else {
        error = "Could not save: " + msg;
      }
    } finally {
      saving = false;
    }
  }

  function onInputKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") add();
  }
</script>

<div class="blocked">
  <div class="add-row">
    <input
      bind:value={newDomain}
      onkeydown={onInputKeyDown}
      placeholder="example.com"
      spellcheck="false"
      autocomplete="off"
    />
    <button onclick={add} disabled={!newDomain.trim()}>Add</button>
  </div>

  {#if local.length === 0}
    <p class="empty">No blocked websites.</p>
  {:else}
    <ul>
      {#each local as domain (domain)}
        <li>
          <span class="domain">{domain}</span>
          <button class="remove" onclick={() => remove(domain)}>✕</button>
        </li>
      {/each}
    </ul>
  {/if}

  <div class="footer">
    {#if error}
      <span class="error">{error}</span>
    {/if}
    <button class="save" onclick={save} disabled={saving}>
      {saving ? "Saving…" : "Save"}
    </button>
  </div>
</div>

<style>
  .blocked {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 20px;
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
    overflow: hidden;
  }

  .add-row {
    display: flex;
    gap: 8px;
    margin-bottom: 16px;
  }

  input {
    flex: 1;
    background: #2d2d2d;
    color: #d4d4d4;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 5px 10px;
    font-size: 13px;
    font-family: inherit;
    outline: none;
  }

  input:focus {
    border-color: #569cd6;
  }

  input::placeholder {
    color: #555;
  }

  ul {
    list-style: none;
    overflow-y: auto;
    flex: 1;
    margin: 0;
  }

  li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 6px 8px;
    border-radius: 4px;
    font-size: 13px;
  }

  li:hover {
    background: #2a2a2a;
  }

  .domain {
    color: #9cdcfe;
  }

  .empty {
    font-size: 13px;
    color: #555;
    margin: 8px 0;
  }

  .footer {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 12px;
    margin-top: 16px;
    padding-top: 12px;
    padding-right: 84px;
    border-top: 1px solid #3d3d3d;
    flex-shrink: 0;
  }

  .error {
    font-size: 12px;
    color: #f48771;
  }

  button {
    background: #3a3a3a;
    color: #d4d4d4;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 3px 10px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
  }

  button:hover:not(:disabled) {
    background: #4a4a4a;
  }

  button:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .remove {
    background: transparent;
    border-color: transparent;
    color: #666;
    padding: 2px 6px;
    font-size: 11px;
  }

  .remove:hover:not(:disabled) {
    background: #3a2020;
    border-color: #6b3030;
    color: #f48771;
  }

  .save {
    padding: 4px 16px;
  }
</style>
