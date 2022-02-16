use std::env;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use const_format::formatcp;
use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const DOCKER_VERSION: &str = "20.10.12";

const DOCKER_WIN64_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_WIN64_SHA: &str = "bd3775ada72492aa1f3c2edb3e81663bd128b9d4f6752ef75953a6af7c219c81";

const DOCKER_LINUX_URL: &str =
    formatcp!("https://download.docker.com/linux/static/stable/x86_64/docker-{DOCKER_VERSION}.tgz");
const DOCKER_LINUX_SHA: &str = "ee9b5be14e54bf92f48c82c2e6a83fbdd1c5329e8f247525a9ed2fe90d9f89a5";

const ALPINE_URL: &str = "https://dl-cdn.alpinelinux.org/alpine/v3.15/releases/x86_64/alpine-minirootfs-3.15.0-x86_64.tar.gz";
const ALPINE_SHA: &str = "ec7ec80a96500f13c189a6125f2dbe8600ef593b87fc4670fe959dc02db727a2";

const DOCKER_COMPOSE_VERSION: &str = "2.2.3";
const DOCKER_COMPOSE_URL: &str = formatcp!("https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_VERSION}/docker-compose-windows-x86_64.exe");
const DOCKER_COMPOSE_SHA: &str = "7ed35698f85d2d67855934b834845461cd454d40f9a07ee72deb88085af0890e";

const DOCKER_SCAN_VERSION: &str = "0.17.0";
const DOCKER_SCAN_URL: &str = formatcp!("https://github.com/docker/scan-cli-plugin/releases/download/v{DOCKER_SCAN_VERSION}/docker-scan_windows_amd64.exe");
const DOCKER_SCAN_SHA: &str = "d6e19957813f28970c5552aa2683277e187a1b7327b3af90194e8f04f1d04021";

const DOCKER_WSL_PROXY_VERSION: &str = "0.0.2";
const DOCKER_WSL_PROXY_URL: &str = formatcp!("https://github.com/slonopotamus/docker-wsl-proxy/releases/download/{DOCKER_WSL_PROXY_VERSION}/docker-wsl-proxy.exe");
const DOCKER_WSL_PROXY_SHA: &str =
    "9fd63aeac811da6f0c0bd503c446054d189a69c53c11850abbae59863bdfacb2";

fn get_dest_dir() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    PathBuf::from(manifest_dir_string)
        .join("target")
        .join(build_type)
}

fn download(uri: &str, expected_sha256: &str) -> bytes::Bytes {
    let data = reqwest::blocking::get(uri).unwrap().bytes().unwrap();
    let actual_sha256 = Sha256::digest(&data);
    if format!("{:x}", actual_sha256) != expected_sha256 {
        panic!(
            "Checksum mismatch for {}: expected {} but got {:x}",
            uri, expected_sha256, actual_sha256
        );
    }
    data
}

fn build_docker(dest_dir: &Path) {
    let compressed_data = download(DOCKER_WIN64_URL, DOCKER_WIN64_SHA);
    let mut zip_archive = ZipArchive::new(Cursor::new(compressed_data)).unwrap();

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

fn download_file(uri: &str, sha256: &str, dest: &Path) {
    // TODO: skip download if file already matches SHA
    let data = download(uri, sha256);
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

fn main() {
    let dest_dir = get_dest_dir();

    build_wsl_tarball(&dest_dir);
    build_docker(&dest_dir);
    build_docker_compose(&dest_dir);
    build_docker_scan_plugin(&dest_dir);
    build_docker_wsl_proxy(&dest_dir);

    let mut res = winres::WindowsResource::new();
    res.set_icon("resources/stevedore.ico");
    res.compile().unwrap();

    println!("cargo:rerun-if-changed=resources/stevedore.ico");
    println!("cargo:rerun-if-changed=build.rs");
}
