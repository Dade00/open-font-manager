import { act, cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { CloudCatalogView } from './CloudCatalogView';
import type { FontManagerApi } from './api';
import type { RemoteFontCatalogSnapshot } from './types';

const snapshot: RemoteFontCatalogSnapshot = {
  provider: 'fontshare',
  fetchedAtUnixMs: 1,
  stale: false,
  lastError: null,
  items: [{
    id: 'family-id',
    provider: 'fontshare',
    family: 'Fixture Serif',
    category: 'serif',
    variants: [{ id: 'regular', label: 'Regular · 400', weight: 400, isItalic: false }],
    previewVariantId: 'regular',
    downloadStrategy: 'zip-archive',
    downloadUrl: 'https://cdn.example.invalid/font.zip',
  }],
};

afterEach(() => cleanup());

function makeApi(overrides: Record<string, unknown> = {}) {
  return {
    loadCachedRemoteCatalog: vi.fn().mockResolvedValue(snapshot),
    refreshRemoteCatalog: vi.fn().mockResolvedValue(snapshot),
    getRemotePreviewBytes: vi.fn().mockResolvedValue(new Uint8Array([1, 2, 3])),
    downloadRemoteFamily: vi.fn().mockResolvedValue({
      report: {
        provider: 'fontshare', familyId: 'family-id', family: 'Fixture Serif',
        savedFiles: 1, indexedFonts: 1, activatedFonts: 1, failures: [],
      },
      fontIds: ['font-id'],
    }),
    ...overrides,
  } as unknown as FontManagerApi;
}

describe('remote cloud catalog', () => {
  it('ignores a refresh response after the provider changes', async () => {
    let resolveRefresh!: (value: RemoteFontCatalogSnapshot) => void;
    const api = makeApi({
      loadCachedRemoteCatalog: vi.fn().mockImplementation((provider: string) => Promise.resolve({ ...snapshot, provider, items: [{ ...snapshot.items[0], id: provider, family: provider }] })),
      refreshRemoteCatalog: vi.fn().mockImplementation(() => new Promise((resolve) => { resolveRefresh = resolve; })),
    });
    const { rerender } = render(<CloudCatalogView api={api} onInstalled={vi.fn()} provider="fontshare" />);
    await screen.findByRole('button', { name: /fontshare .*Aa Bb Cc 0123/ });
    fireEvent.click(screen.getByRole('button', { name: 'Aggiorna catalogo' }));
    rerender(<CloudCatalogView api={api} onInstalled={vi.fn()} provider="fontsource-extra" />);
    await screen.findByRole('button', { name: /fontsource-extra Serif · Regular · 400 Aa Bb Cc 0123/ });
    await act(async () => {
      resolveRefresh({ ...snapshot, provider: 'fontshare', items: [{ ...snapshot.items[0], family: 'Risposta obsoleta' }] });
    });
    expect(screen.queryByText('Risposta obsoleta')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Aggiorna catalogo' })).toBeEnabled();
  });

  it('resets the page after refreshing to a smaller catalog', async () => {
    const many = { ...snapshot, items: Array.from({ length: 120 }, (_, index) => ({ ...snapshot.items[0], id: `family-${index}`, family: `Family ${index}` })) };
    const api = makeApi({ loadCachedRemoteCatalog: vi.fn().mockResolvedValue(many), refreshRemoteCatalog: vi.fn().mockResolvedValue(snapshot) });
    const user = userEvent.setup();
    render(<CloudCatalogView api={api} onInstalled={vi.fn()} provider="fontshare" />);
    await user.click(await screen.findByRole('button', { name: 'Successiva' }));
    expect(screen.getByText('2 / 2')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Aggiorna catalogo' }));
    await screen.findByRole('button', { name: 'Fixture Serif Serif · Regular · 400 Aa Bb Cc 0123' })
    expect(screen.queryByText(/ \/ 2$/)).not.toBeInTheDocument();
  });

  it('loads the catalog without fetching preview bytes until a family is selected', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    render(<CloudCatalogView api={api} onInstalled={vi.fn()} provider="fontshare" />);

    const family = await screen.findByRole('button', { name: 'Fixture Serif Serif · Regular · 400 Aa Bb Cc 0123' });
    expect(api.getRemotePreviewBytes).not.toHaveBeenCalled();
    await user.click(family);

    await waitFor(() => {
      expect(api.getRemotePreviewBytes).toHaveBeenCalledWith('fontshare', 'family-id', 'regular');
    });
  });

  it('downloads and activates a selected remote family through the backend', async () => {
    const user = userEvent.setup();
    const api = makeApi();
    const onInstalled = vi.fn();
    render(<CloudCatalogView api={api} onInstalled={onInstalled} provider="fontshare" />);

      const familyButton = await screen.findByRole('button', { name: 'Fixture Serif Serif · Regular · 400 Aa Bb Cc 0123' });
      await user.click(familyButton);
    await user.click(screen.getByRole('button', { name: 'Installa' }));

    await waitFor(() => {
      expect(api.downloadRemoteFamily).toHaveBeenCalledWith('fontshare', 'family-id', null, true);
      expect(onInstalled).toHaveBeenCalledWith(
        ['font-id'],
        'Fixture Serif: 1 file salvati, 1 font indicizzati, 1 attivati.',
        false,
      );
    });
  });

  it('shows a retry action when the initial catalog request fails', async () => {
    const api = makeApi({
      loadCachedRemoteCatalog: vi.fn().mockResolvedValue(null),
      refreshRemoteCatalog: vi.fn().mockRejectedValue(new Error('Offline')),
    });
    render(<CloudCatalogView api={api} onInstalled={vi.fn()} provider="fontsource-extra" />);

    expect(await screen.findByRole('alert')).toHaveTextContent('Offline');
    expect(screen.getByRole('button', { name: 'Riprova' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Riprova' }));
    await waitFor(() => expect(api.refreshRemoteCatalog).toHaveBeenCalledTimes(2));
  });
});
