export type LibraryTab = 'local' | 'active' | 'favorites' | 'google';
export type RemoteFontProvider = 'google' | 'fontsource-extra' | 'fontshare' | 'nerd-fonts';
export type FontSource = 'local' | 'google' | 'fontsource' | 'fontshare' | 'nerd-fonts';
export type FontCategory =
  | 'all'
  | 'unknown'
  | 'sans-serif'
  | 'serif'
  | 'display'
  | 'handwriting'
  | 'monospace';

export interface FontItem {
  id: string;
  family: string;
  subfamily: string;
  fullName: string;
  weight: number;
  isItalic: boolean;
  filePath: string;
  isVariable: boolean;
  source: FontSource;
  sourceFamilyId: string | null;
  sourceVariantId: string | null;
  category: FontCategory;
  isActive: boolean;
  isInstalled: boolean;
  isFavorite: boolean;
  fileSize: number;
  fileModifiedUnixMs: number;
  missingOnDisk: boolean;
}

export interface FontPage {
  items: FontItem[];
  total: number;
  page: number;
  pageSize: number;
}

export interface Collection {
  id: number;
  name: string;
  fontCount: number;
}

export interface GoogleFontCatalogItem {
  id: string;
  family: string;
  subsets: string[];
  category: string;
  defVariant: string;
  defSubset: string;
  variants: string[];
  popularity: number;
}

export interface CatalogSnapshot {
  items: GoogleFontCatalogItem[];
  stale: boolean;
}

export interface RemoteFontVariant {
  id: string;
  label: string;
  weight: number;
  isItalic: boolean;
  isVariable?: boolean;
}

export interface RemoteFontCatalogItem {
  id: string;
  provider: RemoteFontProvider;
  family: string;
  category: FontCategory;
  variants: RemoteFontVariant[];
  previewVariantId: string | null;
  isVariable?: boolean;
  downloadStrategy: 'direct-files' | 'zip-archive';
  downloadUrl: string | null;
}

export interface RemoteFontCatalogSnapshot {
  provider: RemoteFontProvider;
  items: RemoteFontCatalogItem[];
  fetchedAtUnixMs: number;
  stale: boolean;
  lastError: string | null;
}

export interface RemoteFontInstallReport {
  provider: RemoteFontProvider;
  familyId: string;
  family: string;
  savedFiles: number;
  indexedFonts: number;
  activatedFonts: number;
  failures: { variantId: string; stage: string; reason: string }[];
}

export interface CloudFontImportOutcome {
  report: RemoteFontInstallReport;
  fontIds: string[];
}

export interface ScanRoot {
  path: string;
  addedAtUnixMs: number;
  lastScannedAtUnixMs: number | null;
  isEnabled: boolean;
  lastDiscoveredCandidates: number;
  lastImported: number;
  lastUpdated: number;
  lastSkippedUnchanged: number;
  lastSkippedUnsupported: number;
  lastFailed: number;
  lastError: string | null;
}

export interface ScanRootSettings {
  root: ScanRoot;
  isAccessible: boolean;
}

export interface RemoteCatalogStatus {
  provider: RemoteFontProvider;
  available: boolean;
  itemCount: number;
  fetchedAtUnixMs: number | null;
  stale: boolean | null;
  lastError: string | null;
}

export interface AppSettingsSnapshot {
  dataDirectory: string;
  cacheDirectory: string;
  fontVaultDirectory: string;
  windowsFontsDirectory: string;
  scanRoots: ScanRootSettings[];
  cloudCatalogs: RemoteCatalogStatus[];
}

export interface GooglePreview {
  fontDataBase64: string;
  family: string;
  subfamily: string;
  weight: number;
  isItalic: boolean;
}

export interface FamilyDownloadFailure {
  variantId: string;
  reason: string;
}

export interface FamilyDownloadReport {
  installed: FontItem[];
  failed: FamilyDownloadFailure[];
}

export interface ScanReport {
  root: string;
  discoveredCandidates: number;
  imported: number;
  updated: number;
  skippedUnchanged: number;
  skippedUnsupported: number;
  failed: { path: string; reason: string }[];
}

export interface StartupSnapshot {
  fonts: FontPage;
  collections: Collection[];
  activeCount: number;
  googleCatalog: CatalogSnapshot | null;
  googleError: string | null;
  dataDirectory: string;
}

export interface StartupMaintenanceReport {
  restored: number;
  restoreFailures: string[];
  scanReports: ScanReport[];
  errors: string[];
}
