import { describe, expect, it } from 'vitest';

import { AdaptivePreviewQueue } from './adaptivePreviewQueue';

describe('adaptive preview queue', () => {
  it('adjusts parallel preview work using measured completion times', async () => {
    const clock = { now: 0 };
    const queue = new AdaptivePreviewQueue({
      maxConcurrency: 3,
      fastThresholdMs: 100,
      slowThresholdMs: 300,
      now: () => clock.now,
    });

    await Promise.all(Array.from({ length: 3 }, (_, index) => queue.enqueue(`fast-${index}`, async () => {
      clock.now += 40;
    })));
    expect(queue.concurrency).toBe(2);

    const releaseBurst: Array<() => void> = [];
    let burstStarted = 0;
    const burst = ['burst-1', 'burst-2', 'burst-3'].map((key) => queue.enqueue(key, () => new Promise<void>((resolve) => {
      burstStarted += 1;
      releaseBurst.push(resolve);
    })));
    expect(burstStarted).toBe(2);
    releaseBurst.splice(0).forEach((release) => release());
    for (let attempt = 0; attempt < 10 && burstStarted < 3; attempt += 1) await Promise.resolve();
    expect(burstStarted).toBe(3);
    releaseBurst.splice(0).forEach((release) => release());
    await Promise.all(burst);

    await queue.enqueue('slow-1', async () => { clock.now += 400; });
    await queue.enqueue('slow-2', async () => { clock.now += 400; });
    expect(queue.concurrency).toBe(1);

    const releaseSlow: Array<() => void> = [];
    let slowStarted = 0;
    const slowBurst = ['slow-a', 'slow-b', 'slow-c'].map((key) => queue.enqueue(key, () => new Promise<void>((resolve) => {
      slowStarted += 1;
      releaseSlow.push(resolve);
    })));
    expect(slowStarted).toBe(1);
    releaseSlow.splice(0).forEach((release) => release());
    for (let attempt = 0; attempt < 10 && slowStarted < 2; attempt += 1) await Promise.resolve();
    expect(slowStarted).toBe(2);
    releaseSlow.splice(0).forEach((release) => release());
    for (let attempt = 0; attempt < 10 && slowStarted < 3; attempt += 1) await Promise.resolve();
    releaseSlow.splice(0).forEach((release) => release());
    await Promise.all(slowBurst);
  });

  it('cancels queued previews that leave the viewport before loading', async () => {
    const queue = new AdaptivePreviewQueue({ maxConcurrency: 1 });
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
