<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  let { isDirty = $bindable(false), isActive }: { isDirty?: boolean; isActive: boolean } = $props();

  let notes = $state<string[]>([]);
  let activeNote = $state<string | null>(null);
  let content = $state("");
  let savedContent = $state("");
  let renamingNote = $state<string | null>(null);
  let renameValue = $state("");

  $effect(() => {
    isDirty = content !== savedContent;
  });

  $effect(() => {
    invoke<string[]>("list_notes").then((list) => {
      notes = list;
      if (list.length > 0) selectNote(list[0]);
    });
  });

  async function selectNote(name: string) {
    if (activeNote !== null && activeNote !== name && content !== savedContent) {
      await invoke("save_note", { name: activeNote, content });
    }
    activeNote = name;
    const text = await invoke<string>("load_note", { name });
    content = text;
    savedContent = text;
  }

  async function save() {
    if (!activeNote) return;
    try {
      await invoke("save_note", { name: activeNote, content });
      savedContent = content;
    } catch (e) {
      alert("Could not save: " + e);
    }
  }

  async function createNote() {
    try {
      const name = await invoke<string>("create_note", { name: "" });
      notes = await invoke<string[]>("list_notes");
      await selectNote(name);
    } catch (e) {
      alert("Could not create note: " + e);
    }
  }

  async function deleteNote(name: string, e: MouseEvent) {
    e.stopPropagation();
    if (notes.length <= 1) return;
    try {
      await invoke("delete_note", { name });
      const remaining = notes.filter(n => n !== name);
      notes = remaining;
      if (activeNote === name) await selectNote(remaining[0]);
    } catch (e) {
      alert("Could not delete: " + e);
    }
  }

  function startRename(name: string) {
    renamingNote = name;
    renameValue = name;
  }

  async function confirmRename() {
    if (!renamingNote) return;
    const trimmed = renameValue.trim();
    if (!trimmed || trimmed === renamingNote) {
      renamingNote = null;
      return;
    }
    try {
      await invoke("rename_note", { oldName: renamingNote, newName: trimmed });
      const idx = notes.indexOf(renamingNote);
      const updated = [...notes];
      updated[idx] = trimmed;
      notes = updated;
      if (activeNote === renamingNote) activeNote = trimmed;
    } catch (e) {
      alert("Could not rename: " + e);
    }
    renamingNote = null;
  }

  function focusSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!isActive) return;
    if ((e.metaKey || e.ctrlKey) && e.key === "s") {
      e.preventDefault();
      save();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="editor-layout">
  <aside class="sidebar">
    <div class="sidebar-header">
      <button class="new-btn" onclick={createNote} title="New note">+</button>
    </div>
    <ul class="note-list">
      {#each notes as name (name)}
        <li class="note-item" class:active={activeNote === name}>
          {#if renamingNote === name}
            <input
              class="rename-input"
              bind:value={renameValue}
              use:focusSelect
              onblur={confirmRename}
              onkeydown={(e) => {
                if (e.key === 'Enter') { e.preventDefault(); confirmRename(); }
                if (e.key === 'Escape') { e.preventDefault(); renamingNote = null; }
              }}
            />
          {:else}
            <button
              class="note-btn"
              onclick={() => selectNote(name)}
              ondblclick={() => startRename(name)}
            >
              <span class="note-name">{name}</span>
              {#if activeNote === name && isDirty}
                <span class="dirty-dot">•</span>
              {/if}
            </button>
            {#if notes.length > 1}
              <button class="del-btn" onclick={(e) => deleteNote(name, e)} title="Delete">×</button>
            {/if}
          {/if}
        </li>
      {/each}
    </ul>
  </aside>

  <textarea
    bind:value={content}
    spellcheck="false"
    autocomplete="off"
    placeholder="Start typing..."
    onkeydown={(e) => {
      if (e.key === 'Tab') {
        e.preventDefault();
        const el = e.currentTarget;
        const start = el.selectionStart;
        const end = el.selectionEnd;
        content = content.slice(0, start) + '    ' + content.slice(end);
        requestAnimationFrame(() => {
          el.selectionStart = el.selectionEnd = start + 4;
        });
      }
    }}
  ></textarea>
</div>

<style>
  .editor-layout {
    flex: 1;
    display: flex;
    min-height: 0;
  }

  .sidebar {
    width: 160px;
    flex-shrink: 0;
    background: #252526;
    border-right: 1px solid #3d3d3d;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 4px 6px;
    border-bottom: 1px solid #3d3d3d;
    flex-shrink: 0;
  }

  .new-btn {
    background: transparent;
    border: none;
    color: #888;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    padding: 1px 5px;
    border-radius: 3px;
    font-family: inherit;
  }

  .new-btn:hover {
    color: #d4d4d4;
    background: #3a3a3a;
  }

  .note-list {
    list-style: none;
    margin: 0;
    padding: 4px 0;
    overflow-y: auto;
    flex: 1;
  }

  .note-item {
    display: flex;
    align-items: center;
    position: relative;
  }

  .note-item:hover .del-btn {
    opacity: 1;
  }

  .note-btn {
    flex: 1;
    background: transparent;
    border: none;
    color: #9d9d9d;
    text-align: left;
    padding: 5px 10px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: 0;
    min-width: 0;
  }

  .note-btn:hover {
    background: #2a2d2e;
    color: #d4d4d4;
  }

  .note-item.active .note-btn {
    color: #d4d4d4;
    background: #37373d;
  }

  .note-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .dirty-dot {
    color: #569cd6;
    flex-shrink: 0;
  }

  .del-btn {
    background: transparent;
    border: none;
    color: #666;
    font-size: 14px;
    cursor: pointer;
    padding: 4px 6px;
    opacity: 0;
    transition: opacity 0.1s;
    font-family: inherit;
    flex-shrink: 0;
    border-radius: 0;
  }

  .del-btn:hover {
    color: #f48771;
    background: transparent;
  }

  .rename-input {
    flex: 1;
    background: #3a3a3a;
    border: 1px solid #569cd6;
    border-radius: 2px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 12px;
    padding: 3px 6px;
    margin: 2px 4px;
    outline: none;
    min-width: 0;
    width: calc(100% - 8px);
  }

  textarea {
    flex: 1;
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
