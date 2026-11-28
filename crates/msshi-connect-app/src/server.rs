use msshi_connect_core::{
    connect_hub::{save_connect_hub_prefs, scan_connect_hub, ConnectHubState},
    constants::{APP_NAME, APP_VERSION, SAWERIA_URL},
    github_client::GitHubClient,
    install_runner::{build_dry_run, list_receipts, write_receipt, InstallPlanRequest},
    option_catalog::default_catalog,
    safety::generate_csprng_token_hex,
    sys_probe::{collect_system_probe, detect_doctor_path, resolve_default_msshi_home},
    version_manager::{classify_msshi_update, load_prefs, save_prefs, ConnectPrefs},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

#[derive(Debug, Default, Deserialize)]
struct ClassifyRequestBody {
    target_ref: Option<String>,
    changed_paths: Option<Vec<String>>,
}

fn embedded_asset(rel: &str) -> Option<&'static [u8]> {
    match rel {
        "ui/index.html" => Some(include_bytes!("../../../ui/index.html")),
        "ui/styles/cyber-noir.css" => Some(include_bytes!("../../../ui/styles/cyber-noir.css")),
        "ui/js/api.js" => Some(include_bytes!("../../../ui/js/api.js")),
        "ui/js/state.js" => Some(include_bytes!("../../../ui/js/state.js")),
        "ui/js/app.js" => Some(include_bytes!("../../../ui/js/app.js")),
        "assets/msshi-connect-banner-ui.png" => Some(include_bytes!("../../../assets/msshi-connect-banner-ui.png")),
        "assets/msshi-connect-icon-64.png" => Some(include_bytes!("../../../assets/msshi-connect-icon-64.png")),
        _ => None,
    }
}

fn json_response(status: u16, body: &impl serde::Serialize) -> Response<std::io::Cursor<Vec<u8>>> {
    let raw = serde_json::to_vec(body).unwrap_or_else(|_| b"{}".to_vec());
    let header = Header::from_bytes("Content-Type", "application/json; charset=utf-8").unwrap();
    Response::from_data(raw)
        .with_status_code(StatusCode(status))
        .with_header(header)
}

pub fn embedded_interface_ok() -> bool {
    ["ui/index.html","ui/styles/cyber-noir.css","ui/js/app.js","ui/js/state.js",
     "ui/js/api.js","assets/msshi-connect-icon-64.png","assets/msshi-connect-banner-ui.png"]
        .iter().all(|key| embedded_asset(key).is_some_and(|data| !data.is_empty()))
}

fn static_file_response(rel: &str, mime: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    if let Some(asset) = embedded_asset(rel) {
        return Response::from_data(asset.to_vec()).with_header(Header::from_bytes("Content-Type", mime).unwrap());
    }
    Response::from_string("Not Found").with_status_code(StatusCode(404))
}

fn validate_security(req: &Request, expected_host: &str, token: &str) -> bool {
    let host_ok = req
        .headers()
        .iter()
        .find(|h| h.field.equiv("Host"))
        .map(|h| h.value.as_str().eq_ignore_ascii_case(expected_host))
        .unwrap_or(false);
    if !host_ok {
        return false;
    }
    if let Some(origin) = req.headers().iter().find(|h| h.field.equiv("Origin")) {
        let allowed = format!("http://{expected_host}");
        if !origin.value.as_str().eq_ignore_ascii_case(&allowed) {
            return false;
        }
    }
    req.headers()
        .iter()
        .find(|h| h.field.equiv("X-MSSHI-Token"))
        .map(|h| h.value.as_str() == token)
        .unwrap_or(false)
}

fn launch_desktop_window(url: &str) {
    if cfg!(windows) {
        let edge_candidates = [
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
        ];
        for edge in edge_candidates {
            if Path::new(edge).is_file() {
                let _ = Command::new(edge)
                    .arg(format!("--app={url}"))
                    .arg("--window-size=1220,780")
                    .spawn();
                return;
            }
        }
        let _ = Command::new("cmd").args(["/C", "start", "", url]).spawn();
    }
}

pub fn run_gui_server(port: u16, cache_dir: PathBuf) -> Result<(), String> {
    let bind_addr = format!("127.0.0.1:{port}");
    let server = Server::http(&bind_addr).map_err(|e| format!("Gagal bind {bind_addr}: {e}"))?;
    let actual_port = server.server_addr().to_ip().map(|a| a.port()).unwrap_or(port);
    let expected_host = format!("127.0.0.1:{actual_port}");
    let token = generate_csprng_token_hex();
    let url = format!("http://{expected_host}/#t={token}");

    println!("{APP_NAME} v{APP_VERSION} berjalan di http://{expected_host}/ (Loopback Aman)");
    launch_desktop_window(&url);

    let github = GitHubClient::new(cache_dir);

    for mut req in server.incoming_requests() {
        let path = req.url().split('?').next().unwrap_or("/").to_string();
        let method = req.method().clone();

        if !path.starts_with("/api/") {
            let resp = match path.as_str() {
                "/" | "/index.html" => static_file_response("ui/index.html", "text/html; charset=utf-8"),
                "/styles/cyber-noir.css" => static_file_response("ui/styles/cyber-noir.css", "text/css; charset=utf-8"),
                "/js/api.js" => static_file_response("ui/js/api.js", "application/javascript; charset=utf-8"),
                "/js/state.js" => static_file_response("ui/js/state.js", "application/javascript; charset=utf-8"),
                "/js/app.js" => static_file_response("ui/js/app.js", "application/javascript; charset=utf-8"),
                "/assets/msshi-connect-banner-ui.png" => static_file_response("assets/msshi-connect-banner-ui.png", "image/png"),
                "/assets/msshi-connect-icon-64.png" => static_file_response("assets/msshi-connect-icon-64.png", "image/png"),
                _ => Response::from_string("Not Found").with_status_code(StatusCode(404)),
            };
            let _ = req.respond(resp);
            continue;
        }

        if !validate_security(&req, &expected_host, &token) {
            let _ = req.respond(json_response(403, &serde_json::json!({"error": "Token/Host CSRF tidak valid"})));
            continue;
        }

        let msshi_home = resolve_default_msshi_home();

        let resp = match (method, path.as_str()) {
            (Method::Get, "/api/health") => json_response(200, &serde_json::json!({
                "ok": true, "app": APP_NAME, "version": APP_VERSION, "engine": "rust", "support_url": SAWERIA_URL
            })),
            (Method::Get, "/api/system/probe") => json_response(200, &collect_system_probe(None, None)),
            (Method::Get, "/api/catalog") => json_response(200, &default_catalog()),
            (Method::Get, "/api/github/releases") => json_response(200, &github.fetch_catalog()),
            (Method::Get, "/api/prefs") => json_response(200, &load_prefs(&msshi_home)),
            (Method::Post, "/api/update/classify") => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                let parsed: ClassifyRequestBody = serde_json::from_str(&body).unwrap_or_default();
                let prefs = load_prefs(&msshi_home);
                let target_ref = parsed.target_ref.unwrap_or_else(|| prefs.selected_ref.clone());
                let sample = parsed.changed_paths.filter(|v| !v.is_empty()).unwrap_or_else(|| vec![
                    "scripts/msshi_cli.py".to_string(),
                    "hermes_plugins/msshi-auto-trim/plugin.py".to_string(),
                    "skills/open-code-review/SKILL.md".to_string(),
                    "optional/mcp-for-blender/SKILL.md".to_string(),
                    "optional/raea/SKILL.md".to_string(),
                ]);
                let classified = classify_msshi_update(&target_ref, &sample, &prefs);
                json_response(200, &classified)
            }
            (Method::Put, "/api/prefs") => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<ConnectPrefs>(&body).and_then(|p| Ok(save_prefs(&msshi_home, &p))) {
                    Ok(Ok(saved)) => json_response(200, &saved),
                    Ok(Err(e)) => json_response(400, &serde_json::json!({"error": e})),
                    Err(e) => json_response(400, &serde_json::json!({"error": e.to_string()})),
                }
            }
            (Method::Post, "/api/install/dry-run") => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<InstallPlanRequest>(&body) {
                    Ok(plan) => json_response(200, &build_dry_run(&plan)),
                    Err(e) => json_response(400, &serde_json::json!({"error": e.to_string()})),
                }
            }
            (Method::Post, "/api/install/apply") => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<InstallPlanRequest>(&body) {
                    Ok(plan) => {
                        let dry = build_dry_run(&plan);
                        match write_receipt(&msshi_home, &plan, "PLANNED") {
                            Ok(receipt) => json_response(200, &serde_json::json!({"ok": true, "dry_run": dry, "receipt": receipt})),
                            Err(e) => json_response(500, &serde_json::json!({"error": e})),
                        }
                    }
                    Err(e) => json_response(400, &serde_json::json!({"error": e.to_string()})),
                }
            }
            (Method::Post, "/api/install/core") => {
                let consent = serde_json::from_reader::<_, serde_json::Value>(std::io::Read::take(req.as_reader(), 1024));
                if !consent.ok().is_some_and(|v| v.get("confirm").and_then(|s| s.as_str()) == Some("core-v3.2.1")) {
                    json_response(400, &serde_json::json!({"error": "Konfirmasi pemasangan Core v3.2.1 dahulu."}))
                } else {
                    match install_official_core(&msshi_home) {
                        Ok(()) => json_response(200, &serde_json::json!({"status":"INSTALLED", "ref":"v3.2.1","msshi_home":msshi_home})),
                        Err(message) => json_response(400, &serde_json::json!({"error":message})),
                    }
                }
            }
            (Method::Get, "/api/receipts") => json_response(200, &list_receipts(&msshi_home)),
            (Method::Get, "/api/connect/scan") => {
                let probe = collect_system_probe(None, None);
                json_response(200, &scan_connect_hub(probe.recommend_light_mode))
            }
            (Method::Post, "/api/connect/apply") => {
                let mut body = String::new();
                let _ = req.as_reader().read_to_string(&mut body);
                match serde_json::from_str::<ConnectHubState>(&body) {
                    Ok(st) => match save_connect_hub_prefs(&msshi_home, st) {
                        Ok(saved) => json_response(200, &saved),
                        Err(e) => json_response(500, &serde_json::json!({"error": e})),
                    },
                    Err(e) => json_response(400, &serde_json::json!({"error": e.to_string()})),
                }
            }
            (Method::Post, "/api/launch/doctor") => {
                if let Some(doc) = detect_doctor_path() {
                    let _ = Command::new(&doc).spawn();
                    json_response(200, &serde_json::json!({"ok": true, "launched": doc.display().to_string()}))
                } else {
                    json_response(404, &serde_json::json!({"ok": false, "error": "MSSHI Doctor belum terdeteksi di D:\\MSSHI DOCTOR"}))
                }
            }
            _ => json_response(404, &serde_json::json!({"error": "Endpoint tidak ditemukan"})),
        };
        let _ = req.respond(resp);
    }
    Ok(())
}

fn install_official_core(home: &Path) -> Result<(), String> {
    if !cfg!(windows) { return Err("Pemasang ini memerlukan Windows.".into()); }
    let windir = std::env::var_os("SystemRoot").ok_or("Folder sistem Windows belum tersedia.")?;
    let powershell = PathBuf::from(windir).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let script = include_str!("../../../tools/install-official-core.ps1");
    // Windows PowerShell accepts UTF-16LE here, preserving embedded quotation marks.
    let bytes: Vec<u8> = script.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let encoded = STANDARD.encode(bytes);
    let result = Command::new(powershell)
        .args(["-NoLogo","-NoProfile","-NonInteractive","-EncodedCommand",&encoded])
        .env("MSSHI_HOME",home)
        .status().map_err(|_| "Windows PowerShell tidak berhasil dibuka.")?;
    match result.code() {
        Some(0) => Ok(()),
        Some(2) => Err("Pemasangan dibatalkan di dialog Windows.".into()),
        _ => Err("Pemasangan gagal. Periksa Git, Python 3.10+, jaringan, akses folder, dan pesan pada jendela Connect.".into()),
    }
}
