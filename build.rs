use std::env;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use const_format::formatcp;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const DOCKER_VERSION: &str = "20.10.17";

const DOCKER_WIN64_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_WIN64_SHA: &str = "c5d287db39a369ada1c814070210989d2538f423b79f660a0638fcf14db2a74a";

const DOCKER_LINUX_URL: &str =
    formatcp!("https://download.docker.com/linux/static/stable/x86_64/docker-{DOCKER_VERSION}.tgz");
const DOCKER_LINUX_SHA: &str = "969210917b5548621a2b541caf00f86cc6963c6cf0fb13265b9731c3b98974d9";

const SHMOBY_VERSION: &str = formatcp!("{DOCKER_VERSION}.1");
const SHMOBY_URL: &str = formatcp!(
    "https://github.com/slonopotamus/shmoby/releases/download/v{SHMOBY_VERSION}/dockerd.exe"
);
const SHMOBY_SHA: &str = "0e38b6df7ab6653dddede25517f75e8c17c41692cc46dd145e04e052c8832ed8";

const ALPINE_URL: &str = "https://dl-cdn.alpinelinux.org/alpine/v3.15/releases/x86_64/alpine-minirootfs-3.15.4-x86_64.tar.gz";
const ALPINE_SHA: &str = "6abd0409ccd6b27cb5311e0d475af5a284515eb219626334b29b1c3141d47653";

const DOCKER_COMPOSE_VERSION: &str = "2.6.1";
const DOCKER_COMPOSE_URL: &str = formatcp!("https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_VERSION}/docker-compose-windows-x86_64.exe");
const DOCKER_COMPOSE_SHA: &str = "7e8231012d66c164cfbe46d17aec5f614a2ca4b0fc20dd78bd7b1b3fcbc4f28d";

const DOCKER_SCAN_VERSION: &str = "0.17.0";
const DOCKER_SCAN_URL: &str = formatcp!("https://github.com/docker/scan-cli-plugin/releases/download/v{DOCKER_SCAN_VERSION}/docker-scan_windows_amd64.exe");
const DOCKER_SCAN_SHA: &str = "d6e19957813f28970c5552aa2683277e187a1b7327b3af90194e8f04f1d04021";

const DOCKER_WSL_PROXY_VERSION: &str = "0.0.6";
const DOCKER_WSL_PROXY_URL: &str = formatcp!("https://github.com/slonopotamus/docker-wsl-proxy/releases/download/{DOCKER_WSL_PROXY_VERSION}/docker-wsl-proxy.exe");
const DOCKER_WSL_PROXY_SHA: &str =
    "e31cd75a458248b28f839fd1c94ff5ae07fa97e45a4d6c33a44253d39b682e91";

const KUBECTL_VERSION: &str = "1.24.3";
const KUBECTL_URL: &str = formatcp!("https://storage.googleapis.com/kubernetes-release/release/v{KUBECTL_VERSION}/bin/windows/amd64/kubectl.exe");
const KUBECTL_SHA: &str = "a0ff7131d52721c57a260d2fa9a58928d22e586883d0ca50f40b79d18203989a";

const WINCRED_VERSION: &str = "0.6.4";
const WINCRED_URL: &str = formatcp!("https://github.com/docker/docker-credential-helpers/releases/download/v{WINCRED_VERSION}/docker-credential-wincred-v{WINCRED_VERSION}-amd64.zip");
const WINCRED_SHA: &str = "25031fec7fa0501666d47e63dc7593e2b0e6ad72c6bf13abef5917691ea47e37";

fn get_dest_dir() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    PathBuf::from(manifest_dir_string)
        .join("target")
        .join(build_type)
}

fn unzip(file: &Path, dest_dir: &Path) {
    let compressed_data = File::open(file).unwrap();
    let mut zip_archive = ZipArchive::new(compressed_data).unwrap();

    for i in 0..zip_archive.len() {
        let mut file = zip_archive.by_index(i).unwrap();
        if file.is_dir() {
            continue;
        }

        let path = dest_dir.join(file.enclosed_name().unwrap());

        if let Some(p) = path.parent() {
            create_dir_all(&p).unwrap();
        }

        let mut outfile = File::create(&path).unwrap();
        io::copy(&mut file, &mut outfile).unwrap();
    }
}

fn build_docker(dest_dir: &Path) {
    let compressed_path = dest_dir.join("docker.zip");
    download_file(DOCKER_WIN64_URL, DOCKER_WIN64_SHA, &compressed_path);
    unzip(&compressed_path, dest_dir);
}

fn build_wincred(dest_dir: &Path) {
    let compressed_path = dest_dir.join("docker-credential-wincred.zip");
    download_file(WINCRED_URL, WINCRED_SHA, &compressed_path);
    unzip(&compressed_path, dest_dir);
}

fn build_shmoby(dest_dir: &Path) {
    let dest_path = dest_dir.join("dockerd.exe");
    download_file(SHMOBY_URL, SHMOBY_SHA, &dest_path);
}

fn download_file(uri: &str, expected_sha: &str, dest: &Path) {
    if let Ok(mut file) = File::open(dest) {
        let mut digest = Sha256::new();
        io::copy(&mut file, &mut digest).unwrap();
        let actual_sha = digest.finalize();
        if expected_sha == format!("{:x}", actual_sha) {
            return;
        }
    }

    let data = reqwest::blocking::get(uri).unwrap().bytes().unwrap();
    let actual_sha = Sha256::digest(&data);
    if format!("{:x}", actual_sha) != expected_sha {
        panic!(
            "Checksum mismatch for {}: expected {} but got {:x}",
            uri, expected_sha, actual_sha
        );
    }
    let data = data;
    let mut outfile = File::create(dest).unwrap();
    outfile.write_all(&data).unwrap();
}

fn build_docker_compose(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-compose.exe");
    download_file(DOCKER_COMPOSE_URL, DOCKER_COMPOSE_SHA, &dest_path);
}

fn build_docker_scan_plugin(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-scan.exe");
    download_file(DOCKER_SCAN_URL, DOCKER_SCAN_SHA, &dest_path);
}

fn build_docker_wsl_proxy(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-wsl-proxy.exe");
    download_file(DOCKER_WSL_PROXY_URL, DOCKER_WSL_PROXY_SHA, &dest_path);
}

fn build_kubectl(dest_dir: &Path) {
    let dest_path = dest_dir.join("kubectl.exe");
    download_file(KUBECTL_URL, KUBECTL_SHA, &dest_path);
}

fn run_cmd(cmd: &mut Command) -> String {
    let output = cmd.output().unwrap();
    assert!(
        output.status.success(),
        "{:?} returned {:?}\nSTDOUT:{}\nSTDERR:\n{}",
        cmd,
        output.status,
        std::str::from_utf8(&output.stdout).unwrap(),
        std::str::from_utf8(&output.stderr).unwrap(),
    );
    String::from(std::str::from_utf8(&output.stdout).unwrap().trim())
}

/**
1. You can't use Docker in this function because GitHub Actions doesn't support nested virtualization
2. You can't use WSL2 in this function because Windows Server only has WSL1
*/
fn build_wsl_tarball(dest_dir: &Path) {
    const DISTRIBUTION_NAME: &str = "stevedore";
    const STAGING_DISTRIBUTION_NAME: &str = formatcp!("{DISTRIBUTION_NAME}-staging");

    // Download Alpine Linux rootfs
    let alpine_tgz = dest_dir.join(formatcp!("alpine.tar.gz"));
    download_file(ALPINE_URL, ALPINE_SHA, &alpine_tgz);

    // Download Linux Docker binaries
    let docker_tgz = dest_dir.join("docker.tar.gz");
    download_file(DOCKER_LINUX_URL, DOCKER_LINUX_SHA, &docker_tgz);

    // Remove WSL distribution in case it already exists
    let _ = Command::new("wsl")
        .arg("--unregister")
        .arg(STAGING_DISTRIBUTION_NAME)
        .output();

    // Import Alpine Linux rootfs into WSL
    run_cmd(
        Command::new("wsl")
            .arg("--import")
            .arg(STAGING_DISTRIBUTION_NAME)
            .arg(dest_dir)
            .arg(alpine_tgz),
    );

    // Add docker into WSL
    // TODO(https://github.com/slonopotamus/stevedore/issues/26): Instead, use Docker binaries we just downloaded
    run_cmd(
        Command::new("wsl")
            .arg("--distribution")
            .arg(STAGING_DISTRIBUTION_NAME)
            .arg("--")
            .arg("apk")
            .arg("add")
            .arg("--no-cache")
            .arg("docker")
            .arg("socat"),
    );

    let uncompressed_tarball_path = dest_dir.join(formatcp!("{DISTRIBUTION_NAME}.tar"));
    let compressed_tarball_path = dest_dir.join(formatcp!("{DISTRIBUTION_NAME}.tar.gz"));

    // Export tarball from WSL
    run_cmd(
        Command::new("wsl")
            .arg("--export")
            .arg(STAGING_DISTRIBUTION_NAME)
            .arg(&uncompressed_tarball_path),
    );

    // Compress tarball
    // TODO: pipe directly from wsl --export
    {
        let mut uncompressed_tarball = File::open(uncompressed_tarball_path).unwrap();
        let compressed_tarball = File::create(compressed_tarball_path).unwrap();
        let mut encoder = GzEncoder::new(compressed_tarball, Compression::default());
        std::io::copy(&mut uncompressed_tarball, &mut encoder).unwrap();
    }

    // Cleanup WSL
    run_cmd(
        Command::new("wsl")
            .arg("--unregister")
            .arg(STAGING_DISTRIBUTION_NAME),
    );
}

fn copy_redist_msm(dest_dir: &Path) {
    let tool = cc::windows_registry::find_tool("x86_64-msvc", "cl.exe").unwrap();
    let tools_dir = tool
        .path()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap();
    let version = tools_dir.file_name().unwrap();
    let msm_dir = tools_dir
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Redist")
        .join("MSVC")
        .join(version)
        .join("MergeModules");

    let msm_suffix = "CRT_x64.msm";

    for f in msm_dir.read_dir().unwrap() {
        let f = f.unwrap();
        if f.file_name().to_string_lossy().ends_with(msm_suffix) {
            std::fs::copy(f.path(), dest_dir.join("vcredist.msm")).unwrap();
            return;
        }
    }

    panic!("Failed to find '*{}' {:?}", msm_suffix, msm_dir);
}

fn main() {
    let dest_dir = get_dest_dir();

    copy_redist_msm(&dest_dir);
    build_wsl_tarball(&dest_dir);
    build_docker(&dest_dir);
    build_shmoby(&dest_dir);
    build_docker_compose(&dest_dir);
    build_docker_scan_plugin(&dest_dir);
    build_wincred(&dest_dir);
    build_docker_wsl_proxy(&dest_dir);
    build_kubectl(&dest_dir);

    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/stevedore.ico");
    res.compile().unwrap();

    println!("cargo:rerun-if-changed=resources/stevedore.ico");
    println!("cargo:rerun-if-changed=build.rs");
}
