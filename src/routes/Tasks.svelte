<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { Pencil, RotateCcw, X } from "lucide-svelte";

  type Task = { id: number; title: string; total_seconds: number };

  let tasks = $state<Task[]>([]);
  let newTitle = $state('');
  let editingId = $state<number | null>(null);
  let editingTitle = $state('');

  function formatDuration(s: number): string {
    if (s === 0) return '—';
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${sec}s`;
    return `${sec}s`;
  }

  async function load() {
    tasks = await invoke<Task[]>('get_tasks');
  }

  $effect(() => { load(); });

  async function addTask() {
    const t = newTitle.trim();
    if (!t) return;
    const task = await invoke<Task>('create_task', { title: t });
    tasks = [...tasks, task];
    newTitle = '';
  }

  function startEdit(task: Task) {
    editingId = task.id;
    editingTitle = task.title;
  }

  async function saveEdit() {
    if (editingId === null || !editingTitle.trim()) {
      editingId = null;
      return;
    }
    const updated = await invoke<Task>('update_task', { id: editingId, title: editingTitle.trim() });
    tasks = tasks.map(t => t.id === updated.id ? updated : t);
    editingId = null;
  }

  function cancelEdit() {
    editingId = null;
  }

  async function deleteTask(id: number) {
    await invoke('delete_task', { id });
    tasks = tasks.filter(t => t.id !== id);
  }

  async function resetTime(id: number) {
    await invoke('reset_task_time', { id });
    tasks = tasks.map(t => t.id === id ? { ...t, total_seconds: 0 } : t);
  }
</script>

<div class="tasks">
  {#if tasks.length === 0}
    <p class="empty">No tasks yet. Add one below.</p>
  {:else}
    <ul class="task-list">
      {#each tasks as task (task.id)}
        <li class="task-row">
          {#if editingId === task.id}
            <input
              class="edit-input"
              bind:value={editingTitle}
              onkeydown={(e) => {
                if (e.key === 'Enter') saveEdit();
                if (e.key === 'Escape') cancelEdit();
              }}
              onblur={saveEdit}
              autofocus
            />
          {:else}
            <span class="task-title">{task.title}</span>
            <span class="task-time">{formatDuration(task.total_seconds)}</span>
            <div class="task-actions">
              <button class="icon-btn" onclick={() => startEdit(task)} title="Rename"><Pencil size={12} /></button>
              <button class="icon-btn" onclick={() => resetTime(task.id)} title="Reset time"><RotateCcw size={12} /></button>
              <button class="icon-btn danger" onclick={() => deleteTask(task.id)} title="Delete"><X size={12} /></button>
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}

  <div class="new-task">
    <input
      class="new-input"
      bind:value={newTitle}
      placeholder="New task..."
      onkeydown={(e) => e.key === 'Enter' && addTask()}
    />
    <button class="add-btn" onclick={addTask} disabled={!newTitle.trim()}>Add</button>
  </div>
</div>

<style>
  .tasks {
    flex: 1;
    display: flex;
    flex-direction: column;
    padding: 16px;
    gap: 12px;
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
    overflow-y: auto;
  }

  .empty {
    font-size: 12px;
    color: #555;
    text-align: center;
    padding: 32px 0;
  }

  .task-list {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border-radius: 4px;
    background: #252525;
    border: 1px solid #2d2d2d;
  }

  .task-row:hover {
    background: #2a2a2a;
    border-color: #333;
  }

  .task-title {
    flex: 1;
    font-size: 13px;
    color: #d4d4d4;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-time {
    font-size: 11px;
    color: #666;
    min-width: 52px;
    text-align: right;
    flex-shrink: 0;
  }

  .task-actions {
    display: flex;
    gap: 2px;
    flex-shrink: 0;
  }

  .edit-input {
    flex: 1;
    background: #1e1e1e;
    border: 1px solid #4ec9b0;
    border-radius: 3px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 13px;
    padding: 2px 6px;
    outline: none;
  }

  .icon-btn {
    background: transparent;
    color: #555;
    border: none;
    padding: 2px 5px;
    font-size: 13px;
    cursor: pointer;
    border-radius: 3px;
    font-family: inherit;
    line-height: 1;
    min-width: unset;
  }

  .icon-btn:hover {
    color: #999;
    background: #333;
  }

  .icon-btn.danger:hover {
    color: #f48771;
    background: #3a2828;
  }

  .new-task {
    display: flex;
    gap: 8px;
    margin-top: auto;
    padding-top: 8px;
    border-top: 1px solid #2d2d2d;
  }

  .new-input {
    flex: 1;
    background: #2d2d2d;
    border: 1px solid #444;
    border-radius: 4px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 12px;
    padding: 6px 10px;
    outline: none;
  }

  .new-input:focus {
    border-color: #4ec9b0;
  }

  .new-input::placeholder {
    color: #444;
  }

  .add-btn {
    background: #3a3a3a;
    color: #d4d4d4;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 6px 14px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
  }

  .add-btn:hover:not(:disabled) {
    background: #4a4a4a;
  }

  .add-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }
</style>
