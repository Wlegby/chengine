use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let archive_dir = Path::new("engines_archive");
    let target_binary = Path::new("target/release/chengine"); // <- CHANGE TO YOUR ACTUAL BINARY NAME
                                                              //
                                                              //
    let args: Vec<String> = std::env::args().collect();
    let create_new_version = args.iter().any(|arg| arg == "--new");

    // 1. Ensure the archive directory exists
    if !archive_dir.exists() {
        fs::create_dir_all(archive_dir).expect("Failed to create archive directory");
    }

    // 2. Scan folder to determine the next version number
    let mut max_version = 0;
    if let Ok(entries) = fs::read_dir(archive_dir) {
        for entry in entries.flatten() {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.starts_with("chengine-v") {
                    if let Ok(num) = file_name["chengine-v".len()..].parse::<u32>() {
                        if num > max_version {
                            max_version = num;
                        }
                    }
                }
            }
        }
    }

    let next_version = max_version + 1;
    let new_version_name = format!(
        "chengine-v{}",
        if create_new_version {
            next_version
        } else {
            max_version
        }
    );
    let destination = archive_dir.join(&new_version_name);

    // 3. Compile your engine via standard cargo build --release
    println!("⚙️ Compiling chess engine in release mode...");
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()
        .expect("Failed to run cargo build");

    if !status.success() {
        eprintln!("❌ Compilation failed. Aborting archive.");
        std::process::exit(1);
    }

    // 4. Safely copy the newly built binary to engines_archive/version_x
    if target_binary.exists() {
        fs::copy(target_binary, &destination).expect("Failed to copy binary to archive");
        println!("✅ Successfully saved engine to: {}", destination.display());
    } else {
        eprintln!("❌ Could not find compiled binary at {:?}", target_binary);
        eprintln!(
            "💡 Check that 'your_engine_binary_name' matches the binary name in your Cargo.toml"
        );
    }
}
