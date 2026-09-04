using FusionFramework;
using Xunit;

namespace FusionFramework.Tests;

public class TaskTests
{
    public TaskTests()
    {
        BackgroundTasks.Reset();
    }

    static void WaitDone(string tid, Func<bool> sideEffect, int timeoutMs = 2000)
    {
        var deadline = Environment.TickCount64 + timeoutMs;
        while (Environment.TickCount64 < deadline)
        {
            if (sideEffect() && BackgroundTasks.Status(tid) == "done")
                return;
            Thread.Sleep(20);
        }
    }

    [Fact]
    public void SpawnRunsToDone()
    {
        var n = 0;
        var tid = BackgroundTasks.Spawn(() => Interlocked.Increment(ref n));
        WaitDone(tid, () => n == 1);
        Assert.Equal(1, n);
        Assert.Equal("done", BackgroundTasks.Status(tid));
    }

    [Fact]
    public void SpawnAfterRunsToDone()
    {
        var n = 0;
        var tid = BackgroundTasks.SpawnAfter(80, () => Interlocked.Increment(ref n));
        Assert.Contains(BackgroundTasks.Status(tid), new[] { "pending", "running" });
        Thread.Sleep(30);
        Assert.Equal(0, n);
        WaitDone(tid, () => n == 1);
        Assert.Equal(1, n);
        Assert.Equal("done", BackgroundTasks.Status(tid));
    }

    [Fact]
    public void SpawnAfterCanBeCancelled()
    {
        var n = 0;
        var tid = BackgroundTasks.SpawnAfter(400, () => Interlocked.Increment(ref n));
        Assert.Contains(BackgroundTasks.Status(tid), new[] { "pending", "running" });
        Assert.True(BackgroundTasks.Cancel(tid));
        Thread.Sleep(150);
        Assert.Equal(0, n);
        Assert.Equal("cancelled", BackgroundTasks.Status(tid));
    }

    [Fact]
    public void StatusUnknownId()
    {
        Assert.Null(BackgroundTasks.Status("task-does-not-exist"));
        Assert.False(BackgroundTasks.Cancel("task-does-not-exist"));
    }
}
