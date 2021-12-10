use std::env;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use const_format::formatcp;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const DOCKER_VERSION: &str = "20.10.11";
const DOCKER_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_SHA256: &str = "a60a1d45cc45ffc7cc0337813e6d5a0cad39409416bab04ade6eb4e03a0e5a7c";

const DOCKER_BUILDX_VERSION: &str = "0.6.3";
const DOCKER_BUILDX_URL: &str = formatcp!("https://github.com/docker/buildx/releases/download/v{DOCKER_BUILDX_VERSION}/buildx-v{DOCKER_BUILDX_VERSION}.windows-amd64.exe");
const DOCKER_BUILDX_SHA256: &str =
    "83375d83f5d5424dbbd0e4e877abd01843e7591cf1e22add5494c77df2e732a9";

const DOCKER_COMPOSE_V1_VERSION: &str = "1.29.2";
const DOCKER_COMPOSE_V1_URL: &str = formatcp!("https://github.com/docker/compose/releases/download/{DOCKER_COMPOSE_V1_VERSION}/docker-compose-Windows-x86_64.exe");
const DOCKER_COMPOSE_V1_SHA256: &str =
    "94c3c634e21532eb9783057eac5235ca44b3e14a4c34e73d7eb6b94a2544cc12";

const DOCKER_COMPOSE_V2_VERSION: &str = "2.2.2";
const DOCKER_COMPOSE_V2_URL: &str = formatcp!("https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_V2_VERSION}/docker-compose-windows-x86_64.exe");
const DOCKER_COMPOSE_V2_SHA256: &str =
    "77496c57449437194add809f10634fca96b9253433809446b6986e709fc8c032";

const COMPOSE_SWITCH_VERSION: &str = "1.0.1";
const COMPOSE_SWITCH_URL: &str = formatcp!("https://github.com/docker/compose-switch/releases/download/v{COMPOSE_SWITCH_VERSION}/docker-compose-windows-amd64.exe");
const COMPOSE_SWITCH_SHA256: &str =
    "b9fd276064cae38eb068b1298e2e618d4d48c6eac709b85a983420937c62f207";

const DOCKER_SCAN_VERSION: &str = "0.10.0";
const DOCKER_SCAN_URL: &str = formatcp!("https://github.com/docker/scan-cli-plugin/releases/download/v{DOCKER_SCAN_VERSION}/docker-scan_windows_amd64.exe");
const DOCKER_SCAN_SHA256: &str = "a1500a753eb99806517666539fbe5c7865d95fe522dc61b19c3ed9c25f9d2e37";

fn get_dest_dir() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    PathBuf::from(manifest_dir_string)
        .join("target")
        .join(build_type)
}

fn download(uri: &str, sha256: &str) -> bytes::Bytes {
    let data = reqwest::blocking::get(uri).unwrap().bytes().unwrap();
    let hash = Sha256::digest(&data);
    if format!("{:x}", hash) != sha256 {
        panic!("Checksum mismatch: expected {} but got {:x}", sha256, hash);
    }
    data
}

fn build_docker(dest_dir: &Path) {
    let compressed_data = download(DOCKER_URL, DOCKER_SHA256);
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
    let data = download(uri, sha256);
    let mut outfile = File::create(dest).unwrap();
    outfile.write_all(&data).unwrap();
}

fn build_docker_buildx(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-buildx.exe");
    download_file(DOCKER_BUILDX_URL, DOCKER_BUILDX_SHA256, &dest_path);
}

fn build_docker_compose_v1(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-compose-v1.exe");
    download_file(DOCKER_COMPOSE_V1_URL, DOCKER_COMPOSE_V1_SHA256, &dest_path);
}

fn build_docker_compose_v2(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-compose-v2.exe");
    download_file(DOCKER_COMPOSE_V2_URL, DOCKER_COMPOSE_V2_SHA256, &dest_path);
}

fn build_compose_switch(dest_dir: &Path) {
    let dest_path = dest_dir.join("compose-switch.exe");
    download_file(COMPOSE_SWITCH_URL, COMPOSE_SWITCH_SHA256, &dest_path);
}

fn build_docker_scan_plugin(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-scan.exe");
    download_file(DOCKER_SCAN_URL, DOCKER_SCAN_SHA256, &dest_path);
}

fn main() {
    let dest_dir = get_dest_dir();

    build_docker(&dest_dir);
    build_docker_buildx(&dest_dir);
    build_docker_compose_v1(&dest_dir);
    build_docker_compose_v2(&dest_dir);
    build_compose_switch(&dest_dir);
    build_docker_scan_plugin(&dest_dir);

    println!("cargo:rerun-if-changed=build.rs");
}
