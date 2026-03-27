<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { listen } from "@tauri-apps/api/event";
  import Notes from "./Notes.svelte";
  import Pomodoro from "./Pomodoro.svelte";
  import FocusRestrictions from "./FocusRestrictions.svelte";
  import Todo from "./Todo.svelte";
  import ShortcutGuide from "./ShortcutGuide.svelte";
  import { CircleHelp } from "lucide-svelte";
  let activeTab = $state<"notes" | "pomodoro" | "blocked" | "todo">("notes");
  let showShortcutGuide = $state(false);
  const tabs: Array<typeof activeTab> = [
    "notes",
    "pomodoro",
    "blocked",
    "todo",
  ];

  function onTabKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (showShortcutGuide) {
        showShortcutGuide = false;
        return;
      }
      if (showQuitModal) {
        cancelQuit();
        return;
      }
      if (showPasswordModal) {
        cancelPassword();
        return;
      }
    }
    const target = e.target as HTMLElement;
    if (
      e.key === "?" &&
      target.tagName !== "INPUT" &&
      target.tagName !== "TEXTAREA"
    ) {
      showShortcutGuide = !showShortcutGuide;
      return;
    }
    if (!e.metaKey) return;
    const num = parseInt(e.key);
    if (num >= 1 && num <= tabs.length) {
      e.preventDefault();
      activeTab = tabs[num - 1];
    }
  }

  let notesDirty = $state(false);

  // Blocked websites state
  let blockedDomains = $state<string[]>([]);
  let blockedApps = $state<string[]>([]);
  let blockingActive = $state(false);
  let toggling = $state(false);

  // Track whether focus was auto-activated by pomodoro
  let focusAutoActivated = $state(false);
  // Track if pomodoro is currently in work mode
  let pomodoroWorking = $state(false);

  // Password modal state
  let showPasswordModal = $state(false);
  let pendingPassword = $state("");
  let pendingDomains = $state<string[]>([]);
  let passwordError = $state("");

  // Quit confirmation modal
  let showQuitModal = $state(false);

  $effect(() => {
    const unlistenClose = getCurrentWindow().onCloseRequested((e) => {
      e.preventDefault();
      showQuitModal = true;
    });
    const unlistenTrayQuit = listen("quit-requested", () => {
      showQuitModal = true;
    });
    return () => {
      unlistenClose.then((fn) => fn());
      unlistenTrayQuit.then((fn) => fn());
    };
  });

  function confirmQuit() {
    invoke("app_exit");
  }

  function cancelQuit() {
    showQuitModal = false;
  }

  $effect(() => {
    invoke<string[]>("read_domains").then((domains) => {
      blockedDomains = domains;
    });
    invoke<string[]>("read_blocked_apps")
      .then((apps) => { blockedApps = apps; })
      .catch(() => {});
    invoke<boolean>("get_blocking_status").then((active) => {
      blockingActive = active;
    });
  });

  // Auto-activate focus when pomodoro is running in work mode
  let prevPomodoroWork = $state(false);
  $effect(() => {
    const unlisten = listen<{ running: boolean; mode: string }>(
      "pomodoro-tick",
      (event) => {
        const isWorking =
          event.payload.running && event.payload.mode === "work";
        if (isWorking && !prevPomodoroWork && !blockingActive && !toggling) {
          focusAutoActivated = true;
          invoke("set_focus_active", { active: true }).catch(() => {});
          toggleBlocking(true);
        } else if (
          !isWorking &&
          prevPomodoroWork &&
          blockingActive &&
          focusAutoActivated &&
          !toggling
        ) {
          focusAutoActivated = false;
          invoke("set_focus_active", { active: false }).catch(() => {});
          toggleBlocking(true);
        }
        prevPomodoroWork = isWorking;
        pomodoroWorking = isWorking;
      },
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  });

  async function saveBlocked(domains: string[]) {
    await invoke("save_domains", { domains });
    blockedDomains = domains;
    if (blockingActive) {
      await invoke("write_blocked", { domains });
    }
  }

  async function saveBlockedApps(apps: string[]) {
    await invoke("save_blocked_apps", { apps });
    blockedApps = apps;
  }

  async function toggleBlocking(auto = false) {
    toggling = true;
    if (!auto && blockingActive) {
      // Manual deactivation clears auto flag
      focusAutoActivated = false;
    }
    const newActive = !blockingActive;
    const domains = newActive ? blockedDomains : [];
    let needModal = false;

    // Only modify /etc/hosts if there are domains to block/unblock
    if (blockedDomains.length > 0) {
      try {
        await invoke("write_blocked", { domains });
      } catch (e) {
        if (e === "NeedPassword") {
          needModal = true;
          pendingDomains = domains;
          pendingPassword = "";
          passwordError = "";
          showPasswordModal = true;
        } else if (e !== "Cancelled") {
          alert("Error: " + e);
        }
      }
    }

    if (!needModal) {
      blockingActive = newActive;
      invoke("set_focus_active", { active: newActive }).catch(() => {});
      toggling = false;
    }
  }

  async function submitPassword() {
    passwordError = "";
    try {
      await invoke("write_blocked_with_password", {
        domains: pendingDomains,
        password: pendingPassword,
      });
      blockingActive = true;
      invoke("set_focus_active", { active: true }).catch(() => {});
      showPasswordModal = false;
    } catch (e) {
      if (e === "WrongPassword") {
        passwordError = "Wrong password, try again.";
      } else if (e !== "Cancelled") {
        passwordError = String(e);
      }
    } finally {
      if (!showPasswordModal || !passwordError) {
        pendingPassword = "";
        toggling = false;
      }
    }
  }

  function cancelPassword() {
    showPasswordModal = false;
    pendingPassword = "";
    toggling = false;
  }
</script>

<svelte:window onkeydown={onTabKeyDown} />

<div class="app">
  <header data-tauri-drag-region>
    <nav>
      <button
        class="tab"
        class:active={activeTab === "notes"}
        onclick={() => (activeTab = "notes")}
        >Notes
        <span class="filename">{notesDirty ? " •" : ""}</span></button
      >
      <button
        class="tab"
        class:active={activeTab === "pomodoro"}
        onclick={() => (activeTab = "pomodoro")}>Pomodoro</button
      >
      <button
        class="tab"
        class:active={activeTab === "blocked"}
        onclick={() => (activeTab = "blocked")}>Blocked</button
      >
      <button
        class="tab"
        class:active={activeTab === "todo"}
        onclick={() => (activeTab = "todo")}>Todo</button
      >
    </nav>

    <div class="productivity-toggle">
      <span class="toggle-label">{blockingActive ? "Focus ON" : "Focus"}</span>
      <button
        class="productivity-switch"
        class:active={blockingActive}
        class:toggling
        onclick={() => toggleBlocking()}
        disabled={toggling || (pomodoroWorking && blockingActive)}
        title={pomodoroWorking && blockingActive
          ? "Focus cannot be disabled while Pomodoro is running"
          : blockingActive
            ? "Distracting sites are blocked"
            : "Click to block distracting sites"}
      >
        <span class="switch-track">
          <span class="switch-knob"></span>
        </span>
      </button>
    </div>
  </header>

  {#if activeTab === "notes"}
    <Notes bind:isDirty={notesDirty} isActive={true} />
  {:else if activeTab === "pomodoro"}
    <Pomodoro isActive={true} />
  {:else if activeTab === "blocked"}
    <FocusRestrictions
      domains={blockedDomains}
      apps={blockedApps}
      onSaveDomains={saveBlocked}
      onSaveApps={saveBlockedApps}
      focusActive={blockingActive}
    />
  {:else}
    <Todo />
  {/if}
</div>

{#if showPasswordModal}
  <div class="modal-backdrop" role="presentation" onclick={cancelPassword}>
    <div
      class="modal"
      role="dialog"
      tabindex="0"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <p class="modal-title">Administrator password</p>
      <p class="modal-sub">
        Required to modify /etc/hosts. Stored for this session.
      </p>
      <input
        class="modal-input"
        type="password"
        placeholder="Password"
        bind:value={pendingPassword}
        onkeydown={(e) => e.key === "Enter" && submitPassword()}
      />
      {#if passwordError}<p class="modal-error">{passwordError}</p>{/if}
      <div class="modal-actions">
        <button onclick={cancelPassword}>Cancel</button>
        <button
          class="modal-confirm"
          onclick={submitPassword}
          disabled={!pendingPassword}>OK</button
        >
      </div>
    </div>
  </div>
{/if}

{#if showQuitModal}
  <div class="modal-backdrop" role="presentation" onclick={cancelQuit}>
    <div class="modal" role="dialog" onclick={(e) => e.stopPropagation()}>
      <p class="modal-title">Quit application?</p>
      <p class="modal-sub">Are you sure you want to close the app?</p>
      <div class="modal-actions">
        <button onclick={cancelQuit}>Cancel</button>
        <button
          class="modal-confirm modal-confirm--danger"
          onclick={confirmQuit}>Quit</button
        >
      </div>
    </div>
  </div>
{/if}

<button
  class="help-btn"
  onclick={() => (showShortcutGuide = true)}
  title="Keyboard shortcuts (?)"><CircleHelp size={14} /></button
>

{#if showShortcutGuide}
  <ShortcutGuide onclose={() => (showShortcutGuide = false)} />
{/if}

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
    font-family: "Inter", sans-serif;
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
    font-family: "Space Grotesk", "Inter", sans-serif;
    border-bottom: 2px solid transparent;
    transition: color 0.15s;
  }

  .tab:hover {
    color: #ccc;
    background: #333;
  }

  .tab.active {
    color: #d4d4d4;
    border-bottom-color: #4ec9b0;
    background: #2d2d2d;
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

  .productivity-toggle {
    margin-left: auto;
    margin-right: 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding-bottom: 4px;
  }

  .toggle-label {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 10px;
    color: #555;
    letter-spacing: 0.03em;
    transition: color 0.22s ease;
    white-space: nowrap;
  }

  .productivity-toggle:has(.productivity-switch.active) .toggle-label {
    color: #4ec9b0;
  }

  .productivity-switch {
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
    transition:
      background 0.22s ease,
      border-color 0.22s ease,
      box-shadow 0.22s ease;
    position: relative;
  }

  .switch-knob {
    width: 20px;
    height: 20px;
    border-radius: 50%;
    background: #888;
    transition:
      transform 0.22s ease,
      background 0.22s ease;
    flex-shrink: 0;
  }

  .productivity-switch.active .switch-track {
    background: #4ec9b0;
    border-color: #4ec9b0;
    box-shadow:
      0 0 14px #4ec9b055,
      0 2px 8px #0006;
  }

  .productivity-switch.active .switch-knob {
    transform: translateX(24px);
    background: #fff;
  }

  .productivity-switch:not(.active):hover .switch-track {
    background: #4a4a4a;
    border-color: #666;
  }

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
    padding: 20px;
    width: 280px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    box-shadow: 0 8px 32px #0008;
    font-family: "Inter", sans-serif;
  }

  .modal-title {
    font-family: "Space Grotesk", "Inter", sans-serif;
    font-size: 13px;
    color: #d4d4d4;
    font-weight: 600;
  }

  .modal-sub {
    font-size: 11px;
    color: #888;
    line-height: 1.4;
  }

  .modal-input {
    background: #1e1e1e;
    border: 1px solid #555;
    border-radius: 4px;
    color: #d4d4d4;
    font-family: inherit;
    font-size: 13px;
    padding: 6px 10px;
    outline: none;
  }

  .modal-input:focus {
    border-color: #4ec9b0;
  }

  .modal-error {
    font-size: 11px;
    color: #f48771;
  }

  .modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }

  .modal-confirm {
    background: #4ec9b0;
    border-color: #4ec9b0;
    color: #fff;
  }

  .modal-confirm:hover:not(:disabled) {
    background: #3dab96;
  }

  .modal-confirm--danger {
    background: #e05050;
    border-color: #e05050;
  }

  .modal-confirm--danger:hover:not(:disabled) {
    background: #c43e3e;
  }

  .help-btn {
    position: fixed;
    bottom: 12px;
    left: 12px;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: #2d2d2d;
    border: 1px solid #3d3d3d;
    color: #666;
    font-size: 13px;
    font-family: inherit;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    z-index: 100;
    transition:
      color 0.15s,
      border-color 0.15s;
  }

  .help-btn:hover {
    color: #d4d4d4;
    border-color: #555;
    background: #3a3a3a;
  }

</style>
