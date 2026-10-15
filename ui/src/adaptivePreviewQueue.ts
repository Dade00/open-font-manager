export interface PreviewQueueOptions {
  maxConcurrency: number;
}

interface PreviewJob {
  key: string;
  run: () => Promise<void> | void;
  resolve: () => void;
  reject: (error: unknown) => void;
  promise: Promise<void>;
}

export class PreviewQueue {
  private readonly maxConcurrency: number;
  private readonly waiting: PreviewJob[] = [];
  private readonly queued = new Map<string, PreviewJob>();
  private readonly active = new Map<string, PreviewJob>();
  private disposed = false;

  constructor(options: PreviewQueueOptions) {
    this.maxConcurrency = Math.max(1, Math.floor(options.maxConcurrency));
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
    while (!this.disposed && this.active.size < this.maxConcurrency && this.waiting.length) {
      const job = this.waiting.shift();
      if (!job) return;
      this.queued.delete(job.key);
      this.active.set(job.key, job);
      let result: Promise<void>;
      try {
        result = Promise.resolve(job.run());
      } catch (error) {
        result = Promise.reject(error);
      }
      void result.then(
        () => job.resolve(),
        (error: unknown) => job.reject(error),
      ).finally(() => {
        this.active.delete(job.key);
        this.drain();
      });
    }
  }
}
