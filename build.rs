use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::SystemTime,
};

fn main() {
    println!("cargo:rerun-if-changed=web/ui/package.json");
    println!("cargo:rerun-if-changed=web/ui/package-lock.json");
    println!("cargo:rerun-if-changed=web/ui/src");
    println!("cargo:rerun-if-changed=web/dist/index.html");

    if env::var("SCHEMAUI_WEB_SKIP_BUILD").is_ok() {
        return;
    }

    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let web_ui_dir = manifest_dir.join("web/ui");
    let ts_dir = manifest_dir.join("web/types");
    let _ = fs::create_dir_all(&ts_dir);
    if !web_ui_dir.exists() {
        return;
    }
    let dist_index = manifest_dir.join("web/dist/index.html");

    if !should_build(&web_ui_dir, &dist_index) {
        return;
    }

    if !command_exists("npm") {
        println!(
            "cargo:warning=Skipping web/ui build because npm is not available. Set SCHEMAUI_WEB_SKIP_BUILD=1 to silence this warning."
        );
        return;
    }

    ensure_node_modules(&web_ui_dir);

    println!("cargo:warning=Building web UI bundle via npm run build…");
    let status = Command::new("npm")
        .arg("run")
        .arg("build")
        .current_dir(&web_ui_dir)
        .status()
        .expect("failed to invoke npm");

    if !status.success() {
        panic!("npm run build (web/ui) failed with status {status}");
    }
}

fn should_build(web_ui_dir: &Path, dist_index: &Path) -> bool {
    if env::var("SCHEMAUI_WEB_FORCE_BUILD").is_ok() {
        return true;
    }
    if !dist_index.exists() {
        return true;
    }

    let dist_time = modified_time(dist_index);
    let src_time = modified_time(&web_ui_dir.join("src"));
    let config_time = modified_time(&web_ui_dir.join("package.json"));

    match (dist_time, src_time.max(config_time)) {
        (Some(dist), Some(src)) => src > dist,
        _ => true,
    }
}

fn modified_time(path: &Path) -> Option<SystemTime> {
    if path.is_file() {
        return path.metadata().ok().and_then(|m| m.modified().ok());
    }
    if path.is_dir() {
        let mut latest = None;
        for entry in fs::read_dir(path).ok()? {
            let entry = entry.ok()?;
            let time = modified_time(&entry.path())?;
            if latest.is_none_or(|current| time > current) {
                latest = Some(time);
            }
        }
        return latest;
    }
    None
}

fn command_exists(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn ensure_node_modules(web_ui_dir: &Path) {
    let node_modules = web_ui_dir.join("node_modules");
    let package_lock = web_ui_dir.join("package-lock.json");

    let lock_time = package_lock.metadata().ok().and_then(|m| m.modified().ok());
    let modules_time = node_modules.metadata().ok().and_then(|m| m.modified().ok());

    let needs_install = !node_modules.exists()
        || lock_time
            .zip(modules_time)
            .is_some_and(|(lock, modules)| lock > modules);

    if needs_install {
        println!("cargo:warning=Installing web UI dependencies via npm ci…");
        let status = Command::new("npm")
            .arg("ci")
            .current_dir(web_ui_dir)
            .status()
            .expect("failed to invoke npm");
        if !status.success() {
            panic!("npm ci (web/ui) failed with status {status}");
        }
    }
}
