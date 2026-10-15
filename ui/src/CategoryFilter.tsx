import { useId } from 'react';
import type { FontCategory } from './categories';
import { categoryLabel } from './categories';

const categories: FontCategory[] = ['all', 'unknown', 'sans-serif', 'serif', 'display', 'handwriting', 'monospace'];
const glyphs: Record<FontCategory, string> = { all: 'Aa', unknown: '?', 'sans-serif': 'Ag', serif: 'Ag', display: 'Aa', handwriting: 'ℋ', monospace: 'Mm' };

export function CategoryFilter({ value, onChange }: { value: FontCategory; onChange(value: FontCategory): void }) {
  const name = useId();
  return (
    <fieldset className="category-filter" aria-label="Filtra per categoria">
      {categories.map((category) => (
        <label className="category-option" key={category}>
          <input className="radio radio-sm" checked={value === category} name={name} onChange={() => onChange(category)} type="radio" value={category} />
          <span className="category-glyph" aria-hidden="true" style={{ fontFamily: category === 'monospace' ? 'monospace' : category === 'serif' ? 'serif' : category === 'sans-serif' ? 'sans-serif' : category === 'handwriting' ? 'cursive' : 'inherit' }}>{glyphs[category]}</span>
          <span>{categoryLabel(category)}</span>
        </label>
      ))}
    </fieldset>
  );
}
