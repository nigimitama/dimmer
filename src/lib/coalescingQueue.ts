type Task = () => Promise<unknown>;

/**
 * タスクを1つずつ順番に実行するキュー。同じキーで待っているタスクは最新のものだけを残す。
 * 輝度の変更が並行して走ると、モニターへの書き込み順が入れ替わって古い値が残ることがあるため、これで直列化する
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
        // エラーの通知は各タスクの中で行う
      }
    }
    running = false;
  }

  return {
    enqueue(key: string, task: Task) {
      // 末尾に付け直し、最後に操作したものが最後に適用されるようにする
      pending.delete(key);
      pending.set(key, task);
      void drain();
    },
  };
}
