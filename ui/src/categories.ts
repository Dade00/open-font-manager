export type FontCategory = 'all' | 'unknown' | 'sans-serif' | 'serif' | 'display' | 'handwriting' | 'monospace';


const CATEGORIES: FontCategory[] = ['all', 'unknown', 'sans-serif', 'serif', 'display', 'handwriting', 'monospace'];
const LABELS: Record<FontCategory, string> = {
  all: 'Tutte', unknown: 'Senza categoria', 'sans-serif': 'Sans serif', serif: 'Serif',
  display: 'Display', handwriting: 'Manoscritto', monospace: 'Monospazio',
};

export function categoryLabel(value: string): string {
  const category = normalizeCategory(value);
  return LABELS[category];
}

export function normalizeCategory(value: string): FontCategory {
  return CATEGORIES.includes(value as FontCategory) ? value as FontCategory : 'unknown';
}
