use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn main() {
    // Generate git version info using vergen
    // This must run before any other build steps to ensure version is available
    generate_version_info();

    println!("cargo:rerun-if-changed=migrations");
    println!("cargo:rerun-if-changed=web/src");
    println!("cargo:rerun-if-changed=web/package.json");
    println!("cargo:rerun-if-changed=web/package-lock.json");

    // Configure static linking for musl targets (used by cross for static builds)
    let target = env::var("TARGET").unwrap_or_default();
    if target.contains("musl") {
        configure_musl_static_linking();
    }

    // Always ensure web/build directory exists for include_dir! macro
    // This is required even in dev mode because include_dir! runs at compile time
    let web_build_path = Path::new("web/build");
    if !web_build_path.exists() {
        fs::create_dir_all(web_build_path)
            .expect("Failed to create web/build directory for include_dir! macro");
        println!("Created empty web/build directory for include_dir! macro");
    }

    // Skip web build if environment variable is set
    if std::env::var("SKIP_WEB_BUILD").is_ok() {
        println!("Skipping web build due to SKIP_WEB_BUILD environment variable");
        return;
    }

    // Skip web build in development mode (non-release builds)
    // In dev mode, frontend runs on http://localhost:5173 via Vite dev server
    // Backend runs on http://localhost:1337
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "debug".to_string());
    if profile != "release" {
        println!(
            "Skipping web build in development mode (profile: {})",
            profile
        );
        println!("Frontend should be accessed at http://localhost:5173");
        println!("Backend will run on http://localhost:1337");
        return;
    }

    println!("Building frontend for release...");

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

/// Configure static linking for musl targets
///
/// For musl targets, we build libpq from source (bundled mode) to get all the necessary
/// static libraries including libpgcommon.a and libpgport.a which aren't available in
/// system packages. This is enabled via the bundled-postgres feature.
fn configure_musl_static_linking() {
    println!("cargo:warning=Configuring bundled PostgreSQL build for musl static linking");

    // Check if bundled-postgres feature is enabled
    #[cfg(not(feature = "bundled-postgres"))]
    {
        println!("cargo:warning=bundled-postgres feature not enabled - build may fail!");
        println!("cargo:warning=Use: cargo build --features bundled-postgres");
    }
}

/// Generate version information from git tags using vergen
///
/// This function uses vergen-git2 to generate build-time constants from git metadata.
/// The version is derived from `git describe --tags --always --dirty`, which provides:
/// - For tagged commits: the tag name (e.g., "v0.1.4")
/// - For commits after a tag: tag + commits + hash (e.g., "v0.1.4-2-ge930185")
/// - For dirty working trees: appends "-dirty" (e.g., "v0.1.4-dirty")
/// - For non-git environments: falls back to "0.0.0-dev"
///
/// The generated constants can be accessed via:
/// - `env!("VERGEN_GIT_DESCRIBE")` - Full version with git metadata
/// - `env!("VERGEN_GIT_SHA")` - Commit SHA
fn generate_version_info() {
    use vergen_git2::{BuildBuilder, CargoBuilder, Emitter, Git2Builder};

    let build = BuildBuilder::default()
        .build_timestamp(true)
        .build()
        .expect("Failed to configure build info");

    let cargo = CargoBuilder::default()
        .target_triple(true)
        .build()
        .expect("Failed to configure cargo info");

    let git2 = Git2Builder::default()
        .describe(true, true, None) // Enable describe with dirty flag, no pattern match
        .sha(true) // Include commit SHA
        .build()
        .expect("Failed to configure git info");

    Emitter::default()
        .add_instructions(&build)
        .expect("Failed to add build instructions")
        .add_instructions(&cargo)
        .expect("Failed to add cargo instructions")
        .add_instructions(&git2)
        .expect("Failed to add git instructions")
        .emit()
        .expect("Failed to emit version info");

    println!("cargo:warning=Version info generated from git");
}
