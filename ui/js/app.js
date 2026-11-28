import { apiRequest, initSessionToken } from "./api.js";
import { applyPresetToState, createInitialState, resolveEffectiveRef } from "./state.js";

const state = createInitialState();
const $ = (id) => document.getElementById(id);

function escapeHtml(value) {
  const entities = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
  return String(value).replace(/[&<>"']/g, (character) => entities[character]);
}

function setStatus(msg) {
  $("status-msg").textContent = msg;
}

function renderTabs() {
  document.querySelectorAll(".tab").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".tab").forEach((b) => b.classList.remove("active"));
      document.querySelectorAll(".panel").forEach((p) => p.classList.remove("active"));
      btn.classList.add("active");
      $("tab-" + btn.dataset.tab).classList.add("active");
    });
  });
}

function renderProbe() {
  const p = state.probe;
  if (!p) return;
  $("admin-warning").classList.toggle("hidden", !p.elevated_admin);
  $("inp-msshi-home").value = p.msshi_home;
  $("inp-hermes-home").value = p.hermes_home;
  state.setup.msshi_home = p.msshi_home;
  state.setup.hermes_home = p.hermes_home;
  if (p.recommend_light_mode) {
    state.setup.light_low_ram = true;
    $("chk-light-ram").checked = true;
  }

  const rows = [
    ["Hermes Agent", p.hermes_detected ? `TERDETEKSI (${p.hermes_home})` : "BELUM TERPASANG"],
    ["MSSHI Terpasang", p.installed_msshi_version ? `v${p.installed_msshi_version} (${p.msshi_home})` : "Belum terpasang"],
    ["RAM Perangkat", `${p.available_ram_gb} / ${p.total_ram_gb} GB ${p.recommend_light_mode ? "(Disarankan Mode Hemat 4 GB)" : "(Standar)"}`],
    ["Mode User Windows", p.elevated_admin ? "⚠️ Administrator (Bahaya Admin-Lock)" : "Normal User (AMAN)"],
    ["Auto-Temp Redirect", p.recommended_temp_dir],
    ["Preflight Add-on ML", p.ml_preflight_note],
    ["MSSHI Doctor", p.doctor_detected ? `Terdeteksi (${p.doctor_path})` : "Belum terdeteksi (Repo terpisah)"],
  ];
  $("probe-list").innerHTML = rows
    .map(([k, v]) => `<div class="kv-item"><span class="kv-key">${escapeHtml(k)}</span><strong>${escapeHtml(v)}</strong></div>`)
    .join("");

  $("drive-pills").innerHTML = (p.drives || [])
    .map((d) => `<span class="pill ${d.low_space_warning ? "pill-amber" : "pill-ok"}">Drive ${escapeHtml(d.letter)} ${escapeHtml(d.free_gb)} GB bebas</span>`)
    .join("");
}

function renderGitHubAndPrefs() {
  const g = state.github;
  const pr = state.prefs;
  const on = Boolean(pr.auto_upgrade_enabled);
  $("btn-auto-on").classList.toggle("active", on);
  $("btn-auto-off").classList.toggle("active", !on);
  $("policy-row").classList.toggle("hidden", !on);
  $("sel-policy").value = pr.auto_upgrade_policy;
  $("sel-channel").value = pr.channel;
  $("inp-local-source").value = pr.local_source_path || "";
  $("badge-autoup").textContent = on ? `Auto-Upgrade: ON (${pr.channel})` : `Manual Pin: ${pr.selected_ref}`;

  if (g) {
    const ok = g.connection_status.startsWith("ONLINE");
    $("badge-github").textContent = `GitHub: ${g.connection_status} (${g.latest_stable})`;
    $("badge-github").className = `pill ${ok ? "pill-ok" : "pill-amber"}`;
    const sel = $("sel-version");
    sel.replaceChildren(...(g.refs || []).map((ref) => new Option(`${ref.name} — ${ref.summary}`, ref.name)));
    sel.value = pr.selected_ref || g.latest_stable;
  }
}

function renderCatalog() {
  const c = state.catalog;
  if (!c) return;
  $("sel-mode").innerHTML = c.setup_modes.map((m) => `<option value="${m}">${m}</option>`).join("");
  $("sel-platform").innerHTML = c.target_platforms.map((p) => `<option value="${p}">${p}</option>`).join("");
  $("sel-mode").value = state.setup.mode;
  $("sel-platform").value = state.setup.platform;
  $("sel-default-bot").value = state.setup.default_bot;

  $("core-plugins-list").innerHTML = c.core_plugins.map((p) => `<span class="pill pill-ok">✓ ${p}</span>`).join("");

  $("preset-grid").innerHTML = c.presets
    .map(
      (pr) => `<button type="button" class="preset-card ${state.selectedPreset === pr.id ? "selected" : ""}" data-preset="${pr.id}">
        <strong>${pr.label}</strong><span>${pr.subtitle}</span>
      </button>`
    )
    .join("");

  $("preset-grid").querySelectorAll(".preset-card").forEach((card) => {
    card.addEventListener("click", () => {
      const found = c.presets.find((x) => x.id === card.dataset.preset);
      applyPresetToState(state, found);
      renderCatalog();
      setStatus(`Preset "${found.label}" diterapkan ke Tab 2 & Tab 3.`);
    });
  });

  $("addons-list").innerHTML = c.addons
    .map(
      (a) => `<div class="checkbox-card">
        <label><input type="checkbox" data-addon="${a.id}" ${state.setup.addons.includes(a.id) ? "checked" : ""}>
          <strong>${a.name}</strong> <span class="pill pill-info">${a.owner_bot}</span></label>
        <p class="muted">${a.disk_estimate} · ${a.ram_note} · ${a.os_note}</p>
      </div>`
    )
    .join("");

  $("addons-list").querySelectorAll("input[data-addon]").forEach((cb) => {
    cb.addEventListener("change", () => {
      const id = cb.dataset.addon;
      state.setup.addons = cb.checked
        ? [...new Set([...state.setup.addons, id])]
        : state.setup.addons.filter((x) => x !== id);
    });
  });

  $("sources-list").innerHTML = c.context_sources
    .map(
      (s) => `<div class="checkbox-card">
        <label><input type="checkbox" data-source="${s.id}" ${state.setup.context_sources.includes(s.id) ? "checked" : ""}>
          <strong>${s.name}</strong> <span class="pill pill-amber">${s.owner_bot}</span></label>
        <p class="muted">${s.summary}</p>
      </div>`
    )
    .join("");

  $("sources-list").querySelectorAll("input[data-source]").forEach((cb) => {
    cb.addEventListener("change", () => {
      const id = cb.dataset.source;
      state.setup.context_sources = cb.checked
        ? [...new Set([...state.setup.context_sources, id])]
        : state.setup.context_sources.filter((x) => x !== id);
    });
  });
}

function renderConnectHub() {
  const h = state.hub;
  if (!h) return;
  $("sel-browser").value = h.browser_preference || "none";
  $("inp-browser-label").value = h.browser_profile_label || "";

  $("hub-apps-table").innerHTML = `<table class="hub-table">
    <thead><tr><th>Aplikasi</th><th>Jenis</th><th>Status</th><th>Bot Terhubung</th></tr></thead>
    <tbody>${h.apps
      .map((a) => {
        const cls = a.status === "TERDETEKSI" ? "pill-ok" : a.status.includes("LIGHT") ? "pill-amber" : "pill-danger";
        return `<tr><td><strong>${escapeHtml(a.label)}</strong></td><td>${escapeHtml(a.kind)}</td><td><span class="pill ${cls}">${escapeHtml(a.status)}</span></td><td>${escapeHtml(a.recommended_bots.join(", "))}</td></tr>`;
      })
      .join("")}</tbody></table>`;

  $("hub-bot-matrix").innerHTML = Object.entries(h.bot_mappings || {})
    .map(([bot, list]) => `<div class="kv-item"><span class="kv-key">@${escapeHtml(bot)}</span><strong>${escapeHtml(list.length ? list.join(", ") : "-")}</strong></div>`)
    .join("");
}

function renderReceipts() {
  const list = state.receipts || [];
  if (!list.length) {
    $("receipts-list").innerHTML = `<div class="kv-item"><span class="kv-key">Riwayat</span><span>Belum ada rencana yang disimpan.</span></div>`;
    return;
  }
  $("receipts-list").innerHTML = list
    .map((r) => `<div class="kv-item"><div><strong>${escapeHtml(r.receipt_id)}</strong><div class="muted">${escapeHtml(r.target_ref)} · ${escapeHtml(r.mode)} · ${escapeHtml(r.state)}</div></div>
      <span class="pill pill-ok">${r.state === "PLANNED" ? "Rencana" : "Catatan lama"}</span></div>`)
    .join("");
}

function collectPlanPayload() {
  const targetRef = resolveEffectiveRef(state.prefs, state.github);
  const foundRef = (state.github?.refs || []).find((r) => r.name === targetRef);
  return {
    target_ref: targetRef,
    commit_sha: foundRef?.commit_sha || (targetRef === "main" ? state.github?.main_commit_sha : null) || null,
    msshi_home: $("inp-msshi-home").value.trim() || state.setup.msshi_home,
    hermes_home: $("inp-hermes-home").value.trim() || state.setup.hermes_home,
    mode: $("sel-mode").value || state.setup.mode,
    platform: $("sel-platform").value || state.setup.platform,
    default_bot: $("sel-default-bot").value || state.setup.default_bot,
    light_low_ram: $("chk-light-ram").checked,
    copy_pdf_manual: $("chk-copy-pdf").checked,
    pdf_version: $("sel-pdf-ver").value,
    addons: state.setup.addons,
    context_sources: state.setup.context_sources,
    connect_mode: state.setup.connect_mode,
    local_source_path: $("inp-local-source").value.trim() || null,
  };
}

function wireEvents() {
  for (const [id, key] of [["sel-mode", "mode"], ["sel-platform", "platform"], ["sel-default-bot", "default_bot"]]) {
    $(id).addEventListener("change", () => { state.setup[key] = $(id).value; });
  }
  $("btn-auto-on").addEventListener("click", () => { state.prefs.auto_upgrade_enabled = true; renderGitHubAndPrefs(); });
  $("btn-auto-off").addEventListener("click", () => { state.prefs.auto_upgrade_enabled = false; renderGitHubAndPrefs(); });
  $("sel-policy").addEventListener("change", (e) => { state.prefs.auto_upgrade_policy = e.target.value; });
  $("sel-channel").addEventListener("change", (e) => { state.prefs.channel = e.target.value; renderGitHubAndPrefs(); });
  $("sel-version").addEventListener("change", (e) => { state.prefs.selected_ref = e.target.value; renderGitHubAndPrefs(); });

  $("btn-save-prefs").addEventListener("click", async () => {
    state.prefs.local_source_path = $("inp-local-source").value.trim() || null;
    try {
      state.prefs = await apiRequest("/api/prefs", { method: "PUT", body: state.prefs });
      renderGitHubAndPrefs();
      setStatus("Preferensi versi & auto-upgrade tersimpan di connect-app-prefs.json.");
    } catch (e) { setStatus("Gagal menyimpan preferensi: " + e.message); }
  });

  $("btn-refresh-probe").addEventListener("click", async () => {
    state.probe = await apiRequest("/api/system/probe");
    renderProbe();
    setStatus("Deteksi perangkat & Hermes diperbarui.");
  });

  $("btn-check-github").addEventListener("click", async () => {
    setStatus("Memeriksa rilis & tag GitHub dengan ETag...");
    state.github = await apiRequest("/api/github/releases");
    renderGitHubAndPrefs();
    setStatus(`GitHub diperbarui (${state.github.connection_status}).`);
  });

  $("btn-show-changelog").addEventListener("click", () => {
    $("changelog-body").textContent = state.github?.changelog_preview || "Belum ada changelog dimuat.";
    $("changelog-modal").classList.remove("hidden");
  });
  $("btn-changelog-close").addEventListener("click", () => $("changelog-modal").classList.add("hidden"));

  $("btn-smart-classify").addEventListener("click", async () => {
    const targetRef = resolveEffectiveRef(state.prefs, state.github);
    const classified = await apiRequest("/api/update/classify", {
      method: "POST",
      body: { target_ref: targetRef },
    });
    $("system-silent-banner").innerHTML = classified.system_auto_apply_silent
      ? `✓ <strong>Update Masalah Sistem (${classified.system_patches.length} patch inti):</strong> Contoh perubahan inti yang termasuk dalam kebijakan pilihan Anda.`
      : `ℹ️ Mode Manual aktif: ${classified.system_patches.length} contoh perubahan inti untuk ditinjau.`;

    $("feature-prompt-list").innerHTML = (classified.optional_features || [])
      .map(
        (f) => `<div class="checkbox-card">
          <label><input type="checkbox" data-feat="${escapeHtml(f.id)}" ${f.default_checked ? "checked" : ""}>
            <strong>${escapeHtml(f.title)}</strong> <span class="pill pill-amber">${escapeHtml(f.disk_impact)}</span></label>
          <p class="muted">${escapeHtml(f.warning)}</p>
        </div>`
      )
      .join("");
    $("feature-prompt-modal").classList.remove("hidden");
  });

  $("btn-feature-skip-all").addEventListener("click", async () => {
    const allIds = [...$("feature-prompt-list").querySelectorAll("input[data-feat]")].map((i) => i.dataset.feat);
    state.prefs.skipped_features = [...new Set([...(state.prefs.skipped_features || []), ...allIds])];
    state.prefs = await apiRequest("/api/prefs", { method: "PUT", body: state.prefs });
    $("feature-prompt-modal").classList.add("hidden");
    setStatus("Preferensi tersimpan. Semua fitur contoh dilewati.");
  });

  $("btn-feature-apply-selected").addEventListener("click", async () => {
    const inputs = [...$("feature-prompt-list").querySelectorAll("input[data-feat]")];
    const approved = inputs.filter((i) => i.checked).map((i) => i.dataset.feat);
    const skipped = inputs.filter((i) => !i.checked).map((i) => i.dataset.feat);
    state.prefs.approved_features = approved;
    state.prefs.skipped_features = skipped;
    state.prefs = await apiRequest("/api/prefs", { method: "PUT", body: state.prefs });
    $("feature-prompt-modal").classList.add("hidden");
    setStatus(`Preferensi tersimpan. ${approved.length} fitur dipilih dan ${skipped.length} dilewati.`);
  });

  $("btn-scan-hub").addEventListener("click", async () => {
    state.hub = await apiRequest("/api/connect/scan");
    renderConnectHub();
    setStatus("Pemindaian Connect Hub selesai (0 LLM call).");
  });

  $("btn-reconnect-all").addEventListener("click", async () => {
    state.hub = await apiRequest("/api/connect/scan");
    state.hub.browser_preference = $("sel-browser").value;
    state.hub.browser_profile_label = $("inp-browser-label").value.trim();
    state.hub = await apiRequest("/api/connect/apply", { method: "POST", body: state.hub });
    renderConnectHub();
    setStatus("Peta aplikasi tersimpan. Konfigurasi profil bot dilanjutkan melalui setup MSSHI.");
  });

  $("btn-save-hub").addEventListener("click", async () => {
    if (!state.hub) return;
    state.hub.browser_preference = $("sel-browser").value;
    state.hub.browser_profile_label = $("inp-browser-label").value.trim();
    state.hub = await apiRequest("/api/connect/apply", { method: "POST", body: state.hub });
    renderConnectHub();
    setStatus("Pemetaan Connect Hub disimpan.");
  });

  $("btn-dry-run").addEventListener("click", async () => {
    const dry = await apiRequest("/api/install/dry-run", { method: "POST", body: collectPlanPayload() });
    $("dry-run-box").textContent = JSON.stringify(dry, null, 2);
    document.querySelector('.tab[data-tab="logs"]').click();
    setStatus(`Dry-Run Preview siap (estimasi ${dry.estimated_download_mb} MB · 0 file diubah).`);
  });

  $("btn-install-core").addEventListener("click", async () => {
    if (!state.probe) { setStatus("Tunggu sampai deteksi perangkat selesai."); return; }
    const message = "Unduh CLI Core v3.2.2 resmi dari GitHub ke " + state.probe.msshi_home +
      "? Memerlukan Git dan Python 3.10+. Preset, add-on, serta konfigurasi profil dapat kamu pilih setelahnya lewat msshi setup.";
    if (!window.confirm(message)) return;
    const button = $("btn-install-core");
    button.disabled = true;
    setStatus("Menunggu konfirmasi Windows dan pemasangan Core…");
    try {
      const result = await apiRequest("/api/install/core", {method:"POST",body:{confirm:"core-v3.2.2"}});
      setStatus("CLI Core terpasang. Langkah berikutnya: buka " + result.msshi_home + "\\bin dan jalankan .\\msshi.cmd setup di PowerShell.");
    } catch (e) { setStatus(e.message); }
    finally { button.disabled = false; }
  });
  $("btn-apply-open").addEventListener("click", async () => {
    const dry = await apiRequest("/api/install/dry-run", { method: "POST", body: collectPlanPayload() });
    $("confirm-summary").textContent = [
      `Target Ref     : ${dry.target_ref} (SHA: ${dry.commit_sha || "offline/local"})`,
      `MSSHI Home     : ${dry.msshi_home}`,
      `Hermes Home    : ${dry.hermes_home}`,
      `Auto-Temp      : ${dry.temp_redirect}`,
      `Estimasi Unduh : ~${dry.estimated_download_mb} MB`,
      `Langkah        :\n  - ${dry.steps.join("\n  - ")}`,
      `\n${dry.zone2_guarantee}`,
    ].join("\n");
    $("chk-confirm-zone2").checked = false;
    $("btn-confirm-go").disabled = true;
    $("confirm-modal").classList.remove("hidden");
  });

  $("chk-confirm-zone2").addEventListener("change", (e) => { $("btn-confirm-go").disabled = !e.target.checked; });
  $("btn-confirm-cancel").addEventListener("click", () => $("confirm-modal").classList.add("hidden"));

  $("btn-confirm-go").addEventListener("click", async () => {
    $("confirm-modal").classList.add("hidden");
    setStatus("Menyimpan rencana pilihanmu...");
    try {
      const res = await apiRequest("/api/install/apply", { method: "POST", body: collectPlanPayload() });
      $("dry-run-box").textContent = JSON.stringify(res, null, 2);
      state.receipts = await apiRequest("/api/receipts");
      renderReceipts();
      document.querySelector('.tab[data-tab="logs"]').click();
      setStatus(`Rencana tersimpan: ${res.receipt?.receipt_id || "ok"}`);
    } catch (e) { setStatus("Gagal menyimpan rencana: " + e.message); }
  });

  $("btn-refresh-receipts").addEventListener("click", async () => {
    state.receipts = await apiRequest("/api/receipts");
    renderReceipts();
  });

  $("btn-open-doctor").addEventListener("click", async () => {
    try {
      const r = await apiRequest("/api/launch/doctor", { method: "POST" });
      setStatus(`Membuka MSSHI Doctor (${r.launched})...`);
    } catch (e) { setStatus(e.message); }
  });
}

async function bootstrap() {
  initSessionToken();
  renderTabs();
  wireEvents();
  try {
    const [probe, catalog, prefs, hub, receipts] = await Promise.all([
      apiRequest("/api/system/probe"),
      apiRequest("/api/catalog"),
      apiRequest("/api/prefs"),
      apiRequest("/api/connect/scan"),
      apiRequest("/api/receipts"),
    ]);
    state.probe = probe;
    state.catalog = catalog;
    state.prefs = prefs;
    state.hub = hub;
    state.receipts = receipts;
    renderProbe();
    renderCatalog();
    renderConnectHub();
    renderReceipts();
    renderGitHubAndPrefs();
    state.github = await apiRequest("/api/github/releases");
    renderGitHubAndPrefs();
    setStatus("Siap · Terhubung ke mesin lokal Rust/JS (0 kredit AI).");
  } catch (e) {
    setStatus("Kesalahan inisialisasi: " + e.message);
  }
}

bootstrap();
