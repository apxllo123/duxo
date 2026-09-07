import { app, ipcMain } from 'electron';

export type PlatformName = 'darwin' | 'win32';

export interface DuxoAppInfo {
  name: 'Duxo';
  version: string;
  platform: PlatformName;
  arch: string;
}

export function registerApiHandlers(): void {
  ipcMain.handle('app:info', (): DuxoAppInfo => ({
    name: 'Duxo',
    version: app.getVersion(),
    platform: process.platform as PlatformName,
    arch: process.arch,
  }));
}
