<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Pomodoro from "./Pomodoro.svelte";
  import BlockedWebsites from "./BlockedWebsites.svelte";

  let activeTab = $state<'editor' | 'pomodoro' | 'blocked'>('editor');

  // Editor state
  let content = $state("");
  let savedContent = $state("");

  const isDirty = $derived(content !== savedContent);

  // Blocked websites state
  let blockedDomains = $state<string[]>([]);

  $effect(() => {
    invoke<string>("load_notes").then((text) => {
      content = text;
      savedContent = text;
    });
    invoke<string[]>("read_blocked").then((domains) => {
      blockedDomains = domains;
    });
  });

  async function save() {
    try {
      await invoke("save_notes", { content });
      savedContent = content;
    } catch (e) {
      alert("Could not save: " + e);
    }
  }

  async function saveBlocked(domains: string[]) {
    await invoke("write_blocked", { domains });
    blockedDomains = domains;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (activeTab !== 'editor') return;
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      save();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="app">
  <header>
    <nav>
      <button
        class="tab"
        class:active={activeTab === 'editor'}
        onclick={() => activeTab = 'editor'}
      >Editor</button>
      <button
        class="tab"
        class:active={activeTab === 'pomodoro'}
        onclick={() => activeTab = 'pomodoro'}
      >Pomodoro</button>
      <button
        class="tab"
        class:active={activeTab === 'blocked'}
        onclick={() => activeTab = 'blocked'}
      >Blocked</button>
    </nav>

    {#if activeTab === 'editor'}
      <div class="editor-bar">
        <span class="filename">Notes{isDirty ? ' •' : ''}</span>
        <button onclick={save} disabled={!isDirty}>Save</button>
      </div>
    {/if}
  </header>

  {#if activeTab === 'editor'}
    <textarea
      bind:value={content}
      spellcheck="false"
      autocomplete="off"
      placeholder="Start typing..."
    ></textarea>
  {:else if activeTab === 'pomodoro'}
    <Pomodoro />
  {:else}
    <BlockedWebsites domains={blockedDomains} onSave={saveBlocked} />
  {/if}
</div>

<style>
  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(body) {
    overflow: hidden;
  }

  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
  }

  header {
    display: flex;
    align-items: center;
    background: #2d2d2d;
    border-bottom: 1px solid #3d3d3d;
    flex-shrink: 0;
    user-select: none;
    min-height: 34px;
  }

  nav {
    display: flex;
    flex-shrink: 0;
  }

  .tab {
    background: transparent;
    color: #888;
    border: none;
    border-right: 1px solid #3d3d3d;
    border-radius: 0;
    padding: 6px 16px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    border-bottom: 2px solid transparent;
    transition: color 0.15s;
  }

  .tab:hover {
    color: #ccc;
    background: #333;
  }

  .tab.active {
    color: #d4d4d4;
    border-bottom-color: #569cd6;
    background: #2d2d2d;
  }

  .editor-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex: 1;
    padding: 0 12px;
  }

  .filename {
    font-size: 13px;
    color: #aaa;
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

  textarea {
    flex: 1;
    width: 100%;
    padding: 16px 20px;
    background: #1e1e1e;
    color: #d4d4d4;
    border: none;
    outline: none;
    resize: none;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
    font-size: 14px;
    line-height: 1.6;
    tab-size: 2;
  }

  textarea::placeholder {
    color: #555;
  }
</style>
