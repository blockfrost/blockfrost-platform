fn main() {
    let os = target_os();
    let arch = target_arch();

    git_revision::set();
    features::evaluate(os, arch);
    testgen_hs::ensure(os, arch);
}

fn target_os() -> &'static str {
    (if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        panic!("Unsupported OS");
    }) as _
}

fn target_arch() -> &'static str {
    (if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        panic!("Unsupported architecture");
    }) as _
}
mod git_revision {
    use std::env;

    const GIT_REVISION: &str = "GIT_REVISION";

    pub fn set() {
        use std::process::Command;

        if env::var(GIT_REVISION).is_ok() {
            println!("Environment variable {GIT_REVISION} is set. Not setting.");
            return;
        }

        let git_status = Command::new("git")
            .args(["status", "--porcelain"])
            .output()
            .expect("git-status");

        let revision = if !git_status.stdout.is_empty() {
            "dirty".to_string()
        } else {
            let git_rev_parse = Command::new("git")
                .args(["rev-parse", "HEAD"])
                .output()
                .expect("git-rev-parse");
            String::from_utf8_lossy(&git_rev_parse.stdout)
                .trim()
                .to_string()
        };

        println!("cargo:rustc-env={GIT_REVISION}={revision}");
    }
}

mod testgen_hs {
    use bzip2::read::BzDecoder;
    use std::{
        env,
        fs::{File, create_dir_all, write as fs_write},
        path::{Path, PathBuf},
        process::Command,
    };
    use tar::Archive;
    use zip::ZipArchive;

    const TESTGEN_HS_PATH: &str = "TESTGEN_HS_PATH";

    pub fn ensure(target_os: &str, arch: &str) {
        if env::var(TESTGEN_HS_PATH).is_ok() {
            println!("Environment variable {TESTGEN_HS_PATH} is set. Skipping the download.");
            return;
        }

        let testgen_lib_version = "10.4.1.1";

        let suffix = if target_os == "windows" {
            ".zip"
        } else {
            ".tar.bz2"
        };

        let file_name = format!("testgen-hs-{testgen_lib_version}-{arch}-{target_os}");
        let download_url = format!(
            "https://github.com/input-output-hk/testgen-hs/releases/download/{testgen_lib_version}/{file_name}{suffix}"
        );

        println!("Looking for {file_name}");

        // Use the project’s target directory instead of a system cache location.
        let cargo_manifest_dir =
            env::var("CARGO_MANIFEST_DIR").expect("Unable to find CARGO_MANIFEST_DIR");

        let cargo_target_dir = PathBuf::from(&cargo_manifest_dir)
            .join(env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into()));

        let download_dir = cargo_target_dir.join("testgen-hs");

        create_dir_all(&download_dir).expect("Unable to create testgen directory");

        let archive_name = if target_os == "windows" {
            format!("{file_name}.zip")
        } else {
            format!("{file_name}.tar.bz2")
        };

        let archive_path = download_dir.join(&archive_name);

        // Download the artifact if not already in the target directory.
        if !archive_path.exists() {
            println!("Downloading from: {download_url}");

            let response = reqwest::blocking::get(&download_url)
                .expect("Failed to download archive")
                .bytes()
                .expect("Failed to read archive");

            fs_write(&archive_path, &response).expect("Failed to write archive to disk");

            println!("Downloaded to: {}", archive_path.display());
        } else {
            println!("Using existing archive at: {}", archive_path.display());
        }

        // Either `debug` or `release`:
        let cargo_profile = env::var("PROFILE").expect("Could not read PROFILE");

        // Extraction path inside the target directory.
        let extract_dir = cargo_target_dir.join(cargo_profile);
        create_dir_all(&extract_dir).expect("Unable to create extraction directory");

        let testgen_hs_dir = extract_dir.join("testgen-hs");

        // Extract the artifact if not already extracted.
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
        println!("testgen-hs version: {}", version_output.trim());

        let testgen_lib_version = format!("testgen-hs {testgen_lib_version}");

        if version_output.trim() != testgen_lib_version {
            panic!(
                "Expected testgen-hs version {} but got {}",
                version_output.trim(),
                testgen_lib_version
            );
        }

        // Set environment variable for downstream build steps.
        println!(
            "cargo:rustc-env={}={}",
            TESTGEN_HS_PATH,
            executable.to_string_lossy()
        );
    }

    fn extract_tar_bz2(archive_path: &PathBuf, extract_dir: &PathBuf) {
        let tar_bz2 = File::open(archive_path).expect("Failed to open .tar.bz2 archive");
        let tar = BzDecoder::new(tar_bz2);
        let mut archive = Archive::new(tar);

        archive
            .unpack(extract_dir)
            .expect("Failed to extract .tar.bz2 archive");
    }

    fn extract_zip(archive_path: &PathBuf, extract_dir: &Path) {
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
}

mod features {
    pub fn evaluate(target_os: &str, target_arch: &str) {
        println!("cargo::rustc-check-cfg=cfg(evaluate)");

        // FIXME: @michalrus reenabled `#[cfg(evaluate)]` on Windows in this dirty way:
        #[allow(clippy::overly_complex_bool_expr)]
        if false && target_os == "windows" {
            println!("cargo:warning=Skipping 'evaluate' cfg for {target_os}-{target_arch}");
        } else {
            println!(
                "cargo:warning=Going to build with 'evaluate' cfg for {target_os}-{target_arch}"
            );
            println!("cargo:rustc-cfg=evaluate");
        }
    }
}
