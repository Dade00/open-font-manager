import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { App } from './App';
import type { FontManagerApi } from './api';
import type { StartupSnapshot } from './types';

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
  isFavorite: false,
  fileSize: 4096,
  fileModifiedUnixMs: 0,
  missingOnDisk: false,
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
    refreshGoogleCatalog: vi.fn(),
    previewGoogleVariant: vi.fn(),
    installGoogleVariant: vi.fn(),
    downloadGoogleFamily: vi.fn(),
  };
}

afterEach(() => cleanup());

describe('font manager workbench', () => {
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
});
