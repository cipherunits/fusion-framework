// Tokio background tasks — usage example.
//
// Process-wide fire-and-forget / delayed jobs (not a durable queue).
//
// Build fusion-ffi, then from repo root (or set FUSION_FFI_PATH):
//   cargo build -p fusion-ffi --release
//   # run via a small console project that references FusionFramework
//
// API:
//   BackgroundTasks.Spawn(Action)
//   BackgroundTasks.SpawnAfter(ms, Action)
//   BackgroundTasks.Cancel(id)
//   BackgroundTasks.Status(id)  // pending|running|done|cancelled|failed

using FusionFramework;

BackgroundTasks.Reset();
var count = 0;
void Work()
{
    Interlocked.Increment(ref count);
    Console.WriteLine("  work() ran");
}

// 1) Fire-and-forget
var tid = BackgroundTasks.Spawn(Work);
Console.WriteLine($"spawn     id={tid}  status={BackgroundTasks.Status(tid)}");
for (var i = 0; i < 50; i++)
{
    if (count >= 1 && BackgroundTasks.Status(tid) == "done")
        break;
    Thread.Sleep(20);
}
Console.WriteLine($"          status={BackgroundTasks.Status(tid)}  count={count}");

// 2) Delayed start, then let it finish
var delayed = BackgroundTasks.SpawnAfter(150, Work);
Console.WriteLine($"after     id={delayed}  status={BackgroundTasks.Status(delayed)}");
for (var i = 0; i < 50; i++)
{
    if (count >= 2 && BackgroundTasks.Status(delayed) == "done")
        break;
    Thread.Sleep(20);
}
Console.WriteLine($"          status={BackgroundTasks.Status(delayed)}  count={count}");

// 3) Cancel before the delay elapses
var doomed = BackgroundTasks.SpawnAfter(500, Work);
Console.WriteLine($"cancel    id={doomed}  cancelled={BackgroundTasks.Cancel(doomed)}");
Thread.Sleep(200);
Console.WriteLine($"          status={BackgroundTasks.Status(doomed)}  count={count} (unchanged)");
