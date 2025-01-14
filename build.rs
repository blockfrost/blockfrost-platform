use bzip2::read::BzDecoder;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
use tar::Archive;
use zip::ZipArchive;

fn main() {
    // Tell Cargo to rerun the build script if the build script itself changes.
    println!("cargo:rerun-if-changed=build.rs");

    let testgen_lib_version = "10.1.2.1";

    let target_os = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        panic!("Unsupported OS");
    };

    let arch = if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        panic!("Unsupported architecture");
    };

    let suffix = if target_os == "windows" {
        ".zip"
    } else {
        ".tar.bz2"
    };

    let file_name = format!("testgen-hs-{}-{}-{}", testgen_lib_version, arch, target_os);
    let download_url = format!(
        "https://github.com/input-output-hk/testgen-hs/releases/download/{}/{}{}",
        testgen_lib_version, file_name, suffix
    );

    println!("Fetching {}", file_name);

    // Cache directory for testgen-hs artifacts.
    let cache_dir = dirs::cache_dir()
        .expect("Unable to determine cache directory")
        .join("testgen-hs")
        .join(testgen_lib_version);

    fs::create_dir_all(&cache_dir).expect("Unable to create cache directory");

    let archive_name = if target_os == "windows" {
        format!("{}.zip", file_name)
    } else {
        format!("{}.tar.bz2", file_name)
    };

    let archive_path = cache_dir.join(archive_name);

    // Download the artifact if not already in the cache.
    if !archive_path.exists() {
        println!("Downloading from: {}", download_url);

        let response = reqwest::blocking::get(&download_url)
            .expect("Failed to download archive")
            .bytes()
            .expect("Failed to read archive");
        fs::write(&archive_path, &response).expect("Failed to write archive to disk");

        println!("Downloaded to: {}", archive_path.display());
    } else {
        println!("Using cached archive at: {}", archive_path.display());
    }

    let extract_dir = cache_dir.join("extracted");
    fs::create_dir_all(&extract_dir).expect("Unable to create extraction directory");

    // Extract the artifact if not already extracted.
    let testgen_hs_dir = extract_dir.join("testgen-hs");

    if !testgen_hs_dir.exists() {
        println!("Extracting archive...");
        if target_os == "windows" {
            extract_zip(&archive_path, &extract_dir);
        } else {
            extract_tar_bz2(&archive_path, &extract_dir);
        }
    } else {
        println!("Already extracted at: {}", extract_dir.display());
    }

    // Path to the testgen-hs executable.
    let executable = if target_os == "windows" {
        testgen_hs_dir.join("testgen-hs.exe")
    } else {
        testgen_hs_dir.join("testgen-hs")
    };

    // Verify version by running --version.
    println!("Verifying testgen-hs version...");
    println!("Executing: {:?}", executable);

    let output = Command::new(&executable)
        .arg("--version")
        .output()
        .expect("Failed to execute testgen-hs");

    if !output.status.success() {
        panic!(
            "testgen-hs exited with status {}",
            output.status.code().unwrap_or(-1)
        );
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    println!("testgen-hs version: {}", version_output.trim());

    // Set environment variable for downstream build steps.
    println!(
        "cargo:rustc-env=TESTGEN_HS_PATH={}",
        executable.to_string_lossy()
    );
}

fn extract_tar_bz2(archive_path: &PathBuf, extract_dir: &PathBuf) {
    let tar_bz2 = fs::File::open(&archive_path).expect("Failed to open .tar.bz2 archive");
    let tar = BzDecoder::new(tar_bz2);
    let mut archive = Archive::new(tar);
    archive
        .unpack(extract_dir)
        .expect("Failed to extract .tar.bz2 archive");
}

fn extract_zip(archive_path: &PathBuf, extract_dir: &Path) {
    let file = fs::File::open(archive_path).expect("Failed to open .zip archive");
    let mut archive = ZipArchive::new(file).expect("Failed to read .zip archive");
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("Invalid entry in .zip archive");
        let outpath = match entry.enclosed_name() {
            Some(path) => extract_dir.join(path),
            None => continue,
        };

        if entry.is_dir() {
            fs::create_dir_all(&outpath).expect("Unable to create directory");
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).expect("Unable to create parent directory");
            }

            let mut outfile = fs::File::create(&outpath).expect("Unable to create file");
            std::io::copy(&mut entry, &mut outfile).expect("Unable to write file");
        }
    }
}
