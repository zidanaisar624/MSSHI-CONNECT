// Pure state helpers (tested directly in Node via tests/connect.test.mjs)
export function createInitialState() {
  return {
    probe: null,
    catalog: null,
    github: null,
    hub: null,
    receipts: [],
    selectedPreset: "muza-full",
    prefs: {
      auto_upgrade_enabled: true,
      auto_upgrade_policy: "smart_split",
      channel: "stable",
      selected_ref: "v3.2.1",
      approved_features: [],
      skipped_features: [],
      local_source_path: "",
    },
    setup: {
      mode: "muza-preset",
      platform: "hermes",
      default_bot: "auto",
      light_low_ram: false,
      msshi_home: "D:\\MSSHI",
      hermes_home: "D:\\HermesData",
      copy_pdf_manual: true,
      pdf_version: "latest",
      addons: [],
      context_sources: [],
      connect_mode: "auto-reconnect",
    },
  };
}

export function applyPresetToState(state, preset) {
  if (!preset) return state;
  state.selectedPreset = preset.id;
  state.setup.mode = preset.mode;
  state.setup.default_bot = preset.default_bot;
  state.setup.addons = [...(preset.recommended_addons || [])];
  state.setup.context_sources = [...(preset.recommended_sources || [])];
  return state;
}

export function resolveEffectiveRef(prefs, githubCatalog) {
  if (!prefs.auto_upgrade_enabled) {
    return prefs.selected_ref || "v3.2.1";
  }
  if (prefs.channel === "main") return "main";
  return (githubCatalog && githubCatalog.latest_stable) || prefs.selected_ref || "v3.2.1";
}
