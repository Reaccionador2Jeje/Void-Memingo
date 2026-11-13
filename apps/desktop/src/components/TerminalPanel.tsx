import { FormEvent, KeyboardEvent, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { ArrowUp, CheckCircle2, ChevronRight, CircleX, Eraser, LoaderCircle, TerminalSquare } from 'lucide-react';

type TerminalResult = {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  success: boolean;
};

type Entry = {
  id: number;
  command: string;
  stdout: string;
  stderr: string;
  exitCode: number | null;
  error?: string;
};

type Props = {
  workspace: string | null;
  onLog: (message: string) => void;
};

export default function TerminalPanel({ workspace, onLog }: Props) {
  const [command, setCommand] = useState('');
  const [cwd, setCwd] = useState('');
  const [entries, setEntries] = useState<Entry[]>([]);
  const [history, setHistory] = useState<string[]>([]);
  const [historyIndex, setHistoryIndex] = useState(-1);
  const [running, setRunning] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const outputRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setCwd('');
    setEntries([]);
    setHistory([]);
    setHistoryIndex(-1);
  }, [workspace]);

  useEffect(() => {
    outputRef.current?.scrollTo({ top: outputRef.current.scrollHeight });
  }, [entries]);

  const submit = async (event?: FormEvent) => {
    event?.preventDefault();
    const value = command.trim();
    if (!workspace || !value || running) return;

    setCommand('');
    setHistory((items) => [...items, value]);
    setHistoryIndex(-1);
    const id = Date.now() + Math.random();
    setRunning(true);

    const cdMatch = value.match(/^cd(?:\s+(.+))?$/i);
    if (cdMatch) {
      const target = (cdMatch[1] ?? '').trim().replace(/^(['"])(.*)\1$/, '$2') || '.';
      try {
        const next = await invoke<string>('resolve_terminal_directory', {
          root: workspace,
          currentDirectory: cwd,
          target
        });
        setCwd(next);
        setEntries((items) => [...items, {
          id, command: value, stdout: next ? 'Directory changed to ' + next : 'Directory changed to workspace root.',
          stderr: '', exitCode: 0
        }]);
      } catch (error) {
        setEntries((items) => [...items, {
          id, command: value, stdout: '', stderr: String(error), exitCode: 1
        }]);
      } finally {
        setRunning(false);
        inputRef.current?.focus();
      }
      return;
    }

    try {
      const result = await invoke<TerminalResult>('run_terminal_command', {
        root: workspace,
        currentDirectory: cwd,
        command: value
      });
      setEntries((items) => [...items, {
        id,
        command: value,
        stdout: result.stdout,
        stderr: result.stderr,
        exitCode: result.exitCode
      }]);
      onLog('Terminal: ' + value + ' (exit ' + (result.exitCode ?? 'unknown') + ')');
    } catch (error) {
      setEntries((items) => [...items, {
        id, command: value, stdout: '', stderr: String(error), exitCode: 1, error: String(error)
      }]);
      onLog('Error de terminal: ' + String(error));
    } finally {
      setRunning(false);
      inputRef.current?.focus();
    }
  };

  const handleHistory = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowUp') {
      event.preventDefault();
      const next = Math.max(0, historyIndex < 0 ? history.length - 1 : historyIndex - 1);
      if (history.length) {
        setHistoryIndex(next);
        setCommand(history[next]);
      }
    } else if (event.key === 'ArrowDown' && historyIndex >= 0) {
      event.preventDefault();
      const next = historyIndex + 1;
      if (next >= history.length) {
        setHistoryIndex(-1);
        setCommand('');
      } else {
        setHistoryIndex(next);
        setCommand(history[next]);
      }
    }
  };

  return (
    <div className="terminal-panel">
      <div className="terminal-toolbar">
        <div className="terminal-location">
          <TerminalSquare size={14} />
          <span>{workspace ? workspace.split(/[\\/]/).filter(Boolean).pop() : 'Sin workspace'}</span>
          <ChevronRight size={12} />
          <span className="terminal-cwd">{cwd || '.'}</span>
        </div>
        <div className="terminal-toolbar-actions">
          <button className="subtle-icon" onClick={() => setEntries([])} title="Limpiar terminal" disabled={entries.length === 0}>
            <Eraser size={14} />
          </button>
          <button className="subtle-icon" onClick={() => inputRef.current?.focus()} title="Enfocar terminal">
            <ArrowUp size={14} />
          </button>
        </div>
      </div>

      <div className="terminal-output" ref={outputRef}>
        {!workspace ? (
          <div className="terminal-hint">Abre una carpeta de proyecto para utilizar la terminal.</div>
        ) : entries.length === 0 ? (
          <div className="terminal-hint">Terminal lista. Los comandos se ejecutan en el workspace seleccionado. Escribe <code>help</code> o <code>ls</code> para comenzar.</div>
        ) : entries.map((entry) => (
          <div className="terminal-entry" key={entry.id}>
            <div className="terminal-command-line">
              <span className="terminal-prompt">❯</span>
              <span>{entry.command}</span>
              {entry.exitCode === 0 && <CheckCircle2 size={12} className="terminal-success" />}
              {entry.exitCode !== null && entry.exitCode !== 0 && <CircleX size={12} className="terminal-failure" />}
            </div>
            {entry.stdout && <pre className="terminal-stdout">{entry.stdout}</pre>}
            {entry.stderr && <pre className="terminal-stderr">{entry.stderr}</pre>}
            {entry.exitCode !== null && <div className="terminal-exit">Process exited with code {entry.exitCode}</div>}
          </div>
        ))}
        {running && <div className="terminal-running"><LoaderCircle size={13} /> Ejecutando comando…</div>}
      </div>

      <form className="terminal-input-row" onSubmit={submit}>
        <span className="terminal-prompt">❯</span>
        <input
          ref={inputRef}
          value={command}
          onChange={(event) => setCommand(event.target.value)}
          onKeyDown={handleHistory}
          placeholder={workspace ? 'Escribe un comando…' : 'Abre un workspace primero'}
          disabled={!workspace || running}
          autoComplete="off"
          autoCapitalize="off"
          spellCheck={false}
          aria-label="Comando de terminal"
        />
        <button type="submit" disabled={!workspace || !command.trim() || running} title="Ejecutar comando">
          {running ? <LoaderCircle size={14} /> : <ArrowUp size={14} />}
        </button>
      </form>
    </div>
  );
}
