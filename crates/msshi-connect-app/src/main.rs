mod server;

use msshi_connect_core::{
    connect_hub::scan_connect_hub,
    constants::{APP_NAME, APP_VERSION, SAWERIA_URL, UPSTREAM_FULL_NAME},
    github_client::GitHubClient,
    install_runner::{build_dry_run, list_receipts, InstallPlanRequest},
    option_catalog::default_catalog,
    sys_probe::collect_system_probe,
    version_manager::load_prefs,
};
use std::{env, path::PathBuf, process};

const HELP: &str = "MSSHI Installer & Connect v1.0.2 (Rust + Vanilla JS)\n\
Pemakaian:\n\
  msshi-connect --gui [--port PORT]         Buka antarmuka desktop Cyber-Noir 5-Tab\n\
  msshi-connect --probe [--json]            Cek perangkat, Hermes, RAM, Drive & MSSHI Doctor\n\
  msshi-connect --check-updates [--json]    Cek rilis & tag GitHub (ETag cache)\n\
  msshi-connect --connect-scan [--json]     Pindai aplikasi untuk Connect Hub (0 LLM call)\n\
  msshi-connect --receipts [--json]         Daftar rencana yang telah disimpan\n\
  msshi-connect --version                   Tampilkan versi aplikasi\n";

fn default_cache_dir() -> PathBuf {
    env::var_os("LOCALAPPDATA")
        .map(|base| PathBuf::from(base).join("MSSHI-Connect").join("cache"))
        .unwrap_or_else(|| env::temp_dir().join("msshi-connect-cache"))
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return;
    }
    if args.iter().any(|a| a == "--version") {
        println!("{APP_NAME} v{APP_VERSION} ({UPSTREAM_FULL_NAME}) | Support: {SAWERIA_URL}");
        return;
    }

    if args.iter().any(|a| a == "--self-check") {
        if !server::embedded_interface_ok() { process::exit(1); }
        println!("OK");
        return;
    }
    let as_json = args.iter().any(|a| a == "--json");
    let probe = collect_system_probe(None, None);
    let msshi_home = PathBuf::from(&probe.msshi_home);

    if args.iter().any(|a| a == "--probe") {
        if as_json {
            println!("{}", serde_json::to_string_pretty(&probe).unwrap());
        } else {
            println!("{APP_NAME} v{APP_VERSION}");
            println!("OS / RAM   : {} | {:.1}/{:.1} GB", probe.os, probe.available_ram_gb, probe.total_ram_gb);
            println!("Hermes     : {} ({})", if probe.hermes_detected { "TERDETEKSI" } else { "BELUM TERPASANG" }, probe.hermes_home);
            println!("MSSHI Home : {} (v={:?})", probe.msshi_home, probe.installed_msshi_version);
            println!("Admin Lock : {}", if probe.elevated_admin { "PERINGATAN ADMIN" } else { "Normal User (AMAN)" });
            println!("Doctor App : {}", probe.doctor_path.as_deref().unwrap_or("Belum terdeteksi"));
        }
        return;
    }

    if args.iter().any(|a| a == "--check-updates") {
        let client = GitHubClient::new(default_cache_dir());
        let catalog = client.fetch_catalog();
        if as_json {
            println!("{}", serde_json::to_string_pretty(&catalog).unwrap());
        } else {
            println!("Repo    : {}", catalog.repo);
            println!("Status  : {} (cache={})", catalog.connection_status, catalog.from_cache);
            println!("Latest  : {}", catalog.latest_stable);
            println!("Main SHA: {}", catalog.main_commit_sha.as_deref().unwrap_or("-"));
            println!("Refs    : {} item", catalog.refs.len());
        }
        return;
    }

    if args.iter().any(|a| a == "--connect-scan") {
        let hub = scan_connect_hub(probe.recommend_light_mode);
        println!("{}", serde_json::to_string_pretty(&hub).unwrap());
        return;
    }

    if args.iter().any(|a| a == "--receipts") {
        let receipts = list_receipts(&msshi_home);
        println!("{}", serde_json::to_string_pretty(&receipts).unwrap());
        return;
    }

    if args.iter().any(|a| a == "--dry-run") {
        let prefs = load_prefs(&msshi_home);
        let dry = build_dry_run(&InstallPlanRequest {
            target_ref: prefs.selected_ref,
            commit_sha: None,
            msshi_home: probe.msshi_home,
            hermes_home: probe.hermes_home,
            mode: "muza-preset".into(),
            platform: "hermes".into(),
            default_bot: "auto".into(),
            light_low_ram: probe.recommend_light_mode,
            copy_pdf_manual: true,
            pdf_version: "latest".into(),
            addons: vec![],
            context_sources: vec![],
            connect_mode: "auto-reconnect".into(),
            local_source_path: prefs.local_source_path,
        });
        println!("{}", serde_json::to_string_pretty(&dry).unwrap());
        return;
    }

    if args.iter().any(|a| a == "--catalog") {
        println!("{}", serde_json::to_string_pretty(&default_catalog()).unwrap());
        return;
    }

    // Default or --gui: launch secure loopback UI server + Edge --app / browser window
    let port: u16 = args
        .windows(2)
        .find(|w| w[0] == "--port")
        .and_then(|w| w[1].parse().ok())
        .unwrap_or(0);
    if let Err(err) = server::run_gui_server(port, default_cache_dir()) {
        eprintln!("[ERROR] {err}");
        process::exit(1);
    }
}
