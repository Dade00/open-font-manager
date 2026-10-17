import { describe, expect, it } from 'vitest';
import type { RemoteFontCatalogItem, RemoteFontCatalogSnapshot } from './types';
import { filterGlobalFonts, groupGlobalFonts, rankGlobalFonts, type GlobalFontFilters } from './globalSearch';

const item = (id: string, family: string, category: string, variable?: boolean): RemoteFontCatalogItem => ({
  id, family, provider: 'google', category: category as RemoteFontCatalogItem['category'],
  variants: [{ id: 'regular', label: 'Regular', weight: 400, isItalic: false }],
  previewVariantId: 'regular', isVariable: variable, downloadStrategy: 'direct-files', downloadUrl: null,
});
const snapshot = (items: RemoteFontCatalogItem[]): RemoteFontCatalogSnapshot => ({ provider: 'google', items, fetchedAtUnixMs: 0, stale: false, lastError: null });

describe('ricerca globale', () => {
  it('ordina alfabeticamente e raggruppa nomi equivalenti senza perdere identità fonte', () => {
    const rows = [
      { provider: 'google' as const, font: item('n', 'Nerd Mono', 'monospace') },
      { provider: 'fontshare' as const, font: item('id-b', '  Ａtlas  Pro ', 'sans-serif') },
      { provider: 'google' as const, font: item('id-a', 'Atlas   Pro', 'sans-serif') },
      { provider: 'nerd-fonts' as const, font: item('p', 'Nerd Propo', 'monospace') },
    ];
    const groups = groupGlobalFonts(rankGlobalFonts(rows, 'nerd'));
    expect(groups.map((group) => group.family)).toEqual(['Atlas Pro', 'Nerd Mono', 'Nerd Propo']);
    expect(groups[0].sources.map((row) => `${row.provider}:${row.font.id}`)).toEqual(['fontshare:id-b', 'google:id-a']);
  });

  it('filtra categoria e mantiene variable sconosciuto distinto da statico', () => {
    const rows = ['google', 'fontshare', 'nerd-fonts'].map((provider) => ({ provider: provider as 'google', snapshot: snapshot([item('same', 'Same Name', 'sans-serif', undefined)]) }));
    expect(filterGlobalFonts(rows, { query: 'same', category: 'sans-serif', variable: 'variable' })).toHaveLength(0);
    expect(filterGlobalFonts(rows, { query: 'same', category: 'sans-serif', variable: 'unknown' })).toHaveLength(3);
    expect(filterGlobalFonts(rows, { query: '', category: 'unknown', variable: 'any' })).toHaveLength(0);
  });

  it('ricerca metadata degli stili e non fonde omonimi da provider diversi', () => {
    const rows = ['google', 'fontshare'].map((provider) => ({ provider: provider as 'google', snapshot: snapshot([item('id', 'Atlas', 'sans-serif')]) }));
    rows[1].snapshot.items[0].variants[0].label = 'Italic';
    const result = (filters: GlobalFontFilters) => filterGlobalFonts(rows, filters).map((row) => `${row.provider}:${row.font.id}`);
    expect(result({ query: 'italic', category: 'all', variable: 'any' })).toEqual(['fontshare:id']);
    expect(result({ query: '', category: 'all', variable: 'any' })).toEqual(['fontshare:id', 'google:id']);
  });
});
