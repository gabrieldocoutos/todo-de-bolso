<script lang="ts">
  import { SHORTCUTS } from '$lib/shortcuts';
  import { X } from "lucide-svelte";

  let { onclose }: { onclose: () => void } = $props();
</script>

<div class="modal-backdrop" role="presentation" onclick={onclose}>
  <div class="modal" role="dialog" tabindex="0" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <p class="modal-title">Keyboard Shortcuts</p>
      <button class="close-btn" onclick={onclose}><X size={16} /></button>
    </div>

    <div class="shortcut-grid">
      {#each SHORTCUTS as group}
        <div class="group">
          <div class="group-header">
            <span class="group-label">{group.label}</span>
            <span class="group-line"></span>
          </div>
          {#each group.shortcuts as shortcut}
            <div class="shortcut-row">
              <span class="desc">{shortcut.description}</span>
              <div class="keys">
                {#each shortcut.keys as key}
                  <kbd class="key">{key}</kbd>
                {/each}
              </div>
            </div>
          {/each}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: #000a;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: #1c1b1b;
    border: 1px solid #3d494633;
    border-radius: 8px;
    width: 480px;
    max-height: 80vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    box-shadow: 0 12px 48px #0006;
    font-family: "Inter", sans-serif;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 28px 28px 20px;
    border-bottom: 1px solid #3d49460d;
  }

  .modal-title {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 22px;
    color: #e5e2e1;
    font-weight: 700;
    letter-spacing: -0.02em;
  }

  .close-btn {
    width: 36px;
    height: 36px;
    background: #2a2a2a;
    border: none;
    color: #bccac4;
    cursor: pointer;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition: color 0.15s, background 0.15s;
  }

  .close-btn:hover {
    color: #6de5cb;
    background: #353534;
  }

  .shortcut-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 32px;
    padding: 28px;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .group-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .group-label {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 11px;
    font-weight: 700;
    color: #4ec9b0;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .group-line {
    flex: 1;
    height: 1px;
    background: #3d494633;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 4px 0;
    gap: 12px;
    transition: opacity 0.15s;
  }

  .shortcut-row:hover .desc {
    color: #e5e2e1;
  }

  .keys {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
  }

  .key {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 11px;
    color: #6de5cb;
    background: #353534;
    border: 1px solid #3d494633;
    border-radius: 4px;
    padding: 4px 8px;
    min-width: 28px;
    text-align: center;
    line-height: 1;
  }

  .desc {
    font-size: 13px;
    color: #bccac4;
    transition: color 0.15s;
  }
</style>
