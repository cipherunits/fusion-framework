/**
 * Progressive-enhancement forms for Fusion templates.
 *
 * Usage:
 *   <form data-fusion-form method="post" action="/register">
 *     <div id="fusion-form-status"></div>
 *     <input name="phone" />
 *     <span class="fusion-field-error" data-field="phone"></span>
 *   </form>
 *   <script> /* include builtin fusion/form.js here */ </script>
 *
 * Posts with Accept: application/json and paints ok/errors without leaving the page.
 * Without JS, the browser does a normal HTML POST (server still uses ok/fail).
 */
 
(function () {
  function clearErrors(form) {
    form.querySelectorAll(".fusion-field-error").forEach(function (el) {
      el.textContent = "";
    });
    var status = form.querySelector("#fusion-form-status") || document.getElementById("fusion-form-status");
    if (status) {
      status.textContent = "";
      status.classList.remove("fusion-form-ok", "fusion-form-fail");
    }
  }

  function paintErrors(form, errors) {
    if (!errors || typeof errors !== "object") return;
    Object.keys(errors).forEach(function (key) {
      var el =
        form.querySelector('.fusion-field-error[data-field="' + key + '"]') ||
        document.querySelector('.fusion-field-error[data-field="' + key + '"]');
      if (el) el.textContent = String(errors[key] || "");
    });
  }

  function paintStatus(form, message, ok) {
    var status = form.querySelector("#fusion-form-status") || document.getElementById("fusion-form-status");
    if (!status) return;
    status.textContent = message || (ok ? "OK" : "Error");
    status.classList.toggle("fusion-form-ok", !!ok);
    status.classList.toggle("fusion-form-fail", !ok);
  }

  function fillFields(form, fields) {
    if (!fields || typeof fields !== "object") return;
    Object.keys(fields).forEach(function (key) {
      var input = form.querySelector('[name="' + key + '"]');
      if (!input || input.type === "password") return;
      input.value = fields[key] == null ? "" : String(fields[key]);
    });
  }

  document.addEventListener(
    "submit",
    function (event) {
      var form = event.target;
      if (!form || !form.getAttribute || !form.hasAttribute("data-fusion-form")) return;
      event.preventDefault();
      clearErrors(form);

      var action = form.getAttribute("action") || window.location.pathname;
      var method = (form.getAttribute("method") || "post").toUpperCase();
      var body = new URLSearchParams(new FormData(form));

      fetch(action, {
        method: method,
        headers: {
          Accept: "application/json",
          "Content-Type": "application/x-www-form-urlencoded;charset=UTF-8",
        },
        body: body.toString(),
        credentials: "same-origin",
      })
        .then(function (res) {
          return res.json().then(function (data) {
            return { res: res, data: data };
          });
        })
        .then(function (payload) {
          var data = payload.data || {};
          fillFields(form, data.fields);
          if (data.ok) {
            paintStatus(form, data.message || "Saved", true);
            form.dispatchEvent(new CustomEvent("fusion:form-ok", { detail: data }));
            return;
          }
          paintErrors(form, data.errors);
          paintStatus(form, data.message || "Validation failed", false);
          form.dispatchEvent(new CustomEvent("fusion:form-fail", { detail: data }));
        })
        .catch(function () {
          paintStatus(form, "Request failed", false);
        });
    },
    true
  );
})();
