# /// script
# dependencies = ["playwright"]
# ///
"""Verifica i cataloghi nel WebView2 reale, non nel layout simulato di jsdom.

Avvia Vite con npm run dev e l'app debug con:
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--remote-debugging-port=9223' cargo run --manifest-path src-tauri/Cargo.toml
Poi: uv run scripts/check_catalog_layout.py --output <cartella-screenshot>
Non modifica libreria, font installati o contenuti di Ispirazione.
"""

import argparse
import json
from pathlib import Path

from playwright.sync_api import sync_playwright


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cdp', default='http://127.0.0.1:9223')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    names = ['Google Fonts', 'Fontsource Extra', 'Fontshare', 'Nerd Fonts']
    widths = [1440, 640]
    results = []
    with sync_playwright() as p:
        browser = p.chromium.connect_over_cdp(args.cdp)
        page = next(page for context in browser.contexts for page in context.pages if ':1420' in page.url or 'tauri.localhost' in page.url or page.url.startswith('tauri://localhost'))
        initial_viewport = page.viewport_size
        try:
            for width in widths:
                page.set_viewport_size({'width': width, 'height': 920})
                page.locator('.primary-nav').get_by_role('button', name='Cataloghi vari', exact=False).click()
                for name in names:
                    page.locator('.catalog-picker').get_by_role('button', name=name, exact=True).click()
                    panel = page.locator('.workspace' if name == 'Google Fonts' else '.remote-catalog-view')
                    rows = panel.locator('.font-row')
                    rows.first.wait_for(state='visible', timeout=120000)
                    box = panel.bounding_box()
                    first = rows.first.bounding_box()
                    result = {'width': width, 'catalog': name, 'panel': box, 'rows': rows.count(),
                              'overflow': page.evaluate('document.documentElement.scrollWidth > innerWidth')}
                    assert box and box['height'] > 350, result
                    assert first and first['height'] > 40 and first['y'] < 920, result
                    assert not result['overflow'], result
                    results.append(result)
                    (args.output / 'results.json').write_text(json.dumps(results, ensure_ascii=False, indent=2), encoding='utf-8')
                    page.screenshot(path=str(args.output / f'{name.replace(" ", "-")}-{width}.png'), full_page=True)
            assert len(results) == len(widths) * len(names)
            print(json.dumps(results, ensure_ascii=False, indent=2))
        finally:
            if initial_viewport:
                page.set_viewport_size(initial_viewport)
            browser.close()


if __name__ == '__main__':
    main()
