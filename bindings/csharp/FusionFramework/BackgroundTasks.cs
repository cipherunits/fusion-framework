using System.Runtime.InteropServices;

namespace FusionFramework;

/// <summary>Process-wide Tokio background tasks (Python/Node parity).</summary>
public static class BackgroundTasks
{
    // Must match Native.FusionTask* delegates for P/Invoke marshalling.
    static readonly Native.FusionTaskCallback InvokeAction = static userData =>
    {
        var handle = GCHandle.FromIntPtr(userData);
        if (handle.Target is Action action)
            action();
    };

    static readonly Native.FusionTaskDataFree FreeHandle = static userData =>
    {
        var handle = GCHandle.FromIntPtr(userData);
        if (handle.IsAllocated)
            handle.Free();
    };

    /// <summary>Run <paramref name="action"/> on the Tokio background runtime.</summary>
    public static string Spawn(Action action)
    {
        ArgumentNullException.ThrowIfNull(action);
        var gch = GCHandle.Alloc(action);
        var ptr = Native.fusion_task_spawn(
            InvokeAction,
            GCHandle.ToIntPtr(gch),
            FreeHandle);
        var id = Native.TakeUtf8(ptr);
        if (string.IsNullOrEmpty(id))
            throw new InvalidOperationException("task spawn failed");
        return id;
    }

    /// <summary>Run <paramref name="action"/> after <paramref name="delayMs"/> milliseconds.</summary>
    public static string SpawnAfter(ulong delayMs, Action action)
    {
        ArgumentNullException.ThrowIfNull(action);
        var gch = GCHandle.Alloc(action);
        var ptr = Native.fusion_task_spawn_after(
            delayMs,
            InvokeAction,
            GCHandle.ToIntPtr(gch),
            FreeHandle);
        var id = Native.TakeUtf8(ptr);
        if (string.IsNullOrEmpty(id))
            throw new InvalidOperationException("task spawn_after failed");
        return id;
    }

    /// <summary>Cancel a pending/running task.</summary>
    public static bool Cancel(string taskId)
    {
        var code = Native.fusion_task_cancel(taskId);
        if (code < 0)
            throw new ArgumentException("invalid task id", nameof(taskId));
        return code == 1;
    }

    /// <summary>Status string, or <c>null</c> if unknown.</summary>
    public static string? Status(string taskId)
    {
        var ptr = Native.fusion_task_status(taskId);
        if (ptr == IntPtr.Zero)
            return null;
        return Native.TakeUtf8(ptr);
    }

    /// <summary>Abort and clear all tracked tasks (tests).</summary>
    public static void Reset() => Native.fusion_task_reset();
}
