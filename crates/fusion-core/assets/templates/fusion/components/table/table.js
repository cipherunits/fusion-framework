/**
 * Initializes Fusion tables: client-side paging and header column resize.
 * Safe to load once; also used as a fallback for gallery demos without inline scripts.
 */
(function () {
  /** Wire pagination + column resize for a single `.fusion-table-wrap`. */
  function initTable(root) {
    if (!root || root.getAttribute("data-fusion-ready") === "true") return;
    root.setAttribute("data-fusion-ready", "true");

    var size = parseInt(root.getAttribute("data-page-size") || "0", 10);
    var rows = root.querySelectorAll("tbody tr[data-fusion-row]");
    var pager = root.querySelector(".fusion-table-pager");
    var status = root.querySelector("[data-fusion-status]");
    var prev = root.querySelector("[data-fusion-prev]");
    var next = root.querySelector("[data-fusion-next]");

    if (size > 0 && pager && status && prev && next && rows.length > 0) {
      var page = 0;
      var pages = Math.max(1, Math.ceil(rows.length / size));

      function renderPage() {
        var start = page * size;
        var end = start + size;
        for (var i = 0; i < rows.length; i++) {
          rows[i].hidden = i < start || i >= end;
        }
        status.textContent =
          "Page " + (page + 1) + " / " + pages + " · " + rows.length + " rows";
        prev.disabled = page <= 0;
        next.disabled = page >= pages - 1;
        pager.hidden = pages <= 1;
      }

      prev.addEventListener("click", function () {
        if (page > 0) {
          page -= 1;
          renderPage();
        }
      });
      next.addEventListener("click", function () {
        if (page < pages - 1) {
          page += 1;
          renderPage();
        }
      });
      renderPage();
    }

    if (root.getAttribute("data-resizable") !== "true") return;

    var table = root.querySelector(".fusion-table");
    var cols = root.querySelectorAll("colgroup col[data-fusion-col]");
    var handles = root.querySelectorAll("[data-fusion-col-resize]");
    if (!table || !cols.length || !handles.length) return;
    table.classList.add("fusion-table--sized");

    function startResize(handle, clientX) {
      var th = handle.closest("th");
      if (!th) return;
      var index = Array.prototype.indexOf.call(th.parentNode.children, th);
      var col = cols[index];
      if (!col) return;
      var startX = clientX;
      var startW = th.getBoundingClientRect().width;
      root.classList.add("fusion-table-wrap--resizing");
      handle.classList.add("fusion-table__resize--active");

      function onMove(ev) {
        var nextW = Math.max(72, startW + (ev.clientX - startX));
        var px = nextW + "px";
        col.style.width = px;
        th.style.width = px;
      }

      function onUp() {
        root.classList.remove("fusion-table-wrap--resizing");
        handle.classList.remove("fusion-table__resize--active");
        document.removeEventListener("mousemove", onMove);
        document.removeEventListener("mouseup", onUp);
      }

      document.addEventListener("mousemove", onMove);
      document.addEventListener("mouseup", onUp);
    }

    handles.forEach(function (handle) {
      handle.addEventListener("mousedown", function (ev) {
        ev.preventDefault();
        startResize(handle, ev.clientX);
      });
      handle.addEventListener("keydown", function (ev) {
        if (ev.key !== "ArrowLeft" && ev.key !== "ArrowRight") return;
        ev.preventDefault();
        var th = handle.closest("th");
        if (!th) return;
        var index = Array.prototype.indexOf.call(th.parentNode.children, th);
        var col = cols[index];
        if (!col) return;
        var current = th.getBoundingClientRect().width;
        var delta = ev.key === "ArrowRight" ? 16 : -16;
        var px = Math.max(72, current + delta) + "px";
        col.style.width = px;
        th.style.width = px;
      });
    });
  }

  window.__fusionInitTable = initTable;

  function boot() {
    document.querySelectorAll("[data-fusion-table]").forEach(initTable);
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
