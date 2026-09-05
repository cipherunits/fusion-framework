/**
 * Wires Fusion dropdown triggers: open/close, select, outside click, Escape.
 */
(function () {
  function closeDropdown(root) {
    root.dataset.open = "false";
    var trigger = root.querySelector("[data-fusion-dropdown-trigger]");
    var menu = root.querySelector("[data-fusion-dropdown-menu]");
    if (trigger) trigger.setAttribute("aria-expanded", "false");
    if (menu) menu.hidden = true;
  }

  function openDropdown(root) {
    root.dataset.open = "true";
    var trigger = root.querySelector("[data-fusion-dropdown-trigger]");
    var menu = root.querySelector("[data-fusion-dropdown-menu]");
    if (trigger) trigger.setAttribute("aria-expanded", "true");
    if (menu) menu.hidden = false;
  }

  function toggleDropdown(root) {
    if (root.dataset.open === "true") closeDropdown(root);
    else openDropdown(root);
  }

  document.addEventListener("click", function (event) {
    var trigger = event.target.closest("[data-fusion-dropdown-trigger]");
    var item = event.target.closest("[data-fusion-dropdown-item]");
    var root = event.target.closest("[data-fusion-dropdown]");

    document.querySelectorAll("[data-fusion-dropdown][data-open='true']").forEach(function (openRoot) {
      if (!root || openRoot !== root) closeDropdown(openRoot);
    });

    if (trigger) {
      var dropdown = trigger.closest("[data-fusion-dropdown]");
      if (!dropdown || trigger.disabled) return;
      event.preventDefault();
      toggleDropdown(dropdown);
      return;
    }

    if (item) {
      var menuRoot = item.closest("[data-fusion-dropdown]");
      if (!menuRoot) return;
      var valueEl = menuRoot.querySelector("[data-fusion-dropdown-value]");
      var hidden = menuRoot.querySelector("[data-fusion-dropdown-input]");
      var label = item.textContent.trim();
      var value = item.getAttribute("data-value") || label;

      menuRoot.querySelectorAll("[data-fusion-dropdown-item]").forEach(function (opt) {
        opt.setAttribute("aria-selected", opt === item ? "true" : "false");
      });

      if (valueEl) {
        valueEl.textContent = label;
        valueEl.classList.remove("fusion-dropdown__value--placeholder");
      }
      if (hidden) hidden.value = value;
      closeDropdown(menuRoot);
    }
  });

  document.addEventListener("keydown", function (event) {
    if (event.key !== "Escape") return;
    document.querySelectorAll("[data-fusion-dropdown][data-open='true']").forEach(closeDropdown);
  });
})();
