import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { App } from './App';
import type { FontManagerApi } from './api';
import type { StartupSnapshot } from './types';

const dragDropMock = vi.hoisted(() => {
  type TestEvent = { payload: { type: string; paths?: string[] } };
  const state: { listener?: (event: TestEvent) => void } = {};
  return {
    state,
    onDragDropEvent: vi.fn((listener: (event: TestEvent) => void) => {
      state.listener = listener;
      return Promise.resolve(() => {
        state.listener = undefined;
      });
    }),
  };
});

vi.mock('@tauri-apps/api/webview', () => ({
  getCurrentWebview: () => ({ onDragDropEvent: dragDropMock.onDragDropEvent }),
}));

const fixture = {
  id: 'fixture-serif',
  family: 'Fixture Serif',
  subfamily: 'Regular',
  fullName: 'Fixture Serif Regular',
  weight: 400,
  isItalic: false,
  filePath: 'C:/fonts/fixture.ttf',
  source: 'local' as const,
  sourceFamilyId: null,
  sourceVariantId: null,
  category: 'serif' as const,
  isActive: false,
  isInstalled: false,
  isFavorite: false,
  fileSize: 4096,
  fileModifiedUnixMs: 0,
  missingOnDisk: false,
};

const secondFixture = {
  ...fixture,
  id: 'fixture-sans',
  family: 'Fixture Sans',
  fullName: 'Fixture Sans Regular',
};

const initial: StartupSnapshot = {
  fonts: { items: [fixture], total: 1, page: 0, pageSize: 50 },
  collections: [],
  activeCount: 0,
  googleCatalog: null,
  googleError: null,
  dataDirectory: 'C:/Users/fixture/AppData/Local/RustFontManager/RustFontManager',
};

function makeApi(): FontManagerApi {
  return {
    initializeApp: vi.fn().mockResolvedValue(initial),
    runStartupMaintenance: vi.fn().mockResolvedValue({
      restored: 0,
      restoreFailures: [],
      scanReports: [],
      errors: [],
    }),
    listFonts: vi.fn().mockResolvedValue(initial.fonts),
    listCollections: vi.fn().mockResolvedValue([]),
    createCollection: vi.fn(),
    addFontToCollection: vi.fn().mockResolvedValue(undefined),
    setFontFavorite: vi.fn().mockResolvedValue(undefined),
    setFontActive: vi.fn().mockResolvedValue(undefined),
    getFontPreviewBytes: vi.fn().mockRejectedValue('preview not available in test'),
    getFontSnippet: vi.fn().mockResolvedValue('font-family: "Fixture Serif";'),
    scanDirectory: vi.fn(),
    importZipFont: vi.fn(),
    refreshGoogleCatalog: vi.fn(),
    previewGoogleVariant: vi.fn(),
    installGoogleVariant: vi.fn(),
    downloadGoogleFamily: vi.fn(),
  };
}

afterEach(() => {
  cleanup();
  dragDropMock.state.listener = undefined;
  dragDropMock.onDragDropEvent.mockClear();
  vi.unstubAllGlobals();
});

describe('font manager workbench', () => {
  it('imports a dropped ZIP and returns to the local library', async () => {
    const api = makeApi();
    const importZipFont = vi.fn().mockResolvedValue({
      root: 'C:/app/imported-fonts/test',
      discoveredCandidates: 1,
      imported: 1,
      updated: 0,
      skippedUnchanged: 0,
      skippedUnsupported: 0,
      failed: [],
    });
    render(<App api={{ ...api, importZipFont } as FontManagerApi} />);

    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await waitFor(() => expect(dragDropMock.state.listener).toBeDefined());
    await act(async () => {
      dragDropMock.state.listener?.({
        payload: { type: 'drop', paths: ['C:/Users/test/Family.zip'] },
      });
    });

    await waitFor(() => expect(importZipFont).toHaveBeenCalledWith('C:/Users/test/Family.zip'));
    await waitFor(() => expect(screen.getByRole('status')).toHaveTextContent(/1 nuovi/));
    expect(screen.getByRole('button', { name: /Libreria locale/ })).toHaveAttribute('aria-current', 'page');
  });

  it('lets the user mark the selected font as a favorite', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    render(<App api={api} />);

    expect(await screen.findByRole('heading', { name: 'Fixture Serif' })).toBeInTheDocument();
    const favorite = await screen.findByRole('button', {
      name: 'Aggiungi ai preferiti',
    });
    await user.click(favorite);

    await waitFor(() => {
      expect(api.setFontFavorite).toHaveBeenCalledWith('fixture-serif', true);
    });
  });

  it('initializes the desktop backend only once when the search changes', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    render(<App api={api} />);

    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await user.type(screen.getByRole('searchbox', { name: /Cerca font/ }), 'Serif');

    await waitFor(() => {
      expect(api.initializeApp).toHaveBeenCalledTimes(1);
    });
  });

  it('focuses the search field with Ctrl+K', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    render(<App api={api} />);

    const search = await screen.findByRole('searchbox', { name: /Cerca font/ });
    await user.keyboard('{Control>}k{/Control}');

    expect(search).toHaveFocus();
  });

  it('loads a Google Fonts preview automatically when opening the catalog', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    const family = {
      id: 'fraunces',
      family: 'Fraunces',
      subsets: ['latin'],
      category: 'serif',
      defVariant: 'regular',
      defSubset: 'latin',
      variants: ['regular'],
      popularity: 1,
    };
    vi.mocked(api.initializeApp).mockResolvedValue({
      ...initial,
      googleCatalog: { items: [family], stale: false },
    });
    vi.mocked(api.previewGoogleVariant).mockImplementation(() => new Promise(() => {}));

    render(<App api={api} />);
    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await user.click(screen.getByRole('button', { name: /Google Fonts/ }));

    await waitFor(() => {
      expect(api.previewGoogleVariant).toHaveBeenCalledWith('fraunces', 'regular');
    });
    expect(screen.getByText(/CARICAMENTO ANTEPRIMA/)).toBeInTheDocument();
  });

  it('virtualizes the Google catalog and keeps every family reachable in one list', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    const families = Array.from({ length: 301 }, (_, index) => ({
      id: `google-family-${index + 1}`,
      family: `Google family ${index + 1}`,
      subsets: ['latin'],
      category: index === 300 ? 'handwriting' : 'serif',
      defVariant: 'regular',
      defSubset: 'latin',
      variants: ['regular'],
      popularity: index + 1,
    }));
    vi.mocked(api.initializeApp).mockResolvedValue({
      ...initial,
      googleCatalog: { items: families, stale: false },
    });
    vi.mocked(api.previewGoogleVariant).mockResolvedValue({
      fontDataBase64: 'AA==', family: 'Google family', subfamily: 'Regular', weight: 400, isItalic: false,
    });
    vi.stubGlobal('IntersectionObserver', undefined);

    render(<App api={api} />);
    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await user.click(screen.getByRole('button', { name: /Google Fonts/ }));

    const catalogList = document.querySelector('.google-font-list') as HTMLDivElement;
    await waitFor(() => expect(catalogList.querySelectorAll('.google-font-row').length).toBeGreaterThan(0));
    const mountedRows = catalogList.querySelectorAll('.google-font-row').length;
    expect(mountedRows).toBeLessThan(30);
    expect(mountedRows).toBeLessThan(families.length);
    expect(screen.queryByLabelText('Paginazione')).not.toBeInTheDocument();
    expect(screen.getByRole('textbox', { name: 'Testo di anteprima' })).toHaveValue('Sphinx of black quartz, judge my vow.');
    expect(screen.getByRole('slider', { name: 'Dimensione anteprima' })).toHaveValue('42');
    expect(screen.getAllByText('Sphinx of black quartz, judge my vow.', { selector: '.google-font-row .font-row-sample' })).toHaveLength(mountedRows);

    catalogList.scrollTop = 300 * 108;
    fireEvent.scroll(catalogList);
    expect(await screen.findByRole('button', { name: /Google family 301/ })).toBeInTheDocument();
    await waitFor(() => expect(api.previewGoogleVariant).toHaveBeenCalledWith('google-family-301', 'regular'));

    await user.selectOptions(screen.getByRole('combobox', { name: 'Categoria font' }), 'handwriting');
    expect(screen.getByRole('button', { name: /Google family 301/ })).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Google family 1\b/ })).not.toBeInTheDocument();
  });

  it('shows an installed Google variant in the local library without a separate vault list', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    const googleFont = {
      ...fixture,
      id: 'google-installed-font',
      family: 'Google Display',
      fullName: 'Google Display Regular',
      source: 'google' as const,
      sourceFamilyId: 'google-display',
      sourceVariantId: 'regular',
    };
    const googlePage = { items: [googleFont], total: 1, page: 0, pageSize: 50 };
    let installed = false;
    vi.mocked(api.initializeApp).mockResolvedValue({
      ...initial,
      googleCatalog: {
        items: [{
          id: 'google-display', family: 'Google Display', subsets: ['latin'], category: 'display',
          defVariant: 'regular', defSubset: 'latin', variants: ['regular'], popularity: 1,
        }],
        stale: false,
      },
    });
    vi.mocked(api.listFonts).mockImplementation(async (query) => {
      if (query.tab === 'active') return { items: [], total: 0, page: 0, pageSize: query.pageSize };
      if (query.tab === 'local' && installed) return googlePage;
      return initial.fonts;
    });
    vi.mocked(api.installGoogleVariant).mockImplementation(async () => {
      installed = true;
      return googleFont;
    });
    vi.mocked(api.previewGoogleVariant).mockImplementation(() => new Promise(() => {}));
    vi.stubGlobal('IntersectionObserver', undefined);

    render(<App api={api} />);
    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await user.click(screen.getByRole('button', { name: /Google Fonts/ }));
    await screen.findByRole('heading', { name: 'Google Display' });
    await user.click(await screen.findByRole('button', { name: 'Scarica variante' }));

    await waitFor(() => expect(api.installGoogleVariant).toHaveBeenCalledWith('google-display', 'regular'));
    expect(await screen.findByRole('heading', { name: 'Google Display' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Libreria locale/ })).toHaveAttribute('aria-current', 'page');
    expect(api.listFonts).toHaveBeenCalledWith(expect.objectContaining({ tab: 'local' }));
    expect(document.querySelector('.downloaded-section')).not.toBeInTheDocument();
  });

  it('returns to the local library after downloading a Google family', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    const googleFont = {
      ...fixture,
      id: 'google-family-font',
      family: 'Google Display',
      fullName: 'Google Display Regular',
      source: 'google' as const,
      sourceFamilyId: 'google-display',
      sourceVariantId: 'regular',
    };
    let installed = false;
    vi.mocked(api.initializeApp).mockResolvedValue({
      ...initial,
      googleCatalog: {
        items: [{
          id: 'google-display', family: 'Google Display', subsets: ['latin'], category: 'display',
          defVariant: 'regular', defSubset: 'latin', variants: ['regular'], popularity: 1,
        }],
        stale: false,
      },
    });
    vi.mocked(api.listFonts).mockImplementation(async (query) =>
      query.tab === 'local' && installed
        ? { items: [googleFont], total: 1, page: 0, pageSize: query.pageSize }
        : initial.fonts,
    );
    vi.mocked(api.downloadGoogleFamily).mockImplementation(async () => {
      installed = true;
      return { installed: [googleFont], failed: [] };
    });
    vi.mocked(api.previewGoogleVariant).mockImplementation(() => new Promise(() => {}));
    vi.stubGlobal('IntersectionObserver', undefined);

    render(<App api={api} />);
    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await user.click(screen.getByRole('button', { name: /Google Fonts/ }));
    await user.click(await screen.findByRole('button', { name: /Scarica famiglia/ }));
    await user.click(screen.getByRole('button', { name: 'Conferma' }));

    await waitFor(() => expect(api.downloadGoogleFamily).toHaveBeenCalledWith('google-display'));
    expect(screen.getByRole('button', { name: /Libreria locale/ })).toHaveAttribute('aria-current', 'page');
    expect(await screen.findByRole('button', { name: /Google Display/ })).toBeInTheDocument();
  });

  it('keeps active fonts visible in the local library without a temporary-activation tab', async () => {
    const api = makeApi();
    const activeFont = { ...fixture, isActive: true, isInstalled: true };
    const activePage = { items: [activeFont], total: 1, page: 0, pageSize: 50 };
    vi.mocked(api.initializeApp).mockResolvedValue({
      ...initial,
      fonts: activePage,
      activeCount: 1,
    });
    vi.mocked(api.listFonts).mockResolvedValue(activePage);
    render(<App api={api} />);

    await screen.findByRole('heading', { name: 'Fixture Serif' });
    expect(screen.queryByRole('button', { name: /Attivazione temporanea/ })).not.toBeInTheDocument();
    expect(screen.queryByText(/temporane/)).not.toBeInTheDocument();
    expect(screen.getByText('1 font installato')).toBeInTheDocument();
    expect(screen.getAllByText('Installato')).toHaveLength(2);
  });

  it('loads every local font page into one scrollable list', async () => {
    const api = makeApi();
    const firstPage = { items: [fixture], total: 2, page: 0, pageSize: 1 };
    const secondPage = { items: [secondFixture], total: 2, page: 1, pageSize: 1 };
    vi.mocked(api.initializeApp).mockResolvedValue({ ...initial, fonts: firstPage });
    vi.mocked(api.listFonts).mockImplementation(async (query) => {
      if (query.tab === 'active') return { items: [], total: 0, page: query.page, pageSize: query.pageSize };
      return query.page === 0 ? firstPage : secondPage;
    });

    render(<App api={api} />);

    expect(await screen.findByRole('button', { name: /Fixture Sans/ })).toBeInTheDocument();
    expect(api.listFonts).toHaveBeenCalledWith(expect.objectContaining({ tab: 'local', page: 0, pageSize: 100 }));
    expect(api.listFonts).toHaveBeenCalledWith(expect.objectContaining({ tab: 'local', page: 1, pageSize: 1 }));
  });

  it('requests previews automatically for unselected local fonts', async () => {
    const api = makeApi();
    const fonts = { items: [fixture, secondFixture], total: 2, page: 0, pageSize: 50 };
    vi.mocked(api.initializeApp).mockResolvedValue({ ...initial, fonts });
    vi.mocked(api.listFonts).mockImplementation(async (query) =>
      query.tab === 'active' ? { items: [], total: 0, page: 0, pageSize: 1 } : fonts,
    );
    vi.stubGlobal('IntersectionObserver', undefined);

    render(<App api={api} />);

    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await waitFor(() => {
      expect(api.getFontPreviewBytes).toHaveBeenCalledWith('fixture-sans');
    });
  });

  it('activates a font without describing activation as temporary', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    render(<App api={api} />);

    await screen.findByRole('heading', { name: 'Fixture Serif' });
    await user.click(screen.getByRole('button', { name: 'Installa' }));

    expect(api.setFontActive).toHaveBeenCalledWith('fixture-serif', true);
    expect(await screen.findByText('Font installato su Windows.')).toBeInTheDocument();
    expect(screen.queryByText(/temporane/)).not.toBeInTheDocument();
  });

  it('shows fonts installed outside OFM as installed without offering uninstall', async () => {
    const api = makeApi();
    const systemFont = { ...fixture, isInstalled: true, isActive: false };
    vi.mocked(api.initializeApp).mockResolvedValue({
      ...initial,
      fonts: { items: [systemFont], total: 1, page: 0, pageSize: 50 },
    });
    vi.mocked(api.listFonts).mockResolvedValue({
      items: [systemFont], total: 1, page: 0, pageSize: 50,
    });

    render(<App api={api} />);

    expect(await screen.findByRole('button', { name: 'Già installato' })).toBeDisabled();
    expect(screen.getAllByText('Installato').length).toBeGreaterThan(0);
    expect(api.setFontActive).not.toHaveBeenCalled();
  });

  it('offers uninstall for a font already installed in Windows', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    const installedFont = { ...fixture, isActive: true, isInstalled: true };
    api.initializeApp = vi.fn().mockResolvedValue({
      ...initial,
      fonts: { items: [installedFont], total: 1, page: 0, pageSize: 50 },
      activeCount: 1,
    });
    api.listFonts = vi.fn().mockResolvedValue({
      items: [installedFont], total: 1, page: 0, pageSize: 50,
    });

    render(<App api={api} />);
    await user.click(await screen.findByRole('button', { name: 'Disinstalla' }));

    expect(api.setFontActive).toHaveBeenCalledWith('fixture-serif', false);
  });
});
