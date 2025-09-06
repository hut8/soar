use std::process::Command;
use std::path::Path;

pub fn main() {
    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/package-lock.json");

    // Check if we're in the web directory or the parent directory
    let web_dir = if Path::new("web").exists() {
        "web"
    } else if Path::new("../web").exists() {
        "../web"
    } else {
        panic!("Could not find web directory");
    };

    // Run npm run build in the web directory
    println!("cargo:warning=Running npm run build in {}", web_dir);
    let output = Command::new("npm")
        .args(["run", "build"])
        .current_dir(web_dir)
        .output()
        .expect("Failed to execute npm run build");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        panic!("npm run build failed:\nSTDOUT:\n{}\nSTDERR:\n{}", stdout, stderr);
    }

    println!("cargo:warning=npm run build completed successfully");
}
