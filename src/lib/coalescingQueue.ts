type Task = () => Promise<unknown>;

/**
 * A queue that runs tasks one at a time. Among tasks waiting on the same key, only the latest is kept.
 * Brightness changes running concurrently can reorder writes to the monitor and leave a stale value, so they are serialized here
 */
export function createCoalescingQueue() {
  const pending = new Map<string, Task>();
  let running = false;

  async function drain() {
    if (running) return;
    running = true;
    while (pending.size > 0) {
      const [key, task] = pending.entries().next().value as [string, Task];
      pending.delete(key);
      try {
        await task();
      } catch {
        // Each task reports its own errors
      }
    }
    running = false;
  }

  return {
    enqueue(key: string, task: Task) {
      // Re-append at the end so the most recently touched task is applied last
      pending.delete(key);
      pending.set(key, task);
      void drain();
    },
  };
}
