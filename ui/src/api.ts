import { invoke } from '@tauri-apps/api/core';

import type {
  CatalogSnapshot,
  Collection,
  FamilyDownloadReport,
  FontCategory,
  FontItem,
  FontPage,
  LibraryTab,
  GooglePreview,
  ScanReport,
  StartupMaintenanceReport,
  StartupSnapshot,
} from './types';

export interface FontManagerApi {
  initializeApp(): Promise<StartupSnapshot>;
  runStartupMaintenance(): Promise<StartupMaintenanceReport>;
  listFonts(query: {
    tab: LibraryTab;
    search: string;
    category: FontCategory;
    collectionId: number | null;
    page: number;
    pageSize: number;
  }): Promise<FontPage>;
  listCollections(): Promise<Collection[]>;
  createCollection(name: string): Promise<Collection>;
  addFontToCollection(collectionId: number, fontId: string): Promise<void>;
  setFontFavorite(fontId: string, favorite: boolean): Promise<void>;
  setFontActive(fontId: string, active: boolean): Promise<void>;
  getFontPreviewBytes(fontId: string): Promise<Uint8Array>;
  getFontSnippet(fontId: string, kind: 'css' | 'nextjs'): Promise<string>;
  scanDirectory(path: string): Promise<ScanReport>;
  refreshGoogleCatalog(): Promise<CatalogSnapshot>;
  previewGoogleVariant(familyId: string, variantId: string): Promise<GooglePreview>;
  installGoogleVariant(familyId: string, variantId: string): Promise<FontItem>;
  downloadGoogleFamily(familyId: string): Promise<FamilyDownloadReport>;
}

async function invokePreview(fontId: string): Promise<Uint8Array> {
  const result = await invoke<Uint8Array | number[] | ArrayBuffer>(
    'get_font_preview_bytes',
    { fontId },
  );
  if (result instanceof Uint8Array) return result;
  if (result instanceof ArrayBuffer) return new Uint8Array(result);
  return Uint8Array.from(result);
}

export const fontManagerApi: FontManagerApi = {
  initializeApp: () => invoke('initialize_app'),
  runStartupMaintenance: () => invoke('run_startup_maintenance'),
  listFonts: (query) => invoke('list_fonts', query),
  listCollections: () => invoke('list_collections'),
  createCollection: (name) => invoke('create_collection', { name }),
  addFontToCollection: (collectionId, fontId) =>
    invoke('add_font_to_collection', { collectionId, fontId }),
  setFontFavorite: (fontId, favorite) =>
    invoke('set_font_favorite', { fontId, favorite }),
  setFontActive: (fontId, active) => invoke('set_font_active', { fontId, active }),
  getFontPreviewBytes: invokePreview,
  getFontSnippet: (fontId, kind) => invoke('get_font_snippet', { fontId, kind }),
  scanDirectory: (path) => invoke('scan_directory', { path }),
  refreshGoogleCatalog: () => invoke('refresh_google_catalog'),
  previewGoogleVariant: (familyId, variantId) =>
    invoke('preview_google_variant', { familyId, variantId }),
  installGoogleVariant: (familyId, variantId) =>
    invoke('install_google_variant', { familyId, variantId }),
  downloadGoogleFamily: (familyId) =>
    invoke('download_google_family', { familyId }),
};
