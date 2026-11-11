// T016 spike page script. Served from the same origin as the page (the CSP forbids inline scripts).
(async () => {
  const hash = location.hash.replace(/^#/, "");
  const violations = [];
  document.addEventListener("securitypolicyviolation", (e) => {
    violations.push({ directive: e.effectiveDirective, blocked: e.blockedURI, disposition: e.disposition });
  });
  const report = {
    ready_state_at_start: document.readyState,
    hash_present: hash.length > 0,
    hash_length: hash.length,
    origin: location.origin,
    user_agent: navigator.userAgent,
    has_tauri_global: typeof window.__TAURI__ !== "undefined",
    has_tauri_internals: typeof window.__TAURI_INTERNALS__ !== "undefined",
  };
  const call = async (url, headers) => {
    try {
      const r = await fetch(url, headers ? { headers } : undefined);
      return { status: r.status, body: (await r.text()).slice(0, 200) };
    } catch (e) {
      return { error: String(e) };
    }
  };
  report.fetch_with_token = await call("/api/version", { "x-dashboard-token": hash });
  report.fetch_without_token = await call("/api/version");
  report.fetch_wrong_token = await call("/api/version", { "x-dashboard-token": hash.split("").reverse().join("") });

  // CSP probes: the dashboard CSP must really be enforced in this webview.
  const s = document.createElement("script");
  s.textContent = "window.__inline_ran = true";
  document.body.appendChild(s);
  report.csp_inline_script_ran = window.__inline_ran === true;
  const p = document.createElement("p");
  p.textContent = "x";
  p.setAttribute("style", "color: rgb(255, 0, 0)");
  document.body.appendChild(p);
  report.csp_inline_style_applied = getComputedStyle(p).color === "rgb(255, 0, 0)";
  report.csp_other_origin_fetch = await call(location.origin.replace("127.0.0.1", "localhost") + "/api/version");

  // IPC probes: does a remote (loopback http) origin get Tauri IPC?
  const ipc = async (cmd) => {
    const inv = window.__TAURI_INTERNALS__ && window.__TAURI_INTERNALS__.invoke;
    if (!inv) return { error: "no __TAURI_INTERNALS__.invoke" };
    try {
      return { ok: await Promise.race([inv(cmd), new Promise((_, rej) => setTimeout(() => rej("timeout 3s"), 3000))]) };
    } catch (e) {
      return { error: String(e) };
    }
  };
  report.ipc_app_command_ping = await ipc("ping");
  report.ipc_plugin_app_version = await ipc("plugin:app|version");

  await new Promise((r) => setTimeout(r, 100)); // let violation events arrive
  report.csp_violations = violations;
  report.ready_state_at_post = document.readyState;
  let posted;
  try {
    const r = await fetch("/report", { method: "POST", headers: { "x-dashboard-token": hash, "content-type": "application/json" }, body: JSON.stringify(report) });
    posted = r.status;
  } catch (e) {
    posted = String(e);
  }
  document.getElementById("out").textContent = "t016 report posted: " + posted;
})();
