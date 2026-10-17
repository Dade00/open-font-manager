import { open } from '@tauri-apps/plugin-dialog';
import { useCallback, useEffect, useId, useRef, useState } from 'react';

import type { FontManagerApi } from './api';
import type { CloudFontImportOutcome, FontItem, RemoteFontCatalogItem, RemoteFontProvider } from './types';
import type { Board, BoardItem, Composition, CompositionRole, InspirationApi, InspirationDocument, InspirationFontRef } from './inspirationTypes';
import { inspirationApi as defaultPersistence } from './inspirationApi';
import { InspirationConfirm } from './InspirationConfirm';
import './inspiration-workflow.css';

const PROVIDERS: RemoteFontProvider[] = ['google', 'fontsource-extra', 'fontshare', 'nerd-fonts'];
const PROVIDER_NAMES: Record<RemoteFontProvider, string> = {
  google: 'Google Fonts', 'fontsource-extra': 'Fontsource Extra', fontshare: 'Fontshare', 'nerd-fonts': 'Nerd Fonts',
};
const INITIAL_ROLE: CompositionRole = { font: null, text: '', size: 48, lineHeight: 1.1, letterSpacing: 0, color: '#20201e' };
const INITIAL_BODY: CompositionRole = { font: null, text: '', size: 18, lineHeight: 1.5, letterSpacing: 0, color: '#20201e' };
type FontChoice = { ref: InspirationFontRef; label: string; local?: FontItem; remote?: RemoteFontCatalogItem };

function errorMessage(error: unknown) { return error instanceof Error ? error.message : String(error); }
function id() { return globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`; }
function newComposition(): Composition {
  return { id: id(), title: 'Composizione senza titolo', heading: { ...INITIAL_ROLE, text: 'Il tuo titolo' }, subheading: null, body: { ...INITIAL_BODY, text: 'Scrivi qui il testo di prova.' }, background: '#f3efe7', narrow: false };
}
function newBoard(title = 'Nuova board'): Board { return { id: id(), title, description: '', items: [] }; }
function sameRef(left: InspirationFontRef | null, right: InspirationFontRef) {
  return !!left && left.provider === right.provider && left.familyId === right.familyId && left.variantId === right.variantId && left.localFontId === right.localFontId;
}
function choiceKey(ref: InspirationFontRef) { return JSON.stringify([ref.provider, ref.familyId, ref.localFontId, ref.variantId]); }

export function InspirationView({ api, persistence = defaultPersistence, seedFont = null, onSeedConsumed, onInstalled, onDirtyChange }: {
  api: FontManagerApi;
  persistence?: InspirationApi;
  seedFont?: InspirationFontRef | null;
  onSeedConsumed?: () => void;
  onInstalled?: (fontIds: string[], message: string, failed: boolean) => void;
  onDirtyChange?: (dirty: boolean) => void;
}) {
  const [document, setDocument] = useState<InspirationDocument | null>(null);
  const documentRef = useRef<InspirationDocument | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [saveState, setSaveState] = useState<'saved' | 'dirty' | 'saving'>('saved');
  const [saveError, setSaveError] = useState<string | null>(null);
  const [section, setSection] = useState<'matches' | 'boards'>('matches');
  const [editing, setEditing] = useState<Composition | null>(newComposition);
  const [editingBaseline, setEditingBaseline] = useState<Composition | null>(() => structuredClone(editing));
  const [selectedBoard, setSelectedBoard] = useState<string | null>(null);
  const [boardTitle, setBoardTitle] = useState<string | null>(null);
  const [dialog, setDialog] = useState<{ title: string; action: () => void } | null>(null);
  const [fontChoices, setFontChoices] = useState<FontChoice[]>([]);
  const choicesRef = useRef<FontChoice[]>([]);
  const setChoices = (choices: FontChoice[]) => { choicesRef.current = choices; setFontChoices(choices); };
  const [fontError, setFontError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [isDirty, setIsDirty] = useState(false);
  const dragItem = useRef<string | null>(null);
  const saveChain = useRef<Promise<void>>(Promise.resolve());
  const [notice, setNotice] = useState<string | null>(null);
  const [itemDraft, setItemDraft] = useState<{ kind: 'note' | 'link' | 'font'; title: string; text: string; url: string } | null>(null);
  const [addingComposition, setAddingComposition] = useState(false);
  const [selectedComposition, setSelectedComposition] = useState('');
  const [editingItems, setEditingItems] = useState<string[]>([]);
  const consumedSeed = useRef<string | null>(null);
  const editingDirty = Boolean(editing && JSON.stringify(editing) !== JSON.stringify(editingBaseline));

  function discardContentDraft() {
    if ((itemDraft || addingComposition) && !window.confirm('Il contenuto non è ancora stato aggiunto alla board. Abbandonarlo?')) return false;
    setItemDraft(null);
    setAddingComposition(false);
    return true;
  }

  useEffect(() => {
    let current = true;
    void persistence.load().then((loaded) => {
      if (!current) return;
      documentRef.current = loaded;
      setDocument(loaded);
      setSelectedBoard(loaded.boards[0]?.id ?? null);
    }).catch((cause) => { if (current) setLoadError(errorMessage(cause)); })
      .finally(() => { if (current) setLoading(false); });
    return () => { current = false; };
  }, [persistence]);

  useEffect(() => {
    let current = true;
    setFontError(null);
    const loadAllLocal = async () => {
      const all: FontItem[] = [];
      for (let page = 0; ; page += 1) {
        const result = await api.listFonts({ tab: 'local', search: '', category: 'all', collectionId: null, page, pageSize: 100 });
        all.push(...result.items);
        if (all.length >= result.total || result.items.length === 0) break;
      }
      return all;
    };
    void Promise.allSettled([loadAllLocal(), ...PROVIDERS.map((provider) => api.loadCachedRemoteCatalog(provider))]).then((results) => {
      if (!current) return;
      const [localResult, ...catalogResults] = results;
      const failures: string[] = [];
      const localItems = localResult.status === 'fulfilled' ? localResult.value : [];
      if (localResult.status === 'rejected') failures.push(`Font locali: ${errorMessage(localResult.reason)}`);
      const locals: FontChoice[] = localItems.filter((font) => !font.missingOnDisk).map((font) => ({
        ref: { provider: 'local', familyId: font.sourceFamilyId, variantId: font.sourceVariantId ?? font.subfamily, localFontId: font.id, family: font.family, isVariable: font.isVariable },
        label: `${font.family} · ${font.subfamily || font.fullName}`, local: font,
      }));
      const remotes: FontChoice[] = [];
      catalogResults.forEach((result, index) => {
        if (result.status === 'rejected') { failures.push(`${PROVIDER_NAMES[PROVIDERS[index]]}: ${errorMessage(result.reason)}`); return; }
        result.value?.items.forEach((family) => family.variants.forEach((variant) => {
          remotes.push({ ref: { provider: family.provider, familyId: family.id, variantId: variant.id, localFontId: null, family: family.family, isVariable: variant.isVariable ?? family.isVariable ?? null }, label: `${family.family} · ${variant.label} (${PROVIDER_NAMES[family.provider]})`, remote: family });
        }));
      });
      setChoices([...locals, ...remotes]);
      if (failures.length) setFontError(failures.join(' · '));
    });
    return () => { current = false; };
  }, [api]);

  useEffect(() => {
    if (!seedFont) { consumedSeed.current = null; return; }
    const signature = JSON.stringify(seedFont);
    if (!document || section !== 'matches' || editingDirty || consumedSeed.current === signature) return;
    consumedSeed.current = signature;
    const composition = newComposition();
    composition.heading.font = seedFont;
    composition.heading.text = seedFont.family;
    beginEditing(composition);
    onSeedConsumed?.();
  }, [document, editingDirty, onSeedConsumed, seedFont, section]);

  const pendingSaves = useRef(0);
  const dirty = isDirty || saveState !== 'saved' || itemDraft !== null || addingComposition || boardTitle !== null || editingDirty;

  function beginEditing(composition: Composition, baseline: Composition = composition) {
    const apply = () => { setEditing(composition); setEditingBaseline(structuredClone(baseline)); setDialog(null); };
    if (editingDirty) setDialog({ title: 'Scartare modifiche?', action: apply });
    else apply();
  }

  useEffect(() => { onDirtyChange?.(dirty); return () => onDirtyChange?.(false); }, [dirty, onDirtyChange]);
  const saveFailed = useRef(false);
  const persist = useCallback(async (next: InspirationDocument) => {
    documentRef.current = next;
    setDocument(next);
    setSaveState('dirty');
    setSaveError(null);
    pendingSaves.current += 1;
    setBusy(true);
    setSaveState('saving');
    const operation = saveChain.current.then(async () => {
      const current = documentRef.current ?? next;
      const expectedRevision = current.revision;
      const saved = await persistence.save({ ...current, revision: expectedRevision }, expectedRevision);
      const latest = documentRef.current;
      const updated = latest ? { ...latest, revision: saved.revision } : saved;
      documentRef.current = updated;
      setDocument(updated);
      setSaveError(null);
      saveFailed.current = false;
      setIsDirty(false);
    });
    saveChain.current = operation.then(() => undefined, () => undefined);
    try {
      await operation;
      return true;
    } catch (cause) {
      saveFailed.current = true;
      setSaveError(errorMessage(cause));
      setSaveState('dirty');
      return false;
    } finally {
      pendingSaves.current -= 1;
      if (pendingSaves.current === 0) {
        setBusy(false);
        setSaveState(saveFailed.current ? 'dirty' : 'saved');
      }
    }
  }, [persistence]);

  const updateLocal = useCallback((next: InspirationDocument) => {
    documentRef.current = next;
    setDocument(next);
    setIsDirty(true);
    setSaveState('dirty');
    setSaveError(null);
  }, []);

  const currentBoard = document?.boards.find((board) => board.id === selectedBoard) ?? document?.boards[0] ?? null;

  const saveComposition = useCallback(() => {
    if (!editing || !document) return;
    const title = editing.title.trim() || 'Composizione senza titolo';
    const nextComposition = { ...editing, title };
    setEditing(nextComposition);
    const exists = document.compositions.some((item) => item.id === nextComposition.id);
    const compositions = exists ? document.compositions.map((item) => item.id === nextComposition.id ? nextComposition : item) : [...document.compositions, nextComposition];
    const boards = document.boards.map((board) => ({ ...board, items: board.items.map((item) => item.kind === 'composition' && item.compositionId === nextComposition.id ? { ...item, title } : item) }));
    void persist({ ...document, compositions, boards }).then((saved) => { if (saved) { setEditingBaseline(structuredClone(nextComposition)); } });
  }, [document, editing, persist]);

  function cancelEditing() {
    if (!editing) return;
    const restore = () => {
      const composition = structuredClone(document?.compositions.find((item) => item.id === editing.id) ?? newComposition());
      setEditing(composition);
      setEditingBaseline(structuredClone(composition));
      setDialog(null);
    };
    if (editingDirty) {
      setDialog({ title: 'Scartare modifiche?', action: restore });
      return;
    }
    restore();
  }

  function duplicateComposition(composition: Composition) {
    const copy = { ...structuredClone(composition), id: id(), title: `${composition.title} copia` };
    beginEditing(copy);
  }

  function deleteComposition(composition: Composition) {
    if (!document) return;
    const isReferenced = document.boards.some((board) => board.items.some((item) => item.kind === 'composition' && item.compositionId === composition.id));
    const remove = () => {
      const next = { ...document, compositions: document.compositions.filter((item) => item.id !== composition.id), boards: document.boards.map((board) => ({ ...board, items: board.items.filter((item) => item.kind !== 'composition' || item.compositionId !== composition.id) })) };
      void persist(next);
      setDialog(null);
    };
    if (isReferenced) setDialog({ title: 'Elimina composizione', action: remove });
    else setDialog({ title: 'Elimina composizione', action: remove });
  }

  function assignComposition(composition: Composition) {
    if (!document || !currentBoard) { setNotice('Crea o seleziona una board prima di assegnare la composizione.'); return; }
    if (currentBoard.items.some((item) => item.kind === 'composition' && item.compositionId === composition.id)) { setNotice('Questa composizione è già presente nella board.'); return; }
    const item: BoardItem = { id: id(), kind: 'composition', title: composition.title, compositionId: composition.id };
    void persist({ ...document, boards: document.boards.map((board) => board.id === currentBoard.id ? { ...board, items: [...board.items, item] } : board) });
    setSection('boards');
  }

  async function installMissing(composition: Composition) {
    const refs = [composition.heading.font, composition.subheading?.font ?? null, composition.body.font].filter((font): font is InspirationFontRef => !!font);
    const isInstalled = (font: InspirationFontRef) => {
      if (font.provider === 'local') return fontChoices.some((choice) => choice.local?.id === font.localFontId && choice.local.isInstalled);
      const source = font.provider === 'fontsource-extra' ? 'fontsource' : font.provider;
      return fontChoices.some((choice) => choice.local?.isInstalled && choice.local.source === source && choice.local.sourceFamilyId === font.familyId && choice.local.sourceVariantId === font.variantId);
    };
    const missing = refs.filter((font) => !isInstalled(font));
    const unique = missing.filter((font, index) => missing.findIndex((other) => choiceKey(other) === choiceKey(font)) === index);
    if (!unique.length) { setNotice('I font della composizione risultano già disponibili localmente.'); return; }
    setBusy(true);
    const installedIds: string[] = [];
    const reports: string[] = [];
    let failed = false;
    for (const font of unique) {
      try {
        if (font.provider === 'local') {
          if (!font.localFontId) throw new Error(`Riferimento locale non disponibile: ${font.family}`);
          await api.setFontActive(font.localFontId, true);
          installedIds.push(font.localFontId);
          reports.push(`${font.family}: attivato.`);
        } else if (font.provider === 'nerd-fonts') { failed = true; reports.push(`${font.family}: installazione non disponibile per questo provider.`); } else {
          if (!font.familyId) throw new Error(`Famiglia remota senza identificatore: ${font.family}`);
          const result: CloudFontImportOutcome = await api.downloadRemoteFamily(font.provider, font.familyId, font.variantId, true);
          installedIds.push(...result.fontIds);
          const errors = result.report.failures;
          failed ||= errors.length > 0 || result.fontIds.length === 0;
          reports.push(`${font.family}: ${result.report.indexedFonts} font indicizzati${errors.length ? `, errori: ${errors.map((item) => item.reason).join('; ')}` : ''}.`);
        }
      } catch (cause) { failed = true; reports.push(`${font.family}: ${errorMessage(cause)}`); }
    }
    const summary = reports.join(' ');
    try {
      const localItems: FontItem[] = [];
      for (let page = 0; ; page += 1) {
        const result = await api.listFonts({ tab: 'local', search: '', category: 'all', collectionId: null, page, pageSize: 100 });
        localItems.push(...result.items);
        if (localItems.length >= result.total || result.items.length === 0) break;
      }
      const refreshed = localItems.filter((font) => !font.missingOnDisk).map((font) => ({ ref: { provider: 'local' as const, familyId: font.sourceFamilyId, variantId: font.sourceVariantId ?? font.subfamily, localFontId: font.id, family: font.family, isVariable: font.isVariable }, label: `${font.family} · ${font.subfamily || font.fullName}`, local: font }));
      setChoices([...refreshed, ...choicesRef.current.filter((choice) => choice.remote)]);
    } catch (cause) { setFontError(`Impossibile aggiornare i font locali: ${errorMessage(cause)}`); }
    setBusy(false);
    setNotice(summary);
    onInstalled?.(installedIds, summary, failed);
  }

  function changeBoard(boardId: string, patch: Partial<Board>) {
    if (!document) return;
    updateLocal({ ...document, boards: document.boards.map((board) => board.id === boardId ? { ...board, ...patch } : board) });
  }

  function commitBoardChanges() {
    const latest = documentRef.current;
    if (latest) { setSaveError(null); setIsDirty(false); void persist(latest); }
  }

  function dropItem(board: Board, draggedId: string, targetId: string) {
    const source = board.items.findIndex((item) => item.id === draggedId);
    const target = board.items.findIndex((item) => item.id === targetId);
    if (source < 0 || target < 0 || source === target) return;
    const items = [...board.items];
    const [item] = items.splice(source, 1);
    items.splice(target, 0, item);
    changeBoard(board.id, { items });
    const latest = documentRef.current;
    if (latest) void persist(latest);
  }

  function moveItem(board: Board, itemId: string, direction: -1 | 1) {
    const index = board.items.findIndex((item) => item.id === itemId);
    const target = index + direction;
    if (index < 0 || target < 0 || target >= board.items.length) return;
    const items = [...board.items];
    [items[index], items[target]] = [items[target], items[index]];
    changeBoard(board.id, { items });
    const next = documentRef.current;
    if (next) void persist(next);
  }

  async function addImage() {
    if (!currentBoard || !document) return;
    try {
      const path = await open({ multiple: false, title: 'Scegli un’immagine per la board', filters: [{ name: 'Immagini', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] });
      if (typeof path !== 'string') return;
      setBusy(true);
      const asset = await persistence.importImage(path);
      const item: BoardItem = { id: id(), kind: 'image', title: 'Immagine importata', text: '', asset };
      await persist({ ...documentRef.current!, boards: documentRef.current!.boards.map((board) => board.id === currentBoard.id ? { ...board, items: [...board.items, item] } : board) });
    } catch (cause) { setSaveError(errorMessage(cause)); }
    finally { setBusy(false); }
  }

  function addDraftItem() {
    if (!itemDraft || !currentBoard || !document) return;
    let item: BoardItem;
    if (itemDraft.kind === 'font') {
      const font = fontChoices.find((choice) => choiceKey(choice.ref) === itemDraft.text)?.ref;
      if (!font) { setNotice('Seleziona un font disponibile dalla lista.'); return; }
      item = { id: id(), kind: 'font', title: itemDraft.title.trim() || font.family, font };
    } else if (itemDraft.kind === 'link') {
      let parsed: URL;
      try { parsed = new URL(itemDraft.url); } catch { setNotice('Inserisci un indirizzo HTTP o HTTPS valido.'); return; }
      if (!['http:', 'https:'].includes(parsed.protocol)) { setNotice('Sono consentiti solo link HTTP e HTTPS.'); return; }
      item = { id: id(), kind: 'link', title: itemDraft.title.trim() || parsed.hostname, text: itemDraft.text, url: parsed.toString() };
    } else item = { id: id(), kind: 'note', title: itemDraft.title.trim() || 'Nota', text: itemDraft.text };
    void persist({ ...document, boards: document.boards.map((board) => board.id === currentBoard.id ? { ...board, items: [...board.items, item] } : board) });
    setItemDraft(null);
  }

  function updateItem(board: Board, item: BoardItem, patch: Partial<BoardItem>) {
    const items = board.items.map((entry) => entry.id === item.id ? { ...entry, ...patch } as BoardItem : entry);
    changeBoard(board.id, { items });
  }

  function deleteItem(board: Board, item: BoardItem) {
    if (!window.confirm(`Rimuovere “${item.title}” dalla board? L’archivio originale non verrà eliminato.`)) return;
    changeBoard(board.id, { items: board.items.filter((entry) => entry.id !== item.id) });
    const latest = documentRef.current;
    if (latest) void persist(latest);
  }

  if (loading) return <section className="inspiration-view" aria-label="Ispirazione"><div className="alert" role="status">Lettura delle board e delle composizioni…</div></section>;
  if (loadError || !document) return <section className="inspiration-view" aria-label="Ispirazione"><div className="alert alert-error" role="alert">Impossibile leggere Ispirazione: {loadError ?? 'documento non disponibile'} <button className="btn btn-sm" onClick={() => { setLoading(true); setLoadError(null); void persistence.load().then((loaded) => { documentRef.current = loaded; setDocument(loaded); }).catch((cause) => setLoadError(errorMessage(cause))).finally(() => setLoading(false)); }} type="button">Riprova</button></div></section>;

  return <section className="inspiration-view" aria-label="Ispirazione">
    <header className="inspiration-header">
      <div><h1>Ispirazione</h1><p>Scegli un percorso: confronta due font oppure raccogli riferimenti per un progetto.</p></div>
      <div className="inspiration-save-state" role="status">{saveState === 'saving' ? 'Salvataggio…' : dirty ? 'Modifiche non salvate' : 'Salvato'}</div>
    </header>
    {saveError && <div className="alert alert-error" role="alert">Salvataggio non riuscito: {saveError}. Le modifiche restano in questa schermata. <button className="btn btn-sm" onClick={commitBoardChanges} type="button">Riprova</button>{saveError.toLocaleLowerCase().includes('revision') && <button className="btn btn-sm btn-outline" onClick={() => { if (window.confirm('Ricaricare la versione salvata? Le modifiche locali non salvate andranno perse.')) { void persistence.load().then((fresh) => { documentRef.current = fresh; setDocument(fresh); setEditing(null); setIsDirty(false); setSaveError(null); }); } }} type="button">Ricarica versione salvata…</button>}</div>}
    {fontError && <div className="alert alert-warning" role="alert">Impossibile caricare l’elenco font: {fontError}</div>}
    {notice && <div className="alert alert-info" role="status">{notice}</div>}
    <nav className="tabs tabs-border inspiration-tabs" aria-label="Sezioni Ispirazione">
      <button className={`tab inspiration-tab ${section === 'matches' ? 'tab-active' : ''}`} aria-current={section === 'matches' ? 'page' : undefined} onClick={() => setSection('matches')} type="button"><strong>Abbinamenti</strong><span>Confronta un font per i titoli e uno per i testi</span></button>
      <button className={`tab inspiration-tab ${section === 'boards' ? 'tab-active' : ''}`} aria-current={section === 'boards' ? 'page' : undefined} onClick={() => setSection('boards')} type="button"><strong>Moodboard</strong><span>Raccogli immagini, link e appunti per un progetto</span></button>
    </nav>

    {section === 'matches' ? <div className="inspiration-matches">
      <CompositionEditor composition={editing ?? newComposition()} isExisting={Boolean(editing && document.compositions.some((entry) => entry.id === editing.id))} isDirty={editingDirty} setComposition={(value) => { if (!editingBaseline) setEditingBaseline(structuredClone(editing ?? newComposition())); setEditing(value); }} choices={fontChoices} onCancel={cancelEditing} onSave={saveComposition} />
      <div className="inspiration-match-list">
        <section className="inspiration-live-preview" aria-label="Anteprima abbinamento corrente"><h2>Anteprima</h2><CompositionPreview composition={editing ?? newComposition()} choices={fontChoices} api={api} /></section>
        <div className="inspiration-section-title"><div><h2>I tuoi abbinamenti</h2><p>Un font per il titolo, uno per il testo.</p></div><button className="btn" onClick={() => beginEditing(newComposition())} type="button">Nuovo abbinamento</button></div>
        {document.compositions.length === 0 ? <div className="inspiration-empty"><p>Gli abbinamenti salvati appariranno qui. Scegli i font nella sidebar e premi Salva abbinamento.</p></div> : <ul className="inspiration-compositions">{document.compositions.map((composition) => <li key={composition.id}>
          <h3 className="inspiration-item-title">{composition.title}</h3>
          <p>Titolo: {composition.heading.font?.family ?? 'font non scelto'} · Testo: {composition.body.font?.family ?? 'font non scelto'}</p>
          <CompositionPreview composition={composition} choices={fontChoices} api={api} persistence={persistence} />
          <div className="inspiration-composition-actions"><button className="btn btn-primary" aria-label={`Modifica ${composition.title}`} onClick={() => beginEditing(structuredClone(composition), composition)} type="button">Prova / Modifica</button><button className="btn btn-sm btn-outline" onClick={() => assignComposition(composition)} type="button" disabled={!currentBoard}>Assegna a moodboard</button><details className="dropdown inspiration-secondary"><summary className="btn btn-sm btn-ghost">Altre azioni</summary><ul className="menu dropdown-content"><li><button onClick={() => duplicateComposition(composition)} type="button">Duplica</button></li><li><button disabled={busy} onClick={() => void installMissing(composition)} type="button">Installa i font di questo abbinamento</button></li><li><button onClick={() => deleteComposition(composition)} type="button">Elimina</button></li></ul></details></div>
        </li>)}</ul>}
      </div>
    </div> : <div className="inspiration-boards">
      <aside className="inspiration-board-sidebar"><div className="inspiration-section-title"><h2>Moodboard</h2>{boardTitle === null && <button className="btn btn-sm btn-primary" onClick={() => { if (discardContentDraft()) setBoardTitle(''); }} type="button">Nuova moodboard</button>}</div>
        {boardTitle !== null && <form className="inspiration-item-editor" onSubmit={(event) => {
          event.preventDefault();
          if (!boardTitle.trim()) return;
          const board = newBoard(boardTitle.trim());
          void persist({ ...documentRef.current!, boards: [...documentRef.current!.boards, board] });
          setSelectedBoard(board.id);
          setItemDraft(null);
          setAddingComposition(false);
          setBoardTitle(null);
        }}>
          <label className="form-control"><span className="label-text">Nome moodboard</span><input className="input input-bordered" autoFocus required value={boardTitle} onChange={(event) => setBoardTitle(event.target.value)} placeholder="Es. Identità del ristorante" /></label>
          <button className="btn btn-primary" type="submit" disabled={!boardTitle.trim()}>Crea moodboard</button>
          <button className="btn btn-ghost" type="button" onClick={() => setBoardTitle(null)}>Annulla</button>
        </form>}
        {document.boards.length ? <ul className="menu">{document.boards.map((board) => <li key={board.id}><button className={currentBoard?.id === board.id ? 'active' : ''} onClick={() => { if (board.id === selectedBoard || discardContentDraft()) setSelectedBoard(board.id); }} type="button">{board.title}<span>{board.items.length}</span></button></li>)}</ul> : <p>Crea una board per raccogliere note, link, immagini e font.</p>}
      </aside>
      <div className="inspiration-board-content">{currentBoard ? <>
        <div className="inspiration-board-heading"><div><h2>{currentBoard.title}</h2>{currentBoard.description && <p>{currentBoard.description}</p>}</div><details className="dropdown inspiration-secondary"><summary className="btn btn-sm btn-ghost">Impostazioni board</summary><div className="dropdown-content inspiration-board-settings"><label className="form-control"><span className="label-text">Nome board</span><input className="input input-bordered" value={currentBoard.title} onChange={(event) => changeBoard(currentBoard.id, { title: event.target.value })} onBlur={commitBoardChanges} /></label><label className="form-control"><span className="label-text">Descrizione</span><textarea className="textarea textarea-bordered" value={currentBoard.description} onChange={(event) => changeBoard(currentBoard.id, { description: event.target.value })} onBlur={commitBoardChanges} /></label><button className="btn btn-sm btn-outline" onClick={() => { const copy = { ...structuredClone(currentBoard), id: id(), title: `${currentBoard.title} copia`, items: currentBoard.items.map((item) => ({ ...structuredClone(item), id: id() })) }; void persist({ ...documentRef.current!, boards: [...documentRef.current!.boards, copy] }); setSelectedBoard(copy.id); }} type="button">Duplica board</button><button className="btn btn-sm btn-error btn-outline" onClick={() => { if (window.confirm(`Eliminare la board “${currentBoard.title}”? I font e le immagini originali resteranno invariati.`)) { const boards = documentRef.current!.boards.filter((item) => item.id !== currentBoard.id); void persist({ ...documentRef.current!, boards }); setSelectedBoard(boards[0]?.id ?? null); } }} type="button">Elimina board</button></div></details></div>
        <div className="inspiration-board-actions" onClick={(event) => { const button = (event.target as HTMLElement).closest('button'); button?.closest('details')?.removeAttribute('open'); }}><details className="dropdown"><summary className="btn btn-primary">Aggiungi contenuto</summary><ul className="menu dropdown-content"><li><button onClick={() => { if (discardContentDraft()) void addImage(); }} type="button">Immagine</button></li><li><button onClick={() => { if (discardContentDraft()) setItemDraft({ kind: 'note', title: '', text: '', url: '' }); }} type="button">Nota</button></li><li><button onClick={() => { if (discardContentDraft()) setItemDraft({ kind: 'link', title: '', text: '', url: '' }); }} type="button">Link</button></li><li><button onClick={() => { if (discardContentDraft()) setItemDraft({ kind: 'font', title: '', text: '', url: '' }); }} type="button">Font</button></li><li><button disabled={document.compositions.length === 0} onClick={() => { if (!discardContentDraft()) return; setSelectedComposition(''); setAddingComposition(true); }} type="button">Composizione salvata</button></li></ul></details></div>
        {addingComposition && <div className="inspiration-item-editor" role="group" aria-label="Aggiungi composizione"><h3>Aggiungi composizione salvata</h3><label className="form-control"><span className="label-text">Abbinamento da aggiungere</span><select aria-label="Abbinamento da aggiungere" className="select select-bordered" value={selectedComposition} onChange={(event) => setSelectedComposition(event.target.value)}><option value="">Scegli un abbinamento salvato</option>{document.compositions.map((entry) => <option key={entry.id} value={entry.id}>{entry.title}</option>)}</select></label><button className="btn btn-primary" disabled={!selectedComposition} onClick={() => { const composition = document.compositions.find((entry) => entry.id === selectedComposition); if (!composition || !currentBoard) return; if (currentBoard.items.some((entry) => entry.kind === 'composition' && entry.compositionId === composition.id)) { setNotice('Questa composizione è già presente nella board.'); return; } const item: BoardItem = { id: id(), kind: 'composition', title: composition.title, compositionId: composition.id }; void persist({ ...documentRef.current!, boards: documentRef.current!.boards.map((board) => board.id === currentBoard.id ? { ...board, items: [...board.items, item] } : board) }); setAddingComposition(false); }} type="button">Aggiungi alla board</button><button className="btn btn-ghost" onClick={() => setAddingComposition(false)} type="button">Annulla</button></div>}
        {itemDraft && <div className="inspiration-item-editor" role="group" aria-label={`Nuovo ${itemDraft.kind === 'note' ? 'appunto' : itemDraft.kind === 'link' ? 'link' : 'font'}`}><h3>{itemDraft.kind === 'note' ? 'Nuovo appunto' : itemDraft.kind === 'link' ? 'Nuovo link' : 'Nuovo font'}</h3><label className="form-control"><span className="label-text">Titolo</span><input className="input input-bordered" value={itemDraft.title} onChange={(event) => setItemDraft({ ...itemDraft, title: event.target.value })} /></label>{itemDraft.kind === 'font' ? <label className="form-control"><span className="label-text">Font</span><select aria-label="Font da aggiungere" className="select select-bordered" value={itemDraft.text} onChange={(event) => setItemDraft({ ...itemDraft, text: event.target.value })}><option value="">Scegli un font</option>{fontChoices.map((choice) => <option key={choiceKey(choice.ref)} value={choiceKey(choice.ref)}>{choice.label}</option>)}</select></label> : <>{itemDraft.kind === 'link' && <label className="form-control"><span className="label-text">URL HTTP o HTTPS</span><input className="input input-bordered" type="url" value={itemDraft.url} onChange={(event) => setItemDraft({ ...itemDraft, url: event.target.value })} /></label>}<label className="form-control"><span className="label-text">{itemDraft.kind === 'note' ? 'Testo' : 'Nota'}</span><textarea className="textarea textarea-bordered" value={itemDraft.text} onChange={(event) => setItemDraft({ ...itemDraft, text: event.target.value })} /></label></>}<button className="btn btn-primary" onClick={addDraftItem} type="button">Aggiungi alla board</button><button className="btn btn-ghost" onClick={() => setItemDraft(null)} type="button">Annulla</button></div>}
        {currentBoard.items.length === 0 ? <div className="inspiration-empty"><h3>Board vuota</h3><p>Aggiungi un riferimento o inserisci qui una composizione salvata.</p></div> : <ul className="inspiration-item-grid">{currentBoard.items.map((item, index) => <li data-board-item="" key={item.id} draggable onDragStart={(event) => { dragItem.current = `${currentBoard.id}:${item.id}`; event.dataTransfer.setData('application/x-inspiration-board-item', dragItem.current); event.dataTransfer.effectAllowed = 'move'; }} onDragOver={(event) => { if (event.dataTransfer.types.includes('application/x-inspiration-board-item')) event.preventDefault(); }} onDrop={(event) => { event.preventDefault(); const [boardId, draggedId] = event.dataTransfer.getData('application/x-inspiration-board-item').split(':'); if (boardId === currentBoard.id && draggedId) dropItem(currentBoard, draggedId, item.id); }}>
          <article className="card card-border inspiration-item-card"><div className="card-body"><div className="inspiration-item-meta"><span>{item.kind === 'composition' ? 'Composizione' : item.kind === 'font' ? 'Font' : item.kind === 'image' ? 'Immagine' : item.kind === 'link' ? 'Link' : 'Nota'}</span><div><button className="btn btn-xs btn-ghost" aria-label="Sposta su" disabled={index === 0} onClick={() => moveItem(currentBoard, item.id, -1)} type="button">Su</button><button className="btn btn-xs btn-ghost" aria-label="Sposta giù" disabled={index === currentBoard.items.length - 1} onClick={() => moveItem(currentBoard, item.id, 1)} type="button">Giù</button></div></div>
            {editingItems.includes(item.id) && (item.kind === 'image' || item.kind === 'font' || item.kind === 'composition') && <label className="form-control"><span className="label-text">Titolo riferimento</span><input className="input input-sm input-bordered" value={item.title} onChange={(event) => updateItem(currentBoard, item, { title: event.target.value })} onBlur={commitBoardChanges} /></label>}
            {item.kind === 'image' ? <><ImageItem item={item} persistence={persistence} />{editingItems.includes(item.id) ? <label className="form-control"><span className="label-text">Nota immagine</span><textarea className="textarea textarea-sm textarea-bordered" value={item.text} onChange={(event) => updateItem(currentBoard, item, { text: event.target.value })} onBlur={commitBoardChanges} /></label> : item.text && <p>{item.text}</p>}<button className="btn btn-sm btn-outline" onClick={() => setEditingItems(editingItems.includes(item.id) ? editingItems.filter((value) => value !== item.id) : [...editingItems, item.id])} type="button">{editingItems.includes(item.id) ? 'Fine modifica' : 'Modifica'}</button></> : item.kind === 'composition' ? <CompositionItem item={item} composition={document.compositions.find((entry) => entry.id === item.compositionId)} choices={fontChoices} api={api} persistence={persistence} /> : item.kind === 'font' ? <><p>{item.font.family}, {item.font.provider} · {item.font.variantId ?? 'variante non indicata'}</p>{!fontChoices.some((choice) => sameRef(item.font, choice.ref)) && <p role="status">Font non disponibile, riferimento conservato.</p>}<button className="btn btn-sm btn-outline" onClick={() => setEditingItems(editingItems.includes(item.id) ? editingItems.filter((value) => value !== item.id) : [...editingItems, item.id])} type="button">{editingItems.includes(item.id) ? 'Fine modifica' : 'Modifica'}</button></> : <>{editingItems.includes(item.id) ? <><label className="form-control"><span className="label-text">Titolo</span><input className="input input-sm input-bordered" value={item.title} onChange={(event) => updateItem(currentBoard, item, { title: event.target.value })} onBlur={commitBoardChanges} /></label>{item.kind === 'link' && <label className="form-control"><span className="label-text">URL HTTP o HTTPS</span><input className="input input-sm input-bordered" aria-label="URL riferimento" value={item.url} onChange={(event) => updateItem(currentBoard, item, { url: event.target.value })} onBlur={commitBoardChanges} /></label>}<label className="form-control"><span className="label-text">{item.kind === 'note' ? 'Testo nota' : 'Nota link'}</span><textarea className="textarea textarea-sm textarea-bordered" aria-label={item.kind === 'note' ? 'Testo nota' : 'Nota link'} value={item.text} onChange={(event) => updateItem(currentBoard, item, { text: event.target.value })} onBlur={commitBoardChanges} /></label><button className="btn btn-sm btn-outline" onClick={() => setEditingItems(editingItems.filter((value) => value !== item.id))} type="button">Fine modifica</button></> : <><div><span className="inspiration-item-title">{item.title}</span>{item.text && <p>{item.text}</p>}</div>{item.kind === 'link' && <button className="link" onClick={() => void persistence.openLink(item.url).catch((cause) => setNotice(`Link non aperto: ${errorMessage(cause)}`))} type="button">Apri link</button>}<button className="btn btn-sm btn-outline" onClick={() => setEditingItems([...editingItems, item.id])} type="button">Modifica</button></>}</>}
            <button className="btn btn-xs btn-ghost self-end" onClick={() => deleteItem(currentBoard, item)} type="button">Rimuovi</button>
          </div></article>
        </li>)}</ul>}
      </> : <div className="inspiration-empty"><h3>Raccogli i riferimenti di un progetto</h3><p>Inizia con Nuova moodboard: scegli un nome, poi aggiungi immagini, note, link o font.</p></div>}</div>
    </div>}

    {dialog && <InspirationConfirm title={dialog.title} onConfirm={dialog.action} onCancel={() => setDialog(null)} />}
  </section>;
}

function CompositionEditor({ composition, isExisting, isDirty, setComposition, choices, onCancel, onSave }: { composition: Composition; isExisting: boolean; isDirty: boolean; setComposition(value: Composition): void; choices: FontChoice[]; onCancel(): void; onSave(): void }) {
    const changeRole = (name: 'heading' | 'subheading' | 'body', patch: Partial<CompositionRole>) => {
    if (name === 'subheading') setComposition({ ...composition, subheading: { ...(composition.subheading ?? { ...INITIAL_ROLE, text: '' }), ...patch } });
    else setComposition({ ...composition, [name]: { ...composition[name], ...patch } });
  };
    const role = (name: 'heading' | 'subheading' | 'body', value: CompositionRole | null, label: string, basic = false) => <fieldset className="inspiration-role" key={name}><legend>{label}</legend>{name === 'subheading' && <label className="label cursor-pointer"><span className="label-text">Includi sottotitolo</span><input type="checkbox" className="checkbox" checked={!!value} onChange={(event) => setComposition({ ...composition, subheading: event.target.checked ? { ...INITIAL_ROLE, text: '' } : null })} /></label>}{value && <>
    <label className="form-control"><span className="label-text">Font {label.toLocaleLowerCase()}</span><select aria-label={`Font ${label.toLocaleLowerCase()}`} className="select select-bordered" value={choices.find((choice) => sameRef(value.font, choice.ref)) ? choiceKey(value.font!) : ''} onChange={(event) => changeRole(name, { font: choices.find((choice) => choiceKey(choice.ref) === event.target.value)?.ref ?? null })}><option value="">Nessun font selezionato</option>{choices.map((choice) => <option key={choiceKey(choice.ref)} value={choiceKey(choice.ref)}>{choice.label}</option>)}</select>{value.font && !choices.some((choice) => sameRef(value.font, choice.ref)) && <small role="status">{value.font.family}: font non disponibile, non verrà sostituito senza scelta.</small>}</label>
    <label className="form-control"><span className="label-text">Testo del {label.toLocaleLowerCase()}</span><textarea className="textarea textarea-bordered" aria-label={`Testo del ${label.toLocaleLowerCase()}`} value={value.text} onChange={(event) => changeRole(name, { text: event.target.value })} /></label>
    {basic ? <details className="inspiration-role-advanced"><summary>Personalizza {label.toLocaleLowerCase()}</summary><div className="inspiration-role-controls"><label className="form-control"><span className="label-text">Dimensione</span><input className="input input-bordered" type="number" min="8" max="160" value={value.size} onChange={(event) => changeRole(name, { size: Number(event.target.value) })} /></label><label className="form-control"><span className="label-text">Interlinea</span><input className="input input-bordered" type="number" min="0.6" max="3" step="0.05" value={value.lineHeight} onChange={(event) => changeRole(name, { lineHeight: Number(event.target.value) })} /></label><label className="form-control"><span className="label-text">Spaziatura</span><input className="input input-bordered" type="number" min="-5" max="20" step="0.1" value={value.letterSpacing} onChange={(event) => changeRole(name, { letterSpacing: Number(event.target.value) })} /></label><label className="form-control"><span className="label-text">Colore</span><input className="input input-bordered inspiration-color-input" type="color" value={value.color} onChange={(event) => changeRole(name, { color: event.target.value })} /></label></div></details> : null}
  </>}</fieldset>;
  return <aside className="inspiration-editor" aria-label="Editor abbinamento"><div className="inspiration-editor-heading"><div><h2>{isExisting ? 'Modifica abbinamento' : 'Nuovo abbinamento'}</h2><p>Scegli titolo, font e testo: l’anteprima si aggiorna subito.</p></div></div>
    <label className="form-control"><span className="label-text">Titolo composizione</span><input className="input input-bordered" value={composition.title} onChange={(event) => setComposition({ ...composition, title: event.target.value })} /></label>
    {role('heading', composition.heading, 'Titolo', true)}{role('body', composition.body, 'Testo', true)}<details className="inspiration-advanced"><summary>Personalizza stile e sottotitolo</summary>{composition.subheading ? role('subheading', composition.subheading, 'Sottotitolo', true) : <label className="label cursor-pointer"><span className="label-text">Aggiungi sottotitolo</span><input type="checkbox" className="checkbox" checked={false} onChange={() => setComposition({ ...composition, subheading: { ...INITIAL_ROLE, text: '' } })} /></label>}<div className="inspiration-role-controls"><label className="form-control"><span className="label-text">Sfondo</span><input className="input input-bordered inspiration-color-input" type="color" value={composition.background} onChange={(event) => setComposition({ ...composition, background: event.target.value })} /></label><label className="label cursor-pointer"><span className="label-text">Larghezza ridotta</span><input className="checkbox" type="checkbox" checked={composition.narrow} onChange={(event) => setComposition({ ...composition, narrow: event.target.checked })} /></label></div></details>

    <div className="inspiration-editor-actions">{isDirty && <button className="btn btn-ghost" onClick={onCancel} type="button">Annulla modifiche</button>}<button className="btn btn-primary" onClick={onSave} type="button">{isExisting ? 'Aggiorna abbinamento' : 'Salva abbinamento'}</button></div>
  </aside>;
}

function CompositionPreview({ composition, choices, api }: { composition: Composition; choices: FontChoice[]; api: FontManagerApi; persistence?: InspirationApi }) {
  const instanceId = useId().replace(/:/g, '');
  const compositionRef = useRef(composition); compositionRef.current = composition;
  const roles = [composition.heading, composition.subheading, composition.body].filter((role): role is CompositionRole => !!role);
  const rolesRef = useRef(roles); rolesRef.current = roles;
  const fontSignature = JSON.stringify(roles.map((role) => role.font ? [choiceKey(role.font), choices.some((choice) => sameRef(role.font, choice.ref))] : null));
  const [families, setFamilies] = useState<Record<string, string>>({});
  const [errors, setErrors] = useState<Record<string, string>>({});
  const generation = useRef(0);
  useEffect(() => {
    const currentGeneration = ++generation.current;
    let current = true;
    const loadedFaces: FontFace[] = [];
    const urls: string[] = [];
    const needed = rolesRef.current.map((role, index) => ({ font: role.font, index })).filter((entry): entry is { font: InspirationFontRef; index: number } => !!entry.font);
    setFamilies({}); setErrors({});
    void Promise.all(needed.map(async ({ font, index }) => {
      const key = `${choiceKey(font)}:${index}`;
      const choice = choices.find((candidate) => sameRef(font, candidate.ref));
      if (!choice) { if (current) setErrors((old) => ({ ...old, [key]: `${font.family}: font non disponibile nel catalogo locale/cache.` })); return; }
      try {
        if (font.provider === 'nerd-fonts') throw new Error('Il provider non espone una preview scaricabile.');
        const bytes = font.provider === 'local' ? await api.getFontPreviewBytes(font.localFontId!) : await api.getRemotePreviewBytes(font.provider, font.familyId!, font.variantId!);
        if (!current) return;
        const url = URL.createObjectURL(new Blob([new Uint8Array(bytes).buffer], { type: 'font/ttf' })); urls.push(url);
        const face = new FontFace(`inspiration-preview-${instanceId}-${currentGeneration}-${index}`, `url("${url}")`);
        try {
          await face.load();
          if (!current) { const index = urls.indexOf(url); if (index >= 0) urls.splice(index, 1); URL.revokeObjectURL(url); return; }
          document.fonts.add(face); loadedFaces.push(face);
          setFamilies((old) => ({ ...old, [key]: face.family }));
        } catch (cause) {
          URL.revokeObjectURL(url);
          if (current) setErrors((old) => ({ ...old, [key]: `${font.family}: anteprima non disponibile (${errorMessage(cause)}).` }));
        }
      } catch (cause) { if (current) setErrors((old) => ({ ...old, [key]: `${font.family}: anteprima non disponibile (${errorMessage(cause)}).` })); }
    }));
    return () => { current = false; loadedFaces.forEach((face) => document.fonts.delete(face)); urls.forEach((url) => URL.revokeObjectURL(url)); };
  }, [api, fontSignature]);
  const styleRole = (role: CompositionRole | null): React.CSSProperties => {
    if (!role) return {};
    const key = role.font ? `${choiceKey(role.font)}:${rolesRef.current.indexOf(role)}` : '';
    return { fontFamily: families[key] ? `"${families[key]}"` : undefined, fontSize: `clamp(12px, ${role.size}px, 10vw)`, lineHeight: role.lineHeight, letterSpacing: `${role.letterSpacing}px`, color: role.color, overflowWrap: 'anywhere' };
  };
  const keys = Object.keys(errors);
  return <article className={`inspiration-preview ${composition.narrow ? 'inspiration-preview-narrow' : ''}`} style={{ backgroundColor: composition.background }} aria-label={`Anteprima ${composition.title}`}>
    <div>{composition.heading.font && !choices.some((choice) => sameRef(composition.heading.font, choice.ref)) ? <p role="status">Font non disponibile: {composition.heading.font.family}. Scegli una sostituzione esplicita per provarlo.</p> : null}<p style={styleRole(composition.heading)}>{composition.heading.text || ' '}</p>{composition.subheading && <p style={styleRole(composition.subheading)}>{composition.subheading.text || ' '}</p>}<p style={styleRole(composition.body)}>{composition.body.text || ' '}</p>{keys.map((key) => <small role="status" key={key}>{errors[key]}</small>)}</div>
  </article>;
}

function ImageItem({ item, persistence }: { item: Extract<BoardItem, { kind: 'image' }>; persistence: InspirationApi }) {
  const [url, setUrl] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let current = true; let objectUrl: string | null = null;
    void persistence.getAsset(item.asset.id).then((bytes) => {
      if (!current) return;
      objectUrl = URL.createObjectURL(new Blob([new Uint8Array(bytes).buffer], { type: item.asset.mime }));
      setUrl(objectUrl);
    }).catch((cause) => { if (current) setError(errorMessage(cause)); });
    return () => { current = false; if (objectUrl) URL.revokeObjectURL(objectUrl); };
  }, [item.asset.id, item.asset.mime, persistence]);
  return <><h3 className="inspiration-item-title">{item.title}</h3>{url && <img src={url} alt={item.title} className="inspiration-image" />}{error && <p role="alert">Immagine non disponibile: {error}</p>}<p>{item.text}</p></>;
}

function CompositionItem({ item, composition, choices, api, persistence }: { item: Extract<BoardItem, { kind: 'composition' }>; composition?: Composition; choices: FontChoice[]; api: FontManagerApi; persistence: InspirationApi }) {
  return composition ? <div><h3 className="inspiration-item-title">{item.title}</h3><CompositionPreview composition={composition} choices={choices} api={api} persistence={persistence} /></div> : <p role="alert">Composizione non disponibile: {item.title}. Il riferimento è conservato.</p>;
}
