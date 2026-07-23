use bzip2::read::BzDecoder;
use fs2::FileExt;
use std::{
    env,
    fs::{
        File, OpenOptions, create_dir_all, read_to_string, remove_dir_all, rename,
        write as fs_write,
    },
    path::Path,
    process::Command,
};
use tar::Archive;
use zip::ZipArchive;

const TESTGEN_HS_PATH: &str = "TESTGEN_HS_PATH";
const TESTGEN_HS_VERSION: &str = "11.0.1.0";

pub fn ensure() {
    println!("cargo:rerun-if-env-changed={TESTGEN_HS_PATH}");

    if env::var(TESTGEN_HS_PATH).is_ok() {
        println!("Environment variable {TESTGEN_HS_PATH} is set. Skipping the download.");
        return;
    }

    let target_os = super::target::os();
    let arch = super::target::arch();

    let suffix = if target_os == "windows" {
        ".zip"
    } else {
        ".tar.bz2"
    };

    let file_name = format!("testgen-hs-{TESTGEN_HS_VERSION}-{arch}-{target_os}");
    let download_url = format!(
        "https://github.com/blockfrost/testgen-hs/releases/download/{TESTGEN_HS_VERSION}/{file_name}{suffix}"
    );

    println!("Looking for {file_name}");

    let profile_dir = super::target::profile_dir();
    let download_dir = profile_dir
        .parent()
        .expect("profile dir has no parent")
        .join("testgen-hs");
    create_dir_all(&download_dir).expect("Unable to create testgen directory");

    // Platform, node, and error-decoder build scripts can run concurrently and
    // share both the archive and extraction directories.
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(download_dir.join(".lock"))
        .expect("Unable to open testgen-hs build lock");
    FileExt::lock_exclusive(&lock_file).expect("Unable to lock testgen-hs build directory");

    let archive_name = if target_os == "windows" {
        format!("{file_name}.zip")
    } else {
        format!("{file_name}.tar.bz2")
    };

    let archive_path = download_dir.join(&archive_name);

    // Download the artifact if not already in the target directory.
    if archive_path.exists() {
        println!("Using existing archive at: {}", archive_path.display());
    } else {
        println!("Downloading from: {download_url}");

        let response = reqwest::blocking::get(&download_url)
            .expect("Failed to download archive")
            .error_for_status()
            .expect("Download returned an error status")
            .bytes()
            .expect("Failed to read archive");

        let partial_path = download_dir.join(format!(".{file_name}.download"));
        fs_write(&partial_path, &response).expect("Failed to write archive to disk");
        rename(&partial_path, &archive_path).expect("Failed to install downloaded archive");

        println!("Downloaded to: {}", archive_path.display());
    }

    let testgen_hs_dir = profile_dir.join("testgen-hs");
    let executable = if target_os == "windows" {
        testgen_hs_dir.join("testgen-hs.exe")
    } else {
        testgen_hs_dir.join("testgen-hs")
    };
    let version_file = testgen_hs_dir.join(".version");
    let is_current = executable.exists()
        && read_to_string(&version_file).is_ok_and(|version| version.trim() == TESTGEN_HS_VERSION);

    // Extract the artifact if this profile doesn't have the current version.
    if is_current {
        println!("Already extracted at: {}", testgen_hs_dir.display());
    } else {
        println!("Extracting archive...");
        if testgen_hs_dir.exists() {
            remove_dir_all(&testgen_hs_dir).expect("Unable to remove old extraction directory");
        }
        create_dir_all(&profile_dir).expect("Unable to create extraction directory");
        if target_os == "windows" {
            extract_zip(&archive_path, &profile_dir);
        } else {
            extract_tar_bz2(&archive_path, &profile_dir);
        }
        fs_write(&version_file, TESTGEN_HS_VERSION)
            .expect("Unable to write testgen-hs version file");
    }

    if !executable.is_file() {
        panic!("Archive does not contain {}", executable.display());
    }

    // A cross-compiled target binary cannot run on the build host.
    let host = env::var("HOST").expect("Unable to find build host");
    let target_triple = env::var("TARGET").expect("Unable to find build target");
    if host == target_triple {
        println!("Verifying testgen-hs version...");
        println!("Executing: {executable:?}");

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
        let reported = version_output.trim();
        let expected = format!("testgen-hs {TESTGEN_HS_VERSION}");
        println!("testgen-hs version: {reported}");

        if reported != expected {
            panic!("Expected testgen-hs version {expected} but got {reported}");
        }
    }

    // Set environment variable for downstream build steps.
    println!(
        "cargo:rustc-env={}={}",
        TESTGEN_HS_PATH,
        executable.to_string_lossy()
    );

    FileExt::unlock(&lock_file).expect("Unable to unlock testgen-hs build directory");
}

fn extract_tar_bz2(archive_path: &Path, extract_dir: &Path) {
    let tar_bz2 = File::open(archive_path).expect("Failed to open .tar.bz2 archive");
    let tar = BzDecoder::new(tar_bz2);
    let mut archive = Archive::new(tar);

    archive
        .unpack(extract_dir)
        .expect("Failed to extract .tar.bz2 archive");
}

fn extract_zip(archive_path: &Path, extract_dir: &Path) {
    let file = File::open(archive_path).expect("Failed to open .zip archive");
    let mut archive = ZipArchive::new(file).expect("Failed to read .zip archive");

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("Invalid entry in .zip archive");
        let outpath = match entry.enclosed_name() {
            Some(path) => extract_dir.join(path),
            None => continue,
        };

        if entry.is_dir() {
            create_dir_all(&outpath).expect("Unable to create directory");
        } else {
            if let Some(parent) = outpath.parent() {
                create_dir_all(parent).expect("Unable to create parent directory");
            }

            let mut outfile = File::create(&outpath).expect("Unable to create file");
            std::io::copy(&mut entry, &mut outfile).expect("Unable to write file");
        }
    }
}
