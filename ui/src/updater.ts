import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

export async function checkForAppUpdates(): Promise<void> {
  try {
    const update = await check();
    if (!update) return;

    const confirmed = window.confirm(
      `È disponibile Open Font Manager ${update.version}. Vuoi scaricare e installare l'aggiornamento?`,
    );
    if (!confirmed) return;

    await update.downloadAndInstall();
    await relaunch();
  } catch (error) {
    console.error('Controllo aggiornamenti non riuscito:', error);
  }
}
