export type Shortcut = {
  keys: string[];
  description: string;
};

export type ShortcutGroup = {
  label: string;
  shortcuts: Shortcut[];
};

export const SHORTCUTS: ShortcutGroup[] = [
  {
    label: 'Global',
    shortcuts: [
      { keys: ['⌘', '1–4'], description: 'Switch tabs' },
      { keys: ['?'], description: 'Toggle shortcut guide' },
      { keys: ['Esc'], description: 'Close modal' },
    ],
  },
  {
    label: 'Notes',
    shortcuts: [
      { keys: ['⌘', 'S'], description: 'Save note' },
      { keys: ['⌘', 'N'], description: 'New note' },
      { keys: ['Ctrl', '1–N'], description: 'Select note by number' },
      { keys: ['Tab'], description: 'Insert spaces in editor' },
    ],
  },
  {
    label: 'Pomodoro',
    shortcuts: [
      { keys: ['Space'], description: 'Start / Pause timer' },
    ],
  },
];
