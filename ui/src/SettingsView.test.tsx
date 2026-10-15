import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';

import { SettingsView } from './SettingsView';
import type { FontManagerApi } from './api';
import type { AppSettingsSnapshot } from './types';

const dialogMock = vi.hoisted(() => ({ open: vi.fn() }));
vi.mock('@tauri-apps/plugin-dialog', () => ({ open: dialogMock.open }));

afterEach(() => {
  cleanup();
  dialogMock.open.mockReset();
});

const settings: AppSettingsSnapshot = {
  dataDirectory: 'C:/app/data',
  cacheDirectory: 'C:/app/cache',
  fontVaultDirectory: 'C:/app/data/vault',
  windowsFontsDirectory: 'C:/Users/test/Fonts',
  scanRoots: [{
    isAccessible: true,
    root: {
      path: 'C:/fonts', addedAtUnixMs: 1, lastScannedAtUnixMs: 2, isEnabled: true,
      lastDiscoveredCandidates: 3, lastImported: 1, lastUpdated: 0,
      lastSkippedUnchanged: 1, lastSkippedUnsupported: 0, lastFailed: 0, lastError: null,
    },
  }],
  cloudCatalogs: [
    { provider: 'google', available: true, itemCount: 1, fetchedAtUnixMs: 5, stale: false, lastError: null },
    { provider: 'fontsource-extra', available: false, itemCount: 0, fetchedAtUnixMs: null, stale: null, lastError: null },
    { provider: 'fontshare', available: false, itemCount: 0, fetchedAtUnixMs: null, stale: null, lastError: null },
    { provider: 'nerd-fonts', available: false, itemCount: 0, fetchedAtUnixMs: null, stale: null, lastError: null },
  ],
};

function makeApi(overrides: Record<string, unknown> = {}) {
  return {
    getAppSettings: vi.fn().mockResolvedValue(settings),
    addScanRoot: vi.fn().mockResolvedValue({
      root: 'C:/fonts/new', discoveredCandidates: 0, imported: 0, updated: 0,
      skippedUnchanged: 0, skippedUnsupported: 0, failed: [],
    }),
    scanScanRoot: vi.fn().mockResolvedValue({
      root: 'C:/fonts', discoveredCandidates: 3, imported: 1, updated: 0,
      skippedUnchanged: 1, skippedUnsupported: 0, failed: [],
    }),
    setScanRootEnabled: vi.fn().mockResolvedValue(undefined),
    removeScanRoot: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  } as unknown as FontManagerApi;
}

describe('application settings', () => {
  it('notifies the library after scanning a root', async () => {
    const api = makeApi();
    const onLibraryChanged = vi.fn();
    render(<SettingsView api={api} onLibraryChanged={onLibraryChanged} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Scansiona' }));
    await waitFor(() => expect(onLibraryChanged).toHaveBeenCalledTimes(1));
  });

  it('shows scan diagnostics and the four remote catalog statuses', async () => {
    render(<SettingsView api={makeApi()} />);

    expect(await screen.findByText('C:/fonts')).toBeInTheDocument();
    expect(screen.getByText('1 famiglie in cache')).toBeInTheDocument();
    expect(screen.getByText('Fontsource Extra')).toBeInTheDocument();
    expect(screen.getByText('Fontshare')).toBeInTheDocument();
    expect(screen.getByText('Nerd Fonts')).toBeInTheDocument();
  });

  it('updates a scan root enabled state through the backend', async () => {
    const api = makeApi();
    render(<SettingsView api={api} />);

    fireEvent.click(await screen.findByRole('checkbox', { name: 'Disattiva C:/fonts' }));
    await waitFor(() => expect(api.setScanRootEnabled).toHaveBeenCalledWith('C:/fonts', false));
  });

  it('removes only the scan-root configuration and explains indexed files remain', async () => {
    const api = makeApi();
    render(<SettingsView api={api} />);

    fireEvent.click(await screen.findByRole('button', { name: 'Rimuovi' }));
    await waitFor(() => expect(api.removeScanRoot).toHaveBeenCalledWith('C:/fonts'));
    expect(await screen.findByRole('status')).toHaveTextContent('I file e i font già indicizzati restano invariati.');
  });
});
