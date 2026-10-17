import type { RemoteFontProvider } from './types';

export interface InspirationFontRef {
  provider: RemoteFontProvider | 'local';
  familyId: string | null;
  variantId: string | null;
  localFontId: string | null;
  family: string;
  isVariable: boolean | null;
}

export interface CompositionRole {
  font: InspirationFontRef | null;
  text: string;
  size: number;
  lineHeight: number;
  letterSpacing: number;
  color: string;
}

export interface Composition {
  id: string;
  title: string;
  heading: CompositionRole;
  subheading: CompositionRole | null;
  body: CompositionRole;
  background: string;
  narrow: boolean;
}

export interface InspirationAsset {
  id: string;
  mime: string;
  width: number;
  height: number;
}

export type BoardItem =
  | { id: string; kind: 'note'; title: string; text: string }
  | { id: string; kind: 'link'; title: string; text: string; url: string }
  | { id: string; kind: 'image'; title: string; text: string; asset: InspirationAsset }
  | { id: string; kind: 'font'; title: string; font: InspirationFontRef }
  | { id: string; kind: 'composition'; title: string; compositionId: string };

export interface Board {
  id: string;
  title: string;
  description: string;
  items: BoardItem[];
}

export interface InspirationDocument {
  version: 1;
  revision: number;
  boards: Board[];
  compositions: Composition[];
}

export interface InspirationApi {
  load(): Promise<InspirationDocument>;
  save(document: InspirationDocument, expectedRevision: number): Promise<InspirationDocument>;
  importImage(path: string): Promise<InspirationAsset>;
  getAsset(assetId: string): Promise<Uint8Array>;
  openLink(url: string): Promise<void>;
}
