export interface AdaptivePreviewQueueOptions {
  maxConcurrency: number;
  fastThresholdMs?: number;
  slowThresholdMs?: number;
  now?: () => number;
}

interface PreviewJob {
  key: string;
  run: () => Promise<void> | void;
  resolve: () => void;
  reject: (error: unknown) => void;
  promise: Promise<void>;
}

export class AdaptivePreviewQueue {
  private readonly maxConcurrency: number;
  private readonly fastThresholdMs: number;
  private readonly slowThresholdMs: number;
  private readonly now: () => number;
  private readonly waiting: PreviewJob[] = [];
  private readonly queued = new Map<string, PreviewJob>();
  private readonly active = new Map<string, PreviewJob>();
  private fastCompletions = 0;
  private disposed = false;
  private currentConcurrency = 1;

  constructor(options: AdaptivePreviewQueueOptions) {
    this.maxConcurrency = Math.max(1, Math.floor(options.maxConcurrency));
    this.fastThresholdMs = options.fastThresholdMs ?? 140;
    this.slowThresholdMs = options.slowThresholdMs ?? 500;
    this.now = options.now ?? Date.now;
  }

  get concurrency(): number {
    return this.currentConcurrency;
  }

  enqueue(key: string, run: () => Promise<void> | void): Promise<void> {
    const existing = this.queued.get(key) ?? this.active.get(key);
    if (existing) return existing.promise;
    if (this.disposed) return Promise.resolve();

    let resolve!: () => void;
    let reject!: (error: unknown) => void;
    const promise = new Promise<void>((done, fail) => {
      resolve = done;
      reject = fail;
    });
    const job = { key, run, resolve, reject, promise };
    this.waiting.push(job);
    this.queued.set(key, job);
    this.drain();
    return promise;
  }

  cancel(key: string): boolean {
    const job = this.queued.get(key);
    if (!job) return false;
    this.queued.delete(key);
    const index = this.waiting.indexOf(job);
    if (index !== -1) this.waiting.splice(index, 1);
    job.resolve();
    return true;
  }

  dispose(): void {
    this.disposed = true;
    for (const job of this.waiting.splice(0)) {
      this.queued.delete(job.key);
      job.resolve();
    }
  }

  private drain(): void {
    while (!this.disposed && this.active.size < this.currentConcurrency && this.waiting.length) {
      const job = this.waiting.shift();
      if (!job) return;
      this.queued.delete(job.key);
      this.active.set(job.key, job);
      const startedAt = this.now();
      let result: Promise<void>;
      try {
        result = Promise.resolve(job.run());
      } catch (error) {
        result = Promise.reject(error);
      }
      void result.then(
        () => {
          job.resolve();
          this.complete(job, startedAt);
        },
        (error: unknown) => {
          job.reject(error);
          this.complete(job, startedAt);
        },
      );
    }
  }

  private complete(job: PreviewJob, startedAt: number): void {
    this.active.delete(job.key);
    const elapsed = Math.max(0, this.now() - startedAt);
    if (elapsed >= this.slowThresholdMs) {
      this.currentConcurrency = Math.max(1, this.currentConcurrency - 1);
      this.fastCompletions = 0;
    } else if (elapsed <= this.fastThresholdMs) {
      this.fastCompletions += 1;
      if (this.fastCompletions >= 3 && this.currentConcurrency < this.maxConcurrency) {
        this.currentConcurrency += 1;
        this.fastCompletions = 0;
      }
    } else {
      this.fastCompletions = 0;
    }
    this.drain();
  }
}
