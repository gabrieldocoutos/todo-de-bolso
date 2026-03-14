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
  let blockingActive = $state(false);
  let toggling = $state(false);

  $effect(() => {
    invoke<string>("load_notes").then((text) => {
      content = text;
      savedContent = text;
    });
    invoke<string[]>("read_domains").then((domains) => {
      blockedDomains = domains;
    });
    invoke<boolean>("get_blocking_status").then((active) => {
      blockingActive = active;
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
    await invoke("save_domains", { domains });
    blockedDomains = domains;
    if (blockingActive) {
      await invoke("write_blocked", { domains });
    }
  }

  async function toggleBlocking() {
    toggling = true;
    try {
      if (!blockingActive) {
        await invoke("write_blocked", { domains: blockedDomains });
        blockingActive = true;
      } else {
        await invoke("write_blocked", { domains: [] });
        blockingActive = false;
      }
    } catch (e) {
      if (e !== "Cancelled") alert("Error: " + e);
    } finally {
      toggling = false;
    }
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
  <header data-tauri-drag-region>
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

<button
  class="productivity-switch"
  class:active={blockingActive}
  class:toggling
  onclick={toggleBlocking}
  disabled={toggling}
  title={blockingActive ? 'Productivity mode on' : 'Productivity mode off'}
>
  <span class="switch-track">
    <span class="switch-knob"></span>
  </span>
</button>

<style>
  * {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
  }

  :global(html),
  :global(body) {
    overflow: hidden;
    background: #1e1e1e;
    margin: 0;
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
    padding-top: 28px;
    padding-left: 80px;
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

  .productivity-switch {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 100;
    background: none;
    border: none;
    padding: 0;
    cursor: pointer;
    border-radius: 999px;
    box-shadow: none;
  }

  .productivity-switch:hover:not(:disabled) {
    background: none;
  }

  .productivity-switch:disabled {
    opacity: 0.6;
    cursor: wait;
  }

  .switch-track {
    display: flex;
    align-items: center;
    width: 52px;
    height: 28px;
    border-radius: 999px;
    background: #3a3a3a;
    border: 1px solid #555;
    padding: 3px;
    transition: background 0.22s ease, border-color 0.22s ease, box-shadow 0.22s ease;
    position: relative;
  }

  .switch-knob {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #888;
    transition: transform 0.22s ease, background 0.22s ease;
    flex-shrink: 0;
  }

  .productivity-switch.active .switch-track {
    background: #4ec9b0;
    border-color: #4ec9b0;
    box-shadow: 0 0 14px #4ec9b055, 0 2px 8px #0006;
  }

  .productivity-switch.active .switch-knob {
    transform: translateX(24px);
    background: #fff;
  }

  .productivity-switch:not(.active):hover .switch-track {
    background: #4a4a4a;
    border-color: #666;
  }
</style>
