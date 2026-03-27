<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Plus, X } from "lucide-svelte";

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
    if ((e.metaKey || e.ctrlKey) && e.key === "n") {
      e.preventDefault();
      createNote();
    }
    if (e.ctrlKey && !e.metaKey) {
      const num = parseInt(e.key);
      if (num >= 1 && num <= notes.length) {
        e.preventDefault();
        selectNote(notes[num - 1]);
      }
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="editor-layout">
  <aside class="sidebar">
    <div class="sidebar-header">
      <button class="new-btn" onclick={createNote} title="New note"><Plus size={16} /></button>
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
              <button class="del-btn" onclick={(e) => deleteNote(name, e)} title="Delete"><X size={12} /></button>
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
    background: #131313;
  }

  .sidebar {
    width: 200px;
    flex-shrink: 0;
    background: rgba(32, 31, 31, 0.8);
    backdrop-filter: blur(12px);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .sidebar-header {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    padding: 8px 10px;
    flex-shrink: 0;
  }

  .new-btn {
    background: transparent;
    border: none;
    color: #86948f;
    font-size: 18px;
    line-height: 1;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 4px;
    font-family: "Inter", sans-serif;
  }

  .new-btn:hover {
    color: #6de5cb;
    background: rgba(109, 229, 203, 0.08);
  }

  .note-list {
    list-style: none;
    margin: 0;
    padding: 4px 8px;
    overflow-y: auto;
    flex: 1;
  }

  .note-item {
    display: flex;
    align-items: center;
    position: relative;
    border-radius: 0;
    margin-bottom: 2px;
  }

  .note-item:hover .del-btn {
    opacity: 1;
  }

  .note-item.active {
    background: rgba(109, 229, 203, 0.06);
    border-left: 2px solid #6de5cb;
  }

  .note-item:not(.active) {
    border-left: 2px solid transparent;
  }

  .note-btn {
    flex: 1;
    background: transparent;
    border: none;
    color: #86948f;
    text-align: left;
    padding: 8px 10px;
    font-size: 12px;
    cursor: pointer;
    font-family: "Inter", sans-serif;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: 4px;
    border-radius: 0;
    min-width: 0;
  }

  .note-btn:hover {
    color: #e5e2e1;
  }

  .note-item:not(.active):hover {
    background: rgba(255, 255, 255, 0.03);
  }

  .note-item.active .note-btn {
    color: #e5e2e1;
  }

  .note-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: "Space Grotesk", sans-serif;
    font-weight: 500;
    font-size: 13px;
  }

  .dirty-dot {
    color: #6de5cb;
    flex-shrink: 0;
  }

  .del-btn {
    background: transparent;
    border: none;
    color: #86948f;
    font-size: 14px;
    cursor: pointer;
    padding: 4px 6px;
    opacity: 0;
    transition: opacity 0.15s;
    font-family: "Inter", sans-serif;
    flex-shrink: 0;
    border-radius: 4px;
  }

  .del-btn:hover {
    color: #ffb4ab;
    background: rgba(255, 180, 171, 0.08);
  }

  .rename-input {
    flex: 1;
    background: #2a2a2a;
    border: 1px solid #6de5cb;
    border-radius: 4px;
    color: #e5e2e1;
    font-family: "Space Grotesk", sans-serif;
    font-size: 13px;
    padding: 6px 8px;
    margin: 2px 4px;
    outline: none;
    min-width: 0;
    width: calc(100% - 8px);
  }

  textarea {
    flex: 1;
    padding: 24px 32px;
    background: #131313;
    color: #e5e2e1;
    border: none;
    outline: none;
    resize: none;
    font-family: "JetBrains Mono", "Menlo", "Monaco", "Courier New", monospace;
    font-size: 14px;
    line-height: 1.7;
    tab-size: 2;
  }

  textarea::placeholder {
    color: #86948f;
  }
</style>
