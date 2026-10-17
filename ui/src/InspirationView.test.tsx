import { fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup } from '@testing-library/react';

import type { FontManagerApi } from './api';
import type { FontItem } from './types';
import type { Board, InspirationApi, InspirationDocument } from './inspirationTypes';
import { InspirationView } from './InspirationView';

const empty: InspirationDocument = { version: 1, revision: 0, boards: [], compositions: [] };
let stored: InspirationDocument;
const persistence: InspirationApi = {
  load: vi.fn(async () => structuredClone(stored)),
  save: vi.fn(async (document, expectedRevision) => {
    if (expectedRevision !== stored.revision) throw new Error('Revisione non aggiornata');
    stored = { ...structuredClone(document), revision: stored.revision + 1 };
    return structuredClone(stored);
  }),
  importImage: vi.fn(), getAsset: vi.fn(), openLink: vi.fn(),
};
const api = {
  listFonts: vi.fn(async () => ({ items: [], total: 0, page: 0, pageSize: 100 })),
  loadCachedRemoteCatalog: vi.fn(async () => null),
  getFontPreviewBytes: vi.fn(async () => new Uint8Array()),
  getRemotePreviewBytes: vi.fn(async () => new Uint8Array()),
  setFontActive: vi.fn(async () => undefined),
  downloadRemoteFamily: vi.fn(async () => ({ report: { failures: [], savedFiles: 0, indexedFonts: 0, activatedFonts: 0, provider: 'google', familyId: '', family: '' }, fontIds: [] })),
} as unknown as FontManagerApi;

function board(title: string, items: Board['items'] = []): Board {
  return { id: title.toLowerCase().replaceAll(' ', '-'), title, description: '', items };
}

afterEach(() => { cleanup(); vi.unstubAllGlobals(); });

beforeEach(() => {
  stored = structuredClone(empty);
  vi.clearAllMocks();
});

describe('InspirationView persistence and editing', () => {
  it('reopens persisted compositions and saves updates through revisioned persistence', async () => {
    stored = { ...empty, revision: 2, compositions: [{
      id: 'pair-1', title: 'Titolo', heading: { font: null, text: 'Editoriale', size: 56, lineHeight: 1, letterSpacing: 0, color: '#111111' }, subheading: null,
      body: { font: null, text: 'Testo', size: 18, lineHeight: 1.5, letterSpacing: 0, color: '#222222' }, background: '#ffffff', narrow: false,
    }] };
    const { unmount } = render(<InspirationView api={api} persistence={persistence} />);
    expect(await screen.findByText('Editoriale')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Modifica Titolo' }));
    fireEvent.change(screen.getByLabelText('Testo del titolo'), { target: { value: 'Nuovo titolo' } });
    fireEvent.click(screen.getByRole('button', { name: 'Aggiorna abbinamento' }));
    await waitFor(() => expect(persistence.save).toHaveBeenCalled());
    unmount();
    render(<InspirationView api={api} persistence={persistence} />);
    expect(await screen.findByText('Nuovo titolo')).toBeInTheDocument();
  });

  it('reports a revision conflict and keeps the edited content visible', async () => {
    vi.mocked(persistence.save).mockRejectedValueOnce(new Error('Revisione non aggiornata'));
    render(<InspirationView api={api} persistence={persistence} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Nuovo abbinamento' }));
    fireEvent.change(screen.getByLabelText('Testo del titolo'), { target: { value: 'Non perdere' } });
    fireEvent.click(screen.getByRole('button', { name: 'Salva abbinamento' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Revisione non aggiornata');
    expect(screen.getByLabelText('Testo del titolo')).toHaveValue('Non perdere');
  });

  it('shows an inline editor by default without reporting an untouched draft as dirty', async () => {
    const dirty = vi.fn();
    render(<InspirationView api={api} persistence={persistence} onDirtyChange={dirty} />);
    expect(await screen.findByRole('complementary', { name: 'Editor abbinamento' })).toBeVisible();
    await waitFor(() => expect(dirty).toHaveBeenLastCalledWith(false));
    expect(screen.queryByRole('dialog', { name: 'Editor abbinamento' })).not.toBeInTheDocument();
    fireEvent.change(screen.getByLabelText('Titolo composizione'), { target: { value: 'Prima modifica' } });
    await waitFor(() => expect(dirty).toHaveBeenLastCalledWith(true));
  });

  it('conserva la bozza quando annullo il cambio di abbinamento e cambia solo dopo conferma', async () => {
    const role = { font: null, text: 'Campione A', size: 40, lineHeight: 1, letterSpacing: 0, color: '#111111' };
    stored = { ...empty, compositions: ['A', 'B'].map((title) => ({ id: title, title, heading: { ...role, text: `Campione ${title}` }, subheading: null, body: { ...role, size: 18 }, background: '#ffffff', narrow: false })) };
    render(<InspirationView api={api} persistence={persistence} />);
    fireEvent.click(await screen.findByRole('button', { name: 'Modifica A' }));
    fireEvent.change(screen.getByLabelText('Testo del titolo'), { target: { value: 'Bozza da conservare' } });
    fireEvent.click(screen.getByRole('button', { name: 'Modifica B' }));
    let dialog = screen.getByRole('dialog', { name: 'Scartare modifiche?' });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Annulla' }));
    expect(screen.getByLabelText('Testo del titolo')).toHaveValue('Bozza da conservare');
    fireEvent.click(screen.getByRole('button', { name: 'Modifica B' }));
    dialog = screen.getByRole('dialog', { name: 'Scartare modifiche?' });
    fireEvent.click(within(dialog).getByRole('button', { name: 'Scarta modifiche' }));
    expect(screen.getByLabelText('Testo del titolo')).toHaveValue('Campione B');
    expect(persistence.save).not.toHaveBeenCalled();
  });

  it('applica il font proveniente dai cataloghi anche quando la sidebar è già aperta', async () => {
    const font = { provider: 'google' as const, familyId: 'seed-family', variantId: 'regular', localFontId: null, family: 'Seed Family', isVariable: false };
    const consumed = vi.fn();
    render(<InspirationView api={api} persistence={persistence} seedFont={font} onSeedConsumed={consumed} />);
    await waitFor(() => expect(screen.getByLabelText('Testo del titolo')).toHaveValue('Seed Family'));
    expect(consumed).toHaveBeenCalledTimes(1);
    fireEvent.click(screen.getByRole('button', { name: 'Salva abbinamento' }));
    await waitFor(() => expect(stored.compositions[0]?.heading.font).toEqual(font));
  });

  it('persists board item reordering with keyboard controls', async () => {
    stored = { ...empty, boards: [board('Riferimenti', [
      { id: 'n1', kind: 'note', title: 'Prima', text: 'A' },
      { id: 'n2', kind: 'note', title: 'Seconda', text: 'B' },
    ])] };
    render(<InspirationView api={api} persistence={persistence} />);
    fireEvent.click(await screen.findByRole('button', { name: /Moodboard/ }));
    const secondCard = screen.getByText('Seconda').closest('[data-board-item]') as HTMLElement;
    fireEvent.click(within(secondCard).getByRole('button', { name: 'Sposta su' }));
    await waitFor(() => expect(stored.boards[0].items.map((item) => item.id)).toEqual(['n2', 'n1']));
  });

  it('loads later local pages despite a failed provider and keeps body-only previews stable while typing', async () => {
    const font: FontItem = { id: 'late-font', family: 'Later Family', fullName: 'Later Family Regular', subfamily: 'Regular', weight: 400, isItalic: false, isVariable: false, source: 'local', sourceFamilyId: null, sourceVariantId: null, filePath: 'C:/fonts/later.ttf', category: 'unknown', isActive: false, isInstalled: false, isFavorite: false, fileSize: 1, fileModifiedUnixMs: 0, missingOnDisk: false };
    const firstPage = Array.from({ length: 100 }, (_, index) => ({ ...font, id: `early-${index}` }));
    vi.mocked(api.listFonts).mockResolvedValueOnce({ items: firstPage, total: 101, page: 0, pageSize: 100 }).mockResolvedValueOnce({ items: [font], total: 101, page: 1, pageSize: 100 });
    vi.mocked(api.loadCachedRemoteCatalog).mockRejectedValueOnce(new Error('Provider offline'));
    vi.stubGlobal('FontFace', class { constructor(public family: string) {} async load() { return this; } });
    vi.stubGlobal('URL', { createObjectURL: () => 'blob:test-font', revokeObjectURL: vi.fn() });
    Object.defineProperty(document, 'fonts', { configurable: true, value: new Set() });
    stored = { ...empty, compositions: [{ id: 'body-only', title: 'Corpo soltanto', heading: { font: null, text: 'Titolo', size: 40, lineHeight: 1, letterSpacing: 0, color: '#111111' }, subheading: null, body: { font: { provider: 'local', localFontId: font.id, familyId: null, variantId: 'Regular', family: font.family, isVariable: false }, text: 'Corpo di prova', size: 18, lineHeight: 1.5, letterSpacing: 0, color: '#222222' }, background: '#ffffff', narrow: false }] };
    const dirty = vi.fn();
    render(<InspirationView api={api} persistence={persistence} onDirtyChange={dirty} />);
    await waitFor(() => expect(api.getFontPreviewBytes).toHaveBeenCalledWith('late-font'));
    await waitFor(() => expect(screen.getByText('Corpo di prova').style.fontFamily).toContain('inspiration-preview'));
    expect(api.listFonts).toHaveBeenCalledWith(expect.objectContaining({ page: 1 }));
    const count = vi.mocked(api.getFontPreviewBytes).mock.calls.length;
    fireEvent.click(screen.getByRole('button', { name: 'Modifica Corpo soltanto' }));
    fireEvent.change(screen.getByLabelText('Testo del testo'), { target: { value: 'Testo cambiato' } });
    await waitFor(() => expect(dirty).toHaveBeenLastCalledWith(true));
    expect(api.getFontPreviewBytes).toHaveBeenCalledTimes(count + 1);
    fireEvent.click(screen.getByText('Personalizza testo'));
    expect(screen.getByRole('combobox', { name: 'Font testo' })).toHaveValue(JSON.stringify(['local', null, 'late-font', 'Regular']));
  });

  it('crea una moodboard con nome e aggiunge una nota tramite un solo modulo contestuale', async () => {
    const dirty = vi.fn();
    render(<InspirationView api={api} persistence={persistence} onDirtyChange={dirty} />);
    fireEvent.click(await screen.findByRole('button', { name: /Moodboard/ }));
    expect(screen.getAllByRole('button', { name: 'Nuova moodboard' })).toHaveLength(1);
    fireEvent.click(screen.getByRole('button', { name: 'Nuova moodboard' }));
    expect(screen.getByRole('button', { name: 'Crea moodboard' })).toBeDisabled();
    fireEvent.change(screen.getByLabelText('Nome moodboard'), { target: { value: 'Progetto ristorante' } });
    fireEvent.click(screen.getByRole('button', { name: 'Crea moodboard' }));
    await waitFor(() => expect(stored.boards[0]?.title).toBe('Progetto ristorante'));
    expect(screen.queryByRole('textbox', { name: 'Nome board' })).not.toBeVisible();
    fireEvent.click(screen.getByText('Aggiungi contenuto'));
    fireEvent.click(screen.getByRole('button', { name: 'Nota' }));
    const form = screen.getByRole('group', { name: 'Nuovo appunto' });
    expect(screen.getByText('Aggiungi contenuto').closest('details')).not.toHaveAttribute('open');
    expect(screen.queryByLabelText('URL HTTP o HTTPS')).not.toBeInTheDocument();
    fireEvent.change(within(form).getByLabelText('Titolo'), { target: { value: 'Materiali' } });
    fireEvent.change(within(form).getByLabelText('Testo'), { target: { value: 'Carta e colori caldi' } });
    await waitFor(() => expect(dirty).toHaveBeenLastCalledWith(true));
    fireEvent.click(within(form).getByRole('button', { name: 'Aggiungi alla board' }));
    await waitFor(() => expect(stored.boards[0].items).toEqual([expect.objectContaining({ kind: 'note', title: 'Materiali', text: 'Carta e colori caldi' })]));
    expect(screen.getByText('Materiali')).toBeVisible();
    expect(screen.queryByRole('group', { name: 'Nuovo appunto' })).not.toBeInTheDocument();
  });

  it('conserva la nota non aggiunta quando viene annullato il cambio moodboard', async () => {
    stored = { ...empty, boards: [board('Prima board'), board('Seconda board')] };
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    try {
      render(<InspirationView api={api} persistence={persistence} />);
      fireEvent.click(await screen.findByRole('button', { name: /Moodboard/ }));
      fireEvent.click(screen.getByText('Aggiungi contenuto'));
      fireEvent.click(screen.getByRole('button', { name: 'Nota' }));
      fireEvent.change(screen.getByLabelText('Testo'), { target: { value: 'Non perdere la bozza' } });
      fireEvent.click(screen.getByRole('button', { name: /^Seconda board/ }));
      expect(confirm).toHaveBeenCalled();
      expect(screen.getByLabelText('Testo')).toHaveValue('Non perdere la bozza');
      expect(screen.getByRole('heading', { name: 'Prima board' })).toBeVisible();
      expect(persistence.save).not.toHaveBeenCalled();
    } finally { confirm.mockRestore(); }
  });

  it('requires confirmation before deleting a composition referenced by a board', async () => {
    stored = { ...empty,
      compositions: [{ id: 'pair-1', title: 'Accoppiata', heading: { font: null, text: 'A', size: 40, lineHeight: 1, letterSpacing: 0, color: '#111111' }, subheading: null, body: { font: null, text: 'B', size: 18, lineHeight: 1.4, letterSpacing: 0, color: '#222222' }, background: '#ffffff', narrow: false }],
      boards: [board('Board', [{ id: 'ref', kind: 'composition', title: 'Accoppiata', compositionId: 'pair-1' }])],
    };
    render(<InspirationView api={api} persistence={persistence} />);
    fireEvent.click(await screen.findByRole('button', { name: /Moodboard/ }));
    fireEvent.click(await screen.findByRole('button', { name: /Abbinamenti/ }));
    fireEvent.click(screen.getByText('Altre azioni'));
    fireEvent.click(screen.getByRole('button', { name: 'Elimina' }));
    expect(await screen.findByRole('dialog', { name: 'Elimina composizione' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Annulla' }));
    expect(stored.compositions).toHaveLength(1);
    expect(stored.boards[0].items).toHaveLength(1);
  });
});
