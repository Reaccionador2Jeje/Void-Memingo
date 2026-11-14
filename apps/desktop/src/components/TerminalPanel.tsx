import { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { Plus, TerminalSquare, X } from 'lucide-react';

type TerminalOutput = { sessionId: string; data: string };
type TerminalExit = { sessionId: string; code: number | null };
type Session = { id: string; name: string };
type Props = { workspace: string | null; onLog: (message: string) => void; command?: string | null; onCommandConsumed?: () => void; interruptSignal?: number; onCommandStarted?: (sessionId: string) => void; onCommandFinished?: (exitCode: number, sessionId: string) => void; onCommandOutput?: (data: string, sessionId: string) => void };

function TerminalView({ sessionId, active, onCommandFinished, onCommandOutput }: { sessionId: string; active: boolean; onCommandFinished: (exitCode: number, sessionId: string) => void; onCommandOutput: (data: string, sessionId: string) => void }) {
  const hostRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    const terminal = new Terminal({
      cursorBlink: true,
      cursorStyle: 'bar',
      fontSize: 12,
      fontFamily: '"JetBrains Mono", "Cascadia Code", "SFMono-Regular", Consolas, monospace',
      lineHeight: 1.35,
      scrollback: 5000,
      theme: {
        background: '#11141a', foreground: '#d7dce7', cursor: '#b9b1ff',
        cursorAccent: '#11141a', selectionBackground: '#49416f',
        black: '#171a21', red: '#e58c91', green: '#79d6ad', yellow: '#e4c58a',
        blue: '#8faeff', magenta: '#b9a5ff', cyan: '#82cbd1', white: '#d7dce7',
        brightBlack: '#697284', brightRed: '#f0a0a5', brightGreen: '#91e5bd',
        brightYellow: '#f0d49b', brightBlue: '#a5bdff', brightMagenta: '#c9b9ff',
        brightCyan: '#9be0e5', brightWhite: '#ffffff'
      }
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    let disposed = false;
    let outputUnlisten: (() => void) | undefined;
    let exitUnlisten: (() => void) | undefined;
    let markerBuffer = '';

    const connect = async () => {
      outputUnlisten = await listen<TerminalOutput>('terminal://output', (event) => {
        if (event.payload.sessionId === sessionId) {
          const data = event.payload.data;
          terminal.write(data);
          onCommandOutput(data, sessionId);
          markerBuffer = (markerBuffer + data).slice(-160);
          const match = markerBuffer.match(/\x1b\]777;VOID_MEMINGO_EXIT;(\d+)\x07/);
          if (match) {
            markerBuffer = '';
            onCommandFinished(Number(match[1]), sessionId);
          }
        }
      });
      exitUnlisten = await listen<TerminalExit>('terminal://exit', (event) => {
        if (event.payload.sessionId === sessionId) {
          terminal.writeln('');
          terminal.writeln('\x1b[90m[Shell exited with code ' + (event.payload.code ?? 'unknown') + ']\x1b[0m');
        }
      });
      if (disposed) {
        outputUnlisten?.();
        exitUnlisten?.();
        return;
      }
      fit.fit();
      const dimensions = fit.proposeDimensions();
      if (dimensions) void invoke('resize_terminal', {
        sessionId, cols: dimensions.cols, rows: dimensions.rows
      }).catch(() => undefined);
      // Redraw the prompt if the initial shell prompt arrived before the view mounted.
      void invoke('write_terminal_input', { sessionId, data: '\r' }).catch(() => undefined);
    };

    const dataSubscription = terminal.onData((data) => {
      void invoke('write_terminal_input', { sessionId, data }).catch((error) => {
        terminal.write('\r\n\x1b[31m' + String(error) + '\x1b[0m\r\n');
      });
    });
    const resizeObserver = new ResizeObserver(() => {
      if (disposed) return;
      try {
        fit.fit();
        const dimensions = fit.proposeDimensions();
        if (dimensions) void invoke('resize_terminal', {
          sessionId, cols: dimensions.cols, rows: dimensions.rows
        }).catch(() => undefined);
      } catch {
        // The terminal can briefly have zero size while switching panels.
      }
    });
    resizeObserver.observe(host);
    void connect();

    return () => {
      disposed = true;
      resizeObserver.disconnect();
      dataSubscription.dispose();
      outputUnlisten?.();
      exitUnlisten?.();
      terminal.dispose();
    };
  }, [sessionId, onCommandFinished, onCommandOutput]);

  return <div className="terminal-xterm-host" ref={hostRef} style={{ display: active ? 'block' : 'none' }} />;
}

export default function TerminalPanel({ workspace, onLog, command, onCommandConsumed, interruptSignal = 0, onCommandStarted, onCommandFinished = () => undefined, onCommandOutput = () => undefined }: Props) {
  const [sessions, setSessions] = useState<Session[]>([]);
  const [activeId, setActiveId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [selectedShell, setSelectedShell] = useState('default');
  const isWindows = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);
  const sessionsRef = useRef<Session[]>([]);
  const lastInterruptRef = useRef(interruptSignal);
  sessionsRef.current = sessions;

  const createSession = useCallback(async () => {
    if (!workspace || creating) return;
    setCreating(true);
    try {
      const id = await invoke<string>('start_terminal', {
        root: workspace, currentDirectory: '', cols: 100, rows: 24, shell: selectedShell
      });
      const next = { id, name: 'Terminal ' + (sessionsRef.current.length + 1) };
      setSessions((current) => [...current, next]);
      setActiveId(id);
      onLog('Nueva sesión de terminal: ' + next.name);
    } catch (error) {
      onLog('No se pudo iniciar la terminal: ' + String(error));
    } finally {
      setCreating(false);
    }
  }, [workspace, creating, onLog, selectedShell]);

  useEffect(() => {
    if (!workspace) {
      setSessions([]);
      setActiveId(null);
      return;
    }
    let cancelled = false;
    let createdId: string | null = null;
    void invoke<string>('start_terminal', {
      root: workspace, currentDirectory: '', cols: 100, rows: 24, shell: 'default'
    }).then((id) => {
      createdId = id;
      if (cancelled) {
        void invoke('close_terminal', { sessionId: id }).catch(() => undefined);
        return;
      }
      setSessions([{ id, name: 'Terminal 1' }]);
      setActiveId(id);
      onLog('Terminal persistente iniciada.');
    }).catch((error) => onLog('No se pudo iniciar la terminal: ' + String(error)));

    return () => {
      cancelled = true;
      sessionsRef.current.forEach((session) => {
        void invoke('close_terminal', { sessionId: session.id }).catch(() => undefined);
      });
      if (createdId && !sessionsRef.current.some((session) => session.id === createdId)) {
        void invoke('close_terminal', { sessionId: createdId }).catch(() => undefined);
      }
    };
  }, [workspace, onLog]);

  useEffect(() => {
    if (!command || !activeId) return;
    const trackedCommand = command + '; __void_memingo_exit=$?; printf "\\033]777;VOID_MEMINGO_EXIT;%s\\007" "$__void_memingo_exit"';
    onCommandStarted?.(activeId);
    void invoke('write_terminal_input', {
      sessionId: activeId,
      data: trackedCommand + '\r'
    }).then(() => {
      onLog('Comando enviado a la terminal; seguimiento de salida activo.');
      onCommandConsumed?.();
    }).catch((error) => {
      onCommandFinished(-1, activeId);
      onLog('No se pudo enviar el comando: ' + String(error));
    });
  }, [command, activeId, onCommandConsumed, onCommandFinished, onCommandStarted, onLog]);

  useEffect(() => {
    if (interruptSignal <= lastInterruptRef.current) return;
    lastInterruptRef.current = interruptSignal;
    if (!activeId) return;
    void invoke('write_terminal_input', { sessionId: activeId, data: '\x03' })
      .then(() => onLog('Se envió Ctrl+C a la terminal activa.'))
      .catch((error) => onLog('No se pudo interrumpir la terminal: ' + String(error)));
  }, [interruptSignal, activeId, onLog]);

  const closeSession = (id: string) => {
    void invoke('close_terminal', { sessionId: id }).catch((error) => onLog(String(error)));
    setSessions((current) => {
      const remaining = current.filter((session) => session.id !== id);
      if (activeId === id) setActiveId(remaining[0]?.id ?? null);
      return remaining;
    });
    onLog('Sesión de terminal cerrada.');
  };

  return (
    <div className="terminal-panel">
      <div className="terminal-toolbar">
        <div className="terminal-session-tabs">
          <TerminalSquare size={14} className="terminal-toolbar-icon" />
          {sessions.map((session) => (
            <button key={session.id}
              className={'terminal-session-tab ' + (activeId === session.id ? 'active' : '')}
              onClick={() => setActiveId(session.id)} title={session.name}>
              <span>{session.name}</span>
              <span className="terminal-tab-close" role="button" aria-label={'Cerrar ' + session.name}
                onClick={(event) => { event.stopPropagation(); closeSession(session.id); }}>
                <X size={11} />
              </span>
            </button>
          ))}
        </div>
        <select className="terminal-shell-select" value={selectedShell}
          onChange={(event) => setSelectedShell(event.target.value)} title="Shell para nuevas terminales">
          {isWindows ? (
            <>
              <option value="default">Predeterminado</option>
              <option value="cmd">Command Prompt</option>
              <option value="powershell">PowerShell</option>
              <option value="pwsh">PowerShell 7</option>
            </>
          ) : (
            <>
              <option value="default">Predeterminado</option>
              <option value="sh">sh</option>
              <option value="bash">bash</option>
              <option value="zsh">zsh</option>
              <option value="fish">fish</option>
            </>
          )}
        </select>
        <button className="terminal-new-session" onClick={() => void createSession()}
          disabled={!workspace || creating} title="Nueva terminal"><Plus size={14} /></button>
      </div>
      <div className="terminal-views">
        {!workspace ? (
          <div className="terminal-empty">Abre una carpeta de proyecto para iniciar una terminal.</div>
        ) : sessions.length === 0 ? (
          <div className="terminal-empty"><span>No hay sesiones abiertas.</span>
            <button onClick={() => void createSession()} disabled={creating}>Crear terminal</button>
          </div>
        ) : sessions.map((session) => (
          <TerminalView key={session.id} sessionId={session.id} active={activeId === session.id} onCommandFinished={onCommandFinished} onCommandOutput={onCommandOutput} />
        ))}
      </div>
    </div>
  );
}
