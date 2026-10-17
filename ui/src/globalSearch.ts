import type { FontCategory, FontItem, RemoteFontCatalogItem, RemoteFontCatalogSnapshot, RemoteFontProvider } from './types';
import { normalizeCategory } from './categories';

export interface GlobalFontResult { provider: RemoteFontProvider; font: RemoteFontCatalogItem }
export interface GlobalProviderSnapshot { provider: RemoteFontProvider; snapshot: RemoteFontCatalogSnapshot }
export interface GlobalFontFilters { query: string; category: FontCategory; variable: 'any' | 'variable' | 'static' | 'unknown' }

export function identityKey(row: GlobalFontResult): string { return `${row.provider}:${row.font.id}`; }

export function normalizeFamilyName(value: string): string {
  return value.normalize('NFKC').trim().replace(/\s+/gu, ' ').toLocaleLowerCase().replace(/ß/gu, 'ss').replace(/ς/gu, 'σ');
}

export function rankGlobalFonts(rows: GlobalFontResult[], _query: string): GlobalFontResult[] {
  return [...rows].sort((a, b) => a.font.family.localeCompare(b.font.family, undefined, { sensitivity: 'base' }) || identityKey(a).localeCompare(identityKey(b)));
}

export interface GlobalFontGroup { key: string; family: string; sources: GlobalFontResult[] }

export function groupGlobalFonts(rows: GlobalFontResult[]): GlobalFontGroup[] {
  const groups = new Map<string, GlobalFontGroup>();
  for (const row of rankGlobalFonts(rows, '')) {
    const key = normalizeFamilyName(row.font.family);
    const group = groups.get(key);
    if (group) group.sources.push(row);
    else groups.set(key, { key, family: row.font.family.normalize('NFKC').trim().replace(/\s+/gu, ' '), sources: [row] });
  }
  return [...groups.values()].sort((a, b) => a.family.localeCompare(b.family, undefined, { sensitivity: 'base' }) || a.key.localeCompare(b.key));
}

export function filterGlobalFonts(providers: GlobalProviderSnapshot[], filters: GlobalFontFilters): GlobalFontResult[] {
  const query = filters.query.trim().normalize('NFKC').toLocaleLowerCase();
  const rows = providers.flatMap(({ provider, snapshot }) => snapshot.items.filter((font) => {
    const category = normalizeCategory(font.category);
    if (filters.category !== 'all' && category !== filters.category) return false;
    const variable = font.isVariable ?? (font.variants.length > 0 && font.variants.every((variant) => variant.isVariable !== undefined)
      ? font.variants.some((variant) => variant.isVariable) : undefined);
    if (filters.variable === 'variable' && variable !== true) return false;
    if (filters.variable === 'static' && variable !== false) return false;
    if (filters.variable === 'unknown' && variable !== undefined) return false;
    const searchable = `${font.family} ${font.category} ${font.variants.map((variant) => variant.label).join(' ')}`.toLocaleLowerCase();
    return !query || searchable.includes(query);
  }).map((font) => ({ provider, font } satisfies GlobalFontResult)));
  return rankGlobalFonts(rows, filters.query);
}

export function isInstalled(font: RemoteFontCatalogItem, provider: RemoteFontProvider, installedFonts: FontItem[], variantId?: string): boolean {
  const source = provider === 'google' ? 'google' : provider === 'fontsource-extra' ? 'fontsource' : provider;
  return installedFonts.some((item) => item.isInstalled && item.source === source && item.sourceFamilyId === font.id && (provider !== 'google' || item.sourceVariantId === (variantId ?? font.previewVariantId ?? font.variants[0]?.id)));
}
