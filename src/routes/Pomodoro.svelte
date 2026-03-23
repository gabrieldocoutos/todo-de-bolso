<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { Circle, CircleDot, Pencil, RotateCcw, X } from "lucide-svelte";

  let { isActive }: { isActive: boolean } = $props();

  const ROUND_SIZE = 4;

  let mode = $state<"work" | "break">("work");
  let remaining = $state(25 * 60);
  let running = $state(false);
  let completedSessions = $state(0);
  let activeTaskId = $state<number | null>(null);
  let activeTaskElapsed = $state(0);
  let prevMode = $state<string | null>(null);

  function playNotification() {
    const audio = new Audio("/notification.wav");
    audio.volume = 0.5;
    audio.play().catch(() => {});
  }

  type Task = { id: number; title: string; total_seconds: number };
  let tasks = $state<Task[]>([]);
  let newTitle = $state("");
  let editingId = $state<number | null>(null);
  let editingTitle = $state("");

  function focusOnMount(node: HTMLElement) {
    node.focus();
  }

  const minutes = $derived(String(Math.floor(remaining / 60)).padStart(2, "0"));
  const seconds = $derived(String(remaining % 60).padStart(2, "0"));
  const doneInRound = $derived(completedSessions % ROUND_SIZE);

  type Payload = {
    mode: string;
    remaining: number;
    running: boolean;
    completed_sessions: number;
    active_task_id: number | null;
    active_task_elapsed: number;
  };

  function applyState(s: Payload) {
    if (prevMode !== null && s.mode !== prevMode) {
      playNotification();
    }
    prevMode = s.mode;
    mode = s.mode as "work" | "break";
    remaining = s.remaining;
    running = s.running;
    completedSessions = s.completed_sessions;
    activeTaskId = s.active_task_id;
    activeTaskElapsed = s.active_task_elapsed;
  }

  invoke<Payload>("pomodoro_get_state").then(applyState);
  invoke<Task[]>("get_tasks").then((t) => {
    tasks = t;
  });

  const unlisten = listen<Payload>("pomodoro-tick", ({ payload }) =>
    applyState(payload),
  );
  onDestroy(async () => {
    (await unlisten)();
  });

  async function toggle() {
    await invoke("pomodoro_toggle");
    tasks = await invoke<Task[]>("get_tasks");
  }
  async function reset() {
    await invoke("pomodoro_reset");
    tasks = await invoke<Task[]>("get_tasks");
  }
  function skipBreak() {
    invoke("pomodoro_skip_break");
  }

  async function selectTask(id: number) {
    const next = activeTaskId === id ? null : id;
    await invoke("set_active_task", { id: next });
    tasks = await invoke<Task[]>("get_tasks");
  }

  async function addTask() {
    const t = newTitle.trim();
    if (!t) return;
    const task = await invoke<Task>("create_task", { title: t });
    tasks = [...tasks, task];
    newTitle = "";
  }

  function startEdit(task: Task, e: MouseEvent) {
    e.stopPropagation();
    editingId = task.id;
    editingTitle = task.title;
  }

  async function saveEdit() {
    if (editingId === null || !editingTitle.trim()) {
      editingId = null;
      return;
    }
    const updated = await invoke<Task>("update_task", {
      id: editingId,
      title: editingTitle.trim(),
    });
    tasks = tasks.map((t) => (t.id === updated.id ? updated : t));
    editingId = null;
  }

  async function deleteTask(id: number, e: MouseEvent) {
    e.stopPropagation();
    await invoke("delete_task", { id });
    tasks = tasks.filter((t) => t.id !== id);
  }

  async function resetTime(id: number, e: MouseEvent) {
    e.stopPropagation();
    await invoke("reset_task_time", { id });
    tasks = tasks.map((t) => (t.id === id ? { ...t, total_seconds: 0 } : t));
  }

  function fmtDuration(s: number): string {
    if (s === 0) return "—";
    const h = Math.floor(s / 3600);
    const m = Math.floor((s % 3600) / 60);
    const sec = s % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${sec}s`;
    return `${sec}s`;
  }

  function fmtSession(s: number): string {
    const m = Math.floor(s / 60);
    const sec = s % 60;
    return `${m}:${String(sec).padStart(2, "0")}`;
  }

  function onKeyDown(e: KeyboardEvent) {
    if (!isActive) return;
    const target = e.target as HTMLElement;
    if (target.tagName === "INPUT" || target.tagName === "TEXTAREA") return;
    if (e.key === " ") {
      e.preventDefault();
      toggle();
    }
  }
</script>

<svelte:window onkeydown={onKeyDown} />

<div class="pomodoro">
  <!-- Timer section -->
  <div class="timer-section">
    <div class="mode">{mode === "work" ? "WORK" : "BREAK"}</div>
    <div class="timer">{minutes}:{seconds}</div>
    <div class="controls">
      <button onclick={toggle}>{running ? "Pause" : "Start"}</button>
      <button onclick={reset}>Reset</button>
      {#if mode === "break"}
        <button onclick={skipBreak}>Skip</button>
      {/if}
    </div>
    <div class="sessions">
      <span>Round {Math.floor(completedSessions / ROUND_SIZE) + 1}</span>
      <span class="dots">
        {#each Array(ROUND_SIZE) as _, i}
          <span class:filled={i < doneInRound}>◦</span>
        {/each}
      </span>
      <span>{completedSessions} done</span>
    </div>
  </div>

  <!-- Task section -->
  <div class="task-section">
    <div class="task-list">
      {#each tasks as task (task.id)}
        {@const isActive = task.id === activeTaskId}
        <div class="task-row" class:active={isActive}>
          <button
            class="select-btn"
            class:selected={isActive}
            onclick={() => selectTask(task.id)}
            title={isActive ? "Deselect task" : "Select task"}
          >
            {#if isActive}<CircleDot size={14} />{:else}<Circle size={14} />{/if}
          </button>
          {#if editingId === task.id}
            <input
              class="edit-input"
              bind:value={editingTitle}
              onkeydown={(e) => {
                if (e.key === "Enter") saveEdit();
                if (e.key === "Escape") editingId = null;
              }}
              onblur={saveEdit}
              use:focusOnMount
            />
          {:else}
            <span class="task-title">{task.title}</span>
            <span class="task-time">
              {#if isActive && (running || activeTaskElapsed > 0)}
                <span class="session-time">{fmtSession(activeTaskElapsed)}</span
                >
              {:else}
                {fmtDuration(task.total_seconds)}
              {/if}
            </span>
            <div class="task-actions">
              <button
                class="icon-btn"
                onclick={(e) => startEdit(task, e)}
                title="Rename"><Pencil size={12} /></button
              >
              <button
                class="icon-btn"
                onclick={(e) => resetTime(task.id, e)}
                title="Reset time"><RotateCcw size={12} /></button
              >
              <button
                class="icon-btn danger"
                onclick={(e) => deleteTask(task.id, e)}
                title="Delete"><X size={12} /></button
              >
            </div>
          {/if}
        </div>
      {/each}

      {#if tasks.length === 0}
        <p class="empty">No tasks yet.</p>
      {/if}
    </div>

    <div class="new-task">
      <input
        class="new-input"
        bind:value={newTitle}
        placeholder="New task..."
        onkeydown={(e) => e.key === "Enter" && addTask()}
      />
      <button class="add-btn" onclick={addTask} disabled={!newTitle.trim()}
        >Add</button
      >
    </div>
  </div>
</div>

<style>
  .pomodoro {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
    overflow: hidden;
  }

  /* ── Timer ── */
  .timer-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 20px;
    padding: 28px 0 20px;
    flex-shrink: 0;
    border-bottom: 1px solid #2a2a2a;
  }

  .mode {
    font-size: 11px;
    letter-spacing: 0.25em;
    color: #4ec9b0;
  }

  .timer {
    font-size: 72px;
    font-weight: 200;
    letter-spacing: 0.04em;
    color: #e8e8e8;
    line-height: 1;
  }

  .controls {
    display: flex;
    gap: 10px;
  }

  button {
    background: #3a3a3a;
    color: #d4d4d4;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 6px 20px;
    font-size: 13px;
    cursor: pointer;
    font-family: inherit;
    min-width: 80px;
  }

  button:hover:not(:disabled) {
    background: #4a4a4a;
  }

  .sessions {
    font-size: 11px;
    color: #555;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .dots {
    display: flex;
    gap: 4px;
    font-size: 16px;
  }

  .dots span {
    color: #383838;
    transition: color 0.3s;
  }
  .dots span.filled {
    color: #4ec9b0;
  }

  /* ── Tasks ── */
  .task-section {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    padding: 10px 16px 12px;
    gap: 8px;
  }

  .task-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .empty {
    font-size: 11px;
    color: #444;
    text-align: center;
    padding: 20px 0;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 8px 4px 4px;
    border-radius: 4px;
    border: 1px solid transparent;
  }

  .task-row:hover {
    background: #252525;
    border-color: #2d2d2d;
  }

  .task-row.active {
    background: #1e2e2a;
    border-color: #2a5048;
  }

  .task-row.active .task-title {
    color: #6fdfc5;
  }

  .select-btn {
    background: transparent;
    border: none;
    color: #444;
    font-size: 14px;
    padding: 2px 4px;
    cursor: pointer;
    line-height: 1;
    min-width: unset;
    flex-shrink: 0;
    border-radius: 3px;
  }

  .select-btn:hover {
    background: #2a2a2a;
    color: #888;
  }

  .select-btn.selected {
    color: #4ec9b0;
  }

  .select-btn.selected:hover {
    background: #1a2e2a;
    color: #4ec9b0;
  }

  .task-title {
    flex: 1;
    font-size: 12px;
    color: #c0c0c0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-time {
    font-size: 11px;
    color: #555;
    min-width: 52px;
    text-align: right;
    flex-shrink: 0;
  }

  .session-time {
    color: #4ec9b0;
  }

  .task-actions {
    display: flex;
    gap: 1px;
    flex-shrink: 0;
    opacity: 0;
  }

  .task-row:hover .task-actions,
  .task-row.active .task-actions {
    opacity: 1;
  }

  .icon-btn {
    background: transparent;
    color: #555;
    border: none;
    padding: 2px 4px;
    font-size: 12px;
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

  .edit-input {
    flex: 1;
    background: #1e1e1e;
    border: 1px solid #4ec9b0;
    border-radius: 3px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 12px;
    padding: 2px 6px;
    outline: none;
  }

  .new-task {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
    padding-top: 6px;
    border-top: 1px solid #252525;
  }

  .new-input {
    flex: 1;
    background: #252525;
    border: 1px solid #333;
    border-radius: 4px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 12px;
    padding: 5px 10px;
    outline: none;
  }

  .new-input:focus {
    border-color: #4ec9b0;
  }
  .new-input::placeholder {
    color: #3a3a3a;
  }

  .add-btn {
    background: #3a3a3a;
    color: #d4d4d4;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 5px 14px;
    font-size: 12px;
    cursor: pointer;
    font-family: inherit;
    min-width: unset;
  }

  .add-btn:disabled {
    opacity: 0.3;
    cursor: default;
  }
</style>
