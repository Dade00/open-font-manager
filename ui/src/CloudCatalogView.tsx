import { useEffect, useMemo, useRef, useState } from 'react';

import type { FontManagerApi } from './api';
import type { FontCategory } from './categories';
import type { FontItem, RemoteFontCatalogItem, RemoteFontCatalogSnapshot, RemoteFontProvider } from './types';
import { PreviewQueue } from './adaptivePreviewQueue';
import { CategoryFilter } from './CategoryFilter';
import { categoryLabel, normalizeCategory } from './categories';

const PROVIDERS: Record<Exclude<RemoteFontProvider, 'google'>, { title: string; description: string }> = {
  'fontsource-extra': {
    title: 'Fontsource Extra',
    description: 'Famiglie indipendenti, senza duplicare il catalogo Google Fonts.',
  },
  fontshare: {
    title: 'Fontshare',
    description: 'Famiglie gratuite distribuite da Fontshare.',
  },
  'nerd-fonts': {
    title: 'Nerd Fonts',
    description: 'Font con glifi aggiuntivi, scaricati e installati dal catalogo release.',
  },
};

const PAGE_SIZE = 100;

export function CloudCatalogView({
  api,
  provider,
  onInstalled,
  installedFonts = [],
  previewText = 'Aa Bb Cc 0123',
  previewSize = 28,
}: {
  api: FontManagerApi;
  provider: Exclude<RemoteFontProvider, 'google'>;
  onInstalled(fontIds: string[], message: string, failed: boolean): void;
  installedFonts?: FontItem[];
  previewText?: string;
  previewSize?: number;
}) {
  const [snapshot, setSnapshot] = useState<RemoteFontCatalogSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState('');
  const [page, setPage] = useState(0);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [variantId, setVariantId] = useState('');
  const [previewFont, setPreviewFont] = useState<string | null>(null);
  const [previewLoading, setPreviewLoading] = useState(false);
  const [previewError, setPreviewError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [category, setCategory] = useState<FontCategory>('all');
  const lifecycle = useRef(0);
  const refreshRequest = useRef(0);
  const config = PROVIDERS[provider];

  const selected = useMemo(
    () => snapshot?.items.find((item) => item.id === selectedId) ?? null,
    [selectedId, snapshot],
  );
  const normalizedSearch = search.trim().toLocaleLowerCase();
  const filteredItems = useMemo(() => {
    const items = snapshot?.items ?? [];
    return items.filter((item) => {
      const categoryMatches = category === 'all' || normalizeCategory(item.category) === category;
      const searchMatches = !normalizedSearch || `${item.family} ${item.category} ${item.variants.map((variant) => variant.label).join(' ')}`.toLocaleLowerCase().includes(normalizedSearch);
      return categoryMatches && searchMatches;
    });
  }, [category, normalizedSearch, snapshot]);
  const pageCount = Math.max(1, Math.ceil(filteredItems.length / PAGE_SIZE));
  const visiblePage = Math.min(page, pageCount - 1);
  const pageItems = useMemo(() => filteredItems.slice(visiblePage * PAGE_SIZE, (visiblePage + 1) * PAGE_SIZE), [filteredItems, visiblePage]);
  const installedSource = provider === 'fontsource-extra' ? 'fontsource' : provider;
  const isInstalled = (id: string) => installedFonts.some((font) => font.isInstalled && font.source === installedSource && font.sourceFamilyId === id);
  const selectedVariant = selected?.variants.find((variant) => variant.id === variantId)
    ?? selected?.variants.find((variant) => variant.id === selected.previewVariantId)
    ?? selected?.variants[0]
    ?? null;

  useEffect(() => {
    const generation = ++lifecycle.current;
    let mounted = true;
    setLoading(true);
    setError(null);
    setSnapshot(null);
    setSelectedId(null);
    setVariantId('');
    setPreviewFont(null);
    setSearch('');
    setCategory('all');
    setPage(0);
    setRefreshing(false);
    setBusyId(null);
    setNotice(null);
    void (async () => {
      try {
        const cached = await api.loadCachedRemoteCatalog(provider);
        if (mounted && cached) setSnapshot(cached);
        if (!cached) {
          const fresh = await api.refreshRemoteCatalog(provider);
          if (mounted) setSnapshot(fresh);
        }
      } catch (cause) {
        if (mounted) setError(errorMessage(cause));
      } finally {
        if (mounted) setLoading(false);
      }
    })();
    return () => {
      mounted = false;
      if (lifecycle.current === generation) lifecycle.current += 1;
    };
  }, [api, provider]);

  useEffect(() => {
    setPage(0);
  }, [search]);

  useEffect(() => {
    if (!selected || provider === 'nerd-fonts') {
      setPreviewFont(null);
      setPreviewError(null);
      setPreviewLoading(false);
      return;
    }
    const selectedVariantId = selected.variants.some((variant) => variant.id === variantId)
      ? variantId
      : selected.previewVariantId ?? selected.variants[0]?.id;
    if (!selectedVariantId) {
      setPreviewFont(null);
      setPreviewError('Questa famiglia non espone una variante per l’anteprima.');
      return;
    }
    let current = true;
    let face: FontFace | null = null;
    let objectUrl: string | null = null;
    setPreviewLoading(true);
    setPreviewError(null);
    setPreviewFont(null);
    void (async () => {
      try {
        const bytes = await api.getRemotePreviewBytes(provider, selected.id, selectedVariantId);
        if (!current) return;
        const fontBuffer = new Uint8Array(bytes).buffer;
        objectUrl = URL.createObjectURL(new Blob([fontBuffer], { type: 'font/ttf' }));
        const familyName = `cloud-preview-${provider}-${selected.id.replace(/[^a-zA-Z0-9_-]/g, '-')}`;
        face = new FontFace(familyName, `url("${objectUrl}")`);
        await face.load();
        if (!current || !face) return;
        document.fonts.add(face);
        setPreviewFont(familyName);
      } catch (cause) {
        if (current) setPreviewError(errorMessage(cause));
      } finally {
        if (current) setPreviewLoading(false);
      }
    })();
    return () => {
      current = false;
      if (face) document.fonts.delete(face);
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [api, provider, selected, variantId]);

  async function refreshCatalog() {
    const generation = lifecycle.current;
    const request = ++refreshRequest.current;
    const isCurrent = () => lifecycle.current === generation && refreshRequest.current === request;
    setRefreshing(true);
    setError(null);
    setNotice(null);
    try {
      const fresh = await api.refreshRemoteCatalog(provider);
      if (!isCurrent() || fresh.provider !== provider) return;
      setSnapshot(fresh);
      setSelectedId((current) => fresh.items.some((item) => item.id === current) ? current : null);
      setPage(0);
      setVariantId((current) => {
        const selectedItem = fresh.items.find((item) => item.id === selectedId);
        return selectedItem?.variants.some((variant) => variant.id === current) ? current : selectedItem?.previewVariantId ?? selectedItem?.variants[0]?.id ?? '';
      });
    } catch (cause) {
      if (isCurrent()) setError(errorMessage(cause));
    } finally {
      if (isCurrent()) setRefreshing(false);
    }
  }

  useEffect(() => {
    setPage(0);
  }, [snapshot]);

  async function install(item: RemoteFontCatalogItem) {
    const alreadyInstalled = isInstalled(item.id);
    if (busyId !== null || alreadyInstalled) return;
    const generation = lifecycle.current;
    setBusyId(item.id);
    setNotice(null);
    try {
      const result = await api.downloadRemoteFamily(
        provider,
        item.id,
        null,
        true,
      );
      if (lifecycle.current !== generation) return;
      const report = result.report;
      const failed = report.failures.length > 0;
      const summary = `${item.family}: ${report.savedFiles} file salvati, ${report.indexedFonts} font indicizzati, ${report.activatedFonts} attivati${report.failures.length > 0 ? `, ${report.failures.length} errori: ${report.failures.map((failure) => `${failure.variantId} (${failure.stage}): ${failure.reason}`).join(' · ')}` : ''}.`;
      if (result.fontIds.length) {
        onInstalled(result.fontIds, summary, failed);
      } else {
        setNotice(summary);
      }
    } catch (cause) {
      if (lifecycle.current === generation) setNotice(errorMessage(cause));
    } finally {
      if (lifecycle.current === generation) setBusyId(null);
    }
  }

  function selectFamily(item: RemoteFontCatalogItem) {
    setSelectedId(item.id);
    setVariantId(item.previewVariantId ?? item.variants[0]?.id ?? '');
    setNotice(null);
  }

  return (
    <section aria-label={config.title} className="remote-catalog-view">
      <header className="remote-heading">
        <div>
          <p className="eyebrow">CATALOGO REMOTO</p>
          <h1>{config.title}</h1>
          <p className="page-description">{config.description}</p>
        </div>
        <button className="btn btn-outline" disabled={refreshing} onClick={() => void refreshCatalog()} type="button">
          {refreshing ? <span className="loading loading-spinner loading-xs" /> : null}
          {refreshing ? 'Aggiornamento…' : 'Aggiorna catalogo'}
        </button>
      </header>

      {snapshot?.stale && (
        <div className="alert alert-warning alert-soft remote-alert" role="status">
          <span>Catalogo in cache non aggiornato.{snapshot.lastError ? ` ${snapshot.lastError}` : ''}</span>
        </div>
      )}
      {error && (
        <div className="alert alert-error alert-soft remote-alert" role="alert">
          <span>{error}</span>
          <button className="btn btn-sm" disabled={refreshing} onClick={() => void refreshCatalog()} type="button">Riprova</button>
        </div>
      )}

      <div className="remote-toolbar">
        <label className="search-field">
          <span className="search-glyph" aria-hidden="true">⌕</span>
          <span className="sr-only">Cerca nella famiglia remota</span>
          <input className="input search-input" onChange={(event) => setSearch(event.target.value)} placeholder="Cerca per famiglia, categoria o stile…" type="search" value={search} />
        </label>
        <span className="remote-count" aria-live="polite">{filteredItems.length} famiglie</span>
      </div>
      <CategoryFilter value={category} onChange={setCategory} />

      {loading ? (
        <div className="remote-state" role="status"><span className="loading loading-spinner loading-sm" /> Lettura cache e catalogo…</div>
      ) : (
        <div className="remote-content">
          <div className="remote-results" aria-label="Famiglie del catalogo">
            {provider !== 'nerd-fonts' && <RowPreviewObserver api={api} items={pageItems} provider={provider} />}
            {pageItems.length ? (
              <ul className="list remote-list">
                {pageItems.map((item) => {
                  const installed = isInstalled(item.id);
                  const busy = busyId === item.id;
                  return (
                  <li className="list-row remote-list-row" key={item.id}>
                    <div className="google-font-row">
                    <button aria-pressed={selectedId === item.id} className={`font-row ${selectedId === item.id ? 'is-selected' : ''}`} onClick={() => selectFamily(item)} type="button">
                      <span className="font-row-main">
                        <span className="font-row-family">{item.family}</span>
                        <span className="font-row-meta">{categoryLabel(item.category)} <i>·</i> {item.variants.map((variant) => variant.label).join(', ')}{('isVariable' in item && item.isVariable) ? ' · Variabile' : ''}</span>
                        <span className="font-row-sample" data-preview-id={item.id} data-preview-state="idle" style={{ fontSize: `${Math.min(previewSize, 38)}px`, fontFamily: provider === 'nerd-fonts' ? 'monospace' : undefined }}>{previewText}</span>
                      </span>
                      <span className="font-row-state">{installed && <span className="badge badge-success badge-outline">Installato</span>}</span>
                    </button>
                    <button aria-label={`${installed ? 'Installato' : 'Installa'} ${item.family}`} className="btn btn-ghost btn-xs family-download" disabled={busyId !== null || installed} onClick={() => void install(item)} type="button">{busy ? <span className="loading loading-spinner loading-xs" /> : null}{installed ? 'Installato' : busy ? 'Installazione…' : 'Installa'}</button>
                    </div>
                  </li>
                  );
                })}
              </ul>
            ) : (
              <div className="empty-state remote-empty">
                <span className="empty-index">CATALOGO</span>
                <h2>{snapshot?.items.length ? 'Nessuna famiglia corrisponde alla ricerca.' : 'Il catalogo non contiene famiglie.'}</h2>
                <p>{snapshot?.items.length ? 'Prova un altro nome o una categoria.' : 'Aggiorna il catalogo per riprovare il caricamento.'}</p>
                {!snapshot?.items.length && <button className="btn btn-outline" disabled={refreshing} onClick={() => void refreshCatalog()} type="button">Ricarica catalogo</button>}
              </div>
            )}
            {filteredItems.length > PAGE_SIZE && (
              <div className="remote-pagination">
                <button className="btn btn-ghost btn-sm" disabled={page === 0} onClick={() => setPage((value) => Math.max(0, value - 1))} type="button">Precedente</button>
                <span>{visiblePage + 1} / {pageCount}</span>
                <button className="btn btn-ghost btn-sm" disabled={(visiblePage + 1) * PAGE_SIZE >= filteredItems.length} onClick={() => setPage((value) => Math.min(pageCount - 1, value + 1))} type="button">Successiva</button>
              </div>
            )}
          </div>

          <aside className="card card-border remote-inspector" aria-label="Dettagli famiglia selezionata">
            {selected ? (
              <div className="card-body">
                <p className="eyebrow">{categoryLabel(selected.category)}</p>
                <h2 className="card-title remote-selected-title">{selected.family}</h2>
                <label className="control-label" htmlFor="remote-variant">Variante</label>
                <select className="select select-bordered" disabled={!selected.variants.length} id="remote-variant" onChange={(event) => setVariantId(event.target.value)} value={selectedVariant?.id ?? ''}>
                  {selected.variants.map((variant) => <option key={variant.id} value={variant.id}>{variant.label}</option>)}
                </select>
                <div className="hero-specimen remote-specimen" aria-live="polite">
                  <span className="specimen-caption">ANTEPRIMA SU RICHIESTA</span>
                  {provider === 'nerd-fonts' ? (
                    <p className="remote-preview-message">Nerd Fonts usa glifi da terminale, il campione non viene scaricato.</p>
                  ) : previewLoading ? (
                    <p className="remote-preview-message"><span className="loading loading-spinner loading-xs" /> Caricamento anteprima…</p>
                  ) : previewError ? (
                    <p className="remote-preview-message">Anteprima non disponibile. {previewError}</p>
                  ) : (
                    <p style={{ fontFamily: previewFont ? `"${previewFont}"` : 'Georgia, serif', fontSize: `${Math.min(previewSize, 38)}px` }}>{previewText}</p>
                  )}
                </div>
                <p className="font-row-meta">Il font viene scaricato nel vault e attivato per l’utente Windows.</p>
                <div className="card-actions remote-actions">
                  <button className="btn btn-primary" disabled={busyId !== null || isInstalled(selected.id)} onClick={() => void install(selected)} type="button">
                    {busyId === selected.id && <span className="loading loading-spinner loading-xs" />}
                    {isInstalled(selected.id) ? 'Installato' : busyId === selected.id ? 'Installazione…' : 'Installa'}
                  </button>
                </div>
                {notice && <div className="alert alert-info alert-soft" role="status">{notice}</div>}
              </div>
            ) : (
              <div className="inspector-empty">
                <h2>Scegli una famiglia</h2>
                <p>L’anteprima viene scaricata dal backend solo dopo la selezione. Il catalogo non carica font in anticipo.</p>
              </div>
            )}
          </aside>
        </div>
      )}
    </section>
  );
}

function RowPreviewObserver({ api, items, provider }: { api: FontManagerApi; items: RemoteFontCatalogItem[]; provider: Exclude<RemoteFontProvider, 'google'> }) {
  useEffect(() => {
    if (typeof IntersectionObserver === 'undefined') return;
    const root = document.querySelector<HTMLElement>('.remote-results');
    if (!root) return;
    const queue = new PreviewQueue({ maxConcurrency: 3 });
    const disposers = new Map<string, () => void>();
    const observer = new IntersectionObserver((entries) => {
      for (const entry of entries) {
        const element = entry.target as HTMLElement;
        if (!entry.isIntersecting) continue;
        observer.unobserve(element);
        const item = items.find((candidate) => candidate.id === element.dataset.previewId);
        const variant = item?.variants.find((candidate) => candidate.id === item.previewVariantId) ?? item?.variants[0];
        if (!item || !variant) continue;
        void queue.enqueue(`${provider}:${item.id}:${variant.id}`, async () => {
          let live = true; let face: FontFace | null = null; let url: string | null = null;
          disposers.set(item.id, () => { live = false; if (face) document.fonts.delete(face); if (url) URL.revokeObjectURL(url); });
          try {
            const bytes = await api.getRemotePreviewBytes(provider, item.id, variant.id);
            if (!live) return;
            url = URL.createObjectURL(new Blob([new Uint8Array(bytes).buffer], { type: 'font/ttf' }));
            face = new FontFace(`cloud-row-${provider}-${item.id.replace(/[^a-zA-Z0-9_-]/g, '-')}`, `url("${url}")`);
            await face.load();
            if (!live) return;
            document.fonts.add(face); element.style.fontFamily = `"${face.family}"`; element.dataset.previewState = 'loaded';
          } catch { if (live) element.dataset.previewState = 'error'; }
        });
      }
    }, { root, rootMargin: '160px' });
    root.querySelectorAll<HTMLElement>('[data-preview-id]').forEach((element) => observer.observe(element));
    return () => { observer.disconnect(); queue.dispose(); disposers.forEach((dispose) => dispose()); };
  }, [api, items, provider]);
  return null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}
