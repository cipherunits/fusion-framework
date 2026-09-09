/**
 * Fusion toast notifications (react-hot-toast style).
 * Mount {{<fusion.toast />}} (or a .fusion-toaster host), then call FusionToast.show
 * or click any [data-fusion-toast] control (works with fusion.button markup).
 */
(function () {
  var POSITIONS = [
    "top-left",
    "top-center",
    "top-right",
    "bottom-left",
    "bottom-center",
    "bottom-right",
  ];
  var DEFAULT_DURATION = 3500;
  var seq = 0;
  var timers = {};

  /** Ensure a toaster host exists and return it. */
  function getToaster() {
    var host = document.querySelector("[data-fusion-toaster]");
    if (host) return host;
    host = document.createElement("div");
    host.className = "fusion-toaster";
    host.setAttribute("data-fusion-toaster", "");
    host.setAttribute("data-default-position", "top-center");
    host.setAttribute("data-default-duration", String(DEFAULT_DURATION));
    host.setAttribute("aria-live", "polite");
    host.setAttribute("aria-relevant", "additions");
    POSITIONS.forEach(function (pos) {
      var region = document.createElement("div");
      region.className = "fusion-toaster__region fusion-toaster__region--" + pos;
      region.setAttribute("data-position", pos);
      host.appendChild(region);
    });
    document.body.appendChild(host);
    return host;
  }

  /** Normalize a position string to one of the six supported slots. */
  function normalizePosition(value, fallback) {
    var pos = (value || fallback || "top-center").toLowerCase();
    return POSITIONS.indexOf(pos) >= 0 ? pos : "top-center";
  }

  /** Build the small status icon for a toast variant. */
  function iconSvg(variant) {
    if (variant === "success") {
      return '<svg class="fusion-toast__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>';
    }
    if (variant === "error") {
      return '<svg class="fusion-toast__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m15 9-6 6M9 9l6 6"/></svg>';
    }
    if (variant === "warning") {
      return '<svg class="fusion-toast__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 3 2 20h20L12 3z"/><path d="M12 9v5M12 17h.01"/></svg>';
    }
    if (variant === "info") {
      return '<svg class="fusion-toast__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 8h.01M11 12h1v5h1"/></svg>';
    }
    return '<svg class="fusion-toast__icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M8 12h8"/></svg>';
  }

  /** Remove a toast by id with leave animation. */
  function dismiss(id) {
    var el = document.querySelector('.fusion-toast[data-toast-id="' + id + '"]');
    if (!el) return;
    if (timers[id]) {
      clearTimeout(timers[id]);
      delete timers[id];
    }
    el.classList.remove("fusion-toast--visible");
    el.classList.add("fusion-toast--leaving");
    setTimeout(function () {
      if (el.parentNode) el.parentNode.removeChild(el);
    }, 180);
  }

  /** Dismiss every visible toast. */
  function dismissAll() {
    document.querySelectorAll(".fusion-toast[data-toast-id]").forEach(function (el) {
      dismiss(el.getAttribute("data-toast-id"));
    });
  }

  /**
   * Show a toast notification.
   * @param {string} message
   * @param {{variant?: string, position?: string, duration?: number}} [options]
   * @returns {string} toast id
   */
  function show(message, options) {
    options = options || {};
    var host = getToaster();
    var variant = options.variant || "default";
    var position = normalizePosition(
      options.position,
      host.getAttribute("data-default-position")
    );
    var duration = options.duration;
    if (duration == null) {
      duration = parseInt(host.getAttribute("data-default-duration") || "", 10);
      if (!(duration > 0)) duration = DEFAULT_DURATION;
    }
    var region = host.querySelector('[data-position="' + position + '"]');
    if (!region) region = host.querySelector('[data-position="top-center"]');

    seq += 1;
    var id = "toast-" + seq;
    var el = document.createElement("div");
    el.className = "fusion-toast fusion-toast--" + variant;
    el.setAttribute("role", "status");
    el.setAttribute("data-toast-id", id);
    el.innerHTML =
      iconSvg(variant) +
      '<div class="fusion-toast__body"></div>' +
      '<button type="button" class="fusion-toast__close" aria-label="Dismiss" data-fusion-toast-close>×</button>';
    el.querySelector(".fusion-toast__body").textContent = String(message == null ? "" : message);

    region.appendChild(el);
    requestAnimationFrame(function () {
      el.classList.add("fusion-toast--visible");
    });

    el.querySelector("[data-fusion-toast-close]").addEventListener("click", function () {
      dismiss(id);
    });

    el.addEventListener("mouseenter", function () {
      if (timers[id]) {
        clearTimeout(timers[id]);
        delete timers[id];
      }
    });
    el.addEventListener("mouseleave", function () {
      if (duration > 0) {
        timers[id] = setTimeout(function () {
          dismiss(id);
        }, duration);
      }
    });

    if (duration > 0) {
      timers[id] = setTimeout(function () {
        dismiss(id);
      }, duration);
    }
    return id;
  }

  function withVariant(variant) {
    return function (message, options) {
      options = options || {};
      options.variant = variant;
      return show(message, options);
    };
  }

  /** Handle clicks on [data-fusion-toast] triggers (including fusion.button). */
  function onClick(event) {
    var trigger = event.target.closest("[data-fusion-toast]");
    if (!trigger) return;
    event.preventDefault();
    show(trigger.getAttribute("data-fusion-toast") || trigger.textContent.trim(), {
      variant: trigger.getAttribute("data-toast-variant") || "default",
      position: trigger.getAttribute("data-toast-position") || undefined,
      duration: trigger.hasAttribute("data-toast-duration")
        ? parseInt(trigger.getAttribute("data-toast-duration"), 10)
        : undefined,
    });
  }

  document.addEventListener("click", onClick);

  window.FusionToast = {
    show: show,
    success: withVariant("success"),
    error: withVariant("error"),
    warning: withVariant("warning"),
    info: withVariant("info"),
    dismiss: dismiss,
    dismissAll: dismissAll,
    positions: POSITIONS.slice(),
  };
})();
