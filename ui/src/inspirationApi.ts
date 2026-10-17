import { invoke } from '@tauri-apps/api/core';

import type { InspirationApi, InspirationAsset, InspirationDocument } from './inspirationTypes';

async function bytesFrom(result: Uint8Array | number[] | ArrayBuffer): Promise<Uint8Array> {
  if (result instanceof Uint8Array) return result;
  if (result instanceof ArrayBuffer) return new Uint8Array(result);
  return Uint8Array.from(result);
}

export const inspirationApi: InspirationApi = {
  load: () => invoke<InspirationDocument>('load_inspiration'),
  save: (document, expectedRevision) =>
    invoke<InspirationDocument>('save_inspiration', { document, expectedRevision }),
  importImage: (path) => invoke<InspirationAsset>('import_inspiration_image', { path }),
  getAsset: async (assetId) => bytesFrom(await invoke<Uint8Array | number[] | ArrayBuffer>('get_inspiration_asset', { assetId })),
  openLink: (url) => invoke<void>('open_inspiration_link', { url }),
};
