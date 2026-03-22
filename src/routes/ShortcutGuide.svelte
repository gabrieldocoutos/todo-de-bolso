<script lang="ts">
  import { SHORTCUTS } from '$lib/shortcuts';
  import { X } from "lucide-svelte";

  let { onclose }: { onclose: () => void } = $props();
</script>

<div class="modal-backdrop" role="presentation" onclick={onclose}>
  <div class="modal" role="dialog" tabindex="0" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
    <div class="modal-header">
      <p class="modal-title">Keyboard shortcuts</p>
      <button class="close-btn" onclick={onclose}><X size={14} /></button>
    </div>

    {#each SHORTCUTS as group}
      <div class="group">
        <p class="group-label">{group.label}</p>
        {#each group.shortcuts as shortcut}
          <div class="shortcut-row">
            <div class="keys">
              {#each shortcut.keys as key}
                <kbd class="key">{key}</kbd>
              {/each}
            </div>
            <span class="desc">{shortcut.description}</span>
          </div>
        {/each}
      </div>
    {/each}
  </div>
</div>

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    background: #0008;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .modal {
    background: #2d2d2d;
    border: 1px solid #3d3d3d;
    border-radius: 8px;
    padding: 16px 20px 20px;
    width: 340px;
    max-height: 80vh;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: 0 8px 32px #0008;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .modal-title {
    font-size: 13px;
    color: #d4d4d4;
    font-weight: 600;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: #666;
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 3px;
    font-family: inherit;
    line-height: 1;
  }

  .close-btn:hover {
    color: #d4d4d4;
    background: #3a3a3a;
  }

  .group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .group-label {
    font-size: 10px;
    color: #4ec9b0;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    margin-bottom: 2px;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 0;
  }

  .keys {
    display: flex;
    gap: 4px;
  }

  .key {
    font-family: inherit;
    font-size: 11px;
    color: #ccc;
    background: #1e1e1e;
    border: 1px solid #444;
    border-radius: 4px;
    padding: 3px 8px;
    min-width: 24px;
    text-align: center;
    line-height: 1;
    box-shadow: 0 1px 0 #000;
  }

  .desc {
    font-size: 11px;
    color: #999;
    flex: 1;
    text-align: right;
  }
</style>
