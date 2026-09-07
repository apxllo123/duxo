import { contextBridge, ipcRenderer } from 'electron';

export interface DuxoApi {
  getAppInfo(): Promise<{
    name: 'Duxo';
    version: string;
    platform: 'darwin' | 'win32';
    arch: string;
  }>;
  prepareStartup(): Promise<{
    workspace: string;
    analyzer: string;
  }>;
  onStartupProgress(listener: (progress: {
    message: string;
    percent: number;
    complete: boolean;
  }) => void): () => void;
  pickGameFolder(): Promise<string | null>;
  analyzeGame(gamePath: string): Promise<unknown>;
  buildAdaptationPlan(profile: unknown): Promise<unknown>;
}

const api: DuxoApi = {
  getAppInfo: () => ipcRenderer.invoke('app:info'),
  prepareStartup: () => ipcRenderer.invoke('startup:prepare'),
  onStartupProgress: (listener) => {
    const handler = (_event: Electron.IpcRendererEvent, progress: { message: string; percent: number; complete: boolean }) => {
      listener(progress);
    };
    ipcRenderer.on('startup:progress', handler);
    return () => ipcRenderer.removeListener('startup:progress', handler);
  },
  pickGameFolder: () => ipcRenderer.invoke('dialog:pick-game-folder'),
  analyzeGame: (gamePath) => ipcRenderer.invoke('analyzer:analyze', gamePath),
  buildAdaptationPlan: (profile) => ipcRenderer.invoke('adapter:plan', profile),
};

contextBridge.exposeInMainWorld('duxo', api);
