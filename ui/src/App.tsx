import { open } from '@tauri-apps/plugin-dialog';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { fontManagerApi } from './api';
import type { FontManagerApi } from './api';
import type {
  CatalogSnapshot,
  Collection,
  FontCategory,
  FontItem,
  FontPage,
  GoogleFontCatalogItem,
  LibraryTab,
  StartupSnapshot,
} from './types';
import './styles.css';

const PAGE_SIZE = 50;
const PREVIEW_SAMPLE = 'Sphinx of black quartz, judge my vow.';
const NAVIGATION: { tab: LibraryTab; label: string; description: string }[] = [
  { tab: 'local', label: 'Libreria locale', description: 'Font sul dispositivo' },
  { tab: 'active', label: 'Attivi su Windows', description: 'Disponibili alle app' },
  { tab: 'favorites', label: 'Preferiti', description: 'La tua selezione' },
  { tab: 'google', label: 'Google Fonts', description: 'Catalogo e vault' },
];
const CATEGORIES: { value: FontCategory; label: string }[] = [
  { value: 'all', label: 'Tutte le categorie' },
  { value: 'sans-serif', label: 'Sans serif' },
  { value: 'serif', label: 'Serif' },
  { value: 'display', label: 'Display' },
  { value: 'monospace', label: 'Monospace' },
];

interface Notice {
  message: string;
  kind: 'success' | 'error' | 'info';
}

interface PreviewResource {
  face: FontFace;
  url: string;
  familyName: string;
}

export function App({ api = fontManagerApi }: { api?: FontManagerApi }) {
  const [snapshot, setSnapshot] = useState<StartupSnapshot | null>(null);
  const [fontPage, setFontPage] = useState<FontPage | null>(null);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [googleCatalog, setGoogleCatalog] = useState<CatalogSnapshot | null>(null);
  const [tab, setTab] = useState<LibraryTab>('local');
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState<FontCategory>('all');
  const [collectionId, setCollectionId] = useState<number | null>(null);
  const [page, setPage] = useState(0);
  const [selectedFontId, setSelectedFontId] = useState<string | null>(null);
  const [selectedGoogleId, setSelectedGoogleId] = useState<string | null>(null);
  const [googleVariantId, setGoogleVariantId] = useState('');
  const [previewText, setPreviewText] = useState(PREVIEW_SAMPLE);
  const [previewSize, setPreviewSize] = useState(42);
  const [previewFontFamily, setPreviewFontFamily] = useState<string | null>(null);
  const [previewState, setPreviewState] = useState<'idle' | 'loading' | 'ready' | 'failed'>('idle');
  const [loading, setLoading] = useState(true);
  const [maintenanceBusy, setMaintenanceBusy] = useState(false);
  const [catalogBusy, setCatalogBusy] = useState(false);
  const [operation, setOperation] = useState<string | null>(null);
  const [collectionName, setCollectionName] = useState('');
  const [pendingFamily, setPendingFamily] = useState<GoogleFontCatalogItem | null>(null);
  const [notice, setNotice] = useState<Notice | null>(null);
  const previewResource = useRef<PreviewResource | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const skipFirstQuery = useRef(true);
  const appReady = snapshot !== null;

  const refreshFonts = useCallback(async () => {
    const result = await api.listFonts({
      tab,
      search,
      category: tab === 'google' ? 'all' : category,
      collectionId: tab === 'local' ? collectionId : null,
      page,
      pageSize: PAGE_SIZE,
    });
    setFontPage(result);
    setSelectedFontId((current) =>
      result.items.some((font) => font.id === current) ? current : (result.items[0]?.id ?? null),
    );
    setLoading(false);
  }, [api, category, collectionId, page, search, tab]);

  const refreshActiveCount = useCallback(async () => {
    const active = await api.listFonts({
      tab: 'active',
      search: '',
      category: 'all',
      collectionId: null,
      page: 0,
      pageSize: 1,
    });
    setSnapshot((current) => current && { ...current, activeCount: active.total });
  }, [api]);

  const refreshFontsRef = useRef(refreshFonts);
  const refreshActiveCountRef = useRef(refreshActiveCount);
  useEffect(() => {
    refreshFontsRef.current = refreshFonts;
    refreshActiveCountRef.current = refreshActiveCount;
  }, [refreshActiveCount, refreshFonts]);

  useEffect(() => {
    let mounted = true;
    api
      .initializeApp()
      .then((initial) => {
        if (!mounted) return;
        setSnapshot(initial);
        setFontPage(initial.fonts);
        setCollections(initial.collections);
        setGoogleCatalog(initial.googleCatalog);
        setSelectedFontId(initial.fonts.items[0]?.id ?? null);
        setSelectedGoogleId(initial.googleCatalog?.items[0]?.id ?? null);
        setLoading(false);
        setMaintenanceBusy(true);
        void api
          .runStartupMaintenance()
          .then(async (report) => {
            if (!mounted) return;
            setMaintenanceBusy(false);
            const messages = [...report.errors, ...report.restoreFailures.map((family) => `Ripristino non riuscito: ${family}`)];
            if (messages.length) setNotice({ message: messages.join(' · '), kind: 'error' });
            else if (report.scanReports.length) {
              const imported = report.scanReports.reduce((sum, item) => sum + item.imported, 0);
              const updated = report.scanReports.reduce((sum, item) => sum + item.updated, 0);
              setNotice({ message: `Indice aggiornato: ${imported} nuovi, ${updated} modificati.`, kind: 'info' });
            }
            await Promise.all([refreshFontsRef.current(), refreshActiveCountRef.current()]);
          })
          .catch((error: unknown) => {
            if (!mounted) return;
            setMaintenanceBusy(false);
            setNotice({ message: errorMessage(error), kind: 'error' });
          });
      })
      .catch((error: unknown) => {
        if (!mounted) return;
        setLoading(false);
        setNotice({ message: errorMessage(error), kind: 'error' });
      });
    return () => {
      mounted = false;
    };
  }, [api]);

  useEffect(() => {
    const focusSearch = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLocaleLowerCase() === 'k') {
        event.preventDefault();
        searchInputRef.current?.focus();
      }
    };
    window.addEventListener('keydown', focusSearch);
    return () => window.removeEventListener('keydown', focusSearch);
  }, []);

  useEffect(() => {
    if (!appReady) return;
    if (skipFirstQuery.current) {
      skipFirstQuery.current = false;
      return;
    }
    let cancelled = false;
    const timer = window.setTimeout(() => {
      setLoading(true);
      void refreshFonts().catch((error: unknown) => {
        if (!cancelled) {
          setLoading(false);
          setNotice({ message: errorMessage(error), kind: 'error' });
        }
      });
    }, 140);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [appReady, category, collectionId, page, refreshFonts, tab]);

  useEffect(() => {
    if (tab !== 'google') {
      removePreviewResource(previewResource);
      setPreviewFontFamily(null);
      setPreviewState('idle');
      return;
    }
    removePreviewResource(previewResource);
    setPreviewFontFamily(null);
    setPreviewState('idle');
    setGoogleVariantId((current) => {
      const family = googleCatalog?.items.find((item) => item.id === selectedGoogleId);
      if (!family?.variants.length) return '';
      return family.variants.includes(current) ? current : family.defVariant || family.variants[0];
    });
  }, [googleCatalog, selectedGoogleId, tab]);

  useEffect(() => {
    if (tab === 'google' || !selectedFontId || !fontPage) {
      removePreviewResource(previewResource);
      setPreviewFontFamily(null);
      setPreviewState('idle');
      return;
    }
    let cancelled = false;
    setPreviewState('loading');
    void api
      .getFontPreviewBytes(selectedFontId)
      .then((bytes) => attachPreviewFont(bytes, `ofm-${safeFontKey(selectedFontId)}`, previewResource))
      .then((familyName) => {
        if (cancelled) return;
        setPreviewFontFamily(familyName);
        setPreviewState('ready');
      })
      .catch(() => {
        if (!cancelled) setPreviewState('failed');
      });
    return () => {
      cancelled = true;
      removePreviewResource(previewResource);
    };
  }, [api, selectedFontId, tab]);

  useEffect(() => () => removePreviewResource(previewResource), []);

  const currentFont = useMemo(
    () => fontPage?.items.find((font) => font.id === selectedFontId) ?? null,
    [fontPage, selectedFontId],
  );
  const selectedGoogle = useMemo(
    () => googleCatalog?.items.find((item) => item.id === selectedGoogleId) ?? null,
    [googleCatalog, selectedGoogleId],
  );
  const filteredCatalog = useMemo(() => {
    const needle = search.trim().toLocaleLowerCase();
    return (googleCatalog?.items ?? []).filter((item) => item.family.toLocaleLowerCase().includes(needle));
  }, [googleCatalog, search]);
  const catalogPageCount = Math.max(1, Math.ceil(filteredCatalog.length / PAGE_SIZE));
  const visibleCatalog = filteredCatalog.slice(page * PAGE_SIZE, (page + 1) * PAGE_SIZE);
  const title = collectionId !== null
    ? collections.find((item) => item.id === collectionId)?.name ?? 'Collezione'
    : NAVIGATION.find((item) => item.tab === tab)?.label ?? 'Libreria locale';

  async function handleFavorite(font: FontItem) {
    setOperation(`favorite:${font.id}`);
    try {
      await api.setFontFavorite(font.id, !font.isFavorite);
      await refreshFonts();
      setNotice({
        message: font.isFavorite ? 'Rimosso dai preferiti.' : 'Aggiunto ai preferiti.',
        kind: 'success',
      });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleActivation(font: FontItem) {
    setOperation(`active:${font.id}`);
    try {
      await api.setFontActive(font.id, !font.isActive);
      await Promise.all([refreshFonts(), refreshActiveCount()]);
      setNotice({ message: font.isActive ? 'Font disattivato.' : 'Font attivato temporaneamente su Windows.', kind: 'success' });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleCreateCollection(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const name = collectionName.trim();
    if (!name) return;
    setOperation('collection');
    try {
      const created = await api.createCollection(name);
      setCollections((current) => [...current, created].sort((a, b) => a.name.localeCompare(b.name)));
      setCollectionName('');
      setNotice({ message: `Collezione “${created.name}” creata.`, kind: 'success' });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleAddToCollection(id: number) {
    if (!currentFont) return;
    setOperation('collection-item');
    try {
      await api.addFontToCollection(id, currentFont.id);
      const updated = await api.listCollections();
      setCollections(updated);
      setNotice({ message: 'Font aggiunto alla collezione.', kind: 'success' });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleScanFolder() {
    try {
      const folder = await open({ directory: true, multiple: false, title: 'Scegli una cartella di font' });
      if (typeof folder !== 'string') return;
      setOperation('scan');
      const report = await api.scanDirectory(folder);
      await refreshFonts();
      setNotice({
        message: `Scansione completata: ${report.imported} nuovi, ${report.updated} aggiornati, ${report.failed.length} errori.`,
        kind: report.failed.length ? 'error' : 'success',
      });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleCopy(kind: 'css' | 'nextjs') {
    if (!currentFont) return;
    setOperation(`copy:${kind}`);
    try {
      const snippet = await api.getFontSnippet(currentFont.id, kind);
      await writeText(snippet);
      setNotice({ message: `${kind === 'css' ? 'Snippet CSS' : 'Snippet Next.js'} copiato.`, kind: 'success' });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleRefreshCatalog() {
    setCatalogBusy(true);
    try {
      const refreshed = await api.refreshGoogleCatalog();
      setGoogleCatalog(refreshed);
      setSelectedGoogleId((current) => current ?? refreshed.items[0]?.id ?? null);
      setPage(0);
      setNotice({ message: `Catalogo aggiornato: ${refreshed.items.length} famiglie.`, kind: 'success' });
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setCatalogBusy(false);
    }
  }

  async function handleGooglePreview() {
    if (!selectedGoogle || !googleVariantId) return;
    setOperation('google-preview');
    try {
      const preview = await api.previewGoogleVariant(selectedGoogle.id, googleVariantId);
      const bytes = decodeBase64(preview.fontDataBase64);
      const familyName = await attachPreviewFont(
        bytes,
        `ofm-google-${safeFontKey(selectedGoogle.id)}-${safeFontKey(googleVariantId)}`,
        previewResource,
      );
      setPreviewFontFamily(familyName);
      setPreviewState('ready');
      setNotice({ message: `Anteprima ${preview.family} ${preview.subfamily} pronta.`, kind: 'success' });
    } catch (error) {
      setPreviewState('failed');
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleInstallVariant() {
    if (!selectedGoogle || !googleVariantId) return;
    setOperation('google-install');
    try {
      const font = await api.installGoogleVariant(selectedGoogle.id, googleVariantId);
      setNotice({ message: `${font.family} ${font.subfamily} salvato nel vault locale.`, kind: 'success' });
      if (tab === 'google') await refreshFonts();
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  async function handleDownloadFamily() {
    if (!pendingFamily) return;
    setOperation('google-family');
    try {
      const report = await api.downloadGoogleFamily(pendingFamily.id);
      setNotice({
        message: `${pendingFamily.family}: ${report.installed.length} varianti scaricate, ${report.failed.length} non riuscite.`,
        kind: report.failed.length ? 'error' : 'success',
      });
      setPendingFamily(null);
      if (tab === 'google') await refreshFonts();
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  if (!snapshot && loading) {
    return (
      <main className="app-loading" aria-live="polite">
        <span className="loading loading-spinner loading-md" />
        <p>Preparazione della libreria tipografica…</p>
      </main>
    );
  }

  return (
    <div className="app-shell">
      <aside className="sidebar">
        <div className="brand-lockup">
          <div>
            <p className="brand-name">Open Font Manager</p>
            <p className="brand-subtitle">Libreria tipografica · Windows</p>
          </div>
        </div>

        <div className="sidebar-label">Libreria</div>
        <nav className="primary-nav" aria-label="Sezioni libreria">
          {NAVIGATION.map((item) => (
            <button
              aria-current={tab === item.tab && collectionId === null ? 'page' : undefined}
              className={`nav-item ${tab === item.tab && collectionId === null ? 'is-current' : ''}`}
              key={item.tab}
              onClick={() => {
                setTab(item.tab);
                setCollectionId(null);
                setPage(0);
              }}
              type="button"
            >
              <span>{item.label}</span>
              <span className="nav-description">{item.description}</span>
              {item.tab === 'active' && <span className="nav-count">{snapshot?.activeCount ?? 0}</span>}
            </button>
          ))}
        </nav>

        <div className="collection-heading">
          <span className="sidebar-label">Collezioni</span>
          <span className="collection-total">{collections.length.toString().padStart(2, '0')}</span>
        </div>
        <div className="collection-list">
          {collections.map((collection) => (
            <button
              aria-current={collectionId === collection.id ? 'page' : undefined}
              className={`collection-link ${collectionId === collection.id ? 'is-current' : ''}`}
              key={collection.id}
              onClick={() => {
                setTab('local');
                setCollectionId(collection.id);
                setPage(0);
              }}
              type="button"
            >
              <span>{collection.name}</span>
              <span className="collection-total">{collection.fontCount}</span>
            </button>
          ))}
          {!collections.length && <p className="empty-collections">Le raccolte che crei compariranno qui.</p>}
        </div>
        <form className="collection-form" onSubmit={handleCreateCollection}>
          <label className="sr-only" htmlFor="new-collection">Nome nuova collezione</label>
          <input
            className="input input-bordered collection-input"
            id="new-collection"
            maxLength={80}
            onChange={(event) => setCollectionName(event.target.value)}
            placeholder="Nuova collezione"
            value={collectionName}
          />
          <button aria-label="Crea collezione" className="btn btn-square btn-ghost collection-add" disabled={!collectionName.trim() || operation === 'collection'} type="submit">+</button>
        </form>
        <div className="sidebar-spacer" />
        <div className="vault-note">
          <span className="vault-rule" />
          <p>Vault locale.<br />Attivazione temporanea via GDI.</p>
        </div>
        <button className="btn btn-primary scan-button" disabled={operation === 'scan'} onClick={handleScanFolder} type="button">
          {operation === 'scan' ? 'Scansione…' : '＋  Aggiungi cartella'}
        </button>
      </aside>

      <main className="main-panel">
        <header className="topbar">
          <div className="breadcrumb"><span>OPEN FONT MANAGER</span><span className="breadcrumb-slash">/</span>{title.toLocaleUpperCase()}</div>
          <div className="topbar-status">
            <span className={`status-dot ${maintenanceBusy ? 'is-busy' : ''}`} />
            <span>{maintenanceBusy ? 'Indicizzazione' : `${snapshot?.activeCount ?? 0} attivi`}</span>
          </div>
        </header>

        <div className="workspace">
          <section className="library-panel" aria-label={title}>
            <div className="page-heading">
              <div>
                <p className="eyebrow">CAMPIONI TIPOGRAFICI <span>·</span> 01</p>
                <h1>{tab === 'google' ? 'Trova il tuo prossimo carattere.' : title}</h1>
                <p className="page-description">
                  {tab === 'google'
                    ? 'Esplora famiglie open-source e salva le varianti nel tuo vault.'
                    : tab === 'active'
                      ? 'Attivi con GDI per la sessione; nessun font viene installato permanentemente.'
                      : collectionId !== null
                        ? 'Una selezione curata, pronta per il prossimo progetto.'
                        : 'Cerca, confronta e organizza i caratteri già presenti sul dispositivo.'}
                </p>
              </div>
              <div className="page-total" aria-live="polite">
                <span className="page-total-number">{tab === 'google' ? filteredCatalog.length : fontPage?.total ?? 0}</span>
                <span>{tab === 'google' ? 'famiglie' : 'font'}</span>
              </div>
            </div>

            <div className="toolbar">
              <label className="search-field">
                <span className="search-glyph" aria-hidden="true">⌕</span>
                <span className="sr-only">Cerca font</span>
                <input
                  className="input search-input"
                  onChange={(event) => {
                    setSearch(event.target.value);
                    setPage(0);
                  }}
                  placeholder={tab === 'google' ? 'Cerca nel catalogo…' : 'Cerca per famiglia o stile…'}
                  ref={searchInputRef}
                  type="search"
                  value={search}
                />
                <kbd className="kbd search-shortcut">Ctrl K</kbd>
              </label>
              {tab !== 'google' ? (
                <select aria-label="Categoria font" className="select select-bordered category-select" onChange={(event) => { setCategory(event.target.value as FontCategory); setPage(0); }} value={category}>
                  {CATEGORIES.map((item) => <option key={item.value} value={item.value}>{item.label}</option>)}
                </select>
              ) : (
                <button className="btn btn-outline catalog-refresh" disabled={catalogBusy} onClick={handleRefreshCatalog} type="button">
                  {catalogBusy ? <span className="loading loading-spinner loading-xs" /> : '↻'}
                  {catalogBusy ? 'Aggiornamento…' : 'Aggiorna catalogo'}
                </button>
              )}
            </div>

            {tab === 'google' ? (
              <div className="google-view">
                {googleCatalog?.stale && <div className="catalog-stale" role="status">Mostro la cache: il catalogo potrebbe non essere aggiornato.</div>}
                {!googleCatalog?.items.length && !catalogBusy && (
                  <div className="empty-state">
                    <span className="empty-index">G / 00</span>
                    <h2>Il catalogo è ancora vuoto.</h2>
                    <p>Connettiti per caricare le famiglie Google Fonts. Le varianti scaricate restano nel vault locale.</p>
                    <button className="btn btn-primary" onClick={handleRefreshCatalog} type="button">Carica catalogo</button>
                  </div>
                )}
                {visibleCatalog.map((family, index) => (
                  <article className={`google-row ${selectedGoogleId === family.id ? 'is-selected' : ''}`} key={family.id}>
                    <button className="google-family-select" onClick={() => setSelectedGoogleId(family.id)} type="button">
                      <span className="google-index">{String(page * PAGE_SIZE + index + 1).padStart(2, '0')}</span>
                      <span className="google-family-copy">
                        <span className="google-family-name">{family.family}</span>
                        <span className="google-meta">{family.category} <i>·</i> {family.variants.length} varianti</span>
                      </span>
                      <span className="google-sample" aria-hidden="true">Ag</span>
                    </button>
                    <button className="btn btn-ghost btn-xs family-download" disabled={operation === 'google-family'} onClick={() => setPendingFamily(family)} type="button">Scarica famiglia</button>
                  </article>
                ))}
                {filteredCatalog.length > PAGE_SIZE && (
                  <Pagination page={page} pageCount={catalogPageCount} onPage={setPage} total={filteredCatalog.length} />
                )}
                {pendingFamily && (
                  <div className="family-confirm" role="group" aria-label={`Conferma download ${pendingFamily.family}`}>
                    <div>
                      <strong>Scaricare “{pendingFamily.family}”?</strong>
                      <p>{pendingFamily.variants.length} varianti verranno salvate nel vault locale.</p>
                    </div>
                    <button className="btn btn-primary btn-sm" disabled={operation === 'google-family'} onClick={handleDownloadFamily} type="button">Conferma</button>
                    <button className="btn btn-ghost btn-sm" onClick={() => setPendingFamily(null)} type="button">Annulla</button>
                  </div>
                )}
                {fontPage?.items.length ? (
                  <section className="downloaded-section">
                    <div className="section-rule"><span>IL TUO VAULT</span><span>{fontPage.total.toString().padStart(2, '0')}</span></div>
                    <div className="downloaded-list">
                      {fontPage.items.map((font) => (
                        <button className="downloaded-item" key={font.id} onClick={() => { setTab('local'); setSelectedFontId(font.id); }} type="button">
                          <span>{font.family}</span><small>{font.subfamily} · {font.weight}</small>
                        </button>
                      ))}
                    </div>
                  </section>
                ) : null}
              </div>
            ) : (
              <>
                <div className="specimen-controls">
                  <label className="sample-input-wrap">
                    <span className="control-label">Testo di prova</span>
                    <input aria-label="Testo di anteprima" className="input sample-input" onChange={(event) => setPreviewText(event.target.value)} value={previewText} />
                  </label>
                  <label className="size-control">
                    <span className="control-label">{previewSize}<span>px</span></span>
                    <input aria-label="Dimensione anteprima" className="range range-xs" max={88} min={18} onChange={(event) => setPreviewSize(Number(event.target.value))} type="range" value={previewSize} />
                  </label>
                </div>
                {loading && <div className="list-loading"><span className="loading loading-spinner loading-sm" /> Aggiornamento elenco…</div>}
                {!loading && !fontPage?.items.length && (
                  <div className="empty-state">
                    <span className="empty-index">{tab === 'active' ? 'A / 00' : 'L / 00'}</span>
                    <h2>{tab === 'active' ? 'Nessun font attivo.' : 'Qui non c’è ancora niente.'}</h2>
                    <p>{tab === 'active' ? 'Attiva un font dalla libreria: sarà disponibile alle applicazioni Windows per questa sessione.' : 'Aggiungi una cartella di font oppure modifica la ricerca.'}</p>
                    {tab !== 'active' && <button className="btn btn-primary" onClick={handleScanFolder} type="button">Aggiungi cartella</button>}
                  </div>
                )}
                <div className="font-list" aria-label="Risultati font">
                  {fontPage?.items.map((font, index) => (
                    <button
                      aria-pressed={selectedFontId === font.id}
                      className={`font-row ${selectedFontId === font.id ? 'is-selected' : ''}`}
                      key={font.id}
                      onClick={() => setSelectedFontId(font.id)}
                      type="button"
                    >
                      <span className="font-row-number">{String(page * PAGE_SIZE + index + 1).padStart(2, '0')}</span>
                      <span className="font-row-main">
                        <span className="font-row-family">{font.family}</span>
                        <span className="font-row-meta">{font.subfamily} <i>·</i> {font.weight}{font.isItalic ? ' Italic' : ''}</span>
                        <span className="font-row-sample" style={{ fontFamily: selectedFontId === font.id && previewFontFamily ? `"${previewFontFamily}"` : 'Georgia, serif', fontSize: `${Math.min(previewSize, 38)}px` }}>{previewText || PREVIEW_SAMPLE}</span>
                      </span>
                      <span className="font-row-state">
                        {font.isActive && <span className="badge badge-success badge-outline">Attivo</span>}
                        {font.isFavorite && <span className="favorite-mark" aria-label="Preferito">★</span>}
                        {font.missingOnDisk && <span className="badge badge-error badge-outline">File mancante</span>}
                      </span>
                    </button>
                  ))}
                </div>
                {fontPage && fontPage.total > PAGE_SIZE && (
                  <Pagination page={page} pageCount={Math.ceil(fontPage.total / PAGE_SIZE)} onPage={setPage} total={fontPage.total} />
                )}
              </>
            )}
          </section>

          <aside className="inspector" aria-label="Dettagli font selezionato">
            {tab === 'google' ? (
              selectedGoogle ? (
                <GoogleInspector
                  busy={operation}
                  family={selectedGoogle}
                  fontFamily={previewFontFamily}
                  onInstall={handleInstallVariant}
                  onPreview={handleGooglePreview}
                  onVariant={setGoogleVariantId}
                  previewSize={previewSize}
                  previewState={previewState}
                  previewText={previewText}
                  variantId={googleVariantId}
                />
              ) : <InspectorEmpty title="Scegli una famiglia" text="Seleziona una voce del catalogo per esplorare le varianti." />
            ) : currentFont ? (
              <FontInspector
                busy={operation}
                collections={collections}
                font={currentFont}
                fontFamily={previewFontFamily}
                maintenanceBusy={maintenanceBusy}
                onAddToCollection={handleAddToCollection}
                onCopy={handleCopy}
                onFavorite={handleFavorite}
                onToggleActive={handleActivation}
                previewSize={previewSize}
                previewState={previewState}
                previewText={previewText}
              />
            ) : <InspectorEmpty title="Il prossimo carattere" text="Seleziona un font per aprire il campione e i controlli." />}
          </aside>
        </div>
        <footer className="statusbar">
          <span>{snapshot?.dataDirectory ?? 'Database locale'}</span>
          <span>Fontations · SQLite · Tauri</span>
        </footer>
      </main>

      {notice && (
        <div className={`toast-note toast-${notice.kind}`} role={notice.kind === 'error' ? 'alert' : 'status'}>
          <span>{notice.message}</span>
          <button aria-label="Chiudi notifica" onClick={() => setNotice(null)} type="button">×</button>
        </div>
      )}
    </div>
  );
}

function FontInspector({
  busy,
  collections,
  font,
  fontFamily,
  maintenanceBusy,
  onAddToCollection,
  onCopy,
  onFavorite,
  onToggleActive,
  previewSize,
  previewState,
  previewText,
}: {
  busy: string | null;
  collections: Collection[];
  font: FontItem;
  fontFamily: string | null;
  maintenanceBusy: boolean;
  onAddToCollection(id: number): void;
  onCopy(kind: 'css' | 'nextjs'): void;
  onFavorite(font: FontItem): void;
  onToggleActive(font: FontItem): void;
  previewSize: number;
  previewState: 'idle' | 'loading' | 'ready' | 'failed';
  previewText: string;
}) {
  const isBusy = busy?.endsWith(font.id) ?? false;
  return (
    <div className="inspector-content">
      <div className="inspector-kicker"><span>SPECIMEN</span><span>{font.source === 'google' ? 'GOOGLE' : 'LOCAL'}</span></div>
      <div className="inspector-title-block">
        <p className="eyebrow">{font.category === 'unknown' ? 'UNCLASSIFIED' : font.category.toUpperCase()}</p>
        <h2>{font.family}</h2>
        <p className="font-subtitle">{font.subfamily} <i>·</i> {font.weight}{font.isItalic ? ' italic' : ''}</p>
      </div>
      <div className={`hero-specimen ${previewState === 'loading' ? 'is-loading' : ''}`}>
        <span className="specimen-caption">Aa · {previewState === 'ready' ? 'ANTEPRIMA REALE' : previewState === 'loading' ? 'CARICAMENTO' : 'CAMPIONE'}</span>
        <p style={{ fontFamily: fontFamily ? `"${fontFamily}"` : 'Georgia, serif', fontSize: `${Math.min(previewSize + 12, 64)}px` }}>{previewText || font.family}</p>
        {previewState === 'failed' && <small className="preview-warning">Anteprima non disponibile; mostro un campione di sistema.</small>}
      </div>
      <div className="inspector-actions">
        <button className="btn btn-primary action-primary" disabled={maintenanceBusy || font.missingOnDisk || isBusy} onClick={() => onToggleActive(font)} type="button">
          {busy?.startsWith('active:') ? 'Aggiornamento…' : font.isActive ? 'Disattiva su Windows' : 'Attiva su Windows'}
        </button>
        <button className="btn btn-outline" disabled={isBusy} onClick={() => onFavorite(font)} type="button">
          {font.isFavorite ? 'Rimuovi dai preferiti' : 'Aggiungi ai preferiti'}
        </button>
      </div>
      <div className="inspector-section">
        <label className="control-label" htmlFor="collection-select">COLLEZIONE</label>
        <select className="select select-bordered inspector-select" defaultValue="" disabled={!collections.length || busy === 'collection-item'} id="collection-select" onChange={(event) => { if (event.target.value) onAddToCollection(Number(event.target.value)); }}>
          <option value="">Aggiungi a…</option>
          {collections.map((collection) => <option key={collection.id} value={collection.id}>{collection.name}</option>)}
        </select>
      </div>
      <div className="inspector-section">
        <div className="section-rule"><span>UTILIZZO</span><span>02</span></div>
        <div className="snippet-actions">
          <button className="btn btn-ghost btn-sm" disabled={busy === 'copy:css'} onClick={() => onCopy('css')} type="button">Copia CSS <span aria-hidden="true">↗</span></button>
          <button className="btn btn-ghost btn-sm" disabled={busy === 'copy:nextjs'} onClick={() => onCopy('nextjs')} type="button">Copia Next.js <span aria-hidden="true">↗</span></button>
        </div>
      </div>
      <div className="inspector-section font-facts">
        <div className="section-rule"><span>DETTAGLI</span><span>03</span></div>
        <dl>
          <div><dt>Stile</dt><dd>{font.fullName}</dd></div>
          <div><dt>Dimensione</dt><dd>{formatBytes(font.fileSize)}</dd></div>
          <div><dt>Stato</dt><dd>{font.missingOnDisk ? 'File mancante' : font.isActive ? 'Attivo temporaneamente' : 'Non attivo'}</dd></div>
        </dl>
      </div>
      <p className="inspector-footnote">L’attivazione rende il font disponibile alle applicazioni Windows per la sessione. Il file non viene spostato né installato.</p>
    </div>
  );
}

function GoogleInspector({
  busy,
  family,
  fontFamily,
  onInstall,
  onPreview,
  onVariant,
  previewSize,
  previewState,
  previewText,
  variantId,
}: {
  busy: string | null;
  family: GoogleFontCatalogItem;
  fontFamily: string | null;
  onInstall(): void;
  onPreview(): void;
  onVariant(value: string): void;
  previewSize: number;
  previewState: 'idle' | 'loading' | 'ready' | 'failed';
  previewText: string;
  variantId: string;
}) {
  return (
    <div className="inspector-content">
      <div className="inspector-kicker"><span>GOOGLE FONTS</span><span>{family.category.toUpperCase()}</span></div>
      <div className="inspector-title-block">
        <p className="eyebrow">FAMILY / {String(family.variants.length).padStart(2, '0')} VARIANTS</p>
        <h2>{family.family}</h2>
        <p className="font-subtitle">Open-source type family</p>
      </div>
      <div className="hero-specimen google-hero">
        <span className="specimen-caption">Aa · {previewState === 'ready' ? 'ANTEPRIMA SCARICATA' : 'CATALOGO'}</span>
        <p style={{ fontFamily: fontFamily ? `"${fontFamily}"` : 'Georgia, serif', fontSize: `${Math.min(previewSize + 12, 64)}px` }}>{previewText || family.family}</p>
        {previewState === 'failed' && <small className="preview-warning">Non è stato possibile caricare l’anteprima.</small>}
      </div>
      <div className="inspector-section">
        <label className="control-label" htmlFor="google-variant">VARIANTE</label>
        <select className="select select-bordered inspector-select" id="google-variant" onChange={(event) => onVariant(event.target.value)} value={variantId}>
          {family.variants.map((variant) => <option key={variant} value={variant}>{variantLabel(variant)}</option>)}
        </select>
      </div>
      <div className="inspector-actions google-actions">
        <button className="btn btn-outline" disabled={!variantId || busy === 'google-preview'} onClick={onPreview} type="button">{busy === 'google-preview' ? 'Caricamento…' : 'Prova anteprima'}</button>
        <button className="btn btn-primary action-primary" disabled={!variantId || busy === 'google-install'} onClick={onInstall} type="button">{busy === 'google-install' ? 'Download…' : 'Scarica variante'}</button>
      </div>
      <div className="inspector-section font-facts">
        <div className="section-rule"><span>DETTAGLI</span><span>03</span></div>
        <dl>
          <div><dt>Varianti</dt><dd>{family.variants.length}</dd></div>
          <div><dt>Subsets</dt><dd>{family.subsets.slice(0, 3).join(', ') || '—'}</dd></div>
          <div><dt>Popolarità</dt><dd>#{family.popularity || '—'}</dd></div>
        </dl>
      </div>
      <p className="inspector-footnote">Le anteprime vengono memorizzate nella cache locale. Scaricare una variante la aggiunge al vault, senza installarla in Windows.</p>
    </div>
  );
}

function InspectorEmpty({ title, text }: { title: string; text: string }) {
  return <div className="inspector-empty"><span className="empty-index">OFM / —</span><h2>{title}</h2><p>{text}</p></div>;
}

function Pagination({ page, pageCount, onPage, total }: { page: number; pageCount: number; onPage(value: number): void; total: number }) {
  return (
    <div className="pagination" aria-label="Paginazione">
      <span>{String(page * PAGE_SIZE + 1).padStart(2, '0')}–{Math.min((page + 1) * PAGE_SIZE, total)} <i>/</i> {total}</span>
      <div>
        <button aria-label="Pagina precedente" className="btn btn-ghost btn-sm" disabled={page <= 0} onClick={() => onPage(page - 1)} type="button">←</button>
        <span className="pagination-count">{String(page + 1).padStart(2, '0')} <i>/</i> {String(pageCount).padStart(2, '0')}</span>
        <button aria-label="Pagina successiva" className="btn btn-ghost btn-sm" disabled={page + 1 >= pageCount} onClick={() => onPage(page + 1)} type="button">→</button>
      </div>
    </div>
  );
}

async function attachPreviewFont(
  bytes: Uint8Array,
  familyName: string,
  target: React.MutableRefObject<PreviewResource | null>,
): Promise<string> {
  if (typeof FontFace === 'undefined' || !document.fonts) throw new Error('Font preview is unsupported');
  removePreviewResource(target);
  const buffer = new ArrayBuffer(bytes.byteLength);
  new Uint8Array(buffer).set(bytes);
  const url = URL.createObjectURL(new Blob([buffer], { type: 'font/ttf' }));
  const face = new FontFace(familyName, `url("${url}")`);
  try {
    await face.load();
    document.fonts.add(face);
    target.current = { face, url, familyName };
    return familyName;
  } catch (error) {
    URL.revokeObjectURL(url);
    throw error;
  }
}

function removePreviewResource(target: React.MutableRefObject<PreviewResource | null>) {
  if (!target.current) return;
  document.fonts?.delete(target.current.face);
  URL.revokeObjectURL(target.current.url);
  target.current = null;
}

function decodeBase64(value: string): Uint8Array {
  const binary = window.atob(value);
  return Uint8Array.from(binary, (character) => character.charCodeAt(0));
}

function safeFontKey(value: string): string {
  return value.replace(/[^a-zA-Z0-9_-]/g, '-');
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function variantLabel(value: string): string {
  if (value === 'regular') return 'Regular · 400';
  if (value === 'italic') return 'Italic · 400';
  const weight = value.match(/^([1-9]00)(italic)?$/);
  if (weight) return `${weight[2] ? 'Italic' : 'Regular'} · ${weight[1]}`;
  return value;
}
