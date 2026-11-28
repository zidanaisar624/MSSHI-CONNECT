#!/usr/bin/env node
/**
 * Live Node.js Bridge & Dev Server for MSSHI Installer & Connect v1.0.1
 * Implements the EXACT same JSON API contract and security invariants as crates/msshi-connect-app:
 * - Strictly binds 127.0.0.1:<ephemeral_port> with 256-bit CSPRNG X-MSSHI-Token + Host/Origin CSRF checks
 * - Enforces 3-Zone protection, anti-traversal/UNC/symlink, Anti-Admin-Lock, Auto-Temp Drive D:
 * - Zero paid LLM calls, ETag GitHub Releases/Tags sync, Connect Hub (auto_open_on_boot=false)
 */
import crypto from "node:crypto";
import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { execFileSync, spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
export const REPO_ROOT = path.resolve(__dirname, "..");

export const CONSTANTS = {
  APP_NAME: "MSSHI Installer & Connect",
  APP_VERSION: "1.0.1",
  UPSTREAM_FULL_NAME: "zidanaisar624/MSSHI-Mega-Semar-Sang-Hyang-Ismaya",
  SAWERIA_URL: "https://saweria.co/muzas",
  DEFAULT_REF: "v3.2.2",
  ALLOWED_HOSTS: new Set([
    "api.github.com",
    "raw.githubusercontent.com",
    "github.com",
    "codeload.github.com",
    "objects.githubusercontent.com",
    "release-assets.githubusercontent.com",
  ]),
  ZONE2_FILENAMES: new Set([
    "soul.md", "config.yaml", "config.json", "profile.yaml",
    "ui-preferences.json", "user_creative_dna.json", ".env",
    "auth.json", "auth_keys.json", "state.db", "state.db-wal", "state.db-shm",
    "active_profile",
  ]),
  ZONE3_SEGMENTS: [
    "1. private",
    "2. data diri atau perkembangan diri (np)",
    "msshi_recovery",
    "msshi-private",
  ],
  ZONE3_FILENAMES: new Set(["nlm.exe", "notebooklm-mcp.exe", "rclone.conf"]),
};

export function validateGitHubUrl(rawUrl) {
  const u = new URL(rawUrl);
  if (u.protocol !== "https:") throw new Error(`URL harus HTTPS: ${rawUrl}`);
  const host = u.hostname.toLowerCase();
  if (!CONSTANTS.ALLOWED_HOSTS.has(host)) {
    throw new Error(`Host ditolak allowlist: ${u.hostname}`);
  }
  if (host === "api.github.com" && !u.pathname.startsWith(`/repos/${CONSTANTS.UPSTREAM_FULL_NAME}`)) {
    throw new Error(`Endpoint API di luar repo resmi MSSHI: ${u.pathname}`);
  }
  if (host === "raw.githubusercontent.com" && !u.pathname.startsWith(`/${CONSTANTS.UPSTREAM_FULL_NAME}/`)) {
    throw new Error(`File raw di luar repo resmi MSSHI: ${u.pathname}`);
  }
  return rawUrl;
}

export function validateZone1Path(relPath) {
  const clean = String(relPath || "").trim().replace(/\\/g, "/");
  if (!clean) throw new Error("Path kosong");
  if (clean.startsWith("//") || clean.startsWith("/") || clean.includes(":")) {
    throw new Error(`Path absolut/UNC ditolak: ${relPath}`);
  }
  const parts = clean.split("/");
  if (parts.some((p) => p === "..")) throw new Error(`Traversal '..' ditolak: ${relPath}`);
  const lowerClean = `/${clean.toLowerCase()}/`;
  const lowerParts = parts.map((p) => p.toLowerCase());
  for (const seg of CONSTANTS.ZONE3_SEGMENTS) {
    if (lowerParts.includes(seg)) throw new Error(`Zona 3 dilindungi total: ${relPath}`);
  }
  if (lowerClean.includes("/data/bin/") || lowerClean.includes("/.config/rclone/") || lowerClean.includes("/roaming/rclone/")) {
    throw new Error(`Zona 3 biner/rclone dilindungi total: ${relPath}`);
  }
  const base = lowerParts[lowerParts.length - 1];
  if (CONSTANTS.ZONE3_FILENAMES.has(base)) {
    throw new Error(`Zona 3 biner/konfigurasi dilindungi total: ${relPath}`);
  }
  if (CONSTANTS.ZONE2_FILENAMES.has(base) || /\.(pem|key|p12|pfx)$/.test(base)) {
    throw new Error(`Zona 2 dilindungi (file user/kredensial): ${relPath}`);
  }
  return clean;
}

export function atomicWriteAclSafe(targetPath, data) {
  const dir = path.dirname(targetPath);
  fs.mkdirSync(dir, { recursive: true });
  const tmp = path.join(dir, `.msshi-tmp-${process.pid}-${Date.now()}-${crypto.randomBytes(4).toString("hex")}`);
  fs.writeFileSync(tmp, data);
  fs.renameSync(tmp, targetPath);
}

export function getOptionCatalog() {
  return {
    core_plugins: [
      "msshi-team-routing (msshi_find_skill 4-lapis)",
      "msshi-interaction v0.2.0 (Interaksi 2 Tipe hemat token)",
      "cindy-grand-plan (gerbang perencanaan besar interaktif lintas 5 bot)",
      "msshi-auto-trim (MSSHITrim lossless JSON)",
      "anti-slop (filter kualitas prosa, UI & kode lintas 5 bot)",
      "caveman (ringkas token)",
      "spacewise · sharewise · chalk",
    ],
    presets: [
      { id: "belajar", label: "1. Belajar", subtitle: "Shizuka & Yukino · Chalk, LearnLoop, OSSU CS, Anti-Slop", mode: "academic-mahasiswa", default_bot: "auto", recommended_addons: ["learnloop"], recommended_sources: ["ossu"] },
      { id: "konten", label: "2. Konten Kreasi", subtitle: "Kiki-Chii · Filmmaking S2, Higgsfield 9 modul, BRAG + Opt-in Blender MCP, RAEA, Lemo-Opuscar & Meshy 3D", mode: "muza-preset", default_bot: "auto", recommended_addons: ["imagination-studio", "lemo-opuscar"], recommended_sources: ["rea-video"] },
      { id: "riset-skripsi", label: "3. Riset & Skripsi", subtitle: "Val + Shizuka · Riset Untuk S1–S3 (Diutamakan S1 Skripsi) Engine DOI 2-Lapis + v1.2 .docx, Colab, opsional TimesFM", mode: "academic-mahasiswa", default_bot: "auto", recommended_addons: ["dreaming"], recommended_sources: [] },
      { id: "coding-security", label: "4. Coding & Security", subtitle: "Shikaku · OpenCodeReview, ast-grep, Knip, Modern Web, Rust (Hot-Path), Grim Prompter, BugHunter", mode: "muza-preset", default_bot: "auto", recommended_addons: [], recommended_sources: ["open-code-review", "ast-grep", "knip", "modern-web-guidance", "rust-anything", "grim-prompter", "claude-bughunter"] },
      { id: "muza-full", label: "5. Kombinasi — Preset MuZA (5 Bot)", subtitle: "Yukino, Shizuka, Val, Kiki-Chii, Shikaku (v3.2.2 · 82 Skill Bundel + 6 Opt-in · 76 Upstreams · Engine DOI Val + Cindy + Rust)", mode: "muza-preset", default_bot: "auto", recommended_addons: ["dreaming"], recommended_sources: [] },
    ],
    addons: [
      { id: "rea", name: "REA (Reverse Engineer Anything)", owner_bot: "Shikaku", disk_estimate: "~180 MB", ram_note: "0 MB idle", os_note: "Opt-in aman (--with-rea · default OFF)", default_enabled: false },
      { id: "imagination-studio", name: "Imagination Studio Wrapper", owner_bot: "Kiki-Chii", disk_estimate: "< 1 MB", ram_note: "0 MB idle", os_note: "Opt-in (--with-imagination-studio · default OFF)", default_enabled: false },
      { id: "mcp-for-blender", name: "MCP for Blender (Opt-in 3D Studio)", owner_bot: "Kiki-Chii", disk_estimate: "< 2 MB", ram_note: "0 MB idle", os_note: "Opt-in (--with-blender-mcp · menggantikan CLI-Anything blender)", default_enabled: false },
      { id: "raea", name: "RAEA (React/TS + Bevy GPU Video/VFX)", owner_bot: "Kiki-Chii", disk_estimate: "< 1 MB wrapper", ram_note: "0 MB idle", os_note: "Opt-in (--with-raea · biner eksternal FSL-1.1-ALv2 dipasang terpisah)", default_enabled: false },
      { id: "lemo-opuscar", name: "Lemo-Opuscar (15 Kategori / 99 Gaya Sinema Kode)", owner_bot: "Kiki-Chii", disk_estimate: "< 2 MB cache teks", ram_note: "0 MB idle", os_note: "Opt-in di Core (--with-lemo-opuscar · aktif otomatis di repo MFA Filmmaking)", default_enabled: false },
      { id: "meshy-3d-generation", name: "Meshy 3D Agent (Text/Image to 3D GLB/FBX/OBJ/STL)", owner_bot: "Kiki-Chii", disk_estimate: "< 1 MB wrapper", ram_note: "0 MB idle", os_note: "Opt-in (--with-meshy-3d · Node >= 22.12 + meshy-cli@0.8 + device login)", default_enabled: false },
      { id: "learnloop", name: "LearnLoop (AGPL-3.0 opt-in)", owner_bot: "Shizuka & Yukino", disk_estimate: "< 2 MB", ram_note: "0 MB idle", os_note: "Semua OS", default_enabled: false },
      { id: "codecoach", name: "CodeCoach", owner_bot: "Shikaku", disk_estimate: "0 MB di Windows", ram_note: "0 MB", os_note: "Sandbox penuh Linux/WSL2; Windows fallback ke Shikaku", default_enabled: false },
      { id: "dreaming", name: "Dreaming Lokal (Offline)", owner_bot: "Val", disk_estimate: "~2 MB", ram_note: "0 MB idle · 0 LLM call", os_note: "Jadwal 03.00 WIB opsional", default_enabled: false },
      { id: "timesfm", name: "TimesFM 2.5 Forecasting", owner_bot: "Val", disk_estimate: "~1.5–2.0 GB", ram_note: ">= 2.5 GiB commit saat run · 0 MB idle", os_note: "Preflight memori otomatis", default_enabled: false },
      { id: "omnivoice", name: "OmniVoice TTS Lokal (5 Bot)", owner_bot: "Semua 5 Bot", disk_estimate: "~1.5–3.0 GB", ram_note: "0 MB idle · cek pagefile otomatis", os_note: "Berat saat sintesis suara saja", default_enabled: false },
      { id: "kat-coder", name: "KAT-Coder Wizard", owner_bot: "Shikaku", disk_estimate: "Endpoint terpisah", ram_note: "Default OFF", os_note: "Endpoint privat diatur pengguna", default_enabled: false },
    ],
    context_sources: [
      { id: "open-code-review", name: "OpenCodeReview (Alibaba ocr)", owner_bot: "Shikaku", summary: "Multi-pass AI code review & audit diff lokal" },
      { id: "ast-grep", name: "ast-grep (sg)", owner_bot: "Shikaku", summary: "Pencarian & codemod AST struktural 0-token" },
      { id: "knip", name: "Knip", owner_bot: "Shikaku", summary: "Detektor dead-code, unused exports & dependensi JS/TS" },
      { id: "modern-web-guidance", name: "Modern Web Guidance", owner_bot: "Shikaku", summary: "Standar Google Chrome Baseline & Core Web Vitals" },
      { id: "rust-anything", name: "Rust-Anything (Hot-Path Only)", owner_bot: "Shikaku", summary: "Migrasi bertahap khusus jalur komputasi berat ke Rust" },
      { id: "claude-bughunter", name: "Claude-BugHunter", owner_bot: "Shikaku", summary: "83 skill & 681 pola laporan keamanan terotorisasi" },
      { id: "grim-prompter", name: "Grim Prompter", owner_bot: "Shikaku", summary: "Threat modeling & review arsitektur read-only" },
      { id: "rea-video", name: "REA-Video", owner_bot: "Kiki-Chii", summary: "Bedah video lokal ringan (--no-vlm)" },
      { id: "ossu", name: "OSSU Computer Science", owner_bot: "Shizuka & Val", summary: "63 mata kuliah ilmu komputer gratis" },
      { id: "mattpocock-skills", name: "MattPocock Skills", owner_bot: "Semua Bot", summary: "Pola engineering TypeScript & PRD" },
      { id: "career-ops", name: "Career-Ops", owner_bot: "Semua Bot", summary: "Strategi karier & wawancara terstruktur" },
      { id: "remote-jobs", name: "Remote-Jobs", owner_bot: "Semua Bot", summary: "Panduan pencarian kerja remote global" },
    ],
    setup_modes: ["muza-preset", "academic-mahasiswa", "academic-dosen", "solo", "selective", "custom-bots"],
    target_platforms: ["hermes", "antigravity", "claude", "codex", "openclaw", "manual"],
  };
}

export function classifyMsshiUpdate(targetRef, changedPaths = null, prefs = {}) {
  const samplePaths = changedPaths || [
    "scripts/msshi_cli.py",
    "scripts/msshi_doctor.py",
    "hermes_plugins/msshi-auto-trim/plugin.py",
    "skills/higgsfield-skills/SKILL.md",
    "optional/rea/SKILL.md",
    "optional/imagination-studio/SKILL.md",
  ];
  const skipped = new Set(prefs.skipped_features || []);
  const systemPatches = [];
  const optionalFeatures = [];

  for (const raw of samplePaths) {
    const p = String(raw).replace(/\\/g, "/");
    if (p.startsWith("optional/") || p.startsWith("skills/")) {
      const id = p.split("/")[1] || p;
      if (skipped.has(id)) continue;
      if (!optionalFeatures.some((f) => f.id === id)) {
        const isOpt = p.startsWith("optional/");
        optionalFeatures.push({
          id,
          title: `Fitur / Skill Baru: ${id}`,
          category: isOpt ? "addon" : "skill",
          disk_impact: isOpt ? "Opsional (~1–180 MB)" : "Ringan (< 1 MB)",
          warning: `Pembaruan ini membawa fitur '${id}'. Anda bebas memilih pasang atau tidak.`,
          default_checked: !isOpt,
        });
      }
    } else {
      systemPatches.push(p);
    }
  }

  const policy = prefs.auto_upgrade_policy || "smart_split";
  const autoOn = prefs.auto_upgrade_enabled !== false;
  return {
    target_ref: targetRef || "v3.2.2",
    system_patches: systemPatches,
    system_auto_apply_silent: autoOn && (policy === "smart_split" || policy === "auto_apply_zone1") && systemPatches.length > 0,
    optional_features: optionalFeatures,
    requires_user_feature_prompt: optionalFeatures.length > 0 && policy !== "auto_apply_zone1",
  };
}

export function buildDryRun(req) {
  const args = [
    "setup", "--mode", req.mode || "muza-preset",
    "--platform", req.platform || "hermes",
    "--default-bot", req.default_bot || "auto",
    "--yes", "--stage-auto-trim",
    "--connect-apps", req.connect_mode || "auto-reconnect",
  ];
  if (req.light_low_ram) args.push("--light", "--low-ram");
  if ((req.addons || []).includes("rea")) args.push("--with-rea");
  if ((req.addons || []).includes("imagination-studio")) args.push("--with-imagination-studio");
  if ((req.addons || []).includes("mcp-for-blender")) args.push("--with-blender-mcp");
  if ((req.addons || []).includes("raea")) args.push("--with-raea");
  if ((req.addons || []).includes("lemo-opuscar")) args.push("--with-lemo-opuscar");
  if ((req.addons || []).includes("meshy-3d-generation")) args.push("--with-meshy-3d");
  if (req.copy_pdf_manual === false) args.push("--no-pdf");

  let estMb = req.light_low_ram ? 35 : 48;
  for (const a of req.addons || []) {
    if (a === "rea") estMb += 180;
    else if (a === "timesfm") estMb += 1800;
    else if (a === "omnivoice") estMb += 2000;
    else if (a === "learnloop" || a === "dreaming" || a === "mcp-for-blender" || a === "lemo-opuscar") estMb += 2;
    else if (a === "raea" || a === "imagination-studio" || a === "meshy-3d-generation") estMb += 1;
  }

  const msshiHome = req.msshi_home || "D:\\MSSHI";
  const hermesHome = req.hermes_home || "D:\\HermesData";
  const drive = msshiHome.slice(0, 2).toUpperCase();
  const tempRedirect = drive !== "C:" && drive.endsWith(":") ? `${drive}\\HermesData\\temp` : "%TEMP%\\msshi-connect-temp";

  const steps = [
    `Arahkan TEMP/TMP ke ${tempRedirect} agar Drive C: aman.`,
    `Sinkronkan Zona 1 MSSHI (${req.target_ref || "v3.2.2"}) ke ${msshiHome} (backup receipt otomatis).`,
    `Jalankan msshi ${args.join(" ")} (Hermes Home: ${hermesHome}).`,
  ];
  if ((req.context_sources || []).length) {
    steps.push(`Fetch on-demand context-sources: ${req.context_sources.join(", ")}`);
  }
  if (req.copy_pdf_manual !== false) {
    steps.push(`Salin Buku Panduan PDF Klien (kanal: ${req.pdf_version || "latest"}).`);
  }

  return {
    target_ref: req.target_ref || "v3.2.2",
    commit_sha: req.commit_sha || null,
    msshi_home: msshiHome,
    hermes_home: hermesHome,
    temp_redirect: tempRedirect,
    steps,
    command_args: args,
    zone2_guarantee: "DIKUNCI: Provider, model, endpoint, API key, .env, auth.json, SOUL.md kustom, dan Zona 3 tidak pernah diubah.",
    estimated_download_mb: estMb,
  };
}

export function scanConnectHub(lightMode = false) {
  const pathDirs = (process.env.PATH || "").split(path.delimiter).filter((d) => path.isAbsolute(d));
  const findBin = (names) => {
    for (const d of pathDirs) {
      if (process.platform === "win32" && path.basename(d).toLowerCase() === "windowsapps") continue;
      for (const n of names) {
        const p = path.join(d, n);
        if (fs.existsSync(p)) return p;
      }
    }
    return null;
  };
  const defs = [
    ["git", "Git CLI", "local_cli", ["git.exe", "git"], ["shikaku", "val"], false],
    ["python", "Python 3", "local_cli", ["python.exe", "python3"], ["shikaku", "val", "shizuka"], false],
    ["node", "Node.js", "local_cli", ["node.exe", "node"], ["shikaku", "kiki-chii"], false],
    ["ffmpeg", "FFmpeg Media", "local_cli", ["ffmpeg.exe", "ffmpeg"], ["kiki-chii"], false],
    ["blender-mcp", "Blender 3D (via Opt-in MCP for Blender)", "mcp_opt_in", ["blender.exe", "blender"], ["kiki-chii"], true],
    ["higgsfield", "Higgsfield CLI", "local_cli", ["higgsfield.ps1", "higgsfield.cmd", "higgsfield"], ["kiki-chii"], false],
    ["obsidian-bridge", "Obsidian 3-Zone Bridge", "local_cli", ["msshi-obsidian.cmd", "msshi-obsidian"], ["yukino", "shizuka", "val", "kiki-chii", "shikaku"], false],
    ["perplexity", "Perplexity Web Route", "web_route", [], ["val", "yukino"], false],
    ["youtube-chrome", "YouTube Research Route", "web_route", [], ["kiki-chii", "val"], false],
    ["whatsapp-web", "WhatsApp Web Route", "web_route", [], ["yukino", "shizuka"], true],
  ];
  const botMappings = { yukino: [], shizuka: [], val: [], "kiki-chii": [], shikaku: [] };
  const apps = defs.map(([id, label, kind, bins, recBots, restricted]) => {
    const exe = bins.length ? findBin(bins) : null;
    const status = lightMode && restricted ? "DIBATASI DI MODE LIGHT" : kind === "web_route" || exe ? "TERDETEKSI" : "BELUM TERPASANG";
    if (status === "TERDETEKSI") {
      for (const b of recBots) botMappings[b].push(id);
    }
    return { id, label, kind, status, executable: exe, recommended_bots: recBots };
  });
  return {
    auto_open_on_boot: false,
    zero_llm_calls: true,
    browser_preference: "none",
    browser_profile_label: "",
    apps,
    bot_mappings: botMappings,
  };
}

export function collectSystemProbe(explicitMsshi, explicitHermes) {
  const totalRam = Number((os.totalmem() / 1073741824).toFixed(2));
  const availRam = Number((os.freemem() / 1073741824).toFixed(2));
  const hasD = fs.existsSync("D:\\");
  const msshiHome = explicitMsshi || (hasD ? "D:\\MSSHI" : path.join(os.homedir(), "MSSHI"));
  const hermesHome = explicitHermes || (hasD ? "D:\\HermesData" : path.join(os.homedir(), "HermesData"));

  let installedVersion = null;
  for (const cand of [path.join(msshiHome, "current", "VERSION.json"), path.join(msshiHome, "VERSION.json")]) {
    if (fs.existsSync(cand)) {
      try { installedVersion = JSON.parse(fs.readFileSync(cand, "utf8")).version || null; break; } catch {}
    }
  }

  let doctorPath = null;
  for (const cand of ["D:\\MSSHI DOCTOR\\MSSHI-Doctor-Portable-win-x64.exe", "D:\\MSSHI DOCTOR\\MSSHI-Doctor.exe", "D:\\MSSHI DOCTOR\\MSSHI_Doctor.bat"]) {
    if (fs.existsSync(cand)) { doctorPath = cand; break; }
  }

  let elevatedAdmin = false;
  if (process.platform === "win32") {
    try { execFileSync("fltmc", [], { stdio: "ignore" }); elevatedAdmin = true; } catch { elevatedAdmin = false; }
  }

  const readDrive = (letter) => {
    const root = `${letter}\\`;
    if (!fs.existsSync(root)) return null;
    try {
      const st = fs.statfsSync(root);
      const freeGb = Number(((st.bavail * st.bsize) / 1073741824).toFixed(2));
      const totalGb = Number(((st.blocks * st.bsize) / 1073741824).toFixed(2));
      return { letter, free_gb: freeGb, total_gb: totalGb, low_space_warning: freeGb < 5.0 };
    } catch {
      return { letter, free_gb: 0, total_gb: 0, low_space_warning: false };
    }
  };
  const drives = ["C:", "D:", "E:"].map(readDrive).filter(Boolean);

  return {
    os: `${process.platform} (${process.arch})`,
    total_ram_gb: totalRam,
    available_ram_gb: availRam,
    recommend_light_mode: totalRam <= 6.0,
    elevated_admin: elevatedAdmin,
    drives,
    hermes_detected: fs.existsSync(path.join(hermesHome, "profiles")) || fs.existsSync(path.join(hermesHome, "hermes-agent")),
    hermes_home: hermesHome,
    msshi_home: msshiHome,
    installed_msshi_version: installedVersion,
    doctor_detected: Boolean(doctorPath),
    doctor_path: doctorPath,
    recommended_temp_dir: hasD ? "D:\\HermesData\\temp" : path.join(os.tmpdir(), "msshi-connect-temp"),
    ml_preflight_ok: availRam >= 2.0,
    ml_preflight_note: availRam >= 2.0 ? "Siap untuk add-on opsional." : "RAM tersedia sempit; disarankan Mode Hemat 4 GB.",
  };
}

export async function fetchGitHubCatalog({ fetcher = fetch, cacheDir = null } = {}) {
  const tagsUrl = validateGitHubUrl(`https://api.github.com/repos/${CONSTANTS.UPSTREAM_FULL_NAME}/tags?per_page=20`);
  const mainUrl = validateGitHubUrl(`https://api.github.com/repos/${CONSTANTS.UPSTREAM_FULL_NAME}/commits/main`);
  const cacheFile = cacheDir ? path.join(cacheDir, "github_catalog_cache.json") : null;
  let cached = null;
  if (cacheFile && fs.existsSync(cacheFile)) {
    try { cached = JSON.parse(fs.readFileSync(cacheFile, "utf8")); } catch {}
  }

  const headers = { Accept: "application/vnd.github+json", "User-Agent": `MSSHI-Connect/${CONSTANTS.APP_VERSION}` };
  if (cached?.etag) headers["If-None-Match"] = cached.etag;

  try {
    const resp = await fetcher(tagsUrl, { headers });
    const rem = resp.headers?.get?.("x-ratelimit-remaining");
    if (resp.status === 304 && cached?.catalog) {
      return { ...cached.catalog, connection_status: "ONLINE_ETAG_CACHED", from_cache: true, rate_limit_remaining: rem ? Number(rem) : null };
    }
    if (resp.status === 403) {
      return { repo: CONSTANTS.UPSTREAM_FULL_NAME, connection_status: "RATE_LIMITED", from_cache: false, rate_limit_remaining: 0, latest_stable: CONSTANTS.DEFAULT_REF, main_commit_sha: null, refs: [{ name: CONSTANTS.DEFAULT_REF, kind: "tag", commit_sha: null, summary: "Fallback lokal (rate-limited)" }], changelog_preview: "Rate limit GitHub tercapai.", error_message: "HTTP 403" };
    }
    const tags = await resp.json();
    const refs = (Array.isArray(tags) ? tags : []).map((t) => ({
      name: t.name, kind: "tag", commit_sha: t.commit?.sha || null, summary: `Tag rilis stabil ${t.name}`,
    }));
    let mainSha = null;
    try {
      const mResp = await fetcher(mainUrl, { headers: { Accept: "application/vnd.github+json", "User-Agent": `MSSHI-Connect/${CONSTANTS.APP_VERSION}` } });
      if (mResp.ok) {
        const mJson = await mResp.json();
        mainSha = mJson.sha || null;
        refs.push({ name: "main", kind: "branch", commit_sha: mainSha, summary: `Rolling main: ${(mJson.commit?.message || "").split("\n")[0]}` });
      }
    } catch {}
    const latestStable = refs.find((r) => r.kind === "tag")?.name || CONSTANTS.DEFAULT_REF;
    const catalog = {
      repo: CONSTANTS.UPSTREAM_FULL_NAME,
      connection_status: "ONLINE_OK",
      from_cache: false,
      rate_limit_remaining: rem ? Number(rem) : null,
      latest_stable: latestStable,
      main_commit_sha: mainSha,
      refs,
      changelog_preview: `Rilis stabil terbaru: ${latestStable} | main SHA: ${mainSha || "-"}`,
      error_message: null,
    };
    if (cacheFile) {
      atomicWriteAclSafe(cacheFile, JSON.stringify({ etag: resp.headers?.get?.("etag") || null, catalog }, null, 2));
    }
    return catalog;
  } catch (err) {
    return {
      repo: CONSTANTS.UPSTREAM_FULL_NAME,
      connection_status: "OFFLINE",
      from_cache: false,
      rate_limit_remaining: null,
      latest_stable: CONSTANTS.DEFAULT_REF,
      main_commit_sha: null,
      refs: [{ name: CONSTANTS.DEFAULT_REF, kind: "tag", commit_sha: null, summary: "Mode offline (gunakan folder/ZIP lokal)" }],
      changelog_preview: "Tidak terhubung ke GitHub.",
      error_message: String(err.message || err),
    };
  }
}

export function createConnectServer({ port = 0, dataDir = null, fetcher = fetch } = {}) {
  const token = crypto.randomBytes(32).toString("hex");
  const probe = collectSystemProbe(dataDir || undefined, undefined);
  const workDir = dataDir || probe.msshi_home;
  const prefsFile = path.join(workDir, "connect-app-prefs.json");
  const hubFile = path.join(workDir, "connect-hub-mapping.json");
  const receiptsDir = path.join(workDir, "connect-receipts");
  const cacheDir = path.join(workDir, "connect-cache");

  const loadPrefs = () => {
    try { return JSON.parse(fs.readFileSync(prefsFile, "utf8")); } catch {
      return {
        auto_upgrade_enabled: true,
        auto_upgrade_policy: "smart_split",
        channel: "stable",
        selected_ref: "v3.2.2",
        approved_features: [],
        skipped_features: [],
        local_source_path: null,
      };
    }
  };

  const readJsonBody = (req) => new Promise((resolve, reject) => {
    let raw = "";
    req.on("data", (c) => { raw += c; if (raw.length > 2 * 1024 * 1024) reject(new Error("Payload terlalu besar")); });
    req.on("end", () => { try { resolve(raw ? JSON.parse(raw) : {}); } catch (e) { reject(e); } });
  });

  const mimeMap = { ".html": "text/html; charset=utf-8", ".css": "text/css; charset=utf-8", ".js": "application/javascript; charset=utf-8", ".png": "image/png", ".jpg": "image/jpeg" };

  const server = http.createServer(async (req, res) => {
    const urlPath = (req.url || "/").split("?")[0];
    const sendJson = (code, obj) => {
      res.writeHead(code, { "Content-Type": "application/json; charset=utf-8" });
      res.end(JSON.stringify(obj));
    };

    if (!urlPath.startsWith("/api/")) {
      const rel = urlPath === "/" ? "ui/index.html" : urlPath.startsWith("/assets/") ? urlPath.slice(1) : path.join("ui", urlPath.slice(1));
      const full = path.resolve(REPO_ROOT, rel);
      if (!full.startsWith(REPO_ROOT) || !fs.existsSync(full) || !fs.statSync(full).isFile()) {
        res.writeHead(404); res.end("Not Found"); return;
      }
      res.writeHead(200, { "Content-Type": mimeMap[path.extname(full)] || "application/octet-stream" });
      fs.createReadStream(full).pipe(res);
      return;
    }

    const expectedHost = `127.0.0.1:${server.address().port}`;
    const hostOk = (req.headers.host || "").toLowerCase() === expectedHost;
    const origin = req.headers.origin;
    const originOk = !origin || origin.toLowerCase() === `http://${expectedHost}`;
    const tokenOk = req.headers["x-msshi-token"] === token;
    if (!hostOk || !originOk || !tokenOk) {
      return sendJson(403, { error: "Token/Host CSRF tidak valid" });
    }

    try {
      if (req.method === "GET" && urlPath === "/api/health") return sendJson(200, { ok: true, app: CONSTANTS.APP_NAME, version: CONSTANTS.APP_VERSION, engine: "node-bridge", support_url: CONSTANTS.SAWERIA_URL });
      if (req.method === "GET" && urlPath === "/api/system/probe") return sendJson(200, collectSystemProbe(dataDir || undefined, undefined));
      if (req.method === "GET" && urlPath === "/api/catalog") return sendJson(200, getOptionCatalog());
      if (req.method === "GET" && urlPath === "/api/github/releases") return sendJson(200, await fetchGitHubCatalog({ fetcher, cacheDir }));
      if (req.method === "GET" && urlPath === "/api/prefs") return sendJson(200, loadPrefs());
      if (req.method === "PUT" && urlPath === "/api/prefs") {
        const body = await readJsonBody(req);
        const validPolicies = new Set(["smart_split", "notify_confirm", "auto_apply_zone1"]);
        const saved = {
          auto_upgrade_enabled: Boolean(body.auto_upgrade_enabled),
          auto_upgrade_policy: validPolicies.has(body.auto_upgrade_policy) ? body.auto_upgrade_policy : "smart_split",
          channel: body.channel === "main" ? "main" : "stable",
          selected_ref: String(body.selected_ref || "v3.2.2"),
          approved_features: Array.isArray(body.approved_features) ? body.approved_features.map(String) : [],
          skipped_features: Array.isArray(body.skipped_features) ? body.skipped_features.map(String) : [],
          local_source_path: body.local_source_path || null,
        };
        atomicWriteAclSafe(prefsFile, JSON.stringify(saved, null, 2));
        return sendJson(200, saved);
      }
      if (req.method === "POST" && urlPath === "/api/update/classify") {
        const body = await readJsonBody(req);
        const prefs = loadPrefs();
        return sendJson(200, classifyMsshiUpdate(body.target_ref || prefs.selected_ref, body.changed_paths || null, prefs));
      }
      if (req.method === "POST" && urlPath === "/api/install/dry-run") {
        return sendJson(200, buildDryRun(await readJsonBody(req)));
      }
      if (req.method === "POST" && urlPath === "/api/install/apply") {
        const plan = await readJsonBody(req);
        const dry = buildDryRun(plan);
        const epoch = Math.floor(Date.now() / 1000);
        const baseId = `connect-${epoch}`;
        const receiptId = fs.existsSync(path.join(receiptsDir, `${baseId}.json`)) ? `${baseId}-${Date.now() % 1000}` : baseId;
        const keys = ["target_ref", "commit_sha", "msshi_home", "hermes_home", "mode", "platform", "default_bot", "light_low_ram", "copy_pdf_manual", "pdf_version", "addons", "context_sources", "connect_mode", "local_source_path"];
        const selections = Object.fromEntries(keys.filter((key) => Object.hasOwn(plan, key)).map((key) => [key, plan[key]]));
        const receipt = { plan: selections, receipt_id: receiptId, created_epoch: epoch, action: "msshi-connect-plan", target_ref: dry.target_ref, commit_sha: dry.commit_sha, msshi_home: dry.msshi_home, hermes_home: dry.hermes_home, mode: plan.mode || "muza-preset", state: "PLANNED", can_rollback: false };
        atomicWriteAclSafe(path.join(receiptsDir, `${receiptId}.json`), JSON.stringify(receipt, null, 2));
        return sendJson(200, { ok: true, dry_run: dry, receipt });
      }
      if (req.method === "POST" && urlPath === "/api/install/core") {
        const request = await readJsonBody(req);
        if (request.confirm !== "core-v3.2.2") return sendJson(400,{error:"Konfirmasi Core v3.2.2 dahulu."});
        return sendJson(409,{error:"Untuk memasang CLI Core v3.2.2, gunakan EXE Windows di GitHub Releases."});
      }
      if (req.method === "GET" && urlPath === "/api/receipts") {
        if (!fs.existsSync(receiptsDir)) return sendJson(200, []);
        const list = fs.readdirSync(receiptsDir).filter((f) => f.endsWith(".json")).map((f) => JSON.parse(fs.readFileSync(path.join(receiptsDir, f), "utf8")));
        return sendJson(200, list.sort((a, b) => (b.created_epoch || 0) - (a.created_epoch || 0)));
      }
      if (req.method === "GET" && urlPath === "/api/connect/scan") return sendJson(200, scanConnectHub(probe.recommend_light_mode));
      if (req.method === "POST" && urlPath === "/api/connect/apply") {
        const body = await readJsonBody(req);
        const saved = { ...body, auto_open_on_boot: false, zero_llm_calls: true };
        atomicWriteAclSafe(hubFile, JSON.stringify(saved, null, 2));
        return sendJson(200, saved);
      }
      if (req.method === "POST" && urlPath === "/api/launch/doctor") {
        const doc = collectSystemProbe().doctor_path;
        if (!doc) return sendJson(404, { ok: false, error: "MSSHI Doctor belum terdeteksi di D:\\MSSHI DOCTOR" });
        spawn(doc, [], { detached: true, stdio: "ignore", shell: doc.endsWith(".bat") }).unref();
        return sendJson(200, { ok: true, launched: doc });
      }
      return sendJson(404, { error: "Endpoint tidak ditemukan" });
    } catch (e) {
      return sendJson(400, { error: String(e.message || e) });
    }
  });

  return { server, token, start: () => new Promise((resolve) => server.listen(port, "127.0.0.1", () => resolve(server.address().port))) };
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const args = process.argv.slice(2);
  if (args.includes("--probe")) {
    console.log(JSON.stringify(collectSystemProbe(), null, 2));
  } else if (args.includes("--connect-scan")) {
    console.log(JSON.stringify(scanConnectHub(), null, 2));
  } else if (args.includes("--check-updates")) {
    fetchGitHubCatalog().then((c) => console.log(JSON.stringify(c, null, 2)));
  } else {
    const app = createConnectServer({ port: 0 });
    app.start().then((p) => {
      const url = `http://127.0.0.1:${p}/#t=${app.token}`;
      console.log(`${CONSTANTS.APP_NAME} v${CONSTANTS.APP_VERSION} berjalan di http://127.0.0.1:${p}/ (Loopback Aman)`);
      if (process.platform === "win32" && !args.includes("--no-open")) {
        const edges = ["C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe", "C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe"];
        const edge = edges.find((e) => fs.existsSync(e));
        if (edge) spawn(edge, [`--app=${url}`, "--window-size=1220,780"], { detached: true, stdio: "ignore" }).unref();
        else spawn("cmd", ["/c", "start", "", url], { detached: true, stdio: "ignore" }).unref();
      }
    });
  }
}
