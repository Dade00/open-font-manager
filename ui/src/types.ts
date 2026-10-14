export type LibraryTab = 'local' | 'active' | 'favorites' | 'google';
export type FontSource = 'local' | 'google';
export type FontCategory =
  | 'all'
  | 'unknown'
  | 'sans-serif'
  | 'serif'
  | 'display'
  | 'monospace';

export interface FontItem {
  id: string;
  family: string;
  subfamily: string;
  fullName: string;
  weight: number;
  isItalic: boolean;
  filePath: string;
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
