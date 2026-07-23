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

const HYDRA_NODE_PATH: &str = "HYDRA_NODE_PATH";
const HYDRA_VERSION: &str = "1.0.0";

enum ArchiveKind {
    Zip,
    TarBz2,
}

struct Target {
    /// The `{owner}/{repo}` GitHub release to download from.
    repo: &'static str,
    /// The release asset base name (without the archive extension).
    asset: String,
    kind: ArchiveKind,
}

impl ArchiveKind {
    const fn extension(&self) -> &'static str {
        match self {
            Self::Zip => ".zip",
            Self::TarBz2 => ".tar.bz2",
        }
    }

    fn extract(&self, archive_path: &Path, extract_dir: &Path) {
        match self {
            Self::Zip => extract_zip(archive_path, extract_dir),
            Self::TarBz2 => extract_tar_bz2(archive_path, extract_dir),
        }
    }
}

impl Target {
    fn detect() -> Option<Self> {
        let os = super::target::os();
        let arch = super::target::arch();

        match (arch.as_str(), os.as_str()) {
            ("x86_64", "linux") => Some(Self {
                repo: "cardano-scaling/hydra",
                asset: format!("hydra-x86_64-linux-{HYDRA_VERSION}"),
                kind: ArchiveKind::Zip,
            }),
            ("aarch64", "darwin") => Some(Self {
                repo: "cardano-scaling/hydra",
                asset: format!("hydra-aarch64-darwin-{HYDRA_VERSION}"),
                kind: ArchiveKind::Zip,
            }),
            ("aarch64", "linux") => Some(Self {
                repo: "blockfrost/hydra-aarch64-linux",
                asset: format!("hydra-aarch64-linux-{HYDRA_VERSION}"),
                kind: ArchiveKind::TarBz2,
            }),
            _ => None,
        }
    }

    fn download_url(&self) -> String {
        format!(
            "https://github.com/{}/releases/download/{HYDRA_VERSION}/{}{}",
            self.repo,
            self.asset,
            self.kind.extension()
        )
    }
}

pub fn ensure() {
    println!("cargo:rerun-if-env-changed={HYDRA_NODE_PATH}");

    if env::var(HYDRA_NODE_PATH).is_ok() {
        println!("Environment variable {HYDRA_NODE_PATH} is set. Skipping the download.");
        return;
    }

    let Some(target) = Target::detect() else {
        println!(
            "No prebuilt `hydra-node` for this target; skipping the download. \
             Provide one via {HYDRA_NODE_PATH} if needed."
        );
        return;
    };

    let download_url = target.download_url();
    println!("Looking for {}", target.asset);

    let profile_dir = super::target::profile_dir();

    // Keep the downloaded archive in the shared target root, one level above the
    // profile dir, so the two profiles don't re-download it.
    let download_dir = profile_dir
        .parent()
        .expect("profile dir has no parent")
        .join("hydra-node");
    create_dir_all(&download_dir).expect("Unable to create hydra-node directory");

    // Platform, gateway, and SDK bridge build scripts can run concurrently and
    // share both the archive and extraction directories.
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(download_dir.join(".lock"))
        .expect("Unable to open hydra-node build lock");
    FileExt::lock_exclusive(&lock_file).expect("Unable to lock hydra-node build directory");

    let archive_path = download_dir.join(format!("{}{}", target.asset, target.kind.extension()));

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

        let partial_path = download_dir.join(format!(".{}.download", target.asset));
        fs_write(&partial_path, &response).expect("Failed to write archive to disk");
        rename(&partial_path, &archive_path).expect("Failed to install downloaded archive");

        println!("Downloaded to: {}", archive_path.display());
    }

    let hydra_node_dir = profile_dir.join("hydra-node");
    let executable = hydra_node_dir.join("hydra-node");
    let version_file = hydra_node_dir.join(".version");
    let is_current = executable.exists()
        && read_to_string(&version_file).is_ok_and(|version| version.trim() == HYDRA_VERSION);

    if is_current {
        println!("Already extracted at: {}", hydra_node_dir.display());
    } else {
        println!("Extracting archive...");
        if hydra_node_dir.exists() {
            remove_dir_all(&hydra_node_dir).expect("Unable to remove old extraction directory");
        }
        create_dir_all(&hydra_node_dir).expect("Unable to create extraction directory");
        target.kind.extract(&archive_path, &hydra_node_dir);
        fs_write(&version_file, HYDRA_VERSION).expect("Unable to write hydra-node version file");
    }

    if !executable.is_file() {
        panic!("Archive does not contain {}", executable.display());
    }

    // A cross-compiled target binary cannot run on the build host.
    let host = env::var("HOST").expect("Unable to find build host");
    let target_triple = env::var("TARGET").expect("Unable to find build target");
    if host == target_triple {
        println!("Verifying hydra-node version...");
        println!("Executing: {executable:?}");

        let output = Command::new(&executable)
            .arg("--version")
            .output()
            .expect("Failed to execute hydra-node");

        if !output.status.success() {
            panic!(
                "hydra-node exited with status {}",
                output.status.code().unwrap_or(-1)
            );
        }

        // hydra-node prints e.g. `1.0.0-<commit>`.
        let version_output = String::from_utf8_lossy(&output.stdout);
        let reported = version_output.trim();
        println!("hydra-node version: {reported}");

        if reported.split('-').next() != Some(HYDRA_VERSION) {
            panic!("Expected hydra-node version {HYDRA_VERSION} but got {reported}");
        }
    }

    FileExt::unlock(&lock_file).expect("Unable to unlock hydra-node build directory");
}

fn extract_tar_bz2(archive_path: &Path, extract_dir: &Path) {
    let tar_bz2 = File::open(archive_path).expect("Failed to open .tar.bz2 archive");
    let tar = BzDecoder::new(tar_bz2);
    let mut archive = Archive::new(tar);

    // Preserve symlinks (the top-level `hydra-node` -> `bin/hydra-node`) so the
    // relocatable wrapper keeps working.
    archive
        .unpack(extract_dir)
        .expect("Failed to extract .tar.bz2 archive");
}

fn extract_zip(archive_path: &Path, extract_dir: &Path) {
    let file = File::open(archive_path).expect("Failed to open .zip archive");
    let mut archive = ZipArchive::new(file).expect("Failed to read .zip archive");

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("Invalid entry in .zip archive");
        let path = match entry.enclosed_name() {
            Some(path) => path,
            None => continue,
        };

        if path != Path::new("hydra-node") {
            continue;
        }
        let outpath = extract_dir.join(path);

        if entry.is_dir() {
            create_dir_all(&outpath).expect("Unable to create directory");
        } else {
            if let Some(parent) = outpath.parent() {
                create_dir_all(parent).expect("Unable to create parent directory");
            }

            let mut outfile = File::create(&outpath).expect("Unable to create file");
            std::io::copy(&mut entry, &mut outfile).expect("Unable to write file");

            // Preserve the executable bit (zip stores Unix mode bits).
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = entry.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))
                        .expect("Unable to set file permissions");
                }
            }
        }
    }
}
