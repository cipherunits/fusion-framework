using System.Diagnostics;

namespace FusionFramework;

/// <summary>
/// Process-based auto-reload for development.
/// Parent watches files and restarts a child that runs the real server.
/// </summary>
public static class Reloader
{
    public const string ChildEnvVar = "FUSION_RELOAD_CHILD";

    static readonly HashSet<string> SkipDirs = new(StringComparer.OrdinalIgnoreCase)
    {
        ".git", ".hg", "node_modules", "target", ".venv", "venv",
        "__pycache__", "bin", "obj", "dist", "build", ".idea", ".vs",
    };

    static readonly HashSet<string> Extensions = new(StringComparer.OrdinalIgnoreCase)
    {
        ".cs", ".json", ".html", ".tera", ".js", ".mjs", ".py",
    };

    public static bool IsChild =>
        string.Equals(Environment.GetEnvironmentVariable(ChildEnvVar), "1", StringComparison.Ordinal);

    public static bool Resolve(bool? reload, bool settingsReload) =>
        reload ?? settingsReload;

    public static void RunWithReloader(IEnumerable<string>? watchDirs = null)
    {
        if (IsChild)
            throw new InvalidOperationException("RunWithReloader must not run inside the child process");

        var roots = (watchDirs ?? new[] { Directory.GetCurrentDirectory() })
            .Select(Path.GetFullPath)
            .Distinct(StringComparer.Ordinal)
            .ToList();

        Console.WriteLine($"fusion: reload enabled (watching {string.Join(", ", roots)})");

        Process? child = null;
        var mtimes = Snapshot(Collect(roots));

        void StopChild()
        {
            if (child is null || child.HasExited)
            {
                child = null;
                return;
            }
            try
            {
                child.Kill(entireProcessTree: true);
                child.WaitForExit(5000);
            }
            catch
            {
                /* best effort */
            }
            child = null;
        }

        Process StartChild()
        {
            var fileName = Environment.ProcessPath
                ?? throw new InvalidOperationException("Environment.ProcessPath is unavailable");
            var start = new ProcessStartInfo
            {
                FileName = fileName,
                UseShellExecute = false,
            };
            // Skip argv[0] (executable path); forward the rest.
            foreach (var arg in Environment.GetCommandLineArgs().Skip(1))
                start.ArgumentList.Add(arg);
            start.Environment[ChildEnvVar] = "1";
            return Process.Start(start)
                ?? throw new InvalidOperationException("failed to start reload child");
        }

        Console.CancelKeyPress += (_, e) =>
        {
            e.Cancel = true;
            StopChild();
            Environment.Exit(0);
        };

        child = StartChild();
        while (true)
        {
            Thread.Sleep(500);
            if (child.HasExited)
            {
                Console.WriteLine($"fusion: child exited ({child.ExitCode}); restarting…");
                Thread.Sleep(300);
                child = StartChild();
                mtimes = Snapshot(Collect(roots));
                continue;
            }

            var files = Collect(roots);
            var next = Snapshot(files);
            string? changed = null;
            foreach (var (file, mtime) in next)
            {
                if (!mtimes.TryGetValue(file, out var prev) || mtime > prev)
                {
                    changed = file;
                    break;
                }
            }
            if (changed is null)
            {
                foreach (var file in mtimes.Keys)
                {
                    if (!next.ContainsKey(file))
                    {
                        changed = file;
                        break;
                    }
                }
            }
            if (changed is null) continue;

            var label = changed;
            try { label = Path.GetRelativePath(Directory.GetCurrentDirectory(), changed); } catch { /* keep */ }
            Console.WriteLine($"fusion: change detected ({label}); reloading…");
            StopChild();
            child = StartChild();
            mtimes = Snapshot(Collect(roots));
        }
    }

    static List<string> Collect(IEnumerable<string> roots)
    {
        var files = new List<string>();
        foreach (var root in roots)
        {
            if (File.Exists(root))
            {
                if (Extensions.Contains(Path.GetExtension(root)))
                    files.Add(root);
                continue;
            }
            if (!Directory.Exists(root)) continue;
            foreach (var file in Directory.EnumerateFiles(root, "*", SearchOption.AllDirectories))
            {
                var rel = Path.GetRelativePath(root, file);
                if (rel.Split(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar)
                        .Any(p => SkipDirs.Contains(p)))
                    continue;
                if (Extensions.Contains(Path.GetExtension(file)))
                    files.Add(file);
            }
        }
        return files;
    }

    static Dictionary<string, long> Snapshot(IEnumerable<string> files)
    {
        var map = new Dictionary<string, long>(StringComparer.Ordinal);
        foreach (var file in files)
        {
            try { map[file] = File.GetLastWriteTimeUtc(file).Ticks; }
            catch { /* ignore */ }
        }
        return map;
    }
}
