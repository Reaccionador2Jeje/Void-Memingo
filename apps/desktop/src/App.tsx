import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import TerminalPanel from './components/TerminalPanel';
import { open } from '@tauri-apps/plugin-dialog';
import {
  Activity, AlertCircle, Box, Check, ChevronDown, ChevronRight, CircleHelp,
  Code2, Command, Cpu, FileCode2, FilePlus2, Folder, FolderOpen,
  GitBranch, LayoutDashboard, PanelBottom, PanelLeftClose, Plus, Search,
  Settings, ShieldCheck, X, Zap, Layers3, HardDrive, Monitor, Disc3, Play, Cpu, FolderOpen
} from 'lucide-react';

type FileEntry = {
  name: string;
  path: string;
  isDir: boolean;
  size: number | null;
};

type WorkspaceSearchMatch = { path: string; lineNumber: number; column: number; preview: string };
type DiagnosticReport = { supported: boolean; language: string; diagnostics: { severity: string; message: string; line: number; column: number }[] };

type OpenTab = {
  path: string;
  name: string;
  content: string;
  dirty: boolean;
};

type SidebarView = 'explorer' | 'search' | 'source' | 'boot';
type BootIsoEntry = { name: string; path: string; size_bytes: number; sha256: string; iso9660: boolean; el_torito_descriptor: boolean; status: string };
type BootDownload = { id: string; file_name: string; path: string; downloaded_bytes: number; total_bytes: number | null; connections: number; status: string; error: string | null };
type BootDistribution = { id: string; name: string; description: string; family: string; project_url: string; download_page: string; architectures: string[]; editions: string[]; release_model: string; formats: string[]; verification: string; installation: { delivery: string; requires_full_image: boolean; boot_profile_required: boolean; note: string } };
type BottomView = 'problems' | 'output' | 'terminal';

type OsBase = {
  id: string;
  name: string;
  backend: string;
  backendStatus: string;
  releases: string[];
  architectures: string[];
  desktops: string[];
  bootModes: string[];
  notes: string;
};

type GitRemote = { name: string; url: string; provider: string };
type GitSnapshot = {
  initialized: boolean;
  branch: string;
  status: string;
  stagedSummary: string;
  remotes: GitRemote[];
  branches: string[];
  recentCommits: string[];
};

type OsValidation = { valid: boolean; errors: string[]; warnings: string[] };
type BuildPreflight = { supportedHost: boolean; ready: boolean; distribution: string; checks: { id: string; label: string; available: boolean; required: boolean; detail: string }[]; warnings: string[] };
type BuildPreparation = { backend: string; buildDirectory: string; scriptPath: string; bootTestScriptPath?: string | null; uefiBootTestScriptPath?: string | null; commands: string[]; notes: string[] };
type BuildArtifactReport = { valid: boolean; isoPath: string; sizeBytes: number; sha256: string | null; backend: string | null; checks: { id: string; label: string; passed: boolean; detail: string }[] };
type BuildJobRecord = { id: string; backend: string; status: 'queued' | 'running' | 'succeeded' | 'failed' | 'cancelled'; stage: string; startedAt: number; finishedAt: number | null; exitCode: number | null };

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
  const [searchQuery, setSearchQuery] = useState('');
  const [searchCaseSensitive, setSearchCaseSensitive] = useState(false);
  const [searchBusy, setSearchBusy] = useState(false);
  const [searchResults, setSearchResults] = useState<WorkspaceSearchMatch[]>([]);
  const [diagnosticReport, setDiagnosticReport] = useState<DiagnosticReport | null>(null);
  const [diagnosticsBusy, setDiagnosticsBusy] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [settingsReady, setSettingsReady] = useState(false);
  const [editorFontSize, setEditorFontSize] = useState(13);
  const [editorWordWrap, setEditorWordWrap] = useState(true);
  const [editorTabSize, setEditorTabSize] = useState(2);
  const [sidebarView, setSidebarView] = useState<SidebarView>('explorer');
  const [bottomView, setBottomView] = useState<BottomView>('output');
  const [bottomOpen, setBottomOpen] = useState(true);
  const [terminalMounted, setTerminalMounted] = useState(false);
  const [terminalCommand, setTerminalCommand] = useState<string | null>(null);
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [voidBootDirectory, setVoidBootDirectory] = useState<string | null>(null);
  const [voidBootCatalog, setVoidBootCatalog] = useState<BootDistribution[]>([]);
  const [voidBootCatalogBusy, setVoidBootCatalogBusy] = useState(false);
  const [voidBootSelectedDistro, setVoidBootSelectedDistro] = useState<BootDistribution | null>(null);
  const [voidBootDownloadUrl, setVoidBootDownloadUrl] = useState('');
  const [voidBootDownloadName, setVoidBootDownloadName] = useState('distribution.iso');
  const [voidBootDownloadDirectory, setVoidBootDownloadDirectory] = useState<string | null>(null);
  const [voidBootDownloads, setVoidBootDownloads] = useState<BootDownload[]>([]);
  const [voidBootDownloadBusy, setVoidBootDownloadBusy] = useState(false);
  const [voidBootEntries, setVoidBootEntries] = useState<BootIsoEntry[]>([]);
  const [voidBootBusy, setVoidBootBusy] = useState(false);
  const [voidBootProfileFile, setVoidBootProfileFile] = useState<string | null>(null);
  const [voidBootMenuPreview, setVoidBootMenuPreview] = useState<string | null>(null);
  const [voidBootPreviewBusy, setVoidBootPreviewBusy] = useState(false);
  const [qemuImagePath, setQemuImagePath] = useState<string | null>(null);
  const [qemuBusy, setQemuBusy] = useState(false);
  const [qemuStatus, setQemuStatus] = useState<string | null>(null);
  const [logs, setLogs] = useState<string[]>(['Void Memingo Desktop iniciado.']);
  const [busy, setBusy] = useState(false);
  const [osBuilderOpen, setOsBuilderOpen] = useState(false);
  const [osBases, setOsBases] = useState<OsBase[]>([]);
  const [osBaseId, setOsBaseId] = useState('debian');
  const [osProjectName, setOsProjectName] = useState('My OS');
  const [osProjectId, setOsProjectId] = useState('my-os');
  const [osProjectVersion, setOsProjectVersion] = useState('0.1.0');
  const [osProjectDescription, setOsProjectDescription] = useState('A custom operating system built with Void Memingo');
  const [osPublisher, setOsPublisher] = useState('');
  const [osIsoLabel, setOsIsoLabel] = useState('MY_OS');
  const [osRelease, setOsRelease] = useState('stable');
  const [osDesktop, setOsDesktop] = useState('xfce');
  const [osTheme, setOsTheme] = useState('');
  const [osIconTheme, setOsIconTheme] = useState('');
  const [osWallpaper, setOsWallpaper] = useState('');
  const [osPackages, setOsPackages] = useState('');
  const [osOverlay, setOsOverlay] = useState('filesystem');
  const [osLocale, setOsLocale] = useState('');
  const [osTimezone, setOsTimezone] = useState('');
  const [osHostname, setOsHostname] = useState('my-os');
  const [osServices, setOsServices] = useState('');
  const [osStorageFs, setOsStorageFs] = useState('ext4');
  const [osOutputDir, setOsOutputDir] = useState('dist');
  const [osIsoName, setOsIsoName] = useState('atdev-os-0.1.0.iso');
  const [osValidation, setOsValidation] = useState<OsValidation | null>(null);
  const [buildPreparation, setBuildPreparation] = useState<BuildPreparation | null>(null);
  const [buildPreflight, setBuildPreflight] = useState<BuildPreflight | null>(null);
  const [artifactReport, setArtifactReport] = useState<BuildArtifactReport | null>(null);
  const [osBusy, setOsBusy] = useState(false);
  const [buildRunning, setBuildRunning] = useState(false);
  const [buildJobs, setBuildJobs] = useState<BuildJobRecord[]>([]);
  const [activeBuildJobId, setActiveBuildJobId] = useState<string | null>(null);
  const [buildStage, setBuildStage] = useState('idle');
  const [selectedBuildLog, setSelectedBuildLog] = useState<string | null>(null);
  const activeBuildJobRef = useRef<string | null>(null);
  const activeBuildTerminalSessionRef = useRef<string | null>(null);
  const buildStageRef = useRef('idle');
  const [terminalInterruptSignal, setTerminalInterruptSignal] = useState(0);
  const [gitSnapshot, setGitSnapshot] = useState<GitSnapshot | null>(null);
  const [gitBusy, setGitBusy] = useState(false);
  const [gitCommitMessage, setGitCommitMessage] = useState('');
  const [gitBranchName, setGitBranchName] = useState('');
  const [gitRemoteName, setGitRemoteName] = useState('origin');
  const [gitRemoteUrl, setGitRemoteUrl] = useState('');

  useEffect(() => {
    try {
      const stored = window.localStorage.getItem('void-memingo.editor-settings');
      if (stored) {
        const parsed = JSON.parse(stored) as { fontSize?: number; wordWrap?: boolean; tabSize?: number };
        if (typeof parsed.fontSize === 'number') setEditorFontSize(Math.min(24, Math.max(10, parsed.fontSize)));
        if (typeof parsed.wordWrap === 'boolean') setEditorWordWrap(parsed.wordWrap);
        if (parsed.tabSize === 2 || parsed.tabSize === 4 || parsed.tabSize === 8) setEditorTabSize(parsed.tabSize);
      }
    } catch {
      // Ignore invalid local settings and keep safe defaults.
    }
    setSettingsReady(true);
  }, []);

  useEffect(() => {
    if (!settingsReady) return;
    try {
      window.localStorage.setItem('void-memingo.editor-settings', JSON.stringify({
        fontSize: editorFontSize,
        wordWrap: editorWordWrap,
        tabSize: editorTabSize
      }));
    } catch {
      // Settings remain available for this session if storage is unavailable.
    }
  }, [editorFontSize, editorTabSize, editorWordWrap, settingsReady]);

  useEffect(() => {
    const handleEditorShortcut = (event: KeyboardEvent) => {
      if (!(event.ctrlKey || event.metaKey)) return;
      if (event.shiftKey && event.key.toLowerCase() === 'f') {
        event.preventDefault();
        setOsBuilderOpen(false);
        setSidebarView('search');
        setSidebarOpen(true);
      } else if (event.key === ',') {
        event.preventDefault();
        setSettingsOpen(true);
      }
    };
    window.addEventListener('keydown', handleEditorShortcut);
    return () => window.removeEventListener('keydown', handleEditorShortcut);
  }, []);

  const selectedOsBase = osBases.find((base) => base.id === osBaseId) ?? null;
  const isLinuxHost = typeof navigator !== 'undefined' && /Linux/i.test(navigator.userAgent);

  const activeTab = useMemo(
    () => tabs.find((tab) => tab.path === activePath) ?? null,
    [tabs, activePath]
  );

  const addLog = useCallback((message: string) => {
    const stamp = new Date().toLocaleTimeString();
    setLogs((current) => [...current.slice(-99), '[' + stamp + '] ' + message]);
  }, []);

  const refreshBuildJobs = useCallback(async () => {
    if (!workspace) {
      setBuildJobs([]);
      return;
    }
    try {
      setBuildJobs(await invoke<BuildJobRecord[]>('list_os_build_jobs', { workspace }));
    } catch (error) {
      addLog('No se pudo cargar el historial de compilaciones: ' + String(error));
    }
  }, [addLog, workspace]);

  useEffect(() => { void refreshBuildJobs(); }, [refreshBuildJobs]);

  const handleTerminalCommandStarted = useCallback((sessionId: string) => {
    setBuildRunning(true);
    activeBuildTerminalSessionRef.current = sessionId;
    const jobId = activeBuildJobRef.current;
    if (jobId && workspace) {
      buildStageRef.current = 'building';
      setBuildStage('building');
      void invoke<BuildJobRecord>('update_os_build_job', {
        workspace, jobId, status: 'running', stage: 'building', exitCode: null
      }).then(() => refreshBuildJobs()).catch((error) => addLog('No se pudo actualizar el trabajo de build: ' + String(error)));
    }
  }, [addLog, refreshBuildJobs, workspace]);

  const handleTerminalCommandOutput = useCallback((data: string, sessionId: string) => {
    const jobId = activeBuildJobRef.current;
    if (!jobId || !workspace || activeBuildTerminalSessionRef.current !== sessionId) return;
    void invoke('append_os_build_job_log', { workspace, jobId, data }).catch(() => undefined);
    const output = data.toLowerCase();
    let nextStage = buildStageRef.current;
    const stageMarker = output.match(/void_memingo_stage:([a-z-]+)/);
    if (stageMarker) nextStage = stageMarker[1];
    else if (/lb clean|cleaning|clean workspace|limpiando/.test(output)) nextStage = 'cleaning';
    else if (/lb config|configuring|configuración/.test(output)) nextStage = 'configuring';
    else if (/debootstrap|pacstrap|installing packages|instalando paquetes|apt-get install|pacman -/.test(output)) nextStage = 'installing-packages';
    else if (/mksquashfs|squashfs|filesystem.squashfs/.test(output)) nextStage = 'building-filesystem';
    else if (/creating iso|generating iso|creando iso|copying image|copiando imagen/.test(output)) nextStage = 'creating-image';
    else if (/xorriso|iso 9660|el torito|sha256|checksum|validating|verifying artifact|verificando/.test(output)) nextStage = 'validating';
    else if (/sha256|checksum|validating|verifying artifact|verificando/.test(output)) nextStage = 'validating';
    if (nextStage !== buildStageRef.current) {
      buildStageRef.current = nextStage;
      setBuildStage(nextStage);
      void invoke('update_os_build_job', {
        workspace, jobId, status: 'running', stage: nextStage, exitCode: null
      }).then(() => refreshBuildJobs()).catch(() => undefined);
    }
  }, [refreshBuildJobs, workspace]);

  const handleTerminalCommandFinished = useCallback(async (exitCode: number, sessionId: string) => {
    if (activeBuildTerminalSessionRef.current !== sessionId) return;
    activeBuildTerminalSessionRef.current = null;
    setBuildRunning(false);
    const jobId = activeBuildJobRef.current;
    addLog(exitCode === 0
      ? (jobId ? 'Proceso de compilación finalizado; verificando el artefacto ISO.' : 'Proceso de terminal finalizado correctamente.')
      : 'Proceso finalizado con código ' + exitCode + '.');
    if (jobId && workspace) {
      let finalStatus = exitCode === 0 ? 'succeeded' : (exitCode === 130 || exitCode === 143 ? 'cancelled' : 'failed');
      let finalStage = finalStatus === 'succeeded' ? 'completed' : finalStatus;
      let finalExitCode = exitCode;
      if (exitCode === 0) {
        buildStageRef.current = 'verifying-artifact';
        setBuildStage('verifying-artifact');
        try {
          const projectToml = await invoke<string>('read_text_file', { root: workspace, relativePath: 'void-os.toml' });
          const report = await invoke<BuildArtifactReport>('validate_os_build_artifact', { workspace, projectToml });
          setArtifactReport(report);
          if (!report.valid) {
            finalStatus = 'failed';
            finalStage = 'artifact-verification-failed';
            finalExitCode = 1;
            addLog('La compilación terminó, pero la verificación del ISO falló. Revisa el informe de artefacto.');
          } else {
            addLog('ISO verificado automáticamente: checksum, formato, arranque y manifiesto correctos.');
          }
        } catch (error) {
          setArtifactReport(null);
          finalStatus = 'failed';
          finalStage = 'artifact-verification-failed';
          finalExitCode = 1;
          addLog('No se pudo verificar automáticamente el ISO: ' + String(error));
        }
      }
      buildStageRef.current = finalStage;
      setBuildStage(finalStage);
      try {
        await invoke<BuildJobRecord>('update_os_build_job', {
          workspace, jobId, status: finalStatus, stage: finalStage, exitCode: finalExitCode
        });
        await refreshBuildJobs();
      } catch (error) {
        addLog('No se pudo cerrar el registro del build: ' + String(error));
      }
      activeBuildJobRef.current = null;
      setActiveBuildJobId(null);
    }
  }, [addLog, refreshBuildJobs, workspace]);

  const handleTerminalCommandConsumed = useCallback(() => {
    setTerminalCommand(null);
  }, []);

  const cancelOsBuild = useCallback(() => {
    if (!buildRunning) return;
    if (!window.confirm('Enviar Ctrl+C a la terminal activa para interrumpir el proceso?')) return;
    setTerminalInterruptSignal((current) => current + 1);
    const jobId = activeBuildJobRef.current;
    if (jobId && workspace) {
      buildStageRef.current = 'cancelling';
      setBuildStage('cancelling');
      void invoke('update_os_build_job', { workspace, jobId, status: 'running', stage: 'cancelling', exitCode: null }).catch(() => undefined);
    }
    addLog('Solicitud de interrupción enviada.');
  }, [addLog, buildRunning, workspace]);

  const refreshGitSnapshot = useCallback(async (root: string) => {
    const snapshot = await invoke<GitSnapshot>('git_snapshot', { root });
    setGitSnapshot(snapshot);
    return snapshot;
  }, []);

  const runGitAction = useCallback(async (action: () => Promise<GitSnapshot>, message: string) => {
    if (!workspace) {
      addLog('Abre una carpeta antes de usar Git.');
      return;
    }
    setGitBusy(true);
    try {
      setGitSnapshot(await action());
      addLog(message);
    } catch (error) {
      addLog('Git: ' + String(error));
    } finally {
      setGitBusy(false);
    }
  }, [addLog, workspace]);

  const initializeGit = useCallback(() => runGitAction(
    () => invoke<GitSnapshot>('git_init', { root: workspace! }),
    'Repositorio Git inicializado.'
  ), [runGitAction, workspace]);

  const stageGitChanges = useCallback(() => runGitAction(
    () => invoke<GitSnapshot>('git_stage_all', { root: workspace! }),
    'Cambios preparados (stage all).'
  ), [runGitAction, workspace]);

  const commitGitChanges = useCallback(() => {
    if (!gitCommitMessage.trim()) return;
    void runGitAction(
      () => invoke<GitSnapshot>('git_commit', { root: workspace!, message: gitCommitMessage }),
      'Commit creado.'
    ).then(() => setGitCommitMessage(''));
  }, [gitCommitMessage, runGitAction, workspace]);

  const addGitRemote = useCallback(() => {
    if (!gitRemoteUrl.trim()) return;
    void runGitAction(
      () => invoke<GitSnapshot>('git_add_remote', { root: workspace!, name: gitRemoteName, url: gitRemoteUrl }),
      'Remote ' + gitRemoteName + ' añadido.'
    ).then(() => setGitRemoteUrl(''));
  }, [gitRemoteName, gitRemoteUrl, runGitAction, workspace]);

  const createGitBranch = useCallback(() => {
    if (!gitBranchName.trim()) return;
    void runGitAction(
      () => invoke<GitSnapshot>('git_create_branch', { root: workspace!, branch: gitBranchName }),
      'Rama creada: ' + gitBranchName
    ).then(() => setGitBranchName(''));
  }, [gitBranchName, runGitAction, workspace]);

  const checkoutGitBranch = useCallback((branch: string) => runGitAction(
    () => invoke<GitSnapshot>('git_checkout_branch', { root: workspace!, branch }),
    'Rama activa: ' + branch
  ), [runGitAction, workspace]);

  const fetchGitRemotes = useCallback(() => runGitAction(
    () => invoke<GitSnapshot>('git_fetch_all', { root: workspace! }),
    'Fetch completado.'
  ), [runGitAction, workspace]);

  const pullGitChanges = useCallback(() => runGitAction(
    () => invoke<GitSnapshot>('git_pull_ff_only', { root: workspace! }),
    'Pull completado (fast-forward).'
  ), [runGitAction, workspace]);

  const pushGitChanges = useCallback(() => runGitAction(
    () => invoke<GitSnapshot>('git_push', { root: workspace! }),
    'Push completado.'
  ), [runGitAction, workspace]);

  useEffect(() => {
    invoke<string>('app_version').then(setVersion).catch(() => undefined);
    invoke<OsBase[]>('list_os_base_capabilities')
      .then(setOsBases)
      .catch((error) => addLog('No se pudieron cargar las bases OS: ' + String(error)));
  }, [addLog]);

  useEffect(() => {
    if (!workspace) {
      setGitSnapshot(null);
      return;
    }
    refreshGitSnapshot(workspace).catch((error) => {
      setGitSnapshot(null);
      addLog('No se pudo consultar Git: ' + String(error));
    });
  }, [addLog, refreshGitSnapshot, workspace]);

  const validateOsProject = useCallback(async () => {
    if (!workspace) {
      addLog('Abre primero la carpeta del proyecto OS.');
      return;
    }
    setOsBusy(true);
    try {
      const projectToml = await invoke<string>('read_text_file', { root: workspace, relativePath: 'void-os.toml' });
      const report = await invoke<OsValidation>('validate_os_project', { projectToml });
      setOsValidation(report);
      addLog(report.valid ? 'Configuración OS válida.' : 'La configuración OS contiene errores.');
    } catch (error) {
      setOsValidation({ valid: false, errors: [String(error)], warnings: [] });
      addLog('Error al validar el proyecto OS: ' + String(error));
    } finally {
      setOsBusy(false);
    }
  }, [addLog, workspace]);

  const loadVoidBootCatalog = useCallback(async () => {
    setVoidBootCatalogBusy(true);
    try {
      const entries = await invoke<BootDistribution[]>('list_void_boot_distributions');
      setVoidBootCatalog(entries);
      addLog(`Void Boot Distribution Center: ${entries.length} distribuciones disponibles.`);
    } catch (error) {
      addLog('No se pudo cargar el Distribution Center: ' + String(error));
    } finally {
      setVoidBootCatalogBusy(false);
    }
  }, [addLog]);

  const scanVoidBootDirectory = useCallback(async (directory: string) => {
    setVoidBootBusy(true);
    try {
      const entries = await invoke<BootIsoEntry[]>('scan_void_boot_iso_directory', { directory });
      setVoidBootDirectory(directory);
      setVoidBootEntries(entries);
      addLog(`Void Boot: ${entries.length} ISO(s) inspeccionadas.`);
    } catch (error) {
      setVoidBootEntries([]);
      addLog('No se pudo inspeccionar la carpeta de ISOs: ' + String(error));
    } finally {
      setVoidBootBusy(false);
    }
  }, [addLog]);

  const chooseQemuImage = useCallback(async () => {
    const selected = await open({ multiple: false, title: 'Seleccionar imagen virtual Void Boot', filters: [{ name: 'Imágenes de disco', extensions: ['img', 'raw', 'qcow2'] }] });
    if (typeof selected === 'string') { setQemuImagePath(selected); setQemuStatus(null); }
  }, []);

  const launchVoidBootQemu = useCallback(async () => {
    if (!qemuImagePath) return;
    setQemuBusy(true);
    try {
      const pid = await invoke<number>('start_void_boot_qemu', { imagePath: qemuImagePath });
      setQemuStatus('QEMU iniciado · PID ' + pid + ' · modo snapshot');
      addLog('Void Boot QEMU iniciado (PID ' + pid + '). El archivo original está protegido por snapshot.');
    } catch (error) {
      setQemuStatus('No se pudo iniciar QEMU: ' + String(error));
      addLog('Void Boot QEMU: ' + String(error));
    } finally { setQemuBusy(false); }
  }, [addLog, qemuImagePath]);

  const chooseVoidBootDirectory = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false, title: 'Seleccionar carpeta que contiene ISOs' });
    if (typeof selected === 'string') await scanVoidBootDirectory(selected);
  }, [scanVoidBootDirectory]);

  const refreshVoidBootDownloads = useCallback(async () => {
    try {
      const tasks = await invoke<BootDownload[]>('list_void_boot_downloads');
      setVoidBootDownloads(tasks);
    } catch (error) {
      addLog('No se pudo consultar el gestor de descargas: ' + String(error));
    }
  }, [addLog]);

  useEffect(() => {
    const active = voidBootDownloads.some((task) => task.status === 'downloading' || task.status === 'queued');
    if (!active) return;
    const timer = window.setInterval(() => { void refreshVoidBootDownloads(); }, 1200);
    return () => window.clearInterval(timer);
  }, [voidBootDownloads, refreshVoidBootDownloads]);

  const chooseVoidBootDownloadDirectory = useCallback(async () => {
    const selected = await open({ directory: true, multiple: false, title: 'Carpeta destino (VOIDDATA montada o biblioteca local)' });
    if (typeof selected === 'string') setVoidBootDownloadDirectory(selected);
  }, []);

  const startVoidBootDownload = useCallback(async () => {
    if (!voidBootDownloadUrl.trim() || !voidBootDownloadDirectory || !voidBootDownloadName.trim()) return;
    setVoidBootDownloadBusy(true);
    try {
      await invoke<string>('start_void_boot_download', {
        url: voidBootDownloadUrl.trim(),
        fileName: voidBootDownloadName.trim(),
        directory: voidBootDownloadDirectory
      });
      addLog('Void Boot: descarga iniciada. El archivo .part permite reanudarla si se pausa o se interrumpe.');
      await refreshVoidBootDownloads();
    } catch (error) {
      addLog('No se pudo iniciar la descarga: ' + String(error));
    } finally {
      setVoidBootDownloadBusy(false);
    }
  }, [addLog, refreshVoidBootDownloads, voidBootDownloadDirectory, voidBootDownloadName, voidBootDownloadUrl]);

  const pauseVoidBootDownload = useCallback(async (id: string) => {
    try {
      await invoke('pause_void_boot_download', { id });
      await refreshVoidBootDownloads();
    } catch (error) {
      addLog('No se pudo pausar la descarga: ' + String(error));
    }
  }, [addLog, refreshVoidBootDownloads]);


  const chooseVoidBootProfile = useCallback(async () => {
    const selected = await open({ directory: false, multiple: false, title: 'Seleccionar catálogo de perfiles Void Boot' });
    if (typeof selected !== 'string') return;
    setVoidBootProfileFile(selected);
    setVoidBootPreviewBusy(true);
    try {
      const preview = await invoke<string>('preview_void_boot_menu', { profileFile: selected });
      setVoidBootMenuPreview(preview);
      addLog('Void Boot: vista previa del menú GRUB generada.');
    } catch (error) {
      setVoidBootMenuPreview(null);
      addLog('No se pudo generar la vista previa de Void Boot: ' + String(error));
    } finally {
      setVoidBootPreviewBusy(false);
    }
  }, [addLog]);

  const refreshVoidBootPreview = useCallback(async () => {
    if (!voidBootProfileFile) return;
    setVoidBootPreviewBusy(true);
    try {
      const preview = await invoke<string>('preview_void_boot_menu', { profileFile: voidBootProfileFile });
      setVoidBootMenuPreview(preview);
      addLog('Void Boot: vista previa actualizada.');
    } catch (error) {
      setVoidBootMenuPreview(null);
      addLog('Error al actualizar la vista previa: ' + String(error));
    } finally {
      setVoidBootPreviewBusy(false);
    }
  }, [addLog, voidBootProfileFile]);

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
      setSearchResults([]);
      addLog('Workspace abierto: ' + selected);
    } catch (error) {
      addLog('Error al abrir workspace: ' + String(error));
    } finally {
      setBusy(false);
    }
  }, [addLog, refreshWorkspace]);

  const createOsProject = useCallback(async () => {
    if (!workspace) {
      await openWorkspace();
      return;
    }
    const packageList = osPackages.split(',').map((item) => item.trim()).filter(Boolean);
    const quote = (value: string) => JSON.stringify(value);
    const template = `[project]
name = ${quote(osProjectName)}
id = ${quote(osProjectId)}
version = ${quote(osProjectVersion)}
publisher = ${quote(osPublisher)}
description = ${quote(osProjectDescription)}
iso_label = ${quote(osIsoLabel)}

[base]
distribution = ${quote(osBaseId)}
release = ${quote(osRelease)}
architecture = "${osBaseId === 'arch' ? 'x86_64' : 'amd64'}"

[packages]
install = [${packageList.map(quote).join(', ')}]
remove = []

[filesystem]
overlay = ${quote(osOverlay)}
locale = ${quote(osLocale)}
timezone = ${quote(osTimezone)}
hostname = ${quote(osHostname)}
storage_filesystem = ${quote(osStorageFs)}
services_enabled = [${osServices.split(',').map((item) => item.trim()).filter(Boolean).map(quote).join(', ')}]

[desktop]
environment = ${quote(osDesktop)}
display_manager = ${quote(osDesktop === 'none' ? '' : osBaseId === 'arch' ? (osDesktop === 'gnome' ? 'gdm' : osDesktop === 'kde-plasma' ? 'sddm' : 'lightdm') : (osDesktop === 'gnome' ? 'gdm3' : osDesktop === 'kde-plasma' ? 'sddm' : 'lightdm'))}
applications = []
theme = ${quote(osTheme)}
icon_theme = ${quote(osIconTheme)}
wallpaper = ${quote(osWallpaper)}

[boot]
uefi = true
bios = true
live_session = true
installer = false

[output]
iso_name = ${quote(osIsoName)}
directory = ${quote(osOutputDir)}
`;
    try {
      const rootEntries = await invoke<FileEntry[]>('list_directory', { root: workspace, relativePath: '' });
      if (rootEntries.some((entry) => entry.name === 'void-os.toml' && !entry.isDir)) {
        const confirmed = window.confirm('void-os.toml ya existe. ¿Quieres reemplazarlo con los valores actuales del formulario?');
        if (!confirmed) return;
      }
    } catch (error) {
      addLog('No se pudo comprobar el manifiesto existente: ' + String(error));
      return;
    }
    setOsBusy(true);
    try {
      await invoke('write_text_file', { root: workspace, relativePath: 'void-os.toml', contents: template });
      await refreshWorkspace(workspace);
      addLog('Proyecto OS creado: void-os.toml');
      setOsValidation(null);
    } catch (error) {
      addLog('No se pudo crear void-os.toml: ' + String(error));
    } finally {
      setOsBusy(false);
    }
  }, [addLog, openWorkspace, osBaseId, osDesktop, osHostname, osIconTheme, osIsoLabel, osIsoName, osLocale, osOutputDir, osOverlay, osPackages, osProjectDescription, osProjectId, osProjectName, osProjectVersion, osPublisher, osRelease, osServices, osStorageFs, osTheme, osTimezone, osWallpaper, refreshWorkspace, workspace]);

  const cloneGitRepository = useCallback(async () => {
    const url = window.prompt('URL Git (GitHub, GitLab o servidor compatible):');
    if (!url?.trim()) return;
    const parent = await open({
      directory: true,
      multiple: false,
      title: 'Seleccionar carpeta donde clonar'
    });
    if (typeof parent !== 'string') return;
    const suggested = url.trim().split(/[/:]/).pop()?.replace(/\.git$/i, '') || 'repository';
    const folderName = window.prompt('Nombre de la carpeta del repositorio:', suggested);
    if (!folderName?.trim()) return;
    setGitBusy(true);
    try {
      const clonedPath = await invoke<string>('git_clone_repository', {
        destinationParent: parent,
        url: url.trim(),
        folderName: folderName.trim()
      });
      await refreshWorkspace(clonedPath);
      setWorkspace(clonedPath);
      setTabs([]);
      setActivePath(null);
      setSidebarView('source');
      setSidebarOpen(true);
      addLog('Repositorio clonado: ' + clonedPath);
    } catch (error) {
      addLog('No se pudo clonar el repositorio: ' + String(error));
    } finally {
      setGitBusy(false);
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

  const runWorkspaceSearch = useCallback(async () => {
    if (!workspace || !searchQuery.trim()) return;
    setSearchBusy(true);
    try {
      const results = await invoke<WorkspaceSearchMatch[]>('search_workspace', {
        root: workspace,
        query: searchQuery.trim(),
        caseSensitive: searchCaseSensitive
      });
      setSearchResults(results);
      addLog(`Búsqueda completada: ${results.length} coincidencias.`);
    } catch (error) {
      setSearchResults([]);
      addLog('Error en búsqueda global: ' + String(error));
    } finally {
      setSearchBusy(false);
    }
  }, [addLog, searchCaseSensitive, searchQuery, workspace]);

  const openSearchResult = useCallback((result: WorkspaceSearchMatch) => {
    const name = result.path.split('/').pop() || result.path;
    void openFile({ name, path: result.path, isDir: false, size: null });
  }, [openFile]);

  const runFileDiagnostics = useCallback(async () => {
    if (!workspace || !activeTab) return;
    setDiagnosticsBusy(true);
    try {
      const report = await invoke<DiagnosticReport>('diagnose_text_file', {
        root: workspace,
        relativePath: activeTab.path
      });
      setDiagnosticReport(report);
      addLog(report.supported
        ? report.diagnostics.length
          ? `Diagnóstico de ${activeTab.path}: ${report.diagnostics.length} problema(s).`
          : `Sin errores de sintaxis en ${activeTab.path}.`
        : `No hay analizador integrado para ${report.language}.`);
    } catch (error) {
      setDiagnosticReport(null);
      addLog('No se pudo analizar el archivo: ' + String(error));
    } finally {
      setDiagnosticsBusy(false);
    }
  }, [activeTab, addLog, workspace]);

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
      const path = name.trim().replaceAll('\\', '/');
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

  const checkOsBuildPreflight = useCallback(async (projectToml?: string) => {
    if (!workspace) {
      addLog('Abre primero la carpeta del proyecto OS.');
      return null;
    }
    setOsBusy(true);
    try {
      const manifest = projectToml ?? await invoke<string>('read_text_file', { root: workspace, relativePath: 'void-os.toml' });
      const report = await invoke<BuildPreflight>('check_os_build_preflight', { workspace, projectToml: manifest });
      setBuildPreflight(report);
      addLog(report.ready ? 'Preflight completado: dependencias obligatorias disponibles.' : 'Preflight incompleto: revisa las dependencias requeridas.');
      return report;
    } catch (error) {
      setBuildPreflight(null);
      addLog('No se pudo comprobar el entorno de build: ' + String(error));
      return null;
    } finally {
      setOsBusy(false);
    }
  }, [addLog, workspace]);

  const prepareOsBuild = useCallback(async () => {
    if (!workspace) return;
    setOsBusy(true);
    setArtifactReport(null);
    try {
      const projectToml = await invoke<string>('read_text_file', { root: workspace, relativePath: 'void-os.toml' });
      const result = await invoke<BuildPreparation>('prepare_os_build_project', { workspace, projectToml });
      setBuildPreparation(result);
      addLog(result.backend + ' preparado en ' + result.buildDirectory);
      const preflight = await invoke<BuildPreflight>('check_os_build_preflight', { workspace, projectToml });
      setBuildPreflight(preflight);
      addLog(preflight.ready ? 'Dependencias obligatorias verificadas.' : 'Faltan dependencias obligatorias para ejecutar el build.');
    } catch (error) {
      addLog('No se pudo preparar el backend: ' + String(error));
      setBuildPreparation(null);
    } finally {
      setOsBusy(false);
    }
  }, [addLog, workspace]);

  const validateOsBuildArtifact = useCallback(async () => {
    if (!workspace) return;
    setOsBusy(true);
    try {
      const projectToml = await invoke<string>('read_text_file', { root: workspace, relativePath: 'void-os.toml' });
      const report = await invoke<BuildArtifactReport>('validate_os_build_artifact', { workspace, projectToml });
      setArtifactReport(report);
      addLog(report.valid
        ? 'ISO verificado: SHA-256, formato ISO 9660, catálogo de arranque y manifiesto correctos.'
        : 'La verificación del ISO encontró comprobaciones fallidas.');
    } catch (error) {
      setArtifactReport(null);
      addLog('No se pudo verificar el ISO: ' + String(error));
    } finally {
      setOsBusy(false);
    }
  }, [addLog, workspace]);

  const runOsBuild = useCallback(async () => {
    if (!workspace || !buildPreparation) {
      addLog('Abre el proyecto y prepara primero el backend.');
      return;
    }
    if (!buildPreflight?.ready) {
      addLog('No se puede iniciar: ejecuta el preflight y resuelve las dependencias obligatorias.');
      return;
    }
    if (buildRunning || activeBuildJobRef.current) {
      addLog('Ya hay una compilación activa.');
      return;
    }
    const command = buildPreparation.commands.join(' && ');
    const elevated = buildPreparation.commands.some((entry) => /\bsudo\b/.test(entry));
    const executionMode = elevated ? 'con permisos de administrador' : 'con tu usuario normal';
    const confirmed = window.confirm(
      'Se ejecutará el backend revisado en la terminal integrada ' + executionMode + '.\n\n' +
      command + '\n\nRevisa el script antes de continuar. No se sobrescribirán ISOs existentes. ¿Continuar?'
    );
    if (!confirmed) return;
    try {
      const job = await invoke<BuildJobRecord>('start_os_build_job', {
        workspace, backend: buildPreparation.backend
      });
      activeBuildJobRef.current = job.id;
      buildStageRef.current = 'queued';
      setActiveBuildJobId(job.id);
      setBuildStage('queued');
      setBuildJobs((current) => [job, ...current].slice(0, 30));
      setSelectedBuildLog(null);
      setTerminalMounted(true);
      setBottomOpen(true);
      setBottomView('terminal');
      setTerminalCommand(command);
      addLog('Compilación registrada: ' + job.id + '. Salida e historial guardados en el proyecto.');
    } catch (error) {
      addLog('No se pudo registrar la compilación: ' + String(error));
    }
  }, [addLog, buildPreparation, buildPreflight, buildRunning, workspace]);
  const runOsBootTest = useCallback(() => {
    if (!buildPreparation?.bootTestScriptPath) {
      addLog('Prepara primero el backend para generar el script de prueba.');
      return;
    }
    const qemuCheck = buildPreflight?.checks.find((check) => check.id === 'qemu-system-x86_64');
    if (!qemuCheck?.available) {
      addLog('QEMU no está disponible. Instala qemu-system-x86_64 y vuelve a comprobar los requisitos.');
      return;
    }
    const script = "'" + buildPreparation.bootTestScriptPath.replace(/'/g, "'\\''") + "'";
    const command = 'sh ' + script;
    const confirmed = window.confirm(
      'Se iniciará una máquina virtual QEMU para probar el arranque del ISO.\n\n' +
      command + '\n\nLa prueba requiere que el ISO ya exista. ¿Continuar?'
    );
    if (!confirmed) return;
    setTerminalMounted(true);
    setBottomOpen(true);
    setBottomView('terminal');
    setTerminalCommand(command);
    addLog('Prueba de arranque BIOS solicitada en QEMU.');
  }, [addLog, buildPreparation, buildPreflight]);

  const runOsUefiBootTest = useCallback(() => {
    if (!buildPreparation?.uefiBootTestScriptPath) {
      addLog('Prepara primero el backend para generar el script de prueba UEFI.');
      return;
    }
    const qemuCheck = buildPreflight?.checks.find((check) => check.id === 'qemu-system-x86_64');
    const firmwareCheck = buildPreflight?.checks.find((check) => check.id === 'uefi-firmware');
    if (!qemuCheck?.available || !firmwareCheck?.available) {
      addLog('La prueba UEFI requiere QEMU y firmware OVMF. Instálalos y vuelve a comprobar los requisitos.');
      return;
    }
    const script = "'" + buildPreparation.uefiBootTestScriptPath.replace(/'/g, "'\\''") + "'";
    const command = 'sh ' + script;
    const confirmed = window.confirm(
      'Se iniciará una máquina virtual QEMU con firmware UEFI para probar el ISO.\n\n' +
      command + '\n\nEl ISO debe existir antes de iniciar la prueba. ¿Continuar?'
    );
    if (!confirmed) return;
    setTerminalMounted(true);
    setBottomOpen(true);
    setBottomView('terminal');
    setTerminalCommand(command);
    addLog('Prueba de arranque UEFI solicitada en QEMU.');
  }, [addLog, buildPreparation, buildPreflight]);

  const publishToOsHub = useCallback(async () => {
    if (!workspace) {
      addLog('Abre primero la carpeta del proyecto OS.');
      return;
    }
    const publishableKey = window.prompt('Pega la clave pública (publishable key) de Supabase. No uses la secret/service_role key:');
    if (!publishableKey?.trim()) return;
    const token = window.prompt('Pega tu token personal de Atdev OS Hub. Se usará solo en esta sesión y no se guardará en void-os.toml:');
    if (!token?.trim()) return;
    const slug = osProjectId.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
    if (!slug) {
      addLog('Define un ID válido para el proyecto antes de publicarlo.');
      return;
    }
    const summary = osProjectDescription.trim().slice(0, 180);
    if (summary.length < 5) {
      addLog('La descripción debe tener al menos 5 caracteres para publicar en OS Hub.');
      return;
    }
    const remoteUrl = gitSnapshot?.remotes.find((remote) => remote.name === 'origin')?.url ?? '';
    const sourceUrl = remoteUrl.startsWith('https://') ? remoteUrl : null;
    setOsBusy(true);
    try {
      const response = await fetch('https://pzfvpvioxpcjkxdawnsq.supabase.co/functions/v1/void-memingo-sync', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          apikey: publishableKey.trim(),
          Authorization: 'Bearer ' + token.trim()
        },
        body: JSON.stringify({
          action: 'publish_project',
          project: {
            id: osProjectId.trim(),
            name: osProjectName.trim(),
            slug,
            summary,
            description: osProjectDescription.trim(),
            baseDistribution: osBaseId,
            baseRelease: osRelease,
            desktopEnvironments: [osDesktop],
            architectures: [osBaseId === 'arch' ? 'x86_64' : 'amd64'],
            sourceUrl,
            version: osProjectVersion.trim()
          }
        })
      });
      const result = await response.json();
      if (!response.ok || !result.ok) throw new Error(result.error || 'HTTP ' + response.status);
      addLog('Proyecto publicado/sincronizado en Atdev OS Hub: ' + result.slug);
    } catch (error) {
      addLog('No se pudo publicar en Atdev OS Hub: ' + String(error));
    } finally {
      setOsBusy(false);
    }
  }, [addLog, gitSnapshot, osBaseId, osDesktop, osProjectDescription, osProjectId, osProjectName, osProjectVersion, osRelease, workspace]);

  const registerOsHubBuild = useCallback(async () => {
    if (!workspace) {
      addLog('Abre primero la carpeta del proyecto OS.');
      return;
    }
    const publishableKey = window.prompt('Pega la clave pública (publishable key) de Supabase:');
    if (!publishableKey?.trim()) return;
    const token = window.prompt('Pega tu token personal de Atdev OS Hub:');
    if (!token?.trim()) return;
    const status = window.prompt('Estado de la compilación: succeeded o failed', 'succeeded')?.trim().toLowerCase();
    if (!status || !['succeeded', 'failed', 'cancelled'].includes(status)) {
      addLog('Estado de compilación no válido.');
      return;
    }
    const artifactUrl = window.prompt('URL HTTPS del artefacto (opcional; déjalo vacío si no se publicó):')?.trim() ?? '';
    const sha256 = window.prompt('SHA-256 del artefacto (opcional):')?.trim().toLowerCase() ?? '';
    if (artifactUrl && !artifactUrl.startsWith('https://')) {
      addLog('El enlace del artefacto debe usar HTTPS.');
      return;
    }
    if (sha256 && !/^[a-f0-9]{64}$/.test(sha256)) {
      addLog('El SHA-256 debe contener exactamente 64 caracteres hexadecimales.');
      return;
    }
    setOsBusy(true);
    try {
      const response = await fetch('https://pzfvpvioxpcjkxdawnsq.supabase.co/functions/v1/void-memingo-sync', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          apikey: publishableKey.trim(),
          Authorization: 'Bearer ' + token.trim()
        },
        body: JSON.stringify({
          action: 'build_event',
          external_project_id: osProjectId.trim(),
          version: osProjectVersion.trim(),
          status,
          artifact_url: artifactUrl || null,
          sha256: sha256 || null,
          metadata: {
            builder_version: version,
            base_distribution: osBaseId,
            architecture: osBaseId === 'arch' ? 'x86_64' : 'amd64',
            desktop_environment: osDesktop,
            iso_label: osIsoLabel,
            target_filesystem: osStorageFs
          }
        })
      });
      const result = await response.json();
      if (!response.ok || !result.ok) throw new Error(result.error || 'HTTP ' + response.status);
      addLog('Build registrado en Atdev OS Hub: ' + result.build_status + '.');
    } catch (error) {
      addLog('No se pudo registrar el build en Atdev OS Hub: ' + String(error));
    } finally {
      setOsBusy(false);
    }
  }, [addLog, osBaseId, osDesktop, osIsoLabel, osProjectId, osProjectVersion, osStorageFs, version, workspace]);

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
          <button className="icon-button" title="Ayuda (próximamente)" disabled><CircleHelp size={16} /></button>
          <button className="icon-button" title="Configuración del editor" onClick={() => setSettingsOpen(true)}><Settings size={16} /></button>
          <div className="window-indicator"><span /></div>
        </div>
      </header>

      <div className="workspace-layout">
        <nav className="activity-rail" aria-label="Activity bar">
          <div className="rail-group">
            <button className={'rail-button ' + (sidebarView === 'explorer' ? 'active' : '')} onClick={() => { setOsBuilderOpen(false); setSidebarView('explorer'); setSidebarOpen(true); }} title="Explorador">
              <LayoutDashboard size={20} />
            </button>
            <button className={'rail-button ' + (sidebarView === 'search' ? 'active' : '')} onClick={() => { setOsBuilderOpen(false); setSidebarView('search'); setSidebarOpen(true); }} title="Buscar">
              <Search size={20} />
            </button>
            <button className={'rail-button ' + (sidebarView === 'source' ? 'active' : '')} onClick={() => { setOsBuilderOpen(false); setSidebarView('source'); setSidebarOpen(true); }} title="Control de código fuente">
              <GitBranch size={20} />
            </button>
            <button className="rail-button" onClick={() => setBottomOpen(!bottomOpen)} title="Panel inferior">
              <PanelBottom size={20} />
            </button>
            <button className={'rail-button ' + (sidebarView === 'boot' ? 'active' : '')} onClick={() => { setOsBuilderOpen(false); setSidebarView('boot'); setSidebarOpen(true); void loadVoidBootCatalog(); void invoke<BootDownload[]>('list_void_boot_downloads').then(setVoidBootDownloads).catch((error) => addLog('Descargas Void Boot: ' + String(error))); }} title="Void Boot — administrador de ISOs">
              <Disc3 size={20} />
            </button>
            <button className={'rail-button ' + (osBuilderOpen ? 'active' : '')} onClick={() => { setOsBuilderOpen(true); setSidebarOpen(false); }} title="OS Builder">
              <Layers3 size={20} />
            </button>
          </div>
          <div className="rail-group rail-bottom">
            <button className="rail-button" title="Herramientas de sistema (próximamente)" disabled><Cpu size={20} /></button>
            <button className="rail-button" title="Configuración" onClick={() => setSettingsOpen(true)}><Settings size={20} /></button>
          </div>
        </nav>

        {sidebarOpen && (
          <aside className="sidebar">
            <div className="sidebar-heading">
              <span>{sidebarView === 'explorer' ? 'EXPLORADOR' : sidebarView === 'search' ? 'BÚSQUEDA' : sidebarView === 'source' ? 'CONTROL DE CÓDIGO' : 'VOID BOOT'}</span>
              <button className="subtle-icon" onClick={() => setSidebarOpen(false)} title="Ocultar panel"><PanelLeftClose size={15} /></button>
            </div>

            {sidebarView === 'explorer' && (
              <>
                <div className="workspace-heading">
                  <div className="workspace-label">
                    <ChevronDown size={13} />
                    <span>{workspace ? workspace.split(/[\\\\/]/).filter(Boolean).pop() : 'SIN CARPETA ABIERTA'}</span>
                  </div>
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
              <div className="workspace-search">
                {!workspace ? (
                  <div className="sidebar-empty"><Search size={25} /><p>Abre una carpeta para buscar en sus archivos.</p><button className="text-action" onClick={() => void openWorkspace()}>Abrir carpeta</button></div>
                ) : (
                  <>
                    <form className="workspace-search-form" onSubmit={(event) => { event.preventDefault(); void runWorkspaceSearch(); }}>
                      <input value={searchQuery} onChange={(event) => setSearchQuery(event.target.value)} placeholder="Buscar en archivos" aria-label="Texto a buscar" maxLength={256} />
                      <label><input type="checkbox" checked={searchCaseSensitive} onChange={(event) => setSearchCaseSensitive(event.target.checked)} /> Distinguir mayúsculas</label>
                      <button className="primary-button" type="submit" disabled={searchBusy || !searchQuery.trim()}>{searchBusy ? 'Buscando…' : 'Buscar'}</button>
                    </form>
                    <div className="workspace-search-summary">{searchResults.length} coincidencias · se omiten binarios, dependencias y archivos grandes</div>
                    <div className="workspace-search-results">
                      {searchResults.map((result, index) => (
                        <button className="workspace-search-result" key={result.path + ':' + result.lineNumber + ':' + index} onClick={() => openSearchResult(result)}>
                          <strong>{result.path}</strong>
                          <span>Línea {result.lineNumber}, columna {result.column}</span>
                          <code>{result.preview}</code>
                        </button>
                      ))}
                      {!searchBusy && searchResults.length === 0 && <div className="tree-empty">{searchQuery.trim() ? 'No se encontraron coincidencias.' : 'Escribe un texto y pulsa Buscar.'}</div>}
                    </div>
                  </>
                )}
              </div>
            )}

            {sidebarView === 'boot' && (
              <div className="void-boot-sidebar">
                <div className="git-toolbar"><span>VOID BOOT · DISTRIBUTION CENTER</span><button className="subtle-icon" onClick={() => { void loadVoidBootCatalog(); if (voidBootDirectory) void scanVoidBootDirectory(voidBootDirectory); }} disabled={voidBootCatalogBusy || voidBootBusy} title="Actualizar catálogo y biblioteca"><Activity size={14} /></button></div>
                <p className="git-provider-note">Modo directo: la ISO se descarga en la carpeta elegida, que puede ser la partición de datos VOIDDATA ya montada. No se crea otra copia en el disco interno. La descarga acelerada usa hasta 8 conexiones si el servidor admite HTTP Range. No formatea ni sobrescribe el USB; cada ISO requiere un perfil de arranque probado.</p>
                <div className="git-section-label">DISTRIBUTIONS · {voidBootCatalog.length}</div>
                {voidBootCatalogBusy && <div className="tree-empty">Cargando distribuciones…</div>}
                {!voidBootCatalogBusy && voidBootCatalog.map((distro) => (
                  <div className={'void-boot-distro-card' + (voidBootSelectedDistro?.id === distro.id ? ' selected' : '')} key={distro.id}>
                    <strong>{distro.name}</strong>
                    <span>{distro.description}</span>
                    <small>{distro.family} · {distro.architectures.join(', ')}</small>
                    <small>Ediciones: {distro.editions.join(', ')}</small>
                    <small>Entrega: ISO directa a VOIDDATA · perfil de arranque requerido</small>
                    <button className="text-action" onClick={() => { setVoidBootSelectedDistro(distro); setVoidBootDownloadName(distro.id + '.iso'); setVoidBootDownloadUrl(''); }}>Elegir para descargar</button>
                    <a className="text-action" href={distro.download_page} target="_blank" rel="noreferrer">Abrir página oficial ↗</a>
                  </div>
                ))}
                <div className="git-section-label">DESCARGA DIRECTA A VOIDDATA · HASTA 8 CONEXIONES</div>
                <p className="git-provider-note">{voidBootSelectedDistro ? 'Seleccionada: ' + voidBootSelectedDistro.name : 'Elige una distribución primero.'} Elige como destino la carpeta VOIDDATA montada del pendrive para escribir la ISO directamente allí, sin duplicarla primero en el almacenamiento interno. La transferencia por rangos requiere soporte del servidor; si no lo admite, la descarga se detiene en vez de fingir aceleración. El catálogo contiene páginas oficiales, no enlaces ISO versionados: copia el enlace HTTPS directo de la versión elegida.</p>
                <div className="void-boot-download-form">
                  <input aria-label="Enlace directo HTTPS de la ISO" placeholder="https://servidor-oficial/archivo.iso" value={voidBootDownloadUrl} onChange={(event) => setVoidBootDownloadUrl(event.target.value)} />
                  <input aria-label="Nombre del archivo ISO" placeholder="distribution.iso" value={voidBootDownloadName} onChange={(event) => setVoidBootDownloadName(event.target.value)} />
                  <button className="secondary-button" onClick={() => void chooseVoidBootDownloadDirectory()}>{voidBootDownloadDirectory ? 'Carpeta: ' + voidBootDownloadDirectory.split(/[\\\\/]/).filter(Boolean).pop() : 'Elegir VOIDDATA / destino'}</button>
                  <button className="primary-button" onClick={() => void startVoidBootDownload()} disabled={voidBootDownloadBusy || !voidBootSelectedDistro || !voidBootDownloadUrl.trim() || !voidBootDownloadDirectory}>{voidBootDownloadBusy ? 'Iniciando…' : 'Descargar directamente / reanudar'}</button>
                </div>
                {voidBootDownloads.map((task) => {
                  const percent = task.total_bytes && task.total_bytes > 0 ? Math.min(100, task.downloaded_bytes / task.total_bytes * 100) : null;
                  return <div className="void-boot-download-task" key={task.id}>
                    <strong>{task.file_name}</strong>
                    <span>{(task.downloaded_bytes / 1073741824).toFixed(2)} GiB{task.total_bytes ? ' / ' + (task.total_bytes / 1073741824).toFixed(2) + ' GiB' : ''} · {task.status} · {task.connections} conexiones</span>
                    {percent !== null && <div className="void-boot-download-progress"><span style={{ width: percent + '%' }} /></div>}
                    {task.error && <small>{task.error}</small>}
                    {task.status === 'downloading' && <button className="text-action" onClick={() => void pauseVoidBootDownload(task.id)}>Pausar (reanuda después)</button>}
                    {task.status === 'paused' && <small>Parcial guardado en disco. Pulsa de nuevo «Descargar / reanudar ISO» con la misma URL, nombre y carpeta.</small>}
                    {task.status === 'completed' && <small>Descarga terminada. Escanea la carpeta para inspeccionar la ISO.</small>}
                  </div>;
                })}
                <section className="void-boot-qemu-card">
                  <div className="void-boot-qemu-heading"><span className="void-boot-qemu-icon"><Cpu size={17} /></span><div><strong>QEMU VIRTUAL LAB</strong><small>Pruebas UEFI sin pendrive físico</small></div><span className="void-boot-status-dot" /></div>
                  <p>Selecciona una imagen de disco creada por Void Boot. QEMU arrancará con OVMF/UEFI, red desactivada y snapshot para descartar los cambios al cerrar la máquina virtual.</p>
                  <button className="secondary-button" onClick={() => void chooseQemuImage()}><FolderOpen size={14} /> {qemuImagePath ? qemuImagePath.split(/[\\\\/]/).filter(Boolean).pop() : 'Seleccionar imagen virtual (.img / .qcow2)'}</button>
                  <button className="primary-button" onClick={() => void launchVoidBootQemu()} disabled={!qemuImagePath || qemuBusy}><Play size={14} /> {qemuBusy ? 'Iniciando QEMU…' : 'Iniciar máquina virtual'}</button>
                  {qemuStatus && <small className="void-boot-qemu-status">{qemuStatus}</small>}
                </section>
                <div className="git-section-label">ISO LIBRARY · LOCAL</div>
                <p className="git-provider-note">Inspección de solo lectura. La descarga puede guardar archivos en una carpeta VOIDDATA montada; la preparación del arranque y la modificación de particiones USB siguen deshabilitadas.</p>
                <button className="primary-button" onClick={() => void chooseVoidBootDirectory()} disabled={voidBootBusy}>{voidBootBusy ? 'Inspeccionando…' : 'Seleccionar carpeta de ISOs'}</button>
                <div className="git-section-label">BOOT PROFILES</div>
                <button className="secondary-button" onClick={() => void chooseVoidBootProfile()} disabled={voidBootPreviewBusy}>{voidBootPreviewBusy ? 'Generando vista previa…' : 'Abrir catálogo TOML'}</button>
                {voidBootProfileFile && <div className="git-provider-note" title={voidBootProfileFile}>{voidBootProfileFile.split(/[\\\\/]/).filter(Boolean).pop()}</div>}
                {voidBootMenuPreview && (
                  <div className="void-boot-preview">
                    <div className="git-toolbar"><span>GRUB MENU PREVIEW</span><button className="subtle-icon" onClick={() => void refreshVoidBootPreview()} disabled={voidBootPreviewBusy} title="Actualizar vista previa"><Activity size={14} /></button></div>
                    <pre>{voidBootMenuPreview}</pre>
                  </div>
                )}
                {voidBootDirectory && <div className="git-section-label" title={voidBootDirectory}>CARPETA: {voidBootDirectory.split(/[\\\\/]/).filter(Boolean).pop()}</div>}
                {voidBootEntries.length === 0 && <div className="tree-empty">{voidBootBusy ? 'Leyendo archivos…' : 'No hay ISOs inspeccionadas.'}</div>}
                {voidBootEntries.map((entry) => (
                  <div className="git-remote-row" key={entry.path}>
                    <strong>{entry.name}</strong>
                    <span>{(entry.size_bytes / (1024 * 1024)).toFixed(1)} MiB · {entry.status === 'iso9660_with_el_torito_descriptor' ? 'El Torito detectado' : entry.status === 'iso9660_without_el_torito_descriptor' ? 'ISO 9660, sin descriptor El Torito' : 'Formato no reconocido'}</span>
                    <small title={entry.sha256}>SHA-256: {entry.sha256}</small>
                  </div>
                ))}
              </div>
            )}

            {sidebarView === 'source' && (
              <div className="git-sidebar">
                <div className="git-toolbar">
                  <span>REPOSITORIO</span>
                  <button className="subtle-icon" onClick={() => workspace && void refreshGitSnapshot(workspace).catch((error) => addLog('Git: ' + String(error)))} disabled={!workspace || gitBusy} title="Actualizar Git"><Activity size={14} /></button>
                </div>
                {!workspace ? (
                  <div className="sidebar-empty"><GitBranch size={25} /><p>Abre un proyecto o clona un repositorio Git para comenzar.</p><button className="text-action" onClick={() => void openWorkspace()}>Abrir carpeta</button><button className="text-action" disabled={gitBusy} onClick={() => void cloneGitRepository()}>Clonar repositorio</button></div>
                ) : gitSnapshot === null ? (
                  <div className="sidebar-empty"><GitBranch size={25} /><p>No se pudo consultar el repositorio. Comprueba que Git esté instalado.</p></div>
                ) : !gitSnapshot.initialized ? (
                  <div className="sidebar-empty"><GitBranch size={25} /><p>Esta carpeta todavía no tiene un repositorio Git.</p><button className="primary-button" disabled={gitBusy} onClick={() => void initializeGit()}>Inicializar repositorio</button></div>
                ) : (
                  <>
                    <div className="git-current-branch"><GitBranch size={14} /><strong>{gitSnapshot.branch || 'HEAD separado'}</strong></div>
                    <div className="git-section-label">CAMBIOS</div>
                    <pre className="git-status-output">{gitSnapshot.status || 'Working tree limpio.'}</pre>
                    {gitSnapshot.stagedSummary && <pre className="git-status-output git-staged">{gitSnapshot.stagedSummary}</pre>}
                    <div className="git-actions">
                      <button className="secondary-button" disabled={gitBusy} onClick={() => void stageGitChanges()}>Preparar todos</button>
                      <button className="secondary-button" disabled={gitBusy || gitSnapshot.remotes.length === 0} onClick={() => void pullGitChanges()}>Pull</button>
                      <button className="secondary-button" disabled={gitBusy || gitSnapshot.remotes.length === 0} onClick={() => void pushGitChanges()}>Push</button>
                      <button className="secondary-button" disabled={gitBusy || gitSnapshot.remotes.length === 0} onClick={() => void fetchGitRemotes()}>Fetch</button>
                    </div>
                    <div className="git-commit-box">
                      <input value={gitCommitMessage} onChange={(event) => setGitCommitMessage(event.target.value)} placeholder="Mensaje del commit" maxLength={512} />
                      <button className="primary-button" disabled={gitBusy || !gitCommitMessage.trim()} onClick={() => commitGitChanges()}>Commit</button>
                    </div>
                    <div className="git-section-label">RAMAS</div>
                    <div className="git-branch-create">
                      <input value={gitBranchName} onChange={(event) => setGitBranchName(event.target.value)} placeholder="nueva-rama" />
                      <button className="subtle-icon" disabled={gitBusy || !gitBranchName.trim()} onClick={() => createGitBranch()} title="Crear y cambiar de rama"><Plus size={15} /></button>
                    </div>
                    {gitSnapshot.branches.map((branch) => <button key={branch} className={'git-branch-row ' + (branch === gitSnapshot.branch ? 'active' : '')} onClick={() => branch !== gitSnapshot.branch && void checkoutGitBranch(branch)} disabled={gitBusy}><GitBranch size={12} />{branch}</button>)}
                    <div className="git-section-label">REMOTOS</div>
                    {gitSnapshot.remotes.map((remote) => <div className="git-remote-row" key={remote.name}><strong>{remote.name}</strong><span>{remote.provider}</span><small title={remote.url}>{remote.url}</small></div>)}
                    <div className="git-remote-form">
                      <input value={gitRemoteName} onChange={(event) => setGitRemoteName(event.target.value)} placeholder="origin" aria-label="Nombre del remoto" />
                      <input value={gitRemoteUrl} onChange={(event) => setGitRemoteUrl(event.target.value)} placeholder="https://... o git@..." aria-label="URL Git remota" />
                      <button className="secondary-button" disabled={gitBusy || !gitRemoteUrl.trim()} onClick={() => addGitRemote()}>Añadir remoto</button>
                    </div>
                    <div className="git-section-label">COMMITS RECIENTES</div>
                    {gitSnapshot.recentCommits.map((commit) => <div className="git-commit-row" key={commit}>{commit}</div>)}
                    <p className="git-provider-note">GitHub, GitLab y servidores Git compatibles funcionan mediante Git y las credenciales del sistema. GitVoid se reconoce como experimental hasta que su servicio remoto esté disponible.</p>
                  </>
                )}
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

          {osBuilderOpen ? (
            <section className="os-builder-view">
              <div className="os-builder-header">
                <div>
                  <div className="eyebrow">VOID MEMINGO <span>•</span> DISTRIBUTION WORKBENCH</div>
                  <h1>OS Builder</h1>
                  <p>Diseña tu distribución Linux y prepara su configuración de construcción.</p>
                </div>
                <div className="os-builder-header-actions">
                  <span className="os-builder-state"><Disc3 size={14} /> ISO builder en desarrollo</span>
                  <button className="secondary-button" onClick={() => {
                    setOsProjectName('PondOS'); setOsProjectId('pondos'); setOsProjectVersion('0.1.0');
                    setOsProjectDescription('A friendly Ubuntu-based desktop operating system with an LXQt desktop and a natural pond-inspired identity.');
                    setOsIsoLabel('PONDOS_01'); setOsBaseId('ubuntu'); setOsRelease('24.04 LTS'); setOsDesktop('lxqt');
                    setOsTheme('PondOS-Green'); setOsIconTheme('Papirus'); setOsWallpaper('usr/share/backgrounds/pondos/pondos-wallpaper.png');
                    setOsPackages('lxqt, lightdm, network-manager, git, curl, sudo, papirus-icon-theme');
                    setOsOverlay('filesystem'); setOsLocale('es_CL.UTF-8'); setOsTimezone('America/Santiago'); setOsHostname('pondos');
                    setOsServices('NetworkManager.service'); setOsStorageFs('ext4'); setOsOutputDir('dist'); setOsIsoName('pondos-0.1.0.iso');
                    setOsValidation(null); setBuildPreparation(null); addLog('Perfil PondOS cargado en OS Builder.');
                  }}><Layers3 size={14} /> Cargar perfil PondOS</button>
                </div>
              </div>
              <div className="os-builder-controls os-project-fields">
                <label><span>Nombre del sistema</span><input value={osProjectName} onChange={(event) => setOsProjectName(event.target.value)} /></label>
                <label><span>ID del proyecto</span><input value={osProjectId} onChange={(event) => setOsProjectId(event.target.value)} /></label>
                <label><span>Versión</span><input value={osProjectVersion} onChange={(event) => setOsProjectVersion(event.target.value)} /></label>
                <label><span>Autor / editor</span><input value={osPublisher} onChange={(event) => setOsPublisher(event.target.value)} placeholder="Tu nombre u organización" /></label>
                <label><span>Etiqueta ISO</span><input value={osIsoLabel} onChange={(event) => setOsIsoLabel(event.target.value)} /></label>
                <label className="os-packages-field"><span>Descripción</span><input value={osProjectDescription} onChange={(event) => setOsProjectDescription(event.target.value)} /></label>
              </div>
              <div className="os-builder-section-title"><HardDrive size={16} /> BASE DEL SISTEMA</div>
              <div className="os-base-grid">
                {osBases.map((base) => (
                  <button key={base.id} className={'os-base-card ' + (osBaseId === base.id ? 'selected' : '')} onClick={() => { setOsBaseId(base.id); setOsRelease(base.releases[0]); setOsDesktop(base.desktops.includes('xfce') ? 'xfce' : 'none'); setOsValidation(null); }}>
                    <span className="os-base-mark">{base.id === 'debian' ? 'D' : base.id === 'ubuntu' ? 'U' : 'A'}</span>
                    <b>{base.name}</b>
                    <small>{base.backend}</small>
                    <span className="os-base-status">Backend: {base.backendStatus === 'ready' ? 'Build verificado' : base.backendStatus === 'preview' ? 'Perfil disponible' : 'Pendiente'}</span>
                  </button>
                ))}
              </div>
              <div className="os-builder-controls">
                <label>
                  <span>Versión base</span>
                  <select value={osRelease} onChange={(event) => setOsRelease(event.target.value)}>
                    {(selectedOsBase?.releases ?? []).map((release) => <option key={release} value={release}>{release}</option>)}
                  </select>
                </label>
                <label>
                  <span>Entorno de escritorio</span>
                  <select value={osDesktop} onChange={(event) => setOsDesktop(event.target.value)}>
                    {(selectedOsBase?.desktops ?? []).map((desktop) => <option key={desktop} value={desktop}>{desktop === 'none' ? 'Sin escritorio' : desktop}</option>)}
                  </select>
                </label>
                <label className="os-packages-field">
                  <span>Paquetes adicionales (separados por coma)</span>
                  <input value={osPackages} onChange={(event) => setOsPackages(event.target.value)} placeholder="firefox, git, htop" />
                </label>
              </div>
              <div className="os-builder-controls os-filesystem-controls">
                <label>
                  <span>Overlay del sistema raíz</span>
                  <input value={osOverlay} onChange={(event) => setOsOverlay(event.target.value)} placeholder="filesystem" />
                </label>
                <label>
                  <span>Hostname</span>
                  <input value={osHostname} onChange={(event) => setOsHostname(event.target.value)} placeholder="atdev-os" />
                </label>
                <label>
                  <span>Locale</span>
                  <input value={osLocale} onChange={(event) => setOsLocale(event.target.value)} placeholder="es_CL.UTF-8" />
                </label>
                <label>
                  <span>Zona horaria</span>
                  <input value={osTimezone} onChange={(event) => setOsTimezone(event.target.value)} placeholder="America/Santiago" />
                </label>
                <label>
                  <span>Sistema de archivos de instalación</span>
                  <select value={osStorageFs} onChange={(event) => setOsStorageFs(event.target.value)}>
                    <option value="ext4">ext4</option>
                    <option value="btrfs">Btrfs</option>
                    <option value="xfs">XFS</option>
                  </select>
                </label>
                <label>
                  <span>Servicios a habilitar (separados por coma)</span>
                  <input value={osServices} onChange={(event) => setOsServices(event.target.value)} placeholder="NetworkManager.service" />
                </label>
                <label>
                  <span>Tema GTK / escritorio</span>
                  <input value={osTheme} onChange={(event) => setOsTheme(event.target.value)} placeholder="Adwaita-dark" />
                </label>
                <label>
                  <span>Tema de iconos</span>
                  <input value={osIconTheme} onChange={(event) => setOsIconTheme(event.target.value)} placeholder="Papirus-Dark" />
                </label>
                <label>
                  <span>Wallpaper (ruta dentro del overlay)</span>
                  <input value={osWallpaper} onChange={(event) => setOsWallpaper(event.target.value)} placeholder="usr/share/backgrounds/atdev.jpg" />
                </label>
              </div>
              <div className="os-builder-controls os-output-controls">
                <label>
                  <span>Nombre de la ISO</span>
                  <input value={osIsoName} onChange={(event) => setOsIsoName(event.target.value)} placeholder="atdev-os.iso" />
                </label>
                <label>
                  <span>Carpeta de salida</span>
                  <input value={osOutputDir} onChange={(event) => setOsOutputDir(event.target.value)} placeholder="dist" />
                </label>
              </div>
              <div className="os-builder-grid">
                <div className="os-builder-card">
                  <div className="os-builder-card-heading"><HardDrive size={16} /> Sistema de archivos</div>
                  <p>Paquetes, overlay del sistema raíz, servicios, hostname, idioma, zona horaria y sistema de archivos de instalación.</p>
                  <div className="os-builder-meta">Configuración declarativa · void-os.toml</div>
                </div>
                <div className="os-builder-card">
                  <div className="os-builder-card-heading"><Monitor size={16} /> Escritorio</div>
                  <p>GNOME, KDE Plasma, Xfce, LXQt, Cinnamon, MATE o una instalación mínima sin entorno gráfico.</p>
                  <div className="os-builder-meta">Perfiles según capacidades de cada base</div>
                </div>
              </div>
              <div className="os-builder-actions">
                <button className="primary-button" onClick={() => void createOsProject()} disabled={osBusy}>
                  <FilePlus2 size={15} /> Crear / actualizar void-os.toml
                </button>
                <button className="secondary-button" onClick={() => void validateOsProject()} disabled={osBusy || !workspace}>
                  <ShieldCheck size={15} /> Validar configuración
                </button>
                <button className="secondary-button" onClick={() => void prepareOsBuild()} disabled={osBusy || !workspace}>
                  <HardDrive size={15} /> Preparar backend
                </button>
                <button className="secondary-button" onClick={() => void checkOsBuildPreflight()} disabled={osBusy || !workspace} title="Comprobar dependencias y recursos del host Linux">
                  <ShieldCheck size={15} /> Comprobar requisitos
                </button>
                <button className="secondary-button" onClick={runOsBuild} disabled={osBusy || !isLinuxHost || !workspace || !buildPreparation || !osValidation?.valid || !buildPreflight?.ready} title="Ejecutar el script revisado en la terminal integrada">
                  <Disc3 size={15} /> Build ISO
                </button>
                {buildRunning && (
                  <button className="secondary-button" onClick={cancelOsBuild} title="Enviar Ctrl+C al proceso en ejecución">
                    Interrumpir proceso
                  </button>
                )}
                <button className="secondary-button" onClick={() => void validateOsBuildArtifact()} disabled={osBusy || !workspace || !buildPreparation} title="Comprobar ISO, checksum, catálogo de arranque y manifiesto">
                  Verificar ISO
                </button>
                <button className="secondary-button" onClick={runOsBootTest} disabled={osBusy || !isLinuxHost || !buildPreparation?.bootTestScriptPath || !buildPreflight?.checks.some((check) => check.id === 'qemu-system-x86_64' && check.available)} title="Iniciar una prueba de arranque BIOS del ISO en QEMU">
                  <Cpu size={15} /> Probar ISO (QEMU)
                </button>
                <button className="secondary-button" onClick={runOsUefiBootTest} disabled={osBusy || !isLinuxHost || !buildPreparation?.uefiBootTestScriptPath || !buildPreflight?.checks.some((check) => check.id === 'qemu-system-x86_64' && check.available) || !buildPreflight?.checks.some((check) => check.id === 'uefi-firmware' && check.available)} title="Iniciar una prueba de arranque UEFI del ISO en QEMU">
                  <Cpu size={15} /> Probar UEFI (QEMU)
                </button>
                <button className="secondary-button" onClick={() => void publishToOsHub()} disabled={osBusy || !workspace} title="Sincronizar el manifiesto del sistema con Atdev OS Hub">
                  <Layers3 size={15} /> Publicar en OS Hub
                </button>
                <button className="secondary-button" onClick={() => void registerOsHubBuild()} disabled={osBusy || !workspace} title="Registrar manualmente una compilación en Atdev OS Hub">
                  <Disc3 size={15} /> Registrar build
                </button>
              </div>
              {buildPreflight && (
                <div className={'os-validation ' + (buildPreflight.ready ? 'valid' : 'invalid')}>
                  <b>{buildPreflight.ready ? 'Requisitos obligatorios disponibles' : 'Faltan requisitos para compilar'}</b>
                  {buildPreflight.checks.map((check) => <p key={check.id}>{check.available ? '✓' : check.required ? '✕' : '⚠'} {check.label}: {check.detail}</p>)}
                  {buildPreflight.warnings.map((warning, index) => <p key={'preflight-' + index}>Aviso: {warning}</p>)}
                </div>
              )}
              {buildRunning && (
                <div className="os-validation" role="status">
                  <b>Proceso activo</b>
                  <p>La salida se muestra en la terminal integrada. Puedes interrumpir el proceso con el botón correspondiente o con Ctrl+C desde la terminal.</p>
                </div>
              )}
              {artifactReport && (
                <div className={'os-validation ' + (artifactReport.valid ? 'valid' : 'invalid')} role="status">
                  <b>{artifactReport.valid ? 'Artefacto ISO verificado' : 'El artefacto ISO necesita atención'}</b>
                  <p>Archivo: {artifactReport.isoPath}</p>
                  <p>Tamaño: {(artifactReport.sizeBytes / 1024 / 1024).toFixed(1)} MB · Backend: {artifactReport.backend || 'no identificado'}</p>
                  {artifactReport.sha256 && <p>SHA-256: <code>{artifactReport.sha256}</code></p>}
                  {artifactReport.checks.map((check) => <p key={check.id}>{check.passed ? '✓' : '✕'} {check.label}: {check.detail}</p>)}
                </div>
              )}
              <div className="os-validation">
                <b>Estado de compilación</b>
                <p>Etapa actual: {buildStage === 'idle' ? 'Sin compilación activa' : buildStage.replace(/-/g, ' ')}</p>
                {activeBuildJobId && <p>ID: <code>{activeBuildJobId}</code></p>}
                <p>Los trabajos y sus registros se guardan dentro de <code>.void-memingo/build-jobs/</code> en el proyecto.</p>
              </div>
              {buildJobs.length > 0 && (
                <div className="os-validation">
                  <b>Historial de compilaciones</b>
                  {buildJobs.slice(0, 8).map((job) => (
                    <p key={job.id}>
                      <strong>{job.backend}</strong> · {job.status} · {job.stage.replace(/-/g, ' ')} · {new Date(job.startedAt * 1000).toLocaleString()}
                      {' '}<button className="secondary-button" onClick={() => {
                        if (!workspace) return;
                        void invoke<string>('read_os_build_job_log', { workspace, jobId: job.id })
                          .then(setSelectedBuildLog)
                          .catch((error) => addLog('No se pudo leer el registro: ' + String(error)));
                      }}>Ver registro</button>
                    </p>
                  ))}
                  {selectedBuildLog !== null && (
                    <pre className="build-job-log">{selectedBuildLog || 'El registro está vacío.'}</pre>
                  )}
                </div>
              )}
              {buildPreparation && (
                <div className="os-validation valid">
                  <b>{buildPreparation.backend} preparado</b>
                  <p>Directorio: {buildPreparation.buildDirectory}</p>
                  <p>Script generado: {buildPreparation.scriptPath}</p>
                  {buildPreparation.bootTestScriptPath && <p>Prueba BIOS QEMU: {buildPreparation.bootTestScriptPath}</p>}
                  {buildPreparation.uefiBootTestScriptPath && <p>Prueba UEFI QEMU: {buildPreparation.uefiBootTestScriptPath}</p>}
                  {buildPreparation.commands.map((command, index) => <p key={index}><code>{command}</code></p>)}
                  {buildPreparation.notes.map((note, index) => <p key={'n'+index}>Aviso: {note}</p>)}
                </div>
              )}
              {osValidation && (
                <div className={'os-validation ' + (osValidation.valid ? 'valid' : 'invalid')}>
                  <b>{osValidation.valid ? 'Configuración válida' : 'Hay errores de configuración'}</b>
                  {osValidation.errors.map((error, index) => <p key={'e'+index}>• {error}</p>)}
                  {osValidation.warnings.map((warning, index) => <p key={'w'+index}>Aviso: {warning}</p>)}
                </div>
              )}
              <div className="os-builder-footnote">Cada distribución usa su propio backend. La creación de ISO requiere Linux y las herramientas correspondientes. La terminal muestra la salida en vivo y permite cancelar con Ctrl+C; Ubuntu y Debian pueden solicitar permisos elevados, mientras que Archiso se ejecuta con el usuario normal y espacios de nombres.</div>
            </section>
          ) : activeTab ? (
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
                  onKeyDown={(event) => {
                    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 's') {
                      event.preventDefault();
                      void saveActiveFile();
                      return;
                    }
                    if (event.key !== 'Tab') return;
                    event.preventDefault();
                    const target = event.currentTarget;
                    const start = target.selectionStart;
                    const end = target.selectionEnd;
                    const indent = ' '.repeat(editorTabSize);
                    updateActiveContent(target.value.slice(0, start) + indent + target.value.slice(end));
                    const cursor = start + indent.length;
                    window.requestAnimationFrame(() => target.setSelectionRange(cursor, cursor));
                  }}
                  spellCheck={false}
                  autoCapitalize="off"
                  autoCorrect="off"
                  wrap={editorWordWrap ? 'soft' : 'off'}
                  style={{ fontSize: editorFontSize + 'px', tabSize: editorTabSize, whiteSpace: editorWordWrap ? 'pre-wrap' : 'pre' }}
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

          <section className="bottom-panel" style={{ display: bottomOpen ? 'flex' : 'none' }}>
              <div className="panel-tabs">
                <button className={bottomView === 'problems' ? 'selected' : ''} onClick={() => setBottomView('problems')}>PROBLEMAS</button>
                <button className={bottomView === 'output' ? 'selected' : ''} onClick={() => setBottomView('output')}>SALIDA</button>
                <button className={bottomView === 'terminal' ? 'selected' : ''} onClick={() => { setBottomView('terminal'); setTerminalMounted(true); }}>TERMINAL</button>
                <div className="panel-spacer" />
                <button className="subtle-icon" onClick={() => setBottomOpen(false)} title="Cerrar panel"><X size={14} /></button>
              </div>
              <div className="panel-content">
                {bottomView === 'problems' && (
                  <div className="diagnostics-panel">
                    <div className="diagnostics-toolbar">
                      <span>{activeTab ? activeTab.path : 'Abre un archivo para analizarlo'}</span>
                      <button className="secondary-button" onClick={() => void runFileDiagnostics()} disabled={!activeTab || diagnosticsBusy}>
                        {diagnosticsBusy ? 'Analizando…' : 'Analizar archivo'}
                      </button>
                    </div>
                    {!activeTab ? (
                      <div className="panel-empty">Selecciona un archivo de texto para ejecutar los diagnósticos disponibles.</div>
                    ) : diagnosticReport && !diagnosticReport.supported ? (
                      <div className="panel-empty">No hay analizador integrado para {diagnosticReport.language || 'este tipo de archivo'}. Los diagnósticos actuales cubren JSON y TOML.</div>
                    ) : diagnosticReport && diagnosticReport.diagnostics.length === 0 ? (
                      <div className="panel-empty"><Check size={16} /> No se encontraron errores de sintaxis en {diagnosticReport.language}.</div>
                    ) : diagnosticReport ? (
                      <div className="diagnostics-list">
                        {diagnosticReport.diagnostics.map((item, index) => (
                          <div className="diagnostic-item" key={index}>
                            <strong>{item.severity.toUpperCase()} · Línea {item.line}, columna {item.column}</strong>
                            <span>{item.message}</span>
                          </div>
                        ))}
                      </div>
                    ) : (
                      <div className="panel-empty">Los diagnósticos integrados validan la sintaxis de archivos JSON y TOML. Pulsa Analizar archivo.</div>
                    )}
                  </div>
                )}
                {bottomView === 'output' && (
                  <div className="output-list">
                    {logs.map((line, index) => <div key={index} className="output-line">{line}</div>)}
                  </div>
                )}
                {terminalMounted && (
                  <div className="terminal-panel-host" style={{ display: bottomView === 'terminal' ? 'block' : 'none' }}>
                    <TerminalPanel
                    workspace={workspace}
                    onLog={addLog}
                    command={terminalCommand}
                    onCommandConsumed={handleTerminalCommandConsumed}
                    interruptSignal={terminalInterruptSignal}
                    onCommandStarted={handleTerminalCommandStarted}
                    onCommandFinished={handleTerminalCommandFinished}
                    onCommandOutput={handleTerminalCommandOutput}
                  />
                  </div>
                )}
              </div>
            </section>
        </main>
      </div>

      {settingsOpen && (
        <div className="modal-backdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setSettingsOpen(false); }}>
          <section className="settings-modal" role="dialog" aria-modal="true" aria-label="Configuración del editor">
            <div className="settings-modal-heading">
              <div><span className="eyebrow">PREFERENCIAS</span><h2>Configuración del editor</h2></div>
              <button className="subtle-icon" onClick={() => setSettingsOpen(false)} title="Cerrar"><X size={16} /></button>
            </div>
            <label className="settings-control">
              <span>Tamaño de fuente <strong>{editorFontSize}px</strong></span>
              <input type="range" min={10} max={24} value={editorFontSize} onChange={(event) => setEditorFontSize(Number(event.target.value))} />
            </label>
            <label className="settings-control settings-checkbox">
              <input type="checkbox" checked={editorWordWrap} onChange={(event) => setEditorWordWrap(event.target.checked)} />
              Ajuste de línea
            </label>
            <label className="settings-control">
              <span>Tamaño de indentación</span>
              <select value={editorTabSize} onChange={(event) => setEditorTabSize(Number(event.target.value))}>
                <option value={2}>2 espacios</option><option value={4}>4 espacios</option><option value={8}>8 espacios</option>
              </select>
            </label>
            <div className="settings-modal-actions">
              <button className="secondary-button" onClick={() => { setEditorFontSize(13); setEditorWordWrap(true); setEditorTabSize(2); }}>Restablecer</button>
              <button className="primary-button" onClick={() => setSettingsOpen(false)}>Listo</button>
            </div>
            <p>Las preferencias se guardan localmente en este dispositivo.</p>
          </section>
        </div>
      )}

      <footer className="statusbar">
        <div className="status-left">
          <span className="status-branch"><GitBranch size={12} /> {gitSnapshot?.initialized ? (gitSnapshot.branch || 'HEAD separado') : 'Git no conectado'}</span>
          <span><AlertCircle size={12} /> {diagnosticReport?.diagnostics.length ?? 0}</span>
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