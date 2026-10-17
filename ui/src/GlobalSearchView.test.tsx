import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { GlobalSearchView } from './GlobalSearchView';
import type { FontManagerApi } from './api';
import type { RemoteFontCatalogItem, RemoteFontCatalogSnapshot, RemoteFontProvider } from './types';

const providers: RemoteFontProvider[] = ['google', 'fontsource-extra', 'fontshare', 'nerd-fonts'];
function catalog(provider: RemoteFontProvider, id = 'family', family = 'Atlas'): RemoteFontCatalogSnapshot {
  const font: RemoteFontCatalogItem = { id, provider, family, category: 'sans-serif', variants: [{ id: 'regular', label: 'Regular', weight: 400, isItalic: false }], previewVariantId: 'regular', downloadStrategy: 'direct-files', downloadUrl: null };
  return { provider, items: [font], fetchedAtUnixMs: 0, stale: false, lastError: null };
}
function mockApi(overrides: Partial<FontManagerApi> = {}): FontManagerApi {
  return { initializeApp: vi.fn(), runStartupMaintenance: vi.fn(), listFonts: vi.fn(), listCollections: vi.fn(), createCollection: vi.fn(), addFontToCollection: vi.fn(), setFontFavorite: vi.fn(), setFontActive: vi.fn(), getFontPreviewBytes: vi.fn(), getFontSnippet: vi.fn(), scanDirectory: vi.fn(), importZipFont: vi.fn(), refreshGoogleCatalog: vi.fn(), previewGoogleVariant: vi.fn(), installGoogleVariant: vi.fn(), downloadGoogleFamily: vi.fn(), loadCachedRemoteCatalog: vi.fn(async (provider) => catalog(provider)), refreshRemoteCatalog: vi.fn(async (provider) => catalog(provider)), getRemotePreviewBytes: vi.fn(async () => new Uint8Array()), downloadRemoteFamily: vi.fn(async (provider, familyId) => ({ report: { provider, familyId, family: 'Atlas', savedFiles: 1, indexedFonts: 1, activatedFonts: 1, failures: [] }, fontIds: ['font-id'] })), getAppSettings: vi.fn(), addScanRoot: vi.fn(), scanScanRoot: vi.fn(), setScanRootEnabled: vi.fn(), removeScanRoot: vi.fn(), ...overrides } as unknown as FontManagerApi;
}
const disableIntersectionObserver = () => { vi.stubGlobal('IntersectionObserver', undefined); };
beforeEach(() => { vi.unstubAllGlobals(); cleanup(); });
afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

describe('GlobalSearchView', () => {
  it('mostra subito tutte le famiglie in ordine alfabetico con una sola riga per famiglia', async () => {
    disableIntersectionObserver(); const api = mockApi(); render(<GlobalSearchView api={api} installedFonts={[]} onInstalled={vi.fn()} />);
    await waitFor(() => expect(api.loadCachedRemoteCatalog).toHaveBeenCalledTimes(4));
    expect(screen.getAllByText('Atlas')).toHaveLength(1);
    expect(screen.getByRole('status')).toHaveTextContent('1 famiglie');
    await waitFor(() => expect(api.refreshRemoteCatalog).not.toHaveBeenCalled());
    expect((api.loadCachedRemoteCatalog as ReturnType<typeof vi.fn>).mock.calls.map(([provider]) => provider)).toEqual(expect.arrayContaining(providers));
  });
  it('offre le fonti omonime e usa l’origine selezionata per preview e installazione', async () => {
    disableIntersectionObserver();
    const api = mockApi({ loadCachedRemoteCatalog: vi.fn(async (provider) => catalog(provider, 'shared', provider === 'fontshare' ? 'Atlas' : 'Atlas')) });
    render(<GlobalSearchView api={api} installedFonts={[]} onInstalled={vi.fn()} />);
    await waitFor(() => expect(api.loadCachedRemoteCatalog).toHaveBeenCalledTimes(4));
    expect(screen.getAllByText('Atlas')).toHaveLength(1);
    fireEvent.click(screen.getByRole('button', { name: /Atlas/ }));
    fireEvent.change(screen.getByLabelText('Provenienza'), { target: { value: 'fontshare:shared' } });
    await waitFor(() => expect(api.getRemotePreviewBytes).toHaveBeenCalledWith('fontshare', 'shared', 'regular'));
    fireEvent.click(screen.getAllByRole('button', { name: 'Installa' }).at(-1)!);
    await waitFor(() => expect(api.downloadRemoteFamily).toHaveBeenCalledWith('fontshare', 'shared', null, true));
  });
  it('isola provider falliti e mostra i risultati degli altri cataloghi', async () => {
    disableIntersectionObserver(); const api = mockApi({ loadCachedRemoteCatalog: vi.fn(async (provider) => provider === 'fontshare' ? null : catalog(provider)), refreshRemoteCatalog: vi.fn(async (provider) => { if (provider === 'fontshare') throw new Error('offline'); return catalog(provider); }) });
    render(<GlobalSearchView api={api} installedFonts={[]} onInstalled={vi.fn()} />); fireEvent.change(screen.getByRole('searchbox', { name: 'Cerca font per famiglia, categoria o stile' }), { target: { value: 'Atlas' } });
    expect(await screen.findAllByText('Atlas')).not.toHaveLength(0); expect(await screen.findByText(/Fontshare: errore/)).toBeInTheDocument();
  });
  it('ignora risposte stale di query superate', async () => {
    disableIntersectionObserver(); const api = mockApi();
    render(<GlobalSearchView api={api} installedFonts={[]} onInstalled={vi.fn()} />);
    fireEvent.change(screen.getByRole('searchbox', { name: 'Cerca font per famiglia, categoria o stile' }), { target: { value: 'Atlas' } });
    await waitFor(() => expect(screen.getAllByText('Atlas')).toHaveLength(1));
  });
  it('mantiene la cache disponibile se il refresh di un provider fallisce', async () => {
    disableIntersectionObserver();
    const api = mockApi({ loadCachedRemoteCatalog: vi.fn(async (provider) => provider === 'fontshare' ? null : catalog(provider)), refreshRemoteCatalog: vi.fn(async (provider: RemoteFontProvider) => { if (provider === 'fontshare') throw new Error('offline'); return catalog(provider); }) });
    render(<GlobalSearchView api={api} installedFonts={[]} onInstalled={vi.fn()} />);
    await waitFor(() => expect(api.loadCachedRemoteCatalog).toHaveBeenCalledTimes(4));
    expect(await screen.findByText(/errore/)).toBeInTheDocument();
    expect(screen.getAllByText('Atlas').length).toBeGreaterThan(0);
    expect(screen.getByRole('button', { name: 'Riprova' })).toBeInTheDocument();
  });
  it('installa la singola variante Google e invia il report tramite callback', async () => {
    disableIntersectionObserver(); const callback = vi.fn(); const api = mockApi(); render(<GlobalSearchView api={api} installedFonts={[]} onInstalled={callback} />);
    fireEvent.change(screen.getByRole('searchbox', { name: 'Cerca font per famiglia, categoria o stile' }), { target: { value: 'Atlas' } }); fireEvent.click(await screen.findByRole('button', { name: /Atlas/ })); fireEvent.change(screen.getByLabelText('Provenienza'), { target: { value: 'google:family' } });
    const installButtons = screen.getAllByRole('button', { name: 'Installa' }); fireEvent.click(installButtons[installButtons.length - 1]); await waitFor(() => expect(api.downloadRemoteFamily).toHaveBeenCalledWith('google', 'family', 'regular', true));
    expect(callback).toHaveBeenCalledWith(['font-id'], expect.stringContaining('variante richiesta'), false);
  });
});
