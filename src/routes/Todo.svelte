<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { Circle, Check, ChevronDown, ChevronRight, X, RefreshCw, Square, CheckSquare } from "lucide-svelte";
  import PageHeader from "$lib/PageHeader.svelte";

  type Reminder = { id: number; title: string };

  let reminders = $state<Reminder[]>([]);
  let completed = $state<Reminder[]>([]);
  let completing = $state(new Set<number>());
  let deleting = $state(new Set<number>());
  let loading = $state(false);
  let error = $state('');
  let newTitle = $state('');
  let creating = $state(false);
  let showCompleted = $state(true);

  async function load() {
    loading = true;
    error = '';
    try {
      const [active, done] = await Promise.all([
        invoke<Reminder[]>("get_reminders"),
        invoke<Reminder[]>("get_completed_reminders"),
      ]);
      reminders = active;
      completed = done;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function create() {
    const title = newTitle.trim();
    if (!title) return;
    creating = true;
    try {
      await invoke("create_reminder", { title });
      newTitle = '';
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      creating = false;
    }
  }

  async function complete(id: number) {
    completing = new Set([...completing, id]);
    const item = reminders.find(r => r.id === id);
    try {
      await invoke("complete_reminder", { id });
      reminders = reminders.filter(r => r.id !== id);
      if (item) completed = [item, ...completed];
    } catch (e) {
      error = String(e);
    } finally {
      completing = new Set([...completing].filter(x => x !== id));
    }
  }

  async function remove(id: number) {
    deleting = new Set([...deleting, id]);
    try {
      await invoke("delete_reminder", { id });
      completed = completed.filter(r => r.id !== id);
    } catch (e) {
      error = String(e);
    } finally {
      deleting = new Set([...deleting].filter(x => x !== id));
    }
  }

  async function clearAll() {
    try {
      await invoke("clear_completed_reminders");
      completed = [];
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => { load(); });
</script>

<div class="todo">
  <PageHeader title="Todo">
    {#snippet subtitle()}
      Capture what needs to be done so nothing slips through.
    {/snippet}
  </PageHeader>

  <form class="add-form" onsubmit={(e) => { e.preventDefault(); create(); }}>
    <div class="input-wrap">
      <span class="input-icon"><RefreshCw size={14} /></span>
      <input
        class="add-input"
        type="text"
        placeholder="Add new todo..."
        bind:value={newTitle}
        disabled={creating}
      />
    </div>
    <button type="submit" class="add-btn" disabled={creating || !newTitle.trim()}>ADD</button>
  </form>

  <div class="section-label">ACTIVE TASKS</div>

  {#if error}
    <p class="error">{error}</p>
  {:else if loading}
    <p class="hint">Loading…</p>
  {:else}
    <div class="lists">
      {#if reminders.length === 0}
        <p class="hint">All done! Add a task above to get started.</p>
      {:else}
        <ul>
          {#each reminders as reminder (reminder.id)}
            <li
              class:completing={completing.has(reminder.id)}
              onclick={() => complete(reminder.id)}
              title="Click to mark as done"
            >
              <span class="checkbox">
                {#if completing.has(reminder.id)}
                  <CheckSquare size={16} />
                {:else}
                  <Square size={16} />
                {/if}
              </span>
              <span class="task-title">{reminder.title}</span>
            </li>
          {/each}
        </ul>
      {/if}

      {#if completed.length > 0}
        <div class="completed-bar">
          <button class="completed-header" onclick={() => showCompleted = !showCompleted}>
            <span class="chevron">
              {#if showCompleted}<ChevronDown size={12} />{:else}<ChevronRight size={12} />{/if}
            </span>
            COMPLETED
            <span class="count-chip">{completed.length}</span>
          </button>
          <button class="clear-all-btn" onclick={clearAll}>Clear all</button>
        </div>

        {#if showCompleted}
          <ul class="completed-list">
            {#each completed as item (item.id)}
              <li class="completed-item" class:removing={deleting.has(item.id)}>
                <span class="check-icon"><CheckSquare size={16} /></span>
                <span class="completed-title">{item.title}</span>
                <button
                  class="delete-btn"
                  onclick={() => remove(item.id)}
                  disabled={deleting.has(item.id)}
                  title="Delete permanently"
                ><X size={11} /></button>
              </li>
            {/each}
          </ul>
        {/if}
      {/if}
    </div>
  {/if}

  <div class="watermark">SYSTEM.<br/>IDLE.</div>
</div>

<style>
  .todo {
    display: flex;
    flex-direction: column;
    height: 100%;
    padding: 28px 32px;
    overflow: hidden;
    background: #131313;
    color: #e5e2e1;
    font-family: "Inter", sans-serif;
    position: relative;
  }


  /* ---- Add form ---- */
  .add-form {
    display: flex;
    gap: 10px;
    margin-bottom: 28px;
    flex-shrink: 0;
    align-items: center;
  }

  .input-wrap {
    flex: 1;
    position: relative;
    display: flex;
    align-items: center;
  }

  .input-icon {
    position: absolute;
    left: 12px;
    color: #6de5cb;
    display: flex;
    align-items: center;
    pointer-events: none;
  }

  .add-input {
    width: 100%;
    background: #0e0e0e;
    border: 1px solid rgba(61, 73, 70, 0.15);
    border-radius: 9999px;
    color: #e5e2e1;
    font-family: "Inter", sans-serif;
    font-size: 0.8125rem;
    padding: 10px 16px 10px 38px;
    outline: none;
    transition: border-color 0.15s, box-shadow 0.15s;
  }

  .add-input:focus {
    border-color: #6de5cb;
    box-shadow: 0 0 0 4px rgba(109, 229, 203, 0.12);
  }

  .add-input::placeholder {
    color: #86948f;
  }

  .add-btn {
    background: linear-gradient(135deg, #6de5cb, #4ec9b0);
    color: #00382e;
    border: none;
    border-radius: 9999px;
    padding: 10px 20px;
    font-size: 0.75rem;
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-weight: 700;
    cursor: pointer;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    transition: opacity 0.15s;
    flex-shrink: 0;
  }

  .add-btn:hover:not(:disabled) {
    opacity: 0.9;
  }

  .add-btn:disabled {
    opacity: 0.35;
    cursor: default;
  }

  /* ---- Section label ---- */
  .section-label {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.6875rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    color: #6de5cb;
    margin-bottom: 12px;
    flex-shrink: 0;
  }

  /* ---- Lists ---- */
  .lists {
    overflow-y: auto;
    flex: 1;
  }

  .lists::-webkit-scrollbar {
    width: 4px;
  }

  .lists::-webkit-scrollbar-track {
    background: #131313;
  }

  .lists::-webkit-scrollbar-thumb {
    background: #353535;
    border-radius: 10px;
  }

  ul {
    list-style: none;
    padding: 0;
    margin: 0;
  }

  li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    font-size: 0.8125rem;
    color: #e5e2e1;
    border-radius: 0.375rem;
    cursor: pointer;
    transition: background 0.15s;
  }

  li:hover {
    background: #1b1b1c;
  }

  li.completing {
    opacity: 0.35;
    text-decoration: line-through;
    pointer-events: none;
  }

  .checkbox {
    color: rgba(61, 73, 70, 0.5);
    flex-shrink: 0;
    display: flex;
    align-items: center;
    transition: color 0.15s;
  }

  li:hover .checkbox {
    color: #6de5cb;
  }

  .task-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* ---- Completed section ---- */
  .completed-bar {
    display: flex;
    align-items: center;
    margin-top: 16px;
    padding-top: 16px;
  }

  .completed-header {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1;
    background: none;
    border: none;
    color: #86948f;
    font-size: 0.6875rem;
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.12em;
    padding: 0;
    cursor: pointer;
    transition: color 0.15s;
  }

  .completed-header:hover {
    color: #bccac4;
    background: none;
  }

  .count-chip {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 0.625rem;
    background: #353535;
    color: #80f7dc;
    padding: 2px 8px;
    border-radius: 9999px;
    font-weight: 500;
  }

  .clear-all-btn {
    background: none;
    border: none;
    color: #86948f;
    font-size: 0.625rem;
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    padding: 4px 10px;
    cursor: pointer;
    flex-shrink: 0;
    transition: color 0.15s;
  }

  .clear-all-btn:hover {
    color: #ffb4ab;
    background: none;
  }

  .chevron {
    display: flex;
    align-items: center;
    color: #86948f;
  }

  .completed-item {
    cursor: default;
    color: #86948f;
    text-decoration: line-through;
  }

  .completed-item:hover {
    background: #1b1b1c;
  }

  .check-icon {
    color: rgba(61, 73, 70, 0.4);
    flex-shrink: 0;
    display: flex;
    align-items: center;
  }

  .completed-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .delete-btn {
    background: none;
    border: none;
    color: #86948f;
    padding: 4px;
    cursor: pointer;
    border-radius: 0.25rem;
    display: flex;
    align-items: center;
    justify-content: center;
    opacity: 0;
    transition: opacity 0.15s, color 0.15s, background 0.15s;
    flex-shrink: 0;
  }

  .completed-item:hover .delete-btn {
    opacity: 1;
  }

  .delete-btn:hover {
    color: #ffb4ab;
    background: rgba(147, 0, 10, 0.15);
  }

  .delete-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }

  .completed-item.removing {
    opacity: 0.3;
    pointer-events: none;
  }

  /* ---- Watermark ---- */
  .watermark {
    position: absolute;
    bottom: 32px;
    right: 32px;
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 4rem;
    font-weight: 700;
    color: rgba(255, 255, 255, 0.03);
    line-height: 1;
    text-align: right;
    pointer-events: none;
    user-select: none;
    letter-spacing: -0.02em;
  }

  /* ---- States ---- */
  .hint {
    font-size: 0.8125rem;
    color: #86948f;
    text-align: center;
    margin-top: 40px;
  }

  .error {
    font-size: 0.75rem;
    color: #ffb4ab;
    margin-top: 12px;
    white-space: pre-wrap;
  }
</style>
