import { useCallback, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import {
  Activity, AlertCircle, Box, Check, ChevronDown, ChevronRight, CircleHelp,
  Code2, Command, Cpu, File, FileCode2, FilePlus2, Folder, FolderOpen,
  GitBranch, LayoutDashboard, PanelBottom, PanelLeftClose, Plus, Search,
  Settings, ShieldCheck, TerminalSquare, X, Zap
} from 'lucide-react';

type FileEntry = {
  name: string;
  path: string;
  isDir: boolean;
  size: number | null;
};

type OpenTab = {
  path: string;
  name: string;
  content: string;
  dirty: boolean;
};

type SidebarView = 'explorer' | 'search' | 'source';
type BottomView = 'problems' | 'output' | 'terminal';

function formatBytes(bytes: number | null): string {
  if (bytes === null) return '';
  if (bytes < 1024) return bytes + ' B';
  if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' KB';
  return (bytes / (1024 * 1024)).toFixed(1) + ' MB';
}

function FileNode({
  root,
  entry,
  selectedPath,
  onOpen
}: {
  root: string;
  entry: FileEntry;
  selectedPath: string | null;
  onOpen: (entry: FileEntry) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [children, setChildren] = useState<FileEntry[]>([]);
  const [loading, setLoading] = useState(false);

  const toggle = async () => {
    if (!entry.isDir) {
      onOpen(entry);
      return;
    }
    if (!expanded && children.length === 0) {
      setLoading(true);
      try {
        const result = await invoke<FileEntry[]>('list_directory', {
          root,
          relativePath: entry.path
        });
        setChildren(result);
      } catch (error) {
        console.error(error);
      } finally {
        setLoading(false);
      }
    }
    setExpanded(!expanded);
  };

  return (
    <div className="tree-node">
      <button
        className={'tree-row ' + (selectedPath === entry.path ? 'selected' : '')}
        style={{ paddingLeft: 10 }}
        onClick={toggle}
        title={entry.path}
      >
        {entry.isDir ? (
          <>
            {expanded ? <ChevronDown size={13} /> : <ChevronRight size={13} />}
            {expanded ? <FolderOpen size={15} className="folder-icon" /> : <Folder size={15} className="folder-icon" />}
          </>
        ) : (
          <>
            <span className="tree-indent" />
            <FileCode2 size={14} className="file-icon" />
          </>
        )}
        <span className="tree-name">{entry.name}</span>
        {loading && <span className="tree-loading" />}
      </button>
      {entry.isDir && expanded && (
        <div className="tree-children">
          {children.length === 0 && !loading ? (
            <div className="tree-empty">Carpeta vacía</div>
          ) : children.map((child) => (
            <FileNode
              key={child.path}
              root={root}
              entry={child}
              selectedPath={selectedPath}
              onOpen={onOpen}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function App() {
  const [version, setVersion] = useState('0.5.0');
  const [workspace, setWorkspace] = useState<string | null>(null);
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [tabs, setTabs] = useState<OpenTab[]>([]);
  const [activePath, setActivePath] = useState<string | null>(null);
  const [sidebarView, setSidebarView] = useState<SidebarView>('explorer');
  const [bottomView, setBottomView] = useState<BottomView>('output');
  const [bottomOpen, setBottomOpen] = useState(true);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [logs, setLogs] = useState<string[]>(['Void Memingo Desktop iniciado.']);
  const [busy, setBusy] = useState(false);

  const activeTab = useMemo(
    () => tabs.find((tab) => tab.path === activePath) ?? null,
    [tabs, activePath]
  );

  const addLog = useCallback((message: string) => {
    const stamp = new Date().toLocaleTimeString();
    setLogs((current) => [...current.slice(-99), '[' + stamp + '] ' + message]);
  }, []);

  useEffect(() => {
    invoke<string>('app_version').then(setVersion).catch(() => undefined);
  }, []);

  const refreshWorkspace = useCallback(async (root: string) => {
    const result = await invoke<FileEntry[]>('list_directory', {
      root,
      relativePath: ''
    });
    setFiles(result);
  }, []);

  const openWorkspace = useCallback(async () => {
    const selected = await open({
      directory: true,
      multiple: false,
      title: 'Abrir carpeta de proyecto'
    });
    if (typeof selected !== 'string') return;

    setBusy(true);
    try {
      await refreshWorkspace(selected);
      setWorkspace(selected);
      setTabs([]);
      setActivePath(null);
      addLog('Workspace abierto: ' + selected);
    } catch (error) {
      addLog('Error al abrir workspace: ' + String(error));
    } finally {
      setBusy(false);
    }
  }, [addLog, refreshWorkspace]);

  const openFile = useCallback(async (entry: FileEntry) => {
    const existing = tabs.find((tab) => tab.path === entry.path);
    if (existing) {
      setActivePath(entry.path);
      return;
    }
    if (!workspace) return;

    try {
      const content = await invoke<string>('read_text_file', {
        root: workspace,
        relativePath: entry.path
      });
      setTabs((current) => [...current, {
        path: entry.path,
        name: entry.name,
        content,
        dirty: false
      }]);
      setActivePath(entry.path);
      addLog('Archivo abierto: ' + entry.path);
    } catch (error) {
      addLog('No se pudo abrir ' + entry.name + ': ' + String(error));
    }
  }, [addLog, tabs, workspace]);

  const updateActiveContent = (content: string) => {
    if (!activePath) return;
    setTabs((current) => current.map((tab) =>
      tab.path === activePath ? { ...tab, content, dirty: true } : tab
    ));
  };

  const saveActiveFile = useCallback(async () => {
    if (!workspace || !activeTab) return;
    try {
      await invoke('write_text_file', {
        root: workspace,
        relativePath: activeTab.path,
        contents: activeTab.content
      });
      setTabs((current) => current.map((tab) =>
        tab.path === activeTab.path ? { ...tab, dirty: false } : tab
      ));
      addLog('Guardado: ' + activeTab.path);
    } catch (error) {
      addLog('Error al guardar: ' + String(error));
    }
  }, [activeTab, addLog, workspace]);

  const createFile = useCallback(async () => {
    if (!workspace) {
      await openWorkspace();
      return;
    }
    const name = window.prompt('Ruta del nuevo archivo (ej. src/main.void):');
    if (!name?.trim()) return;
    try {
      await invoke('write_text_file', {
        root: workspace,
        relativePath: name.trim(),
        contents: ''
      });
      await refreshWorkspace(workspace);
      const path = name.trim().replace(/\\\\/g, '/');
      const entry: FileEntry = {
        name: path.split('/').pop() || path,
        path,
        isDir: false,
        size: 0
      };
      await openFile(entry);
      addLog('Archivo creado: ' + path);
    } catch (error) {
      addLog('No se pudo crear el archivo: ' + String(error));
    }
  }, [addLog, openFile, openWorkspace, refreshWorkspace, workspace]);

  const closeTab = (path: string) => {
    const index = tabs.findIndex((tab) => tab.path === path);
    const nextTabs = tabs.filter((tab) => tab.path !== path);
    setTabs(nextTabs);
    if (activePath === path) {
      setActivePath(nextTabs[Math.max(0, index - 1)]?.path ?? nextTabs[0]?.path ?? null);
    }
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const modifier = event.ctrlKey || event.metaKey;
      if (modifier && event.key.toLowerCase() === 's') {
        event.preventDefault();
        void saveActiveFile();
      }
      if (modifier && event.key.toLowerCase() === 'o') {
        event.preventDefault();
        void openWorkspace();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [openWorkspace, saveActiveFile]);

  const language = activeTab?.name.split('.').pop()?.toUpperCase() || 'PLAIN TEXT';
  const lineCount = activeTab?.content.split('\n').length ?? 1;

  return (
    <div className="app-shell">
      <header className="titlebar">
        <div className="brand-lockup">
          <div className="brand-mark"><Command size={17} strokeWidth={2.4} /></div>
          <span className="brand-name">VOID <b>MEMINGO</b></span>
          <span className="brand-divider" />
          <span className="brand-context">DESKTOP</span>
        </div>
        <div className="titlebar-center">
          {workspace ? workspace.split(/[\\\\/]/).filter(Boolean).pop() : 'Sin workspace abierto'}
        </div>
        <div className="titlebar-actions">
          <button className="icon-button" title="Ayuda"><CircleHelp size={16} /></button>
          <button className="icon-button" title="Configuración"><Settings size={16} /></button>
          <div className="window-indicator"><span /></div>
        </div>
      </header>

      <div className="workspace-layout">
        <nav className="activity-rail" aria-label="Activity bar">
          <div className="rail-group">
            <button className={'rail-button ' + (sidebarView === 'explorer' ? 'active' : '')} onClick={() => { setSidebarView('explorer'); setSidebarOpen(true); }} title="Explorador">
              <LayoutDashboard size={20} />
            </button>
            <button className={'rail-button ' + (sidebarView === 'search' ? 'active' : '')} onClick={() => { setSidebarView('search'); setSidebarOpen(true); }} title="Buscar">
              <Search size={20} />
            </button>
            <button className={'rail-button ' + (sidebarView === 'source' ? 'active' : '')} onClick={() => { setSidebarView('source'); setSidebarOpen(true); }} title="Control de código fuente">
              <GitBranch size={20} />
            </button>
            <button className="rail-button" onClick={() => setBottomOpen(!bottomOpen)} title="Panel inferior">
              <PanelBottom size={20} />
            </button>
          </div>
          <div className="rail-group rail-bottom">
            <button className="rail-button" title="Herramientas de sistema"><Cpu size={20} /></button>
            <button className="rail-button" title="Configuración"><Settings size={20} /></button>
          </div>
        </nav>

        {sidebarOpen && (
          <aside className="sidebar">
            <div className="sidebar-heading">
              <span>{sidebarView === 'explorer' ? 'EXPLORADOR' : sidebarView === 'search' ? 'BÚSQUEDA' : 'CONTROL DE CÓDIGO'}</span>
              <button className="subtle-icon" onClick={() => setSidebarOpen(false)} title="Ocultar panel"><PanelLeftClose size={15} /></button>
            </div>

            {sidebarView === 'explorer' && (
              <>
                <div className="workspace-heading">
                  <button className="workspace-label" onClick={() => setSidebarOpen(!sidebarOpen)}>
                    <ChevronDown size={13} />
                    <span>{workspace ? workspace.split(/[\\\\/]/).filter(Boolean).pop() : 'SIN CARPETA ABIERTA'}</span>
                  </button>
                  <div className="workspace-tools">
                    <button className="subtle-icon" onClick={() => void createFile()} title="Nuevo archivo"><FilePlus2 size={15} /></button>
                    <button className="subtle-icon" onClick={() => workspace && void refreshWorkspace(workspace)} title="Actualizar"><Activity size={15} /></button>
                    <button className="subtle-icon" onClick={() => void openWorkspace()} title="Abrir carpeta"><FolderOpen size={15} /></button>
                  </div>
                </div>
                {workspace ? (
                  <div className="file-tree">
                    {files.length === 0 ? <div className="tree-empty root-empty">Esta carpeta está vacía</div> :
                      files.map((entry) => (
                        <FileNode key={entry.path} root={workspace} entry={entry} selectedPath={activePath} onOpen={(item) => void openFile(item)} />
                      ))}
                  </div>
                ) : (
                  <div className="sidebar-empty">
                    <Folder size={25} />
                    <p>No hay ninguna carpeta abierta.</p>
                    <button className="text-action" onClick={() => void openWorkspace()}>Abrir carpeta</button>
                  </div>
                )}
              </>
            )}

            {sidebarView === 'search' && (
              <div className="sidebar-empty">
                <Search size={25} />
                <p>La búsqueda global del workspace llegará en la siguiente etapa.</p>
              </div>
            )}

            {sidebarView === 'source' && (
              <div className="sidebar-empty">
                <GitBranch size={25} />
                <p>La integración Git se añadirá como módulo nativo. No se muestra un estado falso.</p>
              </div>
            )}
          </aside>
        )}

        <main className="main-column">
          <div className="editor-tabs">
            <div className="tabs-list">
              {tabs.map((tab) => (
                <button key={tab.path} className={'editor-tab ' + (activePath === tab.path ? 'active' : '')} onClick={() => setActivePath(tab.path)}>
                  <FileCode2 size={14} />
                  <span>{tab.name}</span>
                  {tab.dirty && <span className="dirty-dot" />}
                  <span className="tab-close" onClick={(event) => { event.stopPropagation(); closeTab(tab.path); }}><X size={13} /></span>
                </button>
              ))}
              {tabs.length === 0 && <div className="empty-tab-label">EDITOR</div>}
            </div>
            <div className="tab-actions">
              {activeTab && <button className="subtle-icon" onClick={() => void saveActiveFile()} title="Guardar (Ctrl+S)"><Check size={15} /></button>}
            </div>
          </div>

          {activeTab ? (
            <section className="editor-area">
              <div className="breadcrumb">
                <span>{workspace?.split(/[\\\\/]/).filter(Boolean).pop()}</span>
                <ChevronRight size={12} />
                <span>{activeTab.path.split('/').join('  ›  ')}</span>
              </div>
              <div className="code-editor">
                <div className="line-gutter" aria-hidden="true">
                  {Array.from({ length: lineCount }, (_, index) => <div key={index}>{index + 1}</div>)}
                </div>
                <textarea
                  className="code-input"
                  value={activeTab.content}
                  onChange={(event) => updateActiveContent(event.target.value)}
                  spellCheck={false}
                  autoCapitalize="off"
                  autoCorrect="off"
                  aria-label="Editor de texto"
                />
              </div>
            </section>
          ) : (
            <section className="welcome-screen">
              <div className="welcome-glow" />
              <div className="welcome-content">
                <div className="welcome-emblem"><Command size={34} strokeWidth={1.7} /></div>
                <div className="eyebrow">ATDEV DEV <span>•</span> WORKSPACE</div>
                <h1>Build beyond<br /><span>the operating system.</span></h1>
                <p className="welcome-copy">Un entorno de desarrollo para diseñar, construir y explorar sistemas operativos con Void Memingo.</p>
                <div className="welcome-actions">
                  <button className="primary-button" onClick={() => void openWorkspace()} disabled={busy}>
                    <FolderOpen size={16} /> {busy ? 'Abriendo…' : 'Abrir carpeta'}
                  </button>
                  <button className="secondary-button" onClick={() => void createFile()}>
                    <Plus size={16} /> Nuevo archivo
                  </button>
                </div>
                <div className="welcome-divider" />
                <div className="feature-grid">
                  <div className="feature-item"><Code2 size={17} /><div><b>Editor de código</b><span>Edición local y guardado real</span></div></div>
                  <div className="feature-item"><ShieldCheck size={17} /><div><b>Workspace seguro</b><span>Acceso limitado a la carpeta abierta</span></div></div>
                  <div className="feature-item"><Box size={17} /><div><b>Proyectos de sistema</b><span>Base para herramientas OS-dev</span></div></div>
                  <div className="feature-item"><Zap size={17} /><div><b>Rust nativo</b><span>Motor local multiplataforma</span></div></div>
                </div>
                <div className="welcome-version">VOID MEMINGO DESKTOP <span>v{version}</span></div>
              </div>
            </section>
          )}

          {bottomOpen && (
            <section className="bottom-panel">
              <div className="panel-tabs">
                <button className={bottomView === 'problems' ? 'selected' : ''} onClick={() => setBottomView('problems')}>PROBLEMAS <span className="count-pill">0</span></button>
                <button className={bottomView === 'output' ? 'selected' : ''} onClick={() => setBottomView('output')}>SALIDA</button>
                <button className={bottomView === 'terminal' ? 'selected' : ''} onClick={() => setBottomView('terminal')}>TERMINAL</button>
                <div className="panel-spacer" />
                <button className="subtle-icon" onClick={() => setBottomOpen(false)} title="Cerrar panel"><X size={14} /></button>
              </div>
              <div className="panel-content">
                {bottomView === 'problems' && (
                  <div className="panel-empty"><Check size={16} /> No hay problemas detectados por el editor.</div>
                )}
                {bottomView === 'output' && (
                  <div className="output-list">
                    {logs.map((line, index) => <div key={index} className="output-line">{line}</div>)}
                  </div>
                )}
                {bottomView === 'terminal' && (
                  <div className="terminal-placeholder">
                    <TerminalSquare size={18} />
                    <div><b>Terminal integrada</b><span>La ejecución de procesos y la terminal interactiva se incorporarán en el siguiente módulo. Esta vista todavía no ejecuta comandos.</span></div>
                  </div>
                )}
              </div>
            </section>
          )}
        </main>
      </div>

      <footer className="statusbar">
        <div className="status-left">
          <span className="status-branch"><GitBranch size={12} /> main</span>
          <span><AlertCircle size={12} /> 0</span>
          <span><Check size={12} /> Workspace local</span>
        </div>
        <div className="status-right">
          {activeTab && <><span>Ln {lineCount}, Col 1</span><span>UTF-8</span><span>{language}</span></>}
          <span className="status-brand"><Command size={12} /> VOID {version}</span>
        </div>
      </footer>
    </div>
  );
}

export default App;