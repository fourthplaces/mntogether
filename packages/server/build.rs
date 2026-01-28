use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    // Set up rebuild triggers for admin-spa
    println!("cargo:rerun-if-changed=../admin-spa/src");
    println!("cargo:rerun-if-changed=../admin-spa/package.json");
    println!("cargo:rerun-if-changed=../admin-spa/vite.config.ts");
    println!("cargo:rerun-if-changed=../admin-spa/index.html");

    // Set up rebuild triggers for web-app
    println!("cargo:rerun-if-changed=../web-app/src");
    println!("cargo:rerun-if-changed=../web-app/package.json");
    println!("cargo:rerun-if-changed=../web-app/vite.config.ts");
    println!("cargo:rerun-if-changed=../web-app/index.html");

    // Skip builds if flag is set
    if env::var("SKIP_FRONTEND_BUILD").is_ok() {
        println!("cargo:warning=Skipping frontend builds (SKIP_FRONTEND_BUILD set)");
        return;
    }

    // Get the workspace root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let server_dir = Path::new(&manifest_dir);
    let packages_dir = server_dir.parent().unwrap();

    // Check package manager availability
    let package_manager = detect_package_manager();
    if package_manager.is_none() {
        println!("cargo:warning=Neither yarn nor npm found - frontend apps will not be built");
        println!("cargo:warning=Install Node.js and yarn/npm to enable embedded frontends");
        return;
    }
    let package_manager = package_manager.unwrap();

    // Build admin-spa
    build_spa(&packages_dir.join("admin-spa"), "admin-spa", &package_manager);

    // Build web-app
    build_spa(&packages_dir.join("web-app"), "web-app", &package_manager);
}

fn detect_package_manager() -> Option<&'static str> {
    let yarn_check = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "yarn --version"])
            .output()
    } else {
        Command::new("sh")
            .args(&["-c", "yarn --version"])
            .output()
    };

    if yarn_check.is_ok() {
        return Some("yarn");
    }

    let npm_check = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", "npm --version"])
            .output()
    } else {
        Command::new("sh")
            .args(&["-c", "npm --version"])
            .output()
    };

    if npm_check.is_ok() {
        Some("npm")
    } else {
        None
    }
}

fn build_spa(spa_dir: &PathBuf, name: &str, package_manager: &str) {
    println!("cargo:warning=Building {} at {:?}", name, spa_dir);

    // Check if node_modules exists, install if not
    let node_modules = spa_dir.join("node_modules");
    if !node_modules.exists() {
        println!("cargo:warning=Installing {} dependencies...", name);
        let install_args = vec!["install"];

        let install = Command::new(package_manager)
            .args(&install_args)
            .current_dir(spa_dir)
            .status();

        if let Err(e) = install {
            println!("cargo:warning=Failed to install {} dependencies: {}", name, e);
            return;
        }
    }

    // Build the SPA
    println!("cargo:warning=Building {}...", name);
    let build_args = if package_manager == "yarn" {
        vec!["build"]
    } else {
        vec!["run", "build"]
    };

    let build = Command::new(package_manager)
        .args(&build_args)
        .current_dir(spa_dir)
        .status();

    match build {
        Ok(status) if status.success() => {
            println!("cargo:warning={} built successfully", name);
        }
        Ok(status) => {
            println!("cargo:warning={} build failed with status: {}", name, status);
        }
        Err(e) => {
            println!("cargo:warning=Failed to build {}: {}", name, e);
        }
    }
}
