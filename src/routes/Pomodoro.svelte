<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  const WORK_DURATION = 25 * 60;
  const BREAK_DURATION = 5 * 60;
  const ROUND_SIZE = 4;

  let mode = $state<'work' | 'break'>('work');
  let remaining = $state(WORK_DURATION);
  let running = $state(false);
  let completedSessions = $state(0);

  const minutes = $derived(String(Math.floor(remaining / 60)).padStart(2, '0'));
  const seconds = $derived(String(remaining % 60).padStart(2, '0'));
  const doneInRound = $derived(completedSessions % ROUND_SIZE);

  const WORK_FULL = WORK_DURATION;
  const BREAK_FULL = BREAK_DURATION;

  $effect(() => {
    const isIdleWork = !running && mode === 'work' && remaining === WORK_FULL;
    const isIdleBreak = !running && mode === 'break' && remaining === BREAK_FULL;
    const isIdle = isIdleWork || isIdleBreak;
    let title: string;
    if (isIdle) {
      title = '';
    } else if (running) {
      title = `${minutes}:${seconds}`;
    } else {
      title = `⏸ ${minutes}:${seconds}`;
    }
    invoke('update_tray_title', { title });
  });

  $effect(() => {
    if (!running) return;
    const id = setInterval(() => {
      remaining -= 1;
      if (remaining <= 0) {
        running = false;
        if (mode === 'work') {
          completedSessions += 1;
          mode = 'break';
          remaining = BREAK_DURATION;
        } else {
          mode = 'work';
          remaining = WORK_DURATION;
        }
      }
    }, 1000);
    return () => clearInterval(id);
  });

  function toggle() {
    running = !running;
  }

  function reset() {
    running = false;
    remaining = mode === 'work' ? WORK_DURATION : BREAK_DURATION;
  }

  function skipBreak() {
    running = false;
    mode = 'work';
    remaining = WORK_DURATION;
  }
</script>

<div class="pomodoro">
  <div class="mode">{mode === 'work' ? 'WORK' : 'BREAK'}</div>
  <div class="timer">{minutes}:{seconds}</div>
  <div class="controls">
    <button onclick={toggle}>{running ? 'Pause' : 'Start'}</button>
    <button onclick={reset}>Reset</button>
    {#if mode === 'break'}
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

<style>
  .pomodoro {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 28px;
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: "Menlo", "Monaco", "Courier New", monospace;
  }

  .mode {
    font-size: 12px;
    letter-spacing: 0.25em;
    color: #569cd6;
  }

  .timer {
    font-size: 80px;
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

  button:hover {
    background: #4a4a4a;
  }

  .sessions {
    font-size: 12px;
    color: #666;
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .dots {
    display: flex;
    gap: 4px;
    font-size: 18px;
  }

  .dots span {
    color: #444;
    transition: color 0.3s;
  }

  .dots span.filled {
    color: #569cd6;
  }
</style>
