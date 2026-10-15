import { open } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useState } from 'react';

import type { FontManagerApi } from './api';
import type { AppSettingsSnapshot, RemoteFontProvider, ScanReport } from './types';

const PROVIDER_LABELS: Record<RemoteFontProvider, string> = {
  google: 'Google Fonts',
  'fontsource-extra': 'Fontsource Extra',
  fontshare: 'Fontshare',
  'nerd-fonts': 'Nerd Fonts',
};

export function SettingsView({ api, onLibraryChanged }: { api: FontManagerApi; onLibraryChanged?: () => void | Promise<void> }) {
  const [settings, setSettings] = useState<AppSettingsSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [busyPath, setBusyPath] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const reload = useCallback(async () => {
    setRefreshing(true);
    setError(null);
    try {
      setSettings(await api.getAppSettings());
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setLoading(false);
      setRefreshing(false);
    }
  }, [api]);

  useEffect(() => {
    void reload();
  }, [reload]);

  async function addRoot() {
    setMessage(null);
    setError(null);
    try {
      const path = await open({ directory: true, multiple: false, title: 'Seleziona una cartella di font' });
      if (typeof path !== 'string') return;
      setBusyPath(path);
      const report = await api.addScanRoot(path);
      setMessage(scanSummary(report));
      await onLibraryChanged?.();
      await reload();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyPath(null);
    }
  }

  async function scanRoot(path: string) {
    setBusyPath(path);
    setMessage(null);
    setError(null);
    try {
      setMessage(scanSummary(await api.scanScanRoot(path)));
      await onLibraryChanged?.();
      await reload();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyPath(null);
    }
  }

  async function toggleRoot(path: string, enabled: boolean) {
    setBusyPath(path);
    setError(null);
    try {
      await api.setScanRootEnabled(path, enabled);
      await reload();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyPath(null);
    }
  }

  async function removeRoot(path: string) {
    setBusyPath(path);
    setError(null);
    setMessage(null);
    try {
      await api.removeScanRoot(path);
      setMessage('Cartella rimossa dalla configurazione. I file e i font già indicizzati restano invariati.');
      await reload();
    } catch (cause) {
      setError(errorMessage(cause));
    } finally {
      setBusyPath(null);
    }
  }

  return (
    <section aria-label="Impostazioni" className="settings-view">
      <header className="remote-heading">
        <div>
          <p className="eyebrow">CONFIGURAZIONE LOCALE</p>
          <h1>Impostazioni</h1>
          <p className="page-description">Percorsi, cartelle monitorate e stato delle cache dei cataloghi.</p>
        </div>
        <button className="btn btn-outline" disabled={refreshing} onClick={() => void reload()} type="button">{refreshing && <span className="loading loading-spinner loading-xs" />} {refreshing ? 'Aggiornamento…' : 'Aggiorna stato'}</button>
      </header>

      {error && <div className="alert alert-error alert-soft remote-alert" role="alert">{error}</div>}
      {message && <div className="alert alert-info alert-soft remote-alert" role="status">{message}</div>}
      {loading ? (
        <div className="remote-state" role="status"><span className="loading loading-spinner loading-sm" /> Lettura impostazioni…</div>
      ) : settings ? (
        <div className="settings-scroll">
          <section className="settings-section" aria-labelledby="paths-heading">
            <div className="settings-section-heading">
              <div>
                <p className="eyebrow">ARCHIVIAZIONE</p>
                <h2 id="paths-heading">Percorsi applicazione</h2>
              </div>
            </div>
            <dl className="settings-paths">
              <div><dt>Database e configurazione</dt><dd>{settings.dataDirectory}</dd></div>
              <div><dt>Cache cataloghi</dt><dd>{settings.cacheDirectory}</dd></div>
              <div><dt>Vault font</dt><dd>{settings.fontVaultDirectory}</dd></div>
              <div><dt>Font Windows</dt><dd>{settings.windowsFontsDirectory}</dd></div>
            </dl>
          </section>

          <section className="settings-section" aria-labelledby="roots-heading">
            <div className="settings-section-heading">
              <div>
                <p className="eyebrow">SCANSIONE</p>
                <h2 id="roots-heading">Cartelle di scansione</h2>
                <p className="page-description">Disattivare o rimuovere una cartella cambia solo la configurazione, non cancella file o font già indicizzati.</p>
              </div>
              <button className="btn btn-primary" disabled={busyPath !== null} onClick={() => void addRoot()} type="button">Aggiungi cartella</button>
            </div>
            {settings.scanRoots.length ? (
              <ul className="list settings-root-list">
                {settings.scanRoots.map(({ root, isAccessible }) => (
                  <li className="list-row settings-root" key={root.path}>
                    <div className="settings-root-info">
                      <strong>{root.path}</strong>
                      <span className="font-row-meta">
                        {isAccessible ? 'Accessibile' : 'Non accessibile'} · {root.lastImported} aggiunti · {root.lastUpdated} aggiornati · {root.lastFailed} errori
                        {root.lastError ? ` · ${root.lastError}` : ''}
                      </span>
                    </div>
                    <label className="settings-toggle-label">
                      <span>{root.isEnabled ? 'Attiva' : 'In pausa'}</span>
                      <input
                        aria-label={`${root.isEnabled ? 'Disattiva' : 'Attiva'} ${root.path}`}
                        checked={root.isEnabled}
                        className="toggle toggle-sm"
                        disabled={busyPath !== null}
                        onChange={(event) => void toggleRoot(root.path, event.target.checked)}
                        type="checkbox"
                      />
                    </label>
                    <button className="btn btn-ghost btn-sm" disabled={busyPath !== null || !root.isEnabled || !isAccessible} onClick={() => void scanRoot(root.path)} type="button">
                      {busyPath === root.path ? <span className="loading loading-spinner loading-xs" /> : null} Scansiona
                    </button>
                    <button className="btn btn-ghost btn-sm" disabled={busyPath !== null} onClick={() => void removeRoot(root.path)} type="button">Rimuovi</button>
                  </li>
                ))}
              </ul>
            ) : (
              <div className="empty-state settings-empty">
                <h3>Nessuna cartella configurata</h3>
                <p>Aggiungi una cartella per includere i tuoi font nelle scansioni.</p>
              </div>
            )}
          </section>

          <section className="settings-section" aria-labelledby="catalogs-heading">
            <div className="settings-section-heading">
              <div>
                <p className="eyebrow">PROVIDER</p>
                <h2 id="catalogs-heading">Cataloghi remoti</h2>
              </div>
            </div>
            <ul className="list settings-catalog-list">
              {settings.cloudCatalogs.map((catalog) => (
                <li className="list-row settings-catalog" key={catalog.provider}>
                  <strong>{PROVIDER_LABELS[catalog.provider]}</strong>
                  <span>{catalog.available ? `${catalog.itemCount} famiglie in cache` : 'Cache vuota'}</span>
                  <span>{catalog.stale ? 'Da aggiornare' : catalog.fetchedAtUnixMs ? `Aggiornato ${formatDate(catalog.fetchedAtUnixMs)}` : 'Mai caricato'}</span>
                  {catalog.lastError && <span className="text-error">{catalog.lastError}</span>}
                </li>
              ))}
            </ul>
          </section>
        </div>
      ) : null}
    </section>
  );
}

function scanSummary(report: ScanReport): string {
  return `Scansione completata: ${report.imported} nuovi, ${report.updated} aggiornati, ${report.skippedUnchanged} invariati, ${report.failed.length} errori.`;
}

function formatDate(timestamp: number): string {
  return new Intl.DateTimeFormat('it-IT', { dateStyle: 'short', timeStyle: 'short' }).format(timestamp);
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
