use std::path::Path;
use std::process::Command;

pub fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/package-lock.json");

    // Skip web build if environment variable is set
    if std::env::var("SKIP_WEB_BUILD").is_ok() {
        println!("Skipping web build due to SKIP_WEB_BUILD environment variable");
        return;
    }

    // Check if we're in the web directory or the parent directory
    let web_dir = if Path::new("web").exists() {
        "web"
    } else if Path::new("../web").exists() {
        "../web"
    } else {
        panic!("Could not find web directory");
    };

    // Run npm install to ensure dependencies are up to date
    println!("Running npm install...");
    let install_output = Command::new("npm")
        .args(["install"])
        .current_dir(web_dir)
        .output()
        .expect("Failed to execute npm install");

    if !install_output.status.success() {
        let stderr = String::from_utf8_lossy(&install_output.stderr);
        let stdout = String::from_utf8_lossy(&install_output.stdout);
        panic!("npm install failed:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}");
    }

    // Run npm run build in the web directory
    println!("Running npm run build...");
    let output = Command::new("npm")
        .args(["run", "build"])
        .current_dir(web_dir)
        .output()
        .expect("Failed to execute npm run build");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("npm run build failed:\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}");
    }
}
