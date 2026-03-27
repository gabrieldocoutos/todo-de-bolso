<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy } from "svelte";
  import { Pencil, RotateCcw, X, Plus } from "lucide-svelte";

  let { isActive }: { isActive: boolean } = $props();

  const ROUND_SIZE = 4;

  let mode = $state<"work" | "break">("work");
  let remaining = $state(25 * 60);
  let running = $state(false);
  let completedSessions = $state(0);
  let activeTaskId = $state<number | null>(null);
  let activeTaskElapsed = $state(0);
  let prevMode = $state<string | null>(null);
  let skipNotification = $state(false);
  let workSecs = $state(25 * 60);
  let breakSecs = $state(5 * 60);

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

  const presets = [
    { label: "STANDARD", workSecs: 25 * 60, breakSecs: 5 * 60 },
    { label: "DEEP FOCUS", workSecs: 50 * 60, breakSecs: 10 * 60 },
    { label: "LONG RUN", workSecs: 90 * 60, breakSecs: 20 * 60 },
    { label: "MICRO", workSecs: 15 * 60, breakSecs: 3 * 60 },
  ];

  const activePresetIndex = $derived(
    presets.findIndex(
      (p) => p.workSecs === workSecs && p.breakSecs === breakSecs,
    ),
  );

  type Payload = {
    mode: string;
    remaining: number;
    running: boolean;
    completed_sessions: number;
    active_task_id: number | null;
    active_task_elapsed: number;
    work_secs: number;
    break_secs: number;
  };

  function applyState(s: Payload) {
    if (prevMode !== null && s.mode !== prevMode && !skipNotification) {
      playNotification();
    }
    skipNotification = false;
    prevMode = s.mode;
    mode = s.mode as "work" | "break";
    remaining = s.remaining;
    running = s.running;
    completedSessions = s.completed_sessions;
    activeTaskId = s.active_task_id;
    activeTaskElapsed = s.active_task_elapsed;
    workSecs = s.work_secs;
    breakSecs = s.break_secs;
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
    skipNotification = true;
    invoke("pomodoro_skip_break");
  }

  async function setPreset(w: number, b: number) {
    const s = await invoke<Payload>("set_pomodoro_config", {
      workSecs: w,
      breakSecs: b,
    });
    applyState(s);
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
    if (s === 0) return "\u2014";
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

  function fmtPreset(secs: number): number {
    return Math.round(secs / 60);
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
  <div class="scroll-area">
    <!-- Hero Timer Section -->
    <section class="hero">
      <div class="status-pill" class:break={mode === "break"}>
        <span class="status-icon">{mode === "work" ? "\u26A1" : "\u2615"}</span>
        <span class="status-text"
          >{running
            ? mode === "work"
              ? "Deep Focus Active"
              : "Break Time"
            : mode === "work"
              ? "Ready to Focus"
              : "Break"}</span
        >
      </div>

      <div class="timer-wrapper">
        <div class="timer-glow"></div>
        <h1 class="timer-display">{minutes}:{seconds}</h1>
        <div class="round-info">
          <span class="round-label">Round</span>
          <div class="round-dots">
            {#each Array(ROUND_SIZE) as _, i}
              <div class="dot" class:filled={i < doneInRound}></div>
            {/each}
          </div>
        </div>
      </div>

      <div class="hero-controls">
        <button class="btn-primary" onclick={toggle}>
          {running ? "Pause" : "Start session"}
        </button>
        <button class="btn-icon" onclick={reset} title="Reset">
          <RotateCcw size={18} />
        </button>
        {#if mode === "break"}
          <button class="btn-secondary" onclick={skipBreak}>Skip</button>
        {/if}
      </div>
    </section>

    <!-- Content Grid -->
    <div class="content-grid">
      <!-- Tasks Card -->
      <div class="card tasks-card">
        <div class="card-header">
          <div>
            <h3 class="card-title">Today's Focus</h3>
            <p class="card-subtitle">
              {completedSessions} completed
            </p>
          </div>
          <button
            class="btn-add"
            onclick={() => {
              const input = document.querySelector<HTMLInputElement>('.new-input');
              input?.focus();
            }}
          >
            <Plus size={14} />
            New Task
          </button>
        </div>

        <div class="task-list">
          {#each tasks as task (task.id)}
            {@const isTaskActive = task.id === activeTaskId}
            <div
              class="task-row"
              class:active={isTaskActive}
              onclick={() => selectTask(task.id)}
              role="button"
              tabindex="0"
              onkeydown={(e) => e.key === "Enter" && selectTask(task.id)}
            >
              <div class="task-indicator" class:active={isTaskActive}>
                {#if isTaskActive}
                  <div class="indicator-pulse"></div>
                {/if}
              </div>
              {#if editingId === task.id}
                <input
                  class="edit-input"
                  bind:value={editingTitle}
                  onkeydown={(e) => {
                    if (e.key === "Enter") saveEdit();
                    if (e.key === "Escape") editingId = null;
                  }}
                  onblur={saveEdit}
                  onclick={(e) => e.stopPropagation()}
                  use:focusOnMount
                />
              {:else}
                <div class="task-content">
                  <h4 class="task-title">{task.title}</h4>
                  <div class="task-times">
                    <span class="task-time">{fmtDuration(task.total_seconds)}</span>
                    {#if isTaskActive && (running || activeTaskElapsed > 0)}
                      <span class="task-session-time">+{fmtSession(activeTaskElapsed)}</span>
                    {/if}
                  </div>
                </div>
                <div class="task-meta">
                  {#if isTaskActive}
                    <span class="chip-active">ACTIVE</span>
                  {/if}
                  <div class="task-actions">
                    <button
                      class="action-btn"
                      onclick={(e) => startEdit(task, e)}
                      title="Rename"><Pencil size={12} /></button
                    >
                    <button
                      class="action-btn"
                      onclick={(e) => resetTime(task.id, e)}
                      title="Reset time"><RotateCcw size={12} /></button
                    >
                    <button
                      class="action-btn danger"
                      onclick={(e) => deleteTask(task.id, e)}
                      title="Delete"><X size={12} /></button
                    >
                  </div>
                </div>
              {/if}
            </div>
          {/each}

          {#if tasks.length === 0}
            <p class="empty">No tasks yet. Add one to get started.</p>
          {/if}
        </div>

        <div class="new-task">
          <input
            class="new-input"
            bind:value={newTitle}
            placeholder="Add a new task..."
            onkeydown={(e) => e.key === "Enter" && addTask()}
          />
          <button
            class="btn-add-small"
            onclick={addTask}
            disabled={!newTitle.trim()}>Add</button
          >
        </div>
      </div>

      <!-- Configurations Card -->
      <div class="card config-card">
        <h4 class="config-title">Configurations</h4>
        <div class="preset-grid">
          {#each presets as preset, i}
            <button
              class="preset-btn"
              class:active={activePresetIndex === i}
              onclick={() => setPreset(preset.workSecs, preset.breakSecs)}
            >
              <span class="preset-label">{preset.label}</span>
              <span class="preset-duration"
                >{fmtPreset(preset.workSecs)} / {fmtPreset(
                  preset.breakSecs,
                )}</span
              >
            </button>
          {/each}
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .pomodoro {
    flex: 1;
    display: flex;
    flex-direction: column;
    background: #131313;
    color: #e5e2e1;
    font-family: "Inter", sans-serif;
    overflow: hidden;
    position: relative;
  }

  .scroll-area {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    padding: 0 24px;
  }

  /* ── Hero Timer ── */
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 28px 0 20px;
    flex-shrink: 0;
  }

  .status-pill {
    display: flex;
    align-items: center;
    gap: 6px;
    background: #1b1b1c;
    padding: 5px 14px;
    border-radius: 999px;
    border: 1px solid rgba(61, 73, 70, 0.2);
    margin-bottom: 16px;
  }

  .status-icon {
    font-size: 11px;
  }

  .status-text {
    font-family: "Space Grotesk", sans-serif;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.2em;
    text-transform: uppercase;
    color: #e5e2e1;
  }

  .status-pill.break {
    border-color: rgba(255, 195, 183, 0.2);
  }

  .status-pill.break .status-text {
    color: #ffc3b7;
  }

  .timer-wrapper {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .timer-glow {
    position: absolute;
    inset: -24px;
    background: rgba(109, 229, 203, 0.04);
    filter: blur(40px);
    border-radius: 50%;
    pointer-events: none;
  }

  .timer-display {
    font-family: "Space Grotesk", sans-serif;
    font-size: 8rem;
    font-weight: 300;
    line-height: 1;
    letter-spacing: -0.04em;
    color: #e5e2e1;
    position: relative;
    user-select: none;
    margin: 0;
  }

  .round-info {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
    margin-top: 8px;
  }

  .round-label {
    font-family: "JetBrains Mono", monospace;
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    color: #bccac4;
  }

  .round-dots {
    display: flex;
    gap: 6px;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: #353535;
    transition: all 0.3s ease;
  }

  .dot.filled {
    background: #6de5cb;
    box-shadow: 0 0 8px rgba(109, 229, 203, 0.5);
  }

  .hero-controls {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-top: 20px;
  }

  .btn-primary {
    padding: 10px 32px;
    background: linear-gradient(135deg, #6de5cb, #4ec9b0);
    color: #00382e;
    font-family: "Space Grotesk", sans-serif;
    font-weight: 700;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    border: none;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    box-shadow: 0 4px 16px rgba(109, 229, 203, 0.15);
    min-width: unset;
  }

  .btn-primary:hover {
    box-shadow: 0 6px 24px rgba(109, 229, 203, 0.25);
    transform: translateY(-1px);
  }

  .btn-primary:active {
    transform: scale(0.97);
  }

  .btn-icon {
    padding: 10px;
    background: transparent;
    border: 1px solid rgba(61, 73, 70, 0.3);
    color: #6de5cb;
    border-radius: 6px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s ease;
    min-width: unset;
  }

  .btn-icon:hover {
    background: #2a2a2a;
  }

  .btn-icon:active {
    transform: scale(0.95);
  }

  .btn-secondary {
    padding: 10px 20px;
    background: transparent;
    border: 1px solid rgba(61, 73, 70, 0.3);
    color: #6de5cb;
    font-family: "Space Grotesk", sans-serif;
    font-weight: 600;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.1em;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s ease;
    min-width: unset;
  }

  .btn-secondary:hover {
    background: rgba(109, 229, 203, 0.1);
  }

  /* ── Content Grid ── */
  .content-grid {
    display: grid;
    grid-template-columns: 1fr 180px;
    gap: 12px;
    padding-bottom: 16px;
    flex: 1;
    min-height: 0;
  }

  .card {
    background: #1b1b1c;
    border-radius: 12px;
    border: 1px solid rgba(61, 73, 70, 0.1);
    overflow: hidden;
  }

  /* ── Tasks Card ── */
  .tasks-card {
    display: flex;
    flex-direction: column;
    padding: 16px;
    min-height: 0;
  }

  .card-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin-bottom: 12px;
    flex-shrink: 0;
  }

  .card-title {
    font-family: "Space Grotesk", sans-serif;
    font-size: 14px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: -0.01em;
    color: #e5e2e1;
    margin: 0;
  }

  .card-subtitle {
    font-family: "JetBrains Mono", monospace;
    font-size: 10px;
    color: #bccac4;
    margin: 2px 0 0;
  }

  .btn-add {
    display: flex;
    align-items: center;
    gap: 4px;
    color: #6de5cb;
    font-family: "Space Grotesk", sans-serif;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.15em;
    background: transparent;
    border: none;
    padding: 4px 8px;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
    min-width: unset;
  }

  .btn-add:hover {
    background: rgba(109, 229, 203, 0.1);
  }

  .task-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .empty {
    font-size: 11px;
    color: #86948f;
    text-align: center;
    padding: 24px 0;
  }

  .task-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 12px;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
    background: rgba(14, 14, 14, 0.5);
    border: 1px solid rgba(61, 73, 70, 0.05);
  }

  .task-row:hover {
    background: #1b1b1c;
    border-color: rgba(61, 73, 70, 0.15);
  }

  .task-row.active {
    background: #2a2a2a;
    border-left: 2px solid #6de5cb;
    padding-left: 10px;
  }

  .task-indicator {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 1px solid #3d4946;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
  }

  .task-indicator.active {
    border-color: #6de5cb;
    border-width: 2px;
  }

  .indicator-pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #6de5cb;
    animation: indicatorPulse 2s ease-in-out infinite;
  }

  @keyframes indicatorPulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }

  .task-content {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .task-title {
    font-family: "Space Grotesk", sans-serif;
    font-size: 12px;
    font-weight: 600;
    color: #bccac4;
    margin: 0;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .task-row.active .task-title {
    color: #e5e2e1;
  }

  .task-times {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .task-time {
    font-family: "JetBrains Mono", monospace;
    font-size: 10px;
    color: #86948f;
  }

  .task-row.active .task-time {
    color: #bccac4;
  }

  .task-session-time {
    font-family: "JetBrains Mono", monospace;
    font-size: 10px;
    color: #6de5cb;
    font-weight: 600;
  }

  .task-meta {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .chip-active {
    padding: 2px 8px;
    background: #353535;
    color: #80f7dc;
    border-radius: 999px;
    font-size: 9px;
    font-weight: 700;
    letter-spacing: 0.05em;
    font-family: "Space Grotesk", sans-serif;
  }

  .task-actions {
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.15s;
  }

  .task-row:hover .task-actions {
    opacity: 1;
  }

  .action-btn {
    background: transparent;
    color: #86948f;
    border: none;
    padding: 3px 5px;
    cursor: pointer;
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    min-width: unset;
  }

  .action-btn:hover {
    color: #bccac4;
    background: rgba(53, 53, 53, 0.8);
  }

  .action-btn.danger:hover {
    color: #ffc3b7;
    background: rgba(147, 0, 10, 0.2);
  }

  .edit-input {
    flex: 1;
    background: #0e0e0e;
    border: 1px solid #6de5cb;
    border-radius: 4px;
    color: #e5e2e1;
    font-family: "Inter", sans-serif;
    font-size: 12px;
    padding: 4px 8px;
    outline: none;
    box-shadow: 0 0 8px rgba(109, 229, 203, 0.15);
  }

  .new-task {
    display: flex;
    gap: 8px;
    flex-shrink: 0;
    padding-top: 10px;
    margin-top: 8px;
  }

  .new-input {
    flex: 1;
    background: #0e0e0e;
    border: 1px solid rgba(61, 73, 70, 0.1);
    border-radius: 4px;
    color: #e5e2e1;
    font-family: "Inter", sans-serif;
    font-size: 12px;
    padding: 6px 10px;
    outline: none;
    transition: border-color 0.2s;
  }

  .new-input:focus {
    border-color: #6de5cb;
    box-shadow: 0 0 6px rgba(109, 229, 203, 0.12);
  }

  .new-input::placeholder {
    color: #3d4946;
  }

  .btn-add-small {
    background: transparent;
    color: #6de5cb;
    border: 1px solid rgba(61, 73, 70, 0.3);
    border-radius: 4px;
    padding: 6px 14px;
    font-family: "Space Grotesk", sans-serif;
    font-size: 11px;
    font-weight: 600;
    cursor: pointer;
    transition: all 0.15s;
    min-width: unset;
  }

  .btn-add-small:hover:not(:disabled) {
    background: rgba(109, 229, 203, 0.1);
  }

  .btn-add-small:disabled {
    opacity: 0.3;
    cursor: default;
  }

  /* ── Config Card ── */
  .config-card {
    padding: 14px;
    display: flex;
    flex-direction: column;
    height: fit-content;
  }

  .config-title {
    font-family: "Space Grotesk", sans-serif;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.2em;
    color: #bccac4;
    margin: 0 0 10px;
  }

  .preset-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px;
  }

  .preset-btn {
    padding: 10px 8px;
    background: #0e0e0e;
    border: 1px solid rgba(61, 73, 70, 0.1);
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    display: flex;
    flex-direction: column;
    gap: 4px;
    transition: all 0.15s;
    min-width: unset;
  }

  .preset-btn:hover {
    border-color: rgba(109, 229, 203, 0.2);
  }

  .preset-btn.active {
    background: #2a2a2a;
    border-color: rgba(109, 229, 203, 0.3);
  }

  .preset-label {
    font-family: "JetBrains Mono", monospace;
    font-size: 9px;
    color: #bccac4;
    letter-spacing: 0.05em;
  }

  .preset-btn.active .preset-label {
    color: #6de5cb;
  }

  .preset-duration {
    font-family: "Space Grotesk", sans-serif;
    font-size: 13px;
    font-weight: 700;
    color: #e5e2e1;
  }
</style>
