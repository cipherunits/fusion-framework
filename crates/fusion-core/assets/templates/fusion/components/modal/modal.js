/**
 * Fusion centered modal — variants success/warning/error, or fully custom.
 * Size (sm|md|lg|xl|CSS length) and animation (scale|fade|slide|none + duration ms)
 * are configurable per open() / data attributes / CSS variables.
 */
(function () {
  var SIZES = { sm: "20rem", md: "28rem", lg: "36rem", xl: "44rem" };
  var ANIMATIONS = ["scale", "fade", "slide", "none"];
  var active = null;
  var lastFocus = null;

  /** Return SVG markup for a status icon. */
  function iconSvg(variant) {
    if (variant === "success") {
      return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M20 6 9 17l-5-5"/></svg>';
    }
    if (variant === "warning") {
      return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M12 3 2 20h20L12 3z"/><path d="M12 9v5M12 17h.01"/></svg>';
    }
    if (variant === "error") {
      return '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m15 9-6 6M9 9l6 6"/></svg>';
    }
    return "";
  }

  /** True when size is a known preset name. */
  function isPresetSize(size) {
    return Object.prototype.hasOwnProperty.call(SIZES, size);
  }

  /** Resolve width CSS value from size option. */
  function resolveWidth(size) {
    if (!size) return SIZES.md;
    if (isPresetSize(size)) return SIZES[size];
    return size;
  }

  /** Normalize animation name. */
  function normalizeAnimation(value) {
    var a = (value || "scale").toLowerCase();
    return ANIMATIONS.indexOf(a) >= 0 ? a : "scale";
  }

  /** Ensure a reusable programmatic modal host exists. */
  function ensureHost() {
    var host = document.getElementById("fusion-modal-host");
    if (host) return host;
    host = document.createElement("div");
    host.id = "fusion-modal-host";
    host.className = "fusion-modal";
    host.setAttribute("data-fusion-modal", "");
    host.setAttribute("data-size", "md");
    host.setAttribute("data-animation", "scale");
    host.setAttribute("data-duration", "220");
    host.hidden = true;
    host.innerHTML =
      '<div class="fusion-modal__backdrop" data-fusion-modal-backdrop></div>' +
      '<div class="fusion-modal__dialog" role="dialog" aria-modal="true" tabindex="-1" data-fusion-modal-dialog>' +
      '<button type="button" class="fusion-modal__close" aria-label="Close" data-fusion-modal-close>×</button>' +
      '<div class="fusion-modal__header">' +
      '<span class="fusion-modal__icon" data-fusion-modal-icon aria-hidden="true"></span>' +
      '<h2 class="fusion-modal__title" id="fusion-modal-host-title" data-fusion-modal-title></h2>' +
      "</div>" +
      '<div class="fusion-modal__body" data-fusion-modal-body></div>' +
      '<div class="fusion-modal__footer" data-fusion-modal-footer>' +
      '<button type="button" class="fusion-btn fusion-btn--secondary" data-fusion-modal-cancel>Cancel</button>' +
      '<button type="button" class="fusion-btn fusion-btn--primary" data-fusion-modal-confirm>OK</button>' +
      "</div></div>";
    document.body.appendChild(host);
    return host;
  }

  /** Apply size / animation / duration knobs onto a modal root. */
  function applyChrome(root, options) {
    options = options || {};
    var size = options.size || root.getAttribute("data-size") || "md";
    var animation = normalizeAnimation(
      options.animation || root.getAttribute("data-animation")
    );
    var duration =
      options.duration != null
        ? options.duration
        : parseInt(root.getAttribute("data-duration") || "220", 10);
    if (!(duration >= 0)) duration = 220;

    root.setAttribute("data-size", isPresetSize(size) ? size : "custom");
    root.setAttribute("data-animation", animation);
    root.setAttribute("data-duration", String(duration));

    var dialog = root.querySelector("[data-fusion-modal-dialog]");
    if (!dialog) return;
    dialog.style.setProperty("--fusion-modal-width", resolveWidth(size));
    dialog.style.setProperty("--fusion-modal-duration", duration + "ms");
    if (options.easing) {
      dialog.style.setProperty("--fusion-modal-easing", options.easing);
    }
  }

  /** Fill title/body/variant/icons/footer labels. */
  function fillContent(root, options) {
    options = options || {};
    var variant = options.variant == null ? root.getAttribute("data-variant") || "" : options.variant;
    variant = String(variant || "");
    root.setAttribute("data-variant", variant);

    var dialog = root.querySelector("[data-fusion-modal-dialog]");
    if (dialog) {
      dialog.className = "fusion-modal__dialog";
      if (variant) dialog.classList.add("fusion-modal__dialog--" + variant);
    }

    var icon = root.querySelector("[data-fusion-modal-icon]");
    if (icon) icon.innerHTML = iconSvg(variant);

    var title = root.querySelector("[data-fusion-modal-title]");
    if (title && options.title != null) title.textContent = options.title;

    var body = root.querySelector("[data-fusion-modal-body]");
    if (body && options.body != null) {
      if (options.html) body.innerHTML = options.body;
      else body.textContent = options.body;
    }

    var cancel = root.querySelector("[data-fusion-modal-cancel]");
    var confirm = root.querySelector("[data-fusion-modal-confirm]");
    if (cancel) {
      if (options.cancelLabel != null) cancel.textContent = options.cancelLabel;
      cancel.hidden = options.showCancel === false;
    }
    if (confirm && options.confirmLabel != null) {
      confirm.textContent = options.confirmLabel;
    }

    root._fusionOnConfirm = typeof options.onConfirm === "function" ? options.onConfirm : null;
    root._fusionOnCancel = typeof options.onCancel === "function" ? options.onCancel : null;
    root._fusionOnClose = typeof options.onClose === "function" ? options.onClose : null;
  }

  /** Open a modal element (or create programmatic host). */
  function open(targetOrOptions, maybeOptions) {
    var root;
    var options;
    if (typeof targetOrOptions === "string" || targetOrOptions instanceof Element) {
      root =
        typeof targetOrOptions === "string"
          ? document.querySelector(targetOrOptions)
          : targetOrOptions;
      options = maybeOptions || {};
    } else {
      options = targetOrOptions || {};
      root = options.el
        ? typeof options.el === "string"
          ? document.querySelector(options.el)
          : options.el
        : ensureHost();
    }
    if (!root) return null;

    if (active && active !== root) close(active, { silent: true });

    applyChrome(root, options);
    fillContent(root, options);

    lastFocus = document.activeElement;
    root.hidden = false;
    // Force reflow so enter transition runs.
    void root.offsetWidth;
    root.classList.add("fusion-modal--open");
    document.body.classList.add("fusion-modal-lock");
    active = root;

    var dialog = root.querySelector("[data-fusion-modal-dialog]");
    if (dialog) dialog.focus();
    return root;
  }

  /** Close the active (or given) modal with leave animation. */
  function close(root, opts) {
    opts = opts || {};
    root = root || active;
    if (!root) return;
    var duration = parseInt(root.getAttribute("data-duration") || "220", 10);
    if (root.getAttribute("data-animation") === "none") duration = 0;

    root.classList.remove("fusion-modal--open");
    document.body.classList.remove("fusion-modal-lock");

    function finish() {
      root.hidden = true;
      if (active === root) active = null;
      if (!opts.silent && typeof root._fusionOnClose === "function") root._fusionOnClose();
      if (lastFocus && typeof lastFocus.focus === "function") {
        try {
          lastFocus.focus();
        } catch (_) {}
      }
      lastFocus = null;
    }

    if (duration > 0) setTimeout(finish, duration);
    else finish();
  }

  /** Confirm handler — runs onConfirm then closes. */
  function confirm(root) {
    root = root || active;
    if (!root) return;
    if (typeof root._fusionOnConfirm === "function") root._fusionOnConfirm();
    close(root);
  }

  /** Cancel handler — runs onCancel then closes. */
  function cancel(root) {
    root = root || active;
    if (!root) return;
    if (typeof root._fusionOnCancel === "function") root._fusionOnCancel();
    close(root);
  }

  /** Read open options from a trigger element's data-* attributes. */
  function optionsFromTrigger(el) {
    var opts = {
      variant: el.getAttribute("data-modal-variant") || "",
      title: el.getAttribute("data-modal-title") || undefined,
      body: el.getAttribute("data-modal-body") || undefined,
      size: el.getAttribute("data-modal-size") || undefined,
      animation: el.getAttribute("data-modal-animation") || undefined,
      duration: el.hasAttribute("data-modal-duration")
        ? parseInt(el.getAttribute("data-modal-duration"), 10)
        : undefined,
      confirmLabel: el.getAttribute("data-modal-confirm") || undefined,
      cancelLabel: el.getAttribute("data-modal-cancel") || undefined,
      showCancel: el.getAttribute("data-modal-show-cancel") !== "false",
    };
    if (el.hasAttribute("data-modal-html")) opts.html = true;
    return opts;
  }

  document.addEventListener("click", function (event) {
    var openTrigger = event.target.closest("[data-fusion-modal-open]");
    if (openTrigger) {
      event.preventDefault();
      var target = openTrigger.getAttribute("data-fusion-modal-open");
      var opts = optionsFromTrigger(openTrigger);
      if (target) open(target, opts);
      else open(opts);
      return;
    }

    var root = event.target.closest("[data-fusion-modal]");
    if (!root || !root.classList.contains("fusion-modal--open")) return;

    if (event.target.closest("[data-fusion-modal-close]")) {
      close(root);
      return;
    }
    if (event.target.closest("[data-fusion-modal-cancel]")) {
      cancel(root);
      return;
    }
    if (event.target.closest("[data-fusion-modal-confirm]")) {
      confirm(root);
      return;
    }
    if (event.target.matches("[data-fusion-modal-backdrop]")) {
      cancel(root);
    }
  });

  document.addEventListener("keydown", function (event) {
    if (event.key !== "Escape" || !active) return;
    cancel(active);
  });

  window.FusionModal = {
    open: open,
    close: close,
    confirm: confirm,
    cancel: cancel,
    success: function (options) {
      options = options || {};
      options.variant = "success";
      return open(options);
    },
    warning: function (options) {
      options = options || {};
      options.variant = "warning";
      return open(options);
    },
    error: function (options) {
      options = options || {};
      options.variant = "error";
      return open(options);
    },
    sizes: Object.keys(SIZES),
    animations: ANIMATIONS.slice(),
  };
})();
