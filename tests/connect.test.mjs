import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

import {
  CONSTANTS,
  buildDryRun,
  classifyMsshiUpdate,
  createConnectServer,
  fetchGitHubCatalog,
  getOptionCatalog,
  scanConnectHub,
  validateGitHubUrl,
  validateZone1Path,
} from "../dev/node-bridge.mjs";
import { applyPresetToState, createInitialState, resolveEffectiveRef } from "../ui/js/state.js";

test("3-Zone safety blocks traversal, UNC, Zone 2 secrets, and Zone 3 private paths", () => {
  assert.throws(() => validateZone1Path("../evil.txt"));
  assert.throws(() => validateZone1Path("\\\\server\\share\\file.txt"));
  assert.throws(() => validateZone1Path("SOUL.md"));
  assert.throws(() => validateZone1Path("profiles/yukino/config.yaml"));
  assert.throws(() => validateZone1Path("profiles/yukino/.env"));
  assert.throws(() => validateZone1Path("active_profile"));
  assert.throws(() => validateZone1Path("data/bin/nlm.exe"));
  assert.throws(() => validateZone1Path("1. Private/notes.md"));
  assert.throws(() => validateZone1Path("2. Data Diri atau Perkembangan Diri (NP)/photo.png"));
  assert.equal(validateZone1Path("scripts/msshi_cli.py"), "scripts/msshi_cli.py");
});

test("GitHub allowlist accepts official MSSHI repo HTTPS and blocks foreign hosts", () => {
  const ok = `https://api.github.com/repos/${CONSTANTS.UPSTREAM_FULL_NAME}/tags`;
  assert.equal(validateGitHubUrl(ok), ok);
  assert.throws(() => validateGitHubUrl(`http://api.github.com/repos/${CONSTANTS.UPSTREAM_FULL_NAME}/tags`));
  assert.throws(() => validateGitHubUrl("https://api.github.com/repos/other/repo/tags"));
  assert.throws(() => validateGitHubUrl("https://raw.githubusercontent.com/other/repo/main/README.md"));
  assert.throws(() => validateGitHubUrl("https://evil.example.com/payload"));
});

test("UI state presets and Auto-Upgrade ON/OFF version resolution work accurately", () => {
  const st = createInitialState();
  const cat = getOptionCatalog();
  const coding = cat.presets.find((p) => p.id === "coding-security");
  applyPresetToState(st, coding);
  assert.equal(st.setup.mode, "muza-preset");
  assert.ok(st.setup.context_sources.includes("open-code-review"));
  assert.ok(st.setup.context_sources.includes("ast-grep"));
  assert.ok(st.setup.context_sources.includes("knip"));
  assert.ok(st.setup.context_sources.includes("rust-anything"));

  st.prefs.auto_upgrade_enabled = false;
  st.prefs.selected_ref = "v3.1.2";
  st.prefs.channel = "main";
  assert.equal(resolveEffectiveRef(st.prefs, { latest_stable: "v3.2.0" }), "v3.1.2");

  st.prefs.auto_upgrade_enabled = true;
  assert.equal(resolveEffectiveRef(st.prefs, { latest_stable: "v3.2.0" }), "main");
  st.prefs.channel = "stable";
  assert.equal(resolveEffectiveRef(st.prefs, { latest_stable: "v3.2.0" }), "v3.2.0");
});

test("Dry-run command builder enforces Auto-Temp D: redirect and light/addon flags", () => {
  const dry = buildDryRun({
    target_ref: "v3.2.0",
    msshi_home: "D:\\MSSHI",
    hermes_home: "D:\\HermesData",
    mode: "muza-preset",
    platform: "hermes",
    default_bot: "auto",
    light_low_ram: true,
    addons: ["rea", "imagination-studio", "mcp-for-blender", "raea", "lemo-opuscar", "meshy-3d-generation"],
    context_sources: ["open-code-review"],
    connect_mode: "auto-reconnect",
  });
  assert.equal(dry.temp_redirect, "D:\\HermesData\\temp");
  assert.ok(dry.command_args.includes("--light"));
  assert.ok(dry.command_args.includes("--with-rea"));
  assert.ok(dry.command_args.includes("--with-imagination-studio"));
  assert.ok(dry.command_args.includes("--with-blender-mcp"));
  assert.ok(dry.command_args.includes("--with-raea"));
  assert.ok(dry.command_args.includes("--with-lemo-opuscar"));
  assert.ok(dry.command_args.includes("--with-meshy-3d"));
});

test("GitHub ETag 304 caching and offline fallback work gracefully", async () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "msshi-connect-gh-"));
  try {
    const mockFetch = async (url, opts) => {
      if (url.includes("/tags") && opts?.headers?.["If-None-Match"] === '"etag-v312"') {
        return { status: 304, ok: false, headers: new Map([["x-ratelimit-remaining", "58"]]) };
      }
      if (url.includes("/tags")) {
        return {
          status: 200,
          ok: true,
          headers: new Map([["etag", '"etag-v312"'], ["x-ratelimit-remaining", "59"]]),
          json: async () => [{ name: "v3.1.2", commit: { sha: "fea245c000000000000000000000000000000000" } }],
        };
      }
      return {
        status: 200,
        ok: true,
        headers: new Map(),
        json: async () => ({ sha: "fea245c000000000000000000000000000000000", commit: { message: "main head" } }),
      };
    };

    const first = await fetchGitHubCatalog({ fetcher: mockFetch, cacheDir: tmp });
    assert.equal(first.connection_status, "ONLINE_OK");
    assert.equal(first.latest_stable, "v3.1.2");

    const second = await fetchGitHubCatalog({ fetcher: mockFetch, cacheDir: tmp });
    assert.equal(second.connection_status, "ONLINE_ETAG_CACHED");
    assert.equal(second.from_cache, true);

    const offline = await fetchGitHubCatalog({ fetcher: async () => { throw new Error("offline"); } });
    assert.equal(offline.connection_status, "OFFLINE");
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("Loopback server enforces CSRF token, persists prefs/receipts, and locks auto_open_on_boot=false", async () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "msshi-connect-srv-"));
  const app = createConnectServer({ port: 0, dataDir: tmp, fetcher: async () => { throw new Error("offline"); } });
  const port = await app.start();
  try {
    const base = `http://127.0.0.1:${port}`;
    const unauth = await fetch(`${base}/api/health`);
    assert.equal(unauth.status, 403);

    const headers = { "X-MSSHI-Token": app.token, "Content-Type": "application/json" };
    const health = await (await fetch(`${base}/api/health`, { headers })).json();
    assert.equal(health.ok, true);
    assert.equal(health.support_url, CONSTANTS.SAWERIA_URL);

    const savedPrefs = await (await fetch(`${base}/api/prefs`, {
      method: "PUT",
      headers,
      body: JSON.stringify({ auto_upgrade_enabled: false, channel: "stable", selected_ref: "v3.1.2" }),
    })).json();
    assert.equal(savedPrefs.auto_upgrade_enabled, false);

    const applied = await (await fetch(`${base}/api/install/apply`, {
      method: "POST",
      headers,
      body: JSON.stringify({ target_ref: "v3.1.2", msshi_home: tmp, hermes_home: tmp, mode: "muza-preset" }),
    })).json();
    assert.equal(applied.ok, true);
    assert.equal(applied.receipt.state, "PLANNED");
    assert.equal(applied.receipt.can_rollback, false);
    assert.equal(applied.receipt.plan.mode, "muza-preset");
    for (const [confirm, expected] of [["", 400], ["core-v3.2.1", 409]]) {
      const attempt = await fetch(`${base}/api/install/core`, {
        method: "POST", headers, body: JSON.stringify({confirm}),
      });
      assert.equal(attempt.status, expected);
    }
    assert.equal(fs.existsSync(path.join(tmp, "bin")), false);
    assert.ok(applied.receipt.receipt_id.startsWith("connect-"));

    const hub = scanConnectHub(true);
    const hubSaved = await (await fetch(`${base}/api/connect/apply`, {
      method: "POST",
      headers,
      body: JSON.stringify({ ...hub, auto_open_on_boot: true }),
    })).json();
    assert.equal(hubSaved.auto_open_on_boot, false);
  } finally {
    app.server.close();
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("Smart 2-Tier Update Classifier classifies example system patches and optional features according to preferences", () => {
  const paths = [
    "scripts/msshi_cli.py",
    "hermes_plugins/msshi-auto-trim/plugin.py",
    "skills/new-ai-video/SKILL.md",
    "optional/rea/SKILL.md",
  ];
  const smart = classifyMsshiUpdate("v3.2.0", paths, {
    auto_upgrade_enabled: true,
    auto_upgrade_policy: "smart_split",
    skipped_features: [],
  });
  assert.equal(smart.system_auto_apply_silent, true);
  assert.equal(smart.system_patches.length, 2);
  assert.equal(smart.requires_user_feature_prompt, true);
  assert.equal(smart.optional_features.length, 2);

  // If user skips "rea", it is excluded from future prompts while system patches still auto-apply
  const afterSkip = classifyMsshiUpdate("v3.2.0", paths, {
    auto_upgrade_enabled: true,
    auto_upgrade_policy: "smart_split",
    skipped_features: ["rea"],
  });
  assert.equal(afterSkip.system_auto_apply_silent, true);
  assert.equal(afterSkip.optional_features.length, 1);
  assert.equal(afterSkip.optional_features[0].id, "new-ai-video");
});
