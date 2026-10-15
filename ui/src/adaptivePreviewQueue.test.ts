import { describe, expect, it } from 'vitest';

import { PreviewQueue } from './adaptivePreviewQueue';

describe('preview queue', () => {
  it('runs jobs FIFO with bounded concurrency and deduplicates keys', async () => {
    const queue = new PreviewQueue({ maxConcurrency: 2 });
    const releases: Array<() => void> = [];
    const started: string[] = [];
    const run = (key: string) => () => new Promise<void>((resolve) => {
      started.push(key);
      releases.push(resolve);
    });

    const first = queue.enqueue('first', run('first'));
    const duplicate = queue.enqueue('first', run('duplicate'));
    const second = queue.enqueue('second', run('second'));
    const third = queue.enqueue('third', run('third'));
    expect(duplicate).toBe(first);
    expect(started).toEqual(['first', 'second']);

    releases.splice(0, 2).forEach((release) => release());
    for (let attempt = 0; attempt < 10 && started.length < 3; attempt += 1) await Promise.resolve();
    expect(started).toEqual(['first', 'second', 'third']);
    releases.splice(0).forEach((release) => release());
    await Promise.all([first, second, third]);
    queue.dispose();
  });

  it('cancels queued previews that leave the viewport before loading', async () => {
    const queue = new PreviewQueue({ maxConcurrency: 1 });
    let releaseActive: (() => void) | undefined;
    let canceledTaskRan = false;

    const active = queue.enqueue('active', () => new Promise<void>((resolve) => { releaseActive = resolve; }));
    const queued = queue.enqueue('offscreen', async () => { canceledTaskRan = true; });
    expect(queue.cancel('offscreen')).toBe(true);
    await queued;
    releaseActive?.();
    await active;
    expect(canceledTaskRan).toBe(false);
  });
});
