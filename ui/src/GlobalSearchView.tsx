import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { FontManagerApi } from './api';
import type { InspirationFontRef } from './inspirationTypes';
import type { FontCategory, FontItem, RemoteFontCatalogSnapshot, RemoteFontProvider } from './types';
import { CategoryFilter } from './CategoryFilter';
import { categoryLabel, normalizeCategory } from './categories';
import { filterGlobalFonts, groupGlobalFonts, isInstalled, type GlobalFontGroup, type GlobalFontResult } from './globalSearch';
import { PreviewQueue } from './adaptivePreviewQueue';

const PROVIDERS: RemoteFontProvider[] = ['google', 'fontsource-extra', 'fontshare', 'nerd-fonts'];
const PROVIDER_NAMES: Record<RemoteFontProvider, string> = { google: 'Google Fonts', 'fontsource-extra': 'Fontsource Extra', fontshare: 'Fontshare', 'nerd-fonts': 'Nerd Fonts' };
type ProviderState = { status: 'idle' | 'loading' | 'ready' | 'error'; snapshot: RemoteFontCatalogSnapshot | null; error: string | null };
type ProviderMap = Record<RemoteFontProvider, ProviderState>;
const idleProviders = (): ProviderMap => Object.fromEntries(PROVIDERS.map((provider) => [provider, { status: 'idle', snapshot: null, error: null }])) as ProviderMap;

export function GlobalSearchView({ api, installedFonts, onInstalled, onTryFont, previewText: initialText = 'Aa Bb Cc 0123', previewSize: initialSize = 28 }: {
  api: FontManagerApi; installedFonts: FontItem[]; onInstalled(fontIds: string[], message: string, failed: boolean): void;
  onTryFont?: (font: InspirationFontRef) => void; previewText?: string; previewSize?: number;
}) {
  const [providers, setProviders] = useState<ProviderMap>(idleProviders);
  const [enabled, setEnabled] = useState<Set<RemoteFontProvider>>(() => new Set(PROVIDERS));
  const [query, setQuery] = useState('');
  const [previewText, setPreviewText] = useState(initialText);
  const [previewSize, setPreviewSize] = useState(initialSize);
  const [category, setCategory] = useState<FontCategory>('all');
  const [variable, setVariable] = useState<'any' | 'variable' | 'static' | 'unknown'>('any');
  const [installedFilter, setInstalledFilter] = useState<'all' | 'installed' | 'not-installed'>('all');
  const [selectedKey, setSelectedKey] = useState<string | null>(null);
  const [selectedSourceKey, setSelectedSourceKey] = useState<string | null>(null);
  const [variantId, setVariantId] = useState('');
  const [busyKey, setBusyKey] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const requestVersions = useRef(new Map<RemoteFontProvider, number>());
  const loads = useRef(new Map<RemoteFontProvider, { version: number; promise: Promise<void> }>());
  const catalogQueue = useRef(new PreviewQueue({ maxConcurrency: 2 }));
  const previewQueue = useRef(new PreviewQueue({ maxConcurrency: 3 }));
  const installLocks = useRef(new Set<string>());
  const [page, setPage] = useState(0);
  useEffect(() => { catalogQueue.current = new PreviewQueue({ maxConcurrency: 2 }); previewQueue.current = new PreviewQueue({ maxConcurrency: 3 }); return () => { catalogQueue.current.dispose(); previewQueue.current.dispose(); }; }, []);
  useEffect(() => { setPage(0); }, [query, category, variable, installedFilter, enabled]);
  const loadProvider = useCallback((provider: RemoteFontProvider, _refresh: boolean): Promise<void> => {
    const existing = loads.current.get(provider);
    if (existing?.promise) return existing.promise;
    const version = (requestVersions.current.get(provider) ?? 0) + 1;
    requestVersions.current.set(provider, version);
    const promise = (async () => {
    setProviders((current) => ({ ...current, [provider]: { ...current[provider], status: 'loading', error: null } }));
    try {
      const fresh = await api.refreshRemoteCatalog(provider);
      if (requestVersions.current.get(provider) !== version || fresh.provider !== provider) return;
      setProviders((current) => ({ ...current, [provider]: { status: 'ready', snapshot: fresh, error: null } }));
    } catch (cause) {
      if (requestVersions.current.get(provider) === version) setProviders((current) => ({ ...current, [provider]: { ...current[provider], status: 'error', error: message(cause) } }));
    } finally {
      if (requestVersions.current.get(provider) === version) loads.current.delete(provider);
    }
    })();
    loads.current.set(provider, { version, promise });
    return promise;
  }, [api]);
  useEffect(() => {
    let live = true;
    void Promise.all(PROVIDERS.map(async (provider) => {
      setProviders((current) => ({ ...current, [provider]: { ...current[provider], status: 'loading', error: null } }));
      try {
        const cached = await api.loadCachedRemoteCatalog(provider);
        if (!live) return { provider, stale: false, failed: false };
        if (cached) setProviders((current) => ({ ...current, [provider]: { status: 'ready', snapshot: cached, error: null } }));
        return { provider, stale: !cached || cached.stale, failed: false };
      } catch (cause) {
        if (live) setProviders((current) => ({ ...current, [provider]: { ...current[provider], status: 'error', error: message(cause) } }));
        return { provider, stale: true, failed: true };
      }
    })).then((results) => {
      if (!live) return;
      results.filter((result) => result.stale).forEach(({ provider }) => void catalogQueue.current.enqueue(`refresh:${provider}`, () => loadProvider(provider, true)));
    });
    return () => { live = false; };
  }, [api, loadProvider]);
  const rows = useMemo(() => filterGlobalFonts(PROVIDERS.filter((provider) => enabled.has(provider) && providers[provider].snapshot).map((provider) => ({ provider, snapshot: providers[provider].snapshot! })), { query, category, variable }), [providers, enabled, query, category, variable]);
  const groups = useMemo(() => groupGlobalFonts(rows).map((group) => ({ ...group, sources: group.sources.filter((row) => { const installed = isInstalled(row.font, row.provider, installedFonts); return installedFilter === 'all' || (installedFilter === 'installed' ? installed : !installed); }) })).filter((group) => group.sources.length > 0), [rows, installedFonts, installedFilter]);
  const pageCount = Math.max(1, Math.ceil(groups.length / 100));
  useEffect(() => { setPage((current) => Math.min(current, pageCount - 1)); }, [pageCount]);
  const visibleRows = groups.slice(page * 100, (page + 1) * 100);
  const selectedGroup = groups.find((group) => group.key === selectedKey) ?? null;
  const selected = selectedGroup?.sources.find((row) => rowKey(row) === selectedSourceKey) ?? selectedGroup?.sources[0] ?? null;
  const selectedVariant = selected?.font.variants.find((v) => v.id === variantId) ?? selected?.font.variants.find((v) => v.id === selected.font.previewVariantId) ?? selected?.font.variants[0] ?? null;
  useEffect(() => { setSelectedKey((current) => current && groups.some((group) => group.key === current) ? current : null); }, [groups]);
  useEffect(() => { if (!selected) { setVariantId(''); return; } setVariantId((current) => selected.font.variants.some((v) => v.id === current) ? current : selected.font.previewVariantId ?? selected.font.variants[0]?.id ?? ''); }, [selected]);
  const refresh = (provider: RemoteFontProvider) => {
    if (loads.current.has(provider)) return;
    setProviders((current) => ({ ...current, [provider]: { ...current[provider], status: 'loading', error: null } }));
    void catalogQueue.current.enqueue(`refresh:${provider}`, () => loadProvider(provider, true));
  };
  const install = async (row: GlobalFontResult, requestedVariantId?: string | null) => {
    const key = rowKey(row); if (installLocks.current.has(key) || isInstalled(row.font, row.provider, installedFonts, requestedVariantId ?? undefined)) return;
    installLocks.current.add(key); setBusyKey(key); setNotice(null);
    try {
      const variant = requestedVariantId ?? row.font.previewVariantId ?? row.font.variants[0]?.id ?? null;
      const result = await api.downloadRemoteFamily(row.provider, row.font.id, row.provider === 'google' ? variant : null, true);
      const report = result.report; const partial = report.failures.length > 0 || report.activatedFonts < report.indexedFonts;
      const scope = row.provider === 'google' ? 'La variante richiesta' : 'La famiglia';
      const summary = `${scope} ${row.font.family}: ${report.activatedFonts} font attivati${partial ? `. Installazione parziale: ${report.failures.map((failure) => `${failure.variantId} (${failure.stage}): ${failure.reason}`).join('; ') || 'alcune varianti non risultano attive'}.` : '.'}`;
      if (result.fontIds.length && report.activatedFonts > 0) onInstalled(result.fontIds, summary, partial); else setNotice(`${summary} Nessun font è stato attivato.`);
    } catch (cause) { setNotice(`${row.font.family}: ${message(cause)}`); }
    finally { installLocks.current.delete(key); setBusyKey(null); }
  };
  return <section className="global-search-view" aria-label="Ricerca Globale"><h1>Ricerca Globale</h1>
    <div className="specimen-controls"><label className="sample-input-wrap"><span className="control-label">Testo di prova</span><input className="input" aria-label="Testo di anteprima" value={previewText} onChange={(event) => setPreviewText(event.target.value)} /></label><label className="size-control"><span className="control-label">{previewSize} px</span><input className="range range-xs" type="range" min={18} max={88} value={previewSize} aria-label="Dimensione anteprima" onChange={(event) => setPreviewSize(Number(event.target.value))} /></label></div>
    <p role="status">{groups.length} famiglie{PROVIDERS.some((provider) => enabled.has(provider) && providers[provider].status !== 'ready') ? ' · risultati parziali' : ''}</p>
    {notice && !selected && <div className="alert" role="status">{notice}</div>}
    <div className="global-search-toolbar"><label className="search-field"><span className="sr-only">Cerca font per famiglia, categoria o stile</span><input className="input search-input" type="search" value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Cerca famiglia, categoria o stile" /></label>
      <label className="global-filter-label">Stato installazione<select className="select select-bordered select-sm" value={installedFilter} onChange={(event) => setInstalledFilter(event.target.value as typeof installedFilter)}><option value="all">Tutti</option><option value="installed">Installati</option><option value="not-installed">Non installati</option></select></label>
      <label className="global-filter-label">Variabili<select className="select select-bordered select-sm" value={variable} onChange={(event) => setVariable(event.target.value as typeof variable)}><option value="any">Qualsiasi</option><option value="variable">Variabile</option><option value="static">Statico</option><option value="unknown">Non specificato</option></select></label></div>
    <fieldset className="global-providers" aria-label="Cataloghi da cercare">{PROVIDERS.map((provider) => <label key={provider} className="label cursor-pointer gap-2"><input className="checkbox checkbox-sm" type="checkbox" checked={enabled.has(provider)} onChange={(event) => setEnabled((old) => { const next = new Set(old); event.target.checked ? next.add(provider) : next.delete(provider); return next; })} /><span>{PROVIDER_NAMES[provider]}</span></label>)}</fieldset>
    <CategoryFilter value={category} onChange={setCategory} />
    <div className="global-provider-status" aria-live="polite">{PROVIDERS.filter((p) => enabled.has(p)).map((provider) => <div key={provider} className="global-provider-state"><span>{PROVIDER_NAMES[provider]}: {providerStatus(providers[provider])}</span>{providers[provider].snapshot?.stale && <span>Cache non aggiornata</span>}{providers[provider].error && <button className="btn btn-ghost btn-xs" type="button" onClick={() => refresh(provider)}>Riprova</button>}</div>)}</div>
    <div className="global-search-content"><div className="global-search-results"><ul className="list">{visibleRows.map((group) => <GlobalResultRow key={group.key} group={group} sourceKey={selectedKey === group.key ? selectedSourceKey : null} installedFonts={installedFonts} busyKey={busyKey} selected={selectedKey === group.key} onSelect={(row) => { setSelectedKey(group.key); setSelectedSourceKey(rowKey(row)); setNotice(null); }} onInstall={(row) => void install(row)} previewText={previewText} previewSize={previewSize} api={api} queue={previewQueue.current} />)}</ul>
      {groups.length > 100 && <nav aria-label="Paginazione risultati"><button type="button" className="btn btn-ghost btn-sm" disabled={page === 0} onClick={() => setPage((current) => current - 1)}>Precedente</button><span>Pagina {page + 1} di {pageCount}</span><button type="button" className="btn btn-ghost btn-sm" disabled={page + 1 >= pageCount} onClick={() => setPage((current) => current + 1)}>Successiva</button></nav>}
      {!groups.length && <p className="global-search-empty">{PROVIDERS.some((p) => enabled.has(p) && providers[p].status === 'loading') ? 'Attendo i cataloghi selezionati…' : PROVIDERS.some((p) => enabled.has(p) && providers[p].status === 'error') ? 'Alcuni cataloghi non sono disponibili. Riprova il provider interessato.' : 'Nessuna famiglia corrisponde ai filtri.'}</p>}</div>
      <aside className="card card-border global-search-inspector" aria-label="Dettagli famiglia selezionata">{selected ? <div className="card-body">
        <p>{PROVIDER_NAMES[selected.provider]} · {categoryLabel(normalizeCategory(selected.font.category))}</p>
        <h2 className="card-title">{selectedGroup?.family ?? selected.font.family}</h2>
        <label className="control-label" htmlFor="global-source">Provenienza</label><select id="global-source" className="select select-bordered" value={rowKey(selected)} onChange={(event) => setSelectedSourceKey(event.target.value)}>{selectedGroup?.sources.map((source) => <option key={rowKey(source)} value={rowKey(source)}>{PROVIDER_NAMES[source.provider]} · {source.font.family}</option>)}</select>
        <label className="control-label" htmlFor="global-variant">Variante</label>
        <select className="select select-bordered" id="global-variant" value={selectedVariant?.id ?? ''} onChange={(event) => setVariantId(event.target.value)} disabled={!selected.font.variants.length}>{selected.font.variants.map((v) => <option key={v.id} value={v.id}>{v.label}</option>)}</select>
        <GlobalInspectorPreview api={api} row={selected} variantId={selectedVariant?.id ?? null} text={previewText} size={previewSize} />
        <button className="btn btn-primary" disabled={busyKey !== null || isInstalled(selected.font, selected.provider, installedFonts, selectedVariant?.id)} onClick={() => void install(selected, selectedVariant?.id)}>{isInstalled(selected.font, selected.provider, installedFonts, selectedVariant?.id) ? 'Installato' : 'Installa'}</button>
        {onTryFont && <button className="btn btn-outline" onClick={() => onTryFont(fontRef(selected, selectedVariant?.id ?? null))}>Prova abbinamento</button>}
        {notice && <p role="status">{notice}</p>}
      </div> : <div className="card-body"><h2>Seleziona un font</h2><p>L’anteprima viene richiesta solo per la famiglia selezionata o visibile.</p></div>}</aside></div></section>;
}

function GlobalResultRow({ group, sourceKey, installedFonts, busyKey, selected, onSelect, onInstall, previewText, previewSize, api, queue }: { group: GlobalFontGroup; sourceKey: string | null; installedFonts: FontItem[]; busyKey: string | null; selected: boolean; onSelect(row: GlobalFontResult): void; onInstall(row: GlobalFontResult): void; previewText: string; previewSize: number; api: FontManagerApi; queue: PreviewQueue }) {
  const row = group.sources.find((source) => rowKey(source) === sourceKey) ?? group.sources[0];
  const installed = isInstalled(row.font, row.provider, installedFonts);
  const sampleRef = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const element = sampleRef.current;
    if (!element || row.provider === 'nerd-fonts' || typeof IntersectionObserver === 'undefined') return;
    let live = true; let resourceFace: FontFace | null = null; let resourceUrl: string | null = null;
    const observer = new IntersectionObserver((entries) => {
      if (!entries.some((entry) => entry.isIntersecting)) return;
      observer.disconnect();
      const variant = row.font.variants.find((v) => v.id === row.font.previewVariantId) ?? row.font.variants[0];
      if (!variant) { element.dataset.previewState = 'unavailable'; return; }
      void queue.enqueue(`row:${rowKey(row)}`, async () => {
        try { const bytes = await api.getRemotePreviewBytes(row.provider, row.font.id, variant.id); if (!live) return; resourceUrl = URL.createObjectURL(new Blob([new Uint8Array(bytes).buffer], { type: 'font/ttf' })); resourceFace = new FontFace(`global-preview-${rowKey(row).replace(/[^a-zA-Z0-9_-]/g, '-')}`, `url("${resourceUrl}")`); await resourceFace.load(); if (!live) return; document.fonts.add(resourceFace); element.style.fontFamily = `"${resourceFace.family}"`; element.dataset.previewState = 'loaded'; }
        catch { if (live) element.dataset.previewState = 'error'; }
      });
    }, { rootMargin: '120px' });
    observer.observe(element); return () => { live = false; observer.disconnect(); queue.cancel(`row:${rowKey(row)}`); if (resourceFace) document.fonts.delete(resourceFace); if (resourceUrl) URL.revokeObjectURL(resourceUrl); };
  }, [api, row.font, row.provider, queue]);
  const variable = row.font.isVariable ?? row.font.variants.find((v) => v.isVariable !== undefined)?.isVariable;
  return <li className="list-row"><div className="global-font-row"><button className={`font-row ${selected ? 'is-selected' : ''}`} type="button" aria-pressed={selected} onClick={() => onSelect(row)}><span className="font-row-main"><span className="font-row-family">{group.family}</span><span className="font-row-meta">{[...new Set(group.sources.map((source) => PROVIDER_NAMES[source.provider]))].join(' · ')} · {categoryLabel(normalizeCategory(row.font.category))} · {row.provider === 'nerd-fonts' ? 'Glifi aggiuntivi' : `${row.font.variants.length} ${row.font.variants.length === 1 ? 'stile' : 'stili'}`}{variable === true ? ' · Variabile' : ''}</span><span ref={sampleRef} className="font-row-sample" style={{ fontSize: `${Math.min(previewSize, 38)}px` }}>{previewText}</span></span></button><div className="flex gap-1">{group.sources.length > 1 && <select className="select select-bordered select-xs" aria-label={`Provenienza per ${group.family}`} value={rowKey(row)} onChange={(event) => { const source = group.sources.find((candidate) => rowKey(candidate) === event.target.value); if (source) onSelect(source); }}><option value={rowKey(row)}>{PROVIDER_NAMES[row.provider]}</option>{group.sources.filter((source) => source !== row).map((source) => <option key={rowKey(source)} value={rowKey(source)}>{PROVIDER_NAMES[source.provider]}</option>)}</select>}<button className="btn btn-ghost btn-xs" type="button" disabled={busyKey === rowKey(row) || installed} onClick={() => onInstall(row)}>{busyKey === rowKey(row) ? 'Installazione…' : installed ? 'Installato' : 'Installa'}</button></div></div></li>;
}

function GlobalInspectorPreview({ api, row, variantId, text, size }: { api: FontManagerApi; row: GlobalFontResult; variantId: string | null; text: string; size: number }) {
  const [state, setState] = useState<'loading' | 'ready' | 'error' | 'none'>('loading'); const [familyName, setFamilyName] = useState('');
  useEffect(() => {
    let live = true; let face: FontFace | null = null; let url: string | null = null;
    if (row.provider === 'nerd-fonts') { setState('none'); return; }
    if (!variantId) { setState('none'); return; }
    setState('loading'); setFamilyName('');
    void (async () => { try { const bytes = await api.getRemotePreviewBytes(row.provider, row.font.id, variantId); if (!live) return; url = URL.createObjectURL(new Blob([new Uint8Array(bytes).buffer], { type: 'font/ttf' })); face = new FontFace(`global-inspector-${rowKey(row).replace(/[^a-zA-Z0-9_-]/g, '-')}`, `url("${url}")`); await face.load(); if (!live || !face) return; document.fonts.add(face); setFamilyName(face.family); setState('ready'); } catch { if (live) setState('error'); } })();
    return () => { live = false; if (face) document.fonts.delete(face); if (url) URL.revokeObjectURL(url); };
  }, [api, row.font, row.provider, variantId]);
  return <div className="hero-specimen"><p style={{ fontSize: `${Math.min(size, 38)}px`, fontFamily: state === 'ready' ? `"${familyName}"` : 'inherit' }}>{text}</p>{state === 'loading' && <span>Caricamento anteprima…</span>}{state === 'error' && <span>Anteprima non disponibile per questa variante.</span>}{state === 'none' && <span>Il provider non espone un campione scaricabile.</span>}</div>;
}
function fontRef(row: GlobalFontResult, variantId: string | null): InspirationFontRef { return { provider: row.provider, familyId: row.font.id, variantId, localFontId: null, family: row.font.family, isVariable: row.font.isVariable ?? null }; }
function rowKey(row: GlobalFontResult): string { return `${row.provider}:${row.font.id}`; }
function providerStatus(state: ProviderState): string { return state.status === 'idle' ? 'in attesa della ricerca' : state.status === 'loading' ? 'caricamento catalogo' : state.status === 'error' ? 'errore' : state.snapshot?.stale ? 'pronto, cache non aggiornata' : 'pronto'; }
function message(error: unknown): string { return error instanceof Error ? error.message : String(error); }
