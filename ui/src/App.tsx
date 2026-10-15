import { open } from '@tauri-apps/plugin-dialog';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

import { fontManagerApi } from './api';
import type { FontManagerApi } from './api';
import { PreviewQueue } from './adaptivePreviewQueue';
import { CloudCatalogView } from './CloudCatalogView';
import { SettingsView } from './SettingsView';
import { CategoryFilter } from './CategoryFilter';
import { categoryLabel, normalizeCategory } from './categories';
import type {
  CatalogSnapshot,
  Collection,
  FontCategory,
  FontItem,
  FontPage,
  GoogleFontCatalogItem,
  LibraryTab,
  StartupSnapshot,
  RemoteFontProvider,
} from './types';
import './styles.css';

const FONT_PAGE_SIZE = 100;
const PREVIEW_SAMPLE = 'Sphinx of black quartz, judge my vow.';
const GOOGLE_ROW_HEIGHT = 108;
const GOOGLE_VIEWPORT_FALLBACK = 520;
const PREVIEW_CONCURRENCY = 3;
type AppSection = 'library' | 'fontsource-extra' | 'fontshare' | 'nerd-fonts' | 'settings';
type NavigationItem = { id: string; section: AppSection; label: string; description: string; tab?: LibraryTab };
const NAVIGATION: NavigationItem[] = [
  { id: 'local', section: 'library', tab: 'local', label: 'Libreria locale', description: 'Font sul dispositivo' },
  { id: 'favorites', section: 'library', tab: 'favorites', label: 'Preferiti', description: 'La tua selezione' },
  { id: 'google', section: 'library', tab: 'google', label: 'Google Fonts', description: 'Catalogo online' },
  { id: 'fontsource-extra', section: 'fontsource-extra', label: 'Fontsource Extra', description: 'Catalogo online' },
  { id: 'fontshare', section: 'fontshare', label: 'Fontshare', description: 'Catalogo online' },
  { id: 'nerd-fonts', section: 'nerd-fonts', label: 'Nerd Fonts', description: 'Catalogo online' },
  { id: 'settings', section: 'settings', label: 'Impostazioni', description: 'Percorsi e scansione' },
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
  const [installedFonts, setInstalledFonts] = useState<FontItem[]>([]);
  const [collections, setCollections] = useState<Collection[]>([]);
  const [googleCatalog, setGoogleCatalog] = useState<CatalogSnapshot | null>(null);
  const [tab, setTab] = useState<LibraryTab>('local');
  const [activeSection, setActiveSection] = useState<AppSection>('library');
  const [search, setSearch] = useState('');
  const [category, setCategory] = useState<FontCategory>('all');
  const [googleCategory, setGoogleCategory] = useState<FontCategory>('all');
  const [googleViewport, setGoogleViewport] = useState({ top: 0, height: GOOGLE_VIEWPORT_FALLBACK });
  const [collectionId, setCollectionId] = useState<number | null>(null);
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

  const [notice, setNotice] = useState<Notice | null>(null);
  const [draggingFiles, setDraggingFiles] = useState(false);
  const previewResource = useRef<PreviewResource | null>(null);
  const fontListRef = useRef<HTMLDivElement | null>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const fontRequestId = useRef(0);
  const pendingSelection = useRef<string | null>(null);
  const skipFirstQuery = useRef(true);
  const appReady = snapshot !== null;
  const selectedGoogle = useMemo(
    () => googleCatalog?.items.find((item) => item.id === selectedGoogleId) ?? null,
    [googleCatalog, selectedGoogleId],
  );

  const refreshFonts = useCallback(async () => {
    const requestId = ++fontRequestId.current;
    const result = await listAllFonts(api, {
      tab,
      search,
      category: tab === 'google' ? 'all' : category,
      collectionId: tab === 'local' ? collectionId : null,
    });
    if (requestId !== fontRequestId.current) return;
    const preferredId = pendingSelection.current ?? selectedFontId;
    const validSelectedId = preferredId && result.items.some((font) => font.id === preferredId)
      ? preferredId
      : result.items[0]?.id ?? null;
    pendingSelection.current = null;
    setSelectedFontId(validSelectedId);
    setFontPage(result);
    setLoading(false);
  }, [api, category, collectionId, search, selectedFontId, tab]);

  const refreshActiveCount = useCallback(async () => {
    const active = await listAllFonts(api, {
      tab: 'active',
      search: '',
      category: 'all',
      collectionId: null,
    });
    setInstalledFonts(active.items);
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
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'enter') {
          setDraggingFiles(event.payload.paths.some((path) => path.toLocaleLowerCase().endsWith('.zip')));
          return;
        }
        if (event.payload.type === 'over') return;
        if (event.payload.type === 'leave') {
          setDraggingFiles(false);
          return;
        }

        setDraggingFiles(false);
        const archives = [...new Set(event.payload.paths.filter((path) => path.toLocaleLowerCase().endsWith('.zip')))];
        if (!archives.length) {
          setNotice({ message: 'Trascina un archivio ZIP con font TTF o OTF.', kind: 'info' });
          return;
        }

        setOperation('zip-import');
        setNotice({
          message: archives.length === 1 ? 'Importazione dello ZIP in corso…' : `Importazione di ${archives.length} ZIP in corso…`,
          kind: 'info',
        });
        void (async () => {
          const reports = [];
          const failures: string[] = [];
          for (const path of archives) {
            try {
              reports.push(await api.importZipFont(path));
            } catch (error) {
              failures.push(errorMessage(error));
            }
          }

          const imported = reports.reduce((sum, report) => sum + report.imported, 0);
          const updated = reports.reduce((sum, report) => sum + report.updated, 0);
          const unchanged = reports.reduce((sum, report) => sum + report.skippedUnchanged, 0);
          const failedFonts = reports.reduce((sum, report) => sum + report.failed.length, 0);
          if (reports.length) {
            const fonts = await listAllFonts(api, {
              tab: 'local',
              search: '',
              category: 'all',
              collectionId: null,
            });
            setTab('local');
            setCollectionId(null);
            setSearch('');
            setCategory('all');
            setFontPage(fonts);
            setSelectedFontId(fonts.items[0]?.id ?? null);
          }

          if (failures.length || failedFonts) {
            setNotice({
              message: `Importazione parziale: ${failures.length} archivi non importati, ${failedFonts} font non leggibili.${failures[0] ? ` ${failures[0]}` : ''}`,
              kind: reports.length ? 'info' : 'error',
            });
          } else {
            setNotice({
              message: `Importazione completata: ${imported} nuovi, ${updated} aggiornati${unchanged ? `, ${unchanged} già presenti` : ''}.`,
              kind: 'success',
            });
          }
        })()
          .catch((error: unknown) => setNotice({ message: errorMessage(error), kind: 'error' }))
          .finally(() => setOperation(null));
      })
      .then((stop) => {
        if (mounted) unlisten = stop;
        else stop();
      })
      .catch((error: unknown) => {
        if (mounted) setNotice({ message: errorMessage(error), kind: 'error' });
      });
    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [api]);

  useEffect(() => {
    let mounted = true;
    api
      .initializeApp()
      .then((initial) => {
        if (!mounted) return;
        setSnapshot(initial);
        setFontPage(initial.fonts);
        setInstalledFonts(initial.fonts.items.filter((font) => font.isActive));
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
            const messages = [...report.errors, ...report.restoreFailures.map((family) => `Installazione non riuscita: ${family}`)];
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
    const requestId = ++fontRequestId.current;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      setLoading(true);
      void refreshFonts().catch((error: unknown) => {
        if (!cancelled && requestId === fontRequestId.current) {
          setLoading(false);
          setNotice({ message: errorMessage(error), kind: 'error' });
        }
      });
    }, 140);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
      fontRequestId.current += 1;
    };
  }, [appReady, category, collectionId, refreshFonts, tab]);

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

  useEffect(() => {
    const list = fontListRef.current;
    if (!list) return;

    let disposed = false;
    let observer: IntersectionObserver | null = null;
    let mutationObserver: MutationObserver | null = null;
    const visible = new Set<HTMLElement>();
    const observed = new Set<HTMLElement>();
    const inFlight = new Map<HTMLElement, { key: string; resource: React.MutableRefObject<PreviewResource | null> }>();
    const resources = new Map<string, React.MutableRefObject<PreviewResource | null>>();
    const queue = new PreviewQueue({ maxConcurrency: PREVIEW_CONCURRENCY });
    const previewKey = (element: HTMLElement) => {
      const fontId = element.dataset.fontPreviewId;
      if (fontId) return `local:${fontId}`;
      const familyId = element.dataset.googlePreviewId;
      const variantId = element.dataset.googlePreviewVariant;
      return familyId && variantId ? `google:${familyId}:${variantId}` : null;
    };
    const load = (element: HTMLElement) => {
      if (disposed) return;
      const fontId = element.dataset.fontPreviewId;
      const familyId = element.dataset.googlePreviewId;
      const variantId = element.dataset.googlePreviewVariant;
      const key = previewKey(element);
      if (!key || resources.has(key) || inFlight.has(element)) return;

      const resource: React.MutableRefObject<PreviewResource | null> = { current: null };
      inFlight.set(element, { key, resource });
      const execute = async () => {
        if (disposed || !visible.has(element) || !element.isConnected) return;
        element.dataset.previewState = 'loading';
        try {
          const bytes = fontId
            ? await api.getFontPreviewBytes(fontId)
            : familyId && variantId
              ? await api.previewGoogleVariant(familyId, variantId).then((preview) => decodeBase64(preview.fontDataBase64))
              : await Promise.reject(new Error('Google preview variant is missing'));
          const familyName = await attachPreviewFont(bytes, `ofm-row-${safeFontKey(key)}`, resource);
          if (disposed || !visible.has(element) || !element.isConnected) {
            removePreviewResource(resource);
            if (resources.get(key) === resource) resources.delete(key);
            return;
          }
          resources.set(key, resource);
          element.style.fontFamily = `"${familyName}"`;
          element.dataset.previewState = 'ready';
        } catch (error) {
          removePreviewResource(resource);
          if (resources.get(key) === resource) resources.delete(key);
          if (!disposed && visible.has(element) && element.isConnected) element.dataset.previewState = 'failed';
          throw error;
        } finally {
          if (inFlight.get(element)?.resource === resource) inFlight.delete(element);
        }
      };

      if (familyId) {
        element.dataset.previewState = 'queued';
        void queue.enqueue(key, execute).then(() => {
          if (inFlight.get(element)?.resource === resource) inFlight.delete(element);
          if (!disposed && visible.has(element) && element.isConnected && !resources.has(key) && element.dataset.previewState !== 'failed') load(element);
        }, () => {
          if (inFlight.get(element)?.resource === resource) inFlight.delete(element);
          if (!disposed && visible.has(element) && element.isConnected) element.dataset.previewState = 'failed';
        });
      } else {
        void execute().catch(() => undefined);
      }
    };
    const release = (element: HTMLElement) => {
      const key = previewKey(element);
      if (!key) return;
      const cancelled = key.startsWith('google:') && queue.cancel(key);
      if (cancelled) inFlight.delete(element);
      const resource = resources.get(key);
      if (resource) {
        removePreviewResource(resource);
        resources.delete(key);
      }
      element.style.fontFamily = 'Georgia, serif';
      element.dataset.previewState = 'idle';
    };

    if (typeof IntersectionObserver !== 'undefined') {
      const intersectionObserver = new IntersectionObserver((entries) => {
        for (const entry of entries) {
          const element = entry.target as HTMLElement;
          if (entry.isIntersecting) {
            visible.add(element);
            load(element);
          } else {
            visible.delete(element);
            release(element);
          }
        }
      }, { root: list, rootMargin: `${GOOGLE_ROW_HEIGHT}px 0px` });
      observer = intersectionObserver;
    }

    const scanSamples = () => {
      const samples = new Set(list.querySelectorAll<HTMLElement>('[data-font-preview-id], [data-google-preview-id]'));
      for (const element of observed) {
        if (samples.has(element)) continue;
        observer?.unobserve(element);
        observed.delete(element);
        visible.delete(element);
        release(element);
      }
      for (const element of samples) {
        if (observed.has(element)) continue;
        observed.add(element);
        if (observer) observer.observe(element);
        else {
          visible.add(element);
          load(element);
        }
      }
    };
    scanSamples();
    if (typeof MutationObserver !== 'undefined') {
      mutationObserver = new MutationObserver(scanSamples);
      mutationObserver.observe(list, { childList: true, subtree: true });
    }

    return () => {
      disposed = true;
      queue.dispose();
      observer?.disconnect();
      mutationObserver?.disconnect();
      resources.forEach(removePreviewResource);
    };
  }, [api, fontPage?.items, googleCatalog?.items, googleCategory, search, tab]);

  useEffect(() => {
    if (tab !== 'google' || !selectedGoogle || !googleVariantId) return;
    let cancelled = false;
    setPreviewFontFamily(null);
    setPreviewState('loading');
    void api
      .previewGoogleVariant(selectedGoogle.id, googleVariantId)
      .then(async (preview) => {
        if (cancelled) return null;
        const familyName = await attachPreviewFont(
          decodeBase64(preview.fontDataBase64),
          `ofm-google-${safeFontKey(selectedGoogle.id)}-${safeFontKey(googleVariantId)}`,
          previewResource,
        );
        if (cancelled) {
          removePreviewResource(previewResource);
          return null;
        }
        return familyName;
      })
      .then((familyName) => {
        if (!cancelled && familyName) {
          setPreviewFontFamily(familyName);
          setPreviewState('ready');
        }
      })
      .catch(() => {
        if (!cancelled) setPreviewState('failed');
      });
    return () => {
      cancelled = true;
      removePreviewResource(previewResource);
    };
  }, [api, googleVariantId, selectedGoogle, tab]);

  useEffect(() => () => removePreviewResource(previewResource), []);

  const currentFont = useMemo(
    () => fontPage?.items.find((font) => font.id === selectedFontId) ?? null,
    [fontPage, selectedFontId],
  );
  const filteredCatalog = useMemo(() => {
    const needle = search.trim().toLocaleLowerCase();
    return (googleCatalog?.items ?? []).filter((item) =>
      item.family.toLocaleLowerCase().includes(needle)
      && (googleCategory === 'all' || normalizeCategory(item.category) === googleCategory),
    );
  }, [googleCatalog, googleCategory, search]);

  const googleOverscan = 3;
  const googleWindow = useMemo(() => {
    const start = Math.max(0, Math.floor(googleViewport.top / GOOGLE_ROW_HEIGHT) - googleOverscan);
    const end = Math.min(
      filteredCatalog.length,
      Math.ceil((googleViewport.top + googleViewport.height) / GOOGLE_ROW_HEIGHT) + googleOverscan,
    );
    return { start, items: filteredCatalog.slice(start, end) };
  }, [filteredCatalog, googleOverscan, googleViewport.height, googleViewport.top]);
  const handleGoogleScroll = useCallback((event: React.UIEvent<HTMLDivElement>) => {
    const list = event.currentTarget;
    setGoogleViewport({ top: list.scrollTop, height: list.clientHeight || GOOGLE_VIEWPORT_FALLBACK });
  }, []);
  useEffect(() => {
    if (tab !== 'google') return;
    if (fontListRef.current) fontListRef.current.scrollTop = 0;
    setGoogleViewport({ top: 0, height: fontListRef.current?.clientHeight || GOOGLE_VIEWPORT_FALLBACK });
  }, [googleCategory, search, tab]);
  const title = activeSection !== 'library'
    ? NAVIGATION.find((item) => item.section === activeSection)?.label ?? 'Impostazioni'
    : collectionId !== null
      ? collections.find((item) => item.id === collectionId)?.name ?? 'Collezione'
      : NAVIGATION.find((item) => item.tab === tab)?.label ?? 'Libreria locale';

  useEffect(() => {
    if (activeSection !== 'library' || tab === 'google') return;
    fontListRef.current?.querySelector('[aria-pressed="true"]')?.scrollIntoView?.({ block: 'nearest' });
  }, [activeSection, tab, fontPage, selectedFontId]);

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
    if (font.isInstalled && !font.isActive) return;
    setOperation(`install:${font.id}`);
    try {
      await api.setFontActive(font.id, !font.isActive);
      await Promise.all([refreshFonts(), refreshActiveCount()]);
      setNotice({ message: font.isActive ? 'Font rimosso da Windows.' : 'Font installato su Windows.', kind: 'success' });
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
      setGoogleCategory('all');
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

  async function handleInstallVariant(family: GoogleFontCatalogItem = selectedGoogle!, variantId: string = googleVariantId) {
    if (!family || !variantId) return;
    setOperation('google-install');
    try {
      const font = await api.installGoogleVariant(family.id, variantId);
      await api.setFontActive(font.id, true);
      handleRemoteInstalled([font.id], `${font.family} ${font.subfamily} installato su Windows.`, false);
    } catch (error) {
      setNotice({ message: errorMessage(error), kind: 'error' });
    } finally {
      setOperation(null);
    }
  }

  function handleRemoteInstalled(fontIds: string[], message: string, failed: boolean) {
    const targetId = fontIds[0] ?? null;
    pendingSelection.current = targetId;
    setActiveSection('library');
    setTab('local');
    setCollectionId(null);
    setSearch('');
    setCategory('all');
    setSelectedFontId(targetId);
    setLoading(true);
    const requestId = ++fontRequestId.current;
    void listAllFonts(api, { tab: 'local', search: '', category: 'all', collectionId: null }).then((result) => {
      if (requestId !== fontRequestId.current) return;
      const targetIndex = targetId ? result.items.findIndex((font) => font.id === targetId) : -1;
      setFontPage(result);
      setSelectedFontId(targetIndex >= 0 ? targetId : result.items[0]?.id ?? null);
      setLoading(false);
    }).catch((error: unknown) => {
      if (requestId !== fontRequestId.current) return;
      setLoading(false);
      setNotice({ message: errorMessage(error), kind: 'error' });
    });
    setNotice({ message, kind: failed ? 'error' : 'success' });
    void refreshActiveCountRef.current().catch((error: unknown) => setNotice({ message: errorMessage(error), kind: 'error' }));
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
        <nav className="primary-nav" aria-label="Sezioni libreria e cataloghi">
          {NAVIGATION.map((item) => {
            const current = item.tab
              ? activeSection === 'library' && tab === item.tab && collectionId === null
              : activeSection === item.section;
            return (
              <button
                aria-current={current ? 'page' : undefined}
                className={`nav-item ${current ? 'is-current' : ''}`}
                key={item.id}
                onClick={() => {
                  setCollectionId(null);
                  if (item.tab) {
                    setActiveSection('library');
                    setTab(item.tab);
                  } else {
                    setTab('local');
                    setActiveSection(item.section);
                  }
                }}
                type="button"
              >
                <span>{item.label}</span>
                <span className="nav-description">{item.description}</span>
              </button>
            );
          })}
        </nav>

        <div className="collection-heading">
          <span className="sidebar-label">Collezioni</span>
          <span className="collection-total">{collections.length}</span>
        </div>
        <div className="collection-list">
          {collections.map((collection) => (
            <button
              aria-current={collectionId === collection.id ? 'page' : undefined}
              className={`collection-link ${collectionId === collection.id ? 'is-current' : ''}`}
              key={collection.id}
              onClick={() => {
                setActiveSection('library');
                setTab('local');
                setCollectionId(collection.id);
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
        <button className="btn btn-primary scan-button" disabled={operation === 'scan'} onClick={handleScanFolder} type="button">
          {operation === 'scan' ? 'Scansione…' : '＋  Aggiungi cartella'}
        </button>
      </aside>

      <main className="main-panel">
        <header className="topbar">
          <div className="breadcrumb"><span>OPEN FONT MANAGER</span><span className="breadcrumb-slash">/</span>{title.toLocaleUpperCase()}</div>
          <div className="topbar-status">
            <span className={`status-dot ${maintenanceBusy ? 'is-busy' : ''}`} />
            <span>{maintenanceBusy ? 'Indicizzazione' : `${snapshot?.activeCount ?? 0} font installat${snapshot?.activeCount === 1 ? 'o' : 'i'}`}</span>
          </div>
        </header>

        {activeSection === 'library' ? (
        <div className="workspace">
          <section className="library-panel" aria-label={title}>
            <div className="page-heading">
              <div>
                <p className="eyebrow">CAMPIONI TIPOGRAFICI</p>
                <h1>{tab === 'google' ? 'Trova il tuo prossimo carattere.' : title}</h1>
                <p className="page-description">
                  {tab === 'google'
                    ? 'Confronta le famiglie e scarica le varianti nella Libreria locale.'
                    : tab === 'active'
                      ? 'Gestisci i font installati da questa libreria.'
                      : collectionId !== null
                        ? 'Una selezione curata, pronta per il prossimo progetto.'
                        : 'Cerca e organizza i font locali e quelli scaricati dai cataloghi cloud.'}
                </p>
              </div>
              <div className="page-total" aria-live="polite">
          <div className="page-total-number">{tab === 'google' ? filteredCatalog.length : fontPage?.total ?? 0}</div>
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
                  }}
                  placeholder={tab === 'google' ? 'Cerca nel catalogo…' : 'Cerca per famiglia o stile…'}
                  ref={searchInputRef}
                  type="search"
                  value={search}
                />
                <kbd className="kbd search-shortcut">Ctrl K</kbd>
              </label>

              {tab === 'google' && (
                <button className="btn btn-outline catalog-refresh" disabled={catalogBusy} onClick={handleRefreshCatalog} type="button">
                  {catalogBusy ? <span className="loading loading-spinner loading-xs" /> : '↻'}
                  {catalogBusy ? 'Aggiornamento…' : 'Aggiorna catalogo'}
                </button>
              )}
            </div>

            <CategoryFilter value={tab === 'google' ? googleCategory : category} onChange={tab === 'google' ? setGoogleCategory : setCategory} />
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

            {tab === 'google' ? (
              <div className="google-view">
                {googleCatalog?.stale && <div className="catalog-stale" role="status">Mostro la cache: il catalogo potrebbe non essere aggiornato.</div>}
                <div className="font-list google-font-list" aria-label="Risultati font" onScroll={handleGoogleScroll} ref={fontListRef}>
                  <div className="google-catalog-spacer" style={{ height: `${filteredCatalog.length * GOOGLE_ROW_HEIGHT}px` }}>
                  {googleWindow.items.map((family, index) => {
                    const variantId = family.defVariant || family.variants[0] || '';
                    const catalogIndex = googleWindow.start + index;
                    const installed = installedFonts.some((font) => font.isActive && font.source === 'google' && font.sourceFamilyId === family.id && font.sourceVariantId === variantId);
                    return (
                      <div className="google-font-row" key={family.id} style={{ height: `${GOOGLE_ROW_HEIGHT}px`, top: `${catalogIndex * GOOGLE_ROW_HEIGHT}px` }}>
                        <button
                          aria-pressed={selectedGoogleId === family.id}
                          className={`font-row ${selectedGoogleId === family.id ? 'is-selected' : ''}`}
                          onClick={() => setSelectedGoogleId(family.id)}
                          type="button"
                        >
                          <span className="font-row-main">
                            <span className="font-row-family">{family.family}</span>
                            <span className="font-row-meta">{categoryLabel(family.category)} <i>·</i> {family.variants.length} varianti</span>
                            <span
                              className="font-row-sample"
                              data-google-preview-id={family.id}
                              data-google-preview-variant={variantId}
                              data-preview-state="idle"
                              style={{ fontSize: `${Math.min(previewSize, 38)}px` }}
                            >{previewText || PREVIEW_SAMPLE}</span>
                          </span>
                          <span className="font-row-state"><span className="badge badge-outline">{family.variants.length} stili</span></span>
                        </button>
                    <button aria-label={`${installed ? 'Installato' : 'Installa'} ${family.family}`} className="btn btn-ghost btn-xs family-download" disabled={operation === 'google-install' || installedFonts.some((font) => font.isActive && font.source === 'google' && font.sourceFamilyId === family.id && font.sourceVariantId === variantId)} onClick={() => { setSelectedGoogleId(family.id); setGoogleVariantId(variantId); void handleInstallVariant(family, variantId); }} type="button">{installedFonts.some((font) => font.isActive && font.source === 'google' && font.sourceFamilyId === family.id && font.sourceVariantId === variantId) ? 'Installato' : operation === 'google-install' ? 'Installazione…' : 'Installa'}</button>
                      </div>
                    );
                  })}
                  </div>
                  {!googleCatalog?.items.length && !catalogBusy && (
                    <div className="empty-state">
                      <span className="empty-index">G</span>
                      <h2>Il catalogo è ancora vuoto.</h2>
                      <p>Carica il catalogo Google Fonts per confrontare i campioni e scaricare le varianti nella Libreria locale.</p>
                      <button className="btn btn-primary" onClick={handleRefreshCatalog} type="button">Carica catalogo</button>
                    </div>
                  )}
                  {!!googleCatalog?.items.length && !filteredCatalog.length && (
                    <div className="empty-state"><span className="empty-index">G</span><h2>Nessuna famiglia trovata.</h2><p>Modifica la ricerca o scegli un’altra categoria.</p></div>
                  )}

                </div>
              </div>
            ) : (
              <>
                {loading && <div className="list-loading"><span className="loading loading-spinner loading-sm" /> Aggiornamento elenco…</div>}
                {!loading && !fontPage?.items.length && (
                  <div className="empty-state">
                    <span className="empty-index">L</span>
                    <h2>Qui non c’è ancora niente.</h2>
                    <p>Aggiungi una cartella di font oppure modifica la ricerca.</p>
                    <button className="btn btn-primary" onClick={handleScanFolder} type="button">Aggiungi cartella</button>
                  </div>
                )}
                <div className="font-list" aria-label="Risultati font" ref={fontListRef}>
                  {fontPage?.items.map((font) => (
                    <button
                      aria-pressed={selectedFontId === font.id}
                      className={`font-row ${selectedFontId === font.id ? 'is-selected' : ''}`}
                      key={font.id}
                      onClick={() => setSelectedFontId(font.id)}
                      type="button"
                    >

                      <span className="font-row-main">
                        <span className="font-row-family">{font.family}</span>
                        <span className="font-row-meta">{font.subfamily} <i>·</i> {font.weight}{font.isItalic ? ' Italic' : ''}</span>
                        <span className="font-row-sample" data-font-preview-id={font.id} style={{ fontSize: `${Math.min(previewSize, 38)}px` }}>{previewText || PREVIEW_SAMPLE}</span>
                      </span>
                      <span className="font-row-state">
                        {font.isVariable && <span className="badge badge-outline">Variabile</span>}
                        {font.isInstalled && <span className="badge badge-success badge-outline">Installato</span>}
                        {font.isFavorite && <span className="favorite-mark" aria-label="Preferito">★</span>}
                        {font.missingOnDisk && <span className="badge badge-error badge-outline">File mancante</span>}
                      </span>
                    </button>
                  ))}
                </div>
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
      ) : activeSection === 'settings' ? (
          <SettingsView api={api} onLibraryChanged={() => Promise.all([refreshFonts(), refreshActiveCount()]).then(() => undefined)} />
        ) : (
          <CloudCatalogView api={api} installedFonts={installedFonts} onInstalled={handleRemoteInstalled} provider={activeSection as Exclude<RemoteFontProvider, 'google'>} />
        )}
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
      {draggingFiles && (
        <div className="drop-target" role="status">
          Rilascia lo ZIP per aggiungere i font alla Libreria locale
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
      <div className="inspector-kicker"><span>SPECIMEN</span><span>{sourceLabel(font.source)}</span></div>
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
        <button className="btn btn-primary action-primary" disabled={maintenanceBusy || font.missingOnDisk || isBusy || (font.isInstalled && !font.isActive)} onClick={() => onToggleActive(font)} type="button">
          {busy?.startsWith('install:') ? 'Aggiornamento…' : font.isInstalled && !font.isActive ? 'Già installato' : font.isActive ? 'Disinstalla' : 'Installa'}
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
        <div className="section-rule"><span>UTILIZZO</span></div>
        <div className="snippet-actions">
          <button className="btn btn-ghost btn-sm" disabled={busy === 'copy:css'} onClick={() => onCopy('css')} type="button">Copia CSS <span aria-hidden="true">↗</span></button>
          <button className="btn btn-ghost btn-sm" disabled={busy === 'copy:nextjs'} onClick={() => onCopy('nextjs')} type="button">Copia Next.js <span aria-hidden="true">↗</span></button>
        </div>
      </div>
      <div className="inspector-section font-facts">
        <div className="section-rule"><span>DETTAGLI</span></div>
        <dl>
          <div><dt>Stile</dt><dd>{font.fullName}</dd></div>
          <div><dt>Dimensione</dt><dd>{formatBytes(font.fileSize)}</dd></div>
          <div><dt>Stato</dt><dd>{font.missingOnDisk ? 'File mancante' : font.isInstalled ? 'Installato' : 'Non installato'}</dd></div>
        </dl>
      </div>
      <p className="inspector-footnote">Installa il font per il tuo utente Windows. Rimuoverlo da Windows non elimina il file dalla libreria locale.</p>
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
        <p className="eyebrow">FAMILY <span>·</span> {family.variants.length} VARIANTS</p>
        <h2>{family.family}</h2>
        <p className="font-subtitle">Open-source type family</p>
      </div>
      <div className="hero-specimen google-hero">
        <span className="specimen-caption">Aa · {previewState === 'ready' ? 'ANTEPRIMA CARICATA' : previewState === 'loading' ? 'CARICAMENTO ANTEPRIMA' : 'CATALOGO'}</span>
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
        {previewState === 'failed' && <button className="btn btn-outline" disabled={!variantId || busy === 'google-preview'} onClick={onPreview} type="button">{busy === 'google-preview' ? 'Caricamento…' : 'Riprova anteprima'}</button>}
        <button className="btn btn-primary action-primary" disabled={!variantId || busy === 'google-install'} onClick={onInstall} type="button">{busy === 'google-install' ? 'Installazione…' : 'Installa'}</button>
      </div>
      <div className="inspector-section font-facts">
        <div className="section-rule"><span>DETTAGLI</span></div>
        <dl>
          <div><dt>Varianti</dt><dd>{family.variants.length}</dd></div>
          <div><dt>Subsets</dt><dd>{family.subsets.slice(0, 3).join(', ') || 'Non disponibili'}</dd></div>
          <div><dt>Popolarità</dt><dd>{family.popularity ? `#${family.popularity}` : 'Non disponibile'}</dd></div>
        </dl>
      </div>
        <p className="inspector-footnote">Installa la variante selezionata per il tuo utente Windows.</p>
    </div>
  );
}

function InspectorEmpty({ title, text }: { title: string; text: string }) {
  return <div className="inspector-empty"><span className="empty-index">OFM</span><h2>{title}</h2><p>{text}</p></div>;
}

async function listAllFonts(
  api: FontManagerApi,
  query: Omit<Parameters<FontManagerApi['listFonts']>[0], 'page' | 'pageSize'>,
): Promise<FontPage> {
  const firstPage = await api.listFonts({ ...query, page: 0, pageSize: FONT_PAGE_SIZE });
  const pageSize = firstPage.pageSize || FONT_PAGE_SIZE;
  const pages = [firstPage];
  const pageCount = Math.ceil(firstPage.total / pageSize);

  for (let page = 1; page < pageCount; page += 1) {
    pages.push(await api.listFonts({ ...query, page, pageSize }));
  }

  return {
    ...firstPage,
    items: pages.flatMap((result) => result.items),
    page: 0,
    pageSize,
  };
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

function sourceLabel(source: FontItem['source']): string {
  switch (source) {
    case 'google': return 'GOOGLE FONTS';
    case 'fontsource': return 'FONTSOURCE';
    case 'fontshare': return 'FONTSHARE';
    case 'nerd-fonts': return 'NERD FONTS';
    default: return 'LOCALE';
  }
}

function variantLabel(value: string): string {

  if (value === 'regular') return 'Regular · 400';
  if (value === 'italic') return 'Italic · 400';
  const weight = value.match(/^([1-9]00)(italic)?$/);
  if (weight) return `${weight[2] ? 'Italic' : 'Regular'} · ${weight[1]}`;
  return value;
}
