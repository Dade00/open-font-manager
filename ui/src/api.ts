import { invoke } from '@tauri-apps/api/core';

import type {
  AppSettingsSnapshot,
  CatalogSnapshot,
  CloudFontImportOutcome,
  Collection,
  FamilyDownloadReport,
  FontCategory,
  FontItem,
  FontPage,
  LibraryTab,
  GooglePreview,
  RemoteFontCatalogSnapshot,
  RemoteFontProvider,
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
  importZipFont(path: string): Promise<ScanReport>;
  refreshGoogleCatalog(): Promise<CatalogSnapshot>;
  previewGoogleVariant(familyId: string, variantId: string): Promise<GooglePreview>;
  installGoogleVariant(familyId: string, variantId: string): Promise<FontItem>;
  downloadGoogleFamily(familyId: string): Promise<FamilyDownloadReport>;
  loadCachedRemoteCatalog(provider: RemoteFontProvider): Promise<RemoteFontCatalogSnapshot | null>;
  refreshRemoteCatalog(provider: RemoteFontProvider): Promise<RemoteFontCatalogSnapshot>;
  getRemotePreviewBytes(provider: RemoteFontProvider, familyId: string, variantId: string): Promise<Uint8Array>;
  downloadRemoteFamily(provider: RemoteFontProvider, familyId: string, variantId: string | null, activate: boolean): Promise<CloudFontImportOutcome>;
  getAppSettings(): Promise<AppSettingsSnapshot>;
  addScanRoot(path: string): Promise<ScanReport>;
  scanScanRoot(path: string): Promise<ScanReport>;
  setScanRootEnabled(path: string, enabled: boolean): Promise<void>;
  removeScanRoot(path: string): Promise<void>;
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

async function invokeRemotePreview(provider: RemoteFontProvider, familyId: string, variantId: string): Promise<Uint8Array> {
  const result = await invoke<Uint8Array | number[] | ArrayBuffer>(
    'get_remote_preview_bytes',
    { provider, familyId, variantId },
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
  importZipFont: (path) => invoke('import_zip_font', { path }),
  refreshGoogleCatalog: () => invoke('refresh_google_catalog'),
  previewGoogleVariant: (familyId, variantId) =>
    invoke('preview_google_variant', { familyId, variantId }),
  installGoogleVariant: (familyId, variantId) =>
    invoke('install_google_variant', { familyId, variantId }),
  downloadGoogleFamily: (familyId) =>
    invoke('download_google_family', { familyId }),
  loadCachedRemoteCatalog: (provider) => invoke('load_cached_remote_catalog', { provider }),
  refreshRemoteCatalog: (provider) => invoke('refresh_remote_catalog', { provider }),
  getRemotePreviewBytes: invokeRemotePreview,
  downloadRemoteFamily: (provider, familyId, variantId, activate) =>
    invoke('download_remote_family', { provider, familyId, variantId, activate }),
  getAppSettings: () => invoke('get_app_settings'),
  addScanRoot: (path) => invoke('add_scan_root', { path }),
  scanScanRoot: (path) => invoke('scan_scan_root', { path }),
  setScanRootEnabled: (path, enabled) => invoke('set_scan_root_enabled', { path, enabled }),
  removeScanRoot: (path) => invoke('remove_scan_root', { path }),
};
