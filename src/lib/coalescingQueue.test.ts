import { describe, expect, it } from "vitest";
import { createCoalescingQueue } from "./coalescingQueue";

function deferred() {
  let resolve!: () => void;
  const promise = new Promise<void>((r) => (resolve = r));
  return { promise, resolve };
}

const flush = () => new Promise((r) => setTimeout(r, 0));

describe("createCoalescingQueue", () => {
  it("runs one task at a time and keeps only the latest pending task per key", async () => {
    const queue = createCoalescingQueue();
    const started: string[] = [];
    const gate = deferred();
    let running = 0;
    let maxRunning = 0;
    const task = (name: string, wait?: Promise<void>) => async () => {
      started.push(name);
      running++;
      maxRunning = Math.max(maxRunning, running);
      await wait;
      running--;
    };

    queue.enqueue("all", task("all=60", gate.promise));
    queue.enqueue("all", task("all=70"));
    queue.enqueue("all", task("all=80"));
    await flush();
    expect(started).toEqual(["all=60"]);

    gate.resolve();
    await flush();
    await flush();
    expect(started).toEqual(["all=60", "all=80"]);
    expect(maxRunning).toBe(1);
  });

  it("runs tasks for different keys in the order of their latest enqueue", async () => {
    const queue = createCoalescingQueue();
    const started: string[] = [];
    const gate = deferred();
    const task = (name: string, wait?: Promise<void>) => async () => {
      started.push(name);
      await wait;
    };

    queue.enqueue("a", task("a=10", gate.promise));
    queue.enqueue("all", task("all=60"));
    queue.enqueue("b", task("b=50"));
    queue.enqueue("all", task("all=70"));
    gate.resolve();
    await flush();
    await flush();
    await flush();
    expect(started).toEqual(["a=10", "b=50", "all=70"]);
  });

  it("keeps running after a task rejects", async () => {
    const queue = createCoalescingQueue();
    const started: string[] = [];
    queue.enqueue("a", async () => {
      started.push("fail");
      throw new Error("x");
    });
    queue.enqueue("b", async () => {
      started.push("ok");
    });
    await flush();
    await flush();
    expect(started).toEqual(["fail", "ok"]);
  });
});
