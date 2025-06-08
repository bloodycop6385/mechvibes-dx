use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

// Build-time path constants (must be kept in sync with src/state/paths.rs)
const DATA_DIR: &str = "data";
const MANIFEST_JSON: &str = "data/manifest.json";
const CONFIG_JSON: &str = "./data/config.json";
const SOUNDPACK_CACHE_JSON: &str = "data/soundpack_cache.json";
const SOUNDPACKS_DIR: &str = "./soundpacks";

fn main() {
    println!("cargo:rerun-if-changed=app.config.json");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=assets/icon.ico"); // Set Windows app icon and metadata
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;
        if Path::new("assets/icon.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/icon.ico");

            // Set version information
            res.set("CompanyName", "Hải Nguyễn");
            res.set(
                "FileDescription",
                "MechVibes DX - Enhanced mechanical keyboard sound simulator"
            );
            res.set("LegalCopyright", "Copyright © 2025 Hải Nguyễn");
            res.set("ProductName", "MechVibes DX");
            res.set("ProductVersion", "0.1.0");
            res.set("FileVersion", "0.1.0"); // Check build profile and environment variables for admin privileges requirement
            let force_admin =
                env::var("MECHVIBES_FORCE_ADMIN").unwrap_or_default().to_lowercase() == "true";
            let skip_admin =
                env::var("MECHVIBES_NO_ADMIN").unwrap_or_default().to_lowercase() == "true";

            // Default behavior (CHANGED for better UX):
            // - All builds: Standard user privileges by default (no UAC prompts)
            // - Admin mode only when explicitly requested via MECHVIBES_FORCE_ADMIN=true
            // This provides better user experience while keeping admin functionality available
            let request_admin = if skip_admin {
                false // Explicitly skip admin privileges
            } else if force_admin {
                true // Explicitly request admin privileges
            } else {
                false // Default: standard user privileges for all builds
            };
            if request_admin {
                println!("🔐 Building with administrator privileges manifest");
                println!("   ↳ Admin privileges enabled via MECHVIBES_FORCE_ADMIN=true");
                println!("   ↳ This will require UAC prompt on every startup");
                res.set_manifest(create_admin_manifest());
            } else {
                println!("👤 Building with standard user privileges manifest");
                if skip_admin {
                    println!("   ↳ Admin privileges explicitly disabled (MECHVIBES_NO_ADMIN=true)");
                } else {
                    println!("   ↳ Default build - no UAC prompts, good user experience");
                    println!("   ↳ Use MECHVIBES_FORCE_ADMIN=true for admin mode if needed");
                }
                res.set_manifest(create_standard_manifest());
            }

            if let Err(e) = res.compile() {
                eprintln!("Warning: Failed to compile Windows resources: {}", e);
            } else {
                println!("✅ Windows resources compiled successfully with UAC manifest");
            }
        } else {
            eprintln!("Warning: assets/icon.ico not found, skipping Windows resource compilation");
        }
    }

    // Only generate manifest for release builds
    if env::var("PROFILE").unwrap_or_default() == "release" {
        generate_manifest_for_production();
    }

    // Set git information if available
    if let Ok(output) = Command::new("git").args(&["rev-parse", "HEAD"]).output() {
        if output.status.success() {
            let git_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_HASH={}", git_hash);
        }
    }

    if let Ok(output) = Command::new("git").args(&["rev-parse", "--abbrev-ref", "HEAD"]).output() {
        if output.status.success() {
            let git_branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:rustc-env=GIT_BRANCH={}", git_branch);
        }
    }
}

fn generate_manifest_for_production() {
    println!("🏗️  Generating production manifest...");

    // Create data directory if it doesn't exist
    if !Path::new("data").exists() {
        let _ = fs::create_dir_all("data");
    }

    // Read app.config.json
    let config_content = match fs::read_to_string("app.config.json") {
        Ok(content) => content,
        Err(_) => {
            eprintln!("❌ app.config.json not found! Creating default...");
            create_default_config();
            fs::read_to_string("app.config.json").expect("Failed to read created config")
        }
    };

    // Parse config
    let config: serde_json::Value = serde_json
        ::from_str(&config_content)
        .expect("Failed to parse app.config.json");

    // Create manifest with build information
    let manifest =
        serde_json::json!({
        "app": {
            "name": config["app"]["name"],
            "version": config["app"]["version"],
            "description": config["app"]["description"],
            "build_date": chrono::Utc::now().to_rfc3339(),
            "git_commit": env::var("GIT_HASH").ok(),
            "git_branch": env::var("GIT_BRANCH").unwrap_or_else(|_| "main".to_string()),
            "build_type": "release"
        },
        "compatibility": config["compatibility"],
        "paths": config["paths"],
        "metadata": {
            "created_at": chrono::Utc::now().to_rfc3339(),
            "last_updated": chrono::Utc::now().to_rfc3339(),
            "platform": get_target_platform()
        }
    });
    // Write manifest
    let manifest_content = serde_json
        ::to_string_pretty(&manifest)
        .expect("Failed to serialize manifest");

    fs::write(MANIFEST_JSON, manifest_content).expect("Failed to write manifest file");

    println!("✅ Production manifest generated");
}

fn create_default_config() {
    let default_config =
        serde_json::json!({
        "app": {
            "name": "MechVibes DX",
            "version": "0.1.0",
            "description": "Enhanced mechanical keyboard sound simulator"
        },
        "compatibility": {
            "config_version": "1.0",
            "soundpack_version": "1.0",
            "cache_version": "1.0",
            "minimum_app_version": "0.1.0"        },        "paths": {
            "config_file": CONFIG_JSON,
            "soundpack_cache": SOUNDPACK_CACHE_JSON,
            "soundpacks_dir": SOUNDPACKS_DIR,
            "data_dir": DATA_DIR
        }
    });

    let config_content = serde_json
        ::to_string_pretty(&default_config)
        .expect("Failed to serialize default config");

    fs::write("app.config.json", config_content).expect("Failed to write default config");
}

fn get_target_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "unknown"
    }
}

fn create_admin_manifest() -> &'static str {
    r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.0.0.0"
    processorArchitecture="*"
    name="MechVibesDX"
    type="win32"
  />
  <description>MechVibes DX - Enhanced mechanical keyboard sound simulator (Administrator Mode)</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges xmlns="urn:schemas-microsoft-com:asm.v3">
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <!-- Windows 8 -->
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <!-- Windows 7 -->
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
</assembly>
"#
}

fn create_standard_manifest() -> &'static str {
    r#"
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.0.0.0"
    processorArchitecture="*"
    name="MechVibesDX"
    type="win32"
  />
  <description>MechVibes DX - Enhanced mechanical keyboard sound simulator</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges xmlns="urn:schemas-microsoft-com:asm.v3">
        <requestedExecutionLevel level="asInvoker" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
      <!-- Windows 8.1 -->
      <supportedOS Id="{1f676c76-80e1-4239-95bb-83d0f6d0da78}"/>
      <!-- Windows 8 -->
      <supportedOS Id="{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}"/>
      <!-- Windows 7 -->
      <supportedOS Id="{35138b9a-5d96-4fbd-8e2d-a2440225f93a}"/>
    </application>
  </compatibility>
</assembly>
"#
}
