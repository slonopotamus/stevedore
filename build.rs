use std::env;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use const_format::formatcp;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const DOCKER_VERSION: &str = "20.10.12";

const DOCKER_WIN64_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_WIN64_SHA: &str = "bd3775ada72492aa1f3c2edb3e81663bd128b9d4f6752ef75953a6af7c219c81";

const DOCKER_COMPOSE_VERSION: &str = "2.2.2";
const DOCKER_COMPOSE_URL: &str = formatcp!("https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_VERSION}/docker-compose-windows-x86_64.exe");
const DOCKER_COMPOSE_SHA: &str = "77496c57449437194add809f10634fca96b9253433809446b6986e709fc8c032";

const DOCKER_SCAN_VERSION: &str = "0.16.0";
const DOCKER_SCAN_URL: &str = formatcp!("https://github.com/docker/scan-cli-plugin/releases/download/v{DOCKER_SCAN_VERSION}/docker-scan_windows_amd64.exe");
const DOCKER_SCAN_SHA: &str = "552677f8650d9d5bc91b706e76ebc60a8d54176e6eafc6a34f897b53e8540a31";

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

fn main() {
    let dest_dir = get_dest_dir();

    build_docker(&dest_dir);
    build_docker_compose(&dest_dir);
    build_docker_scan_plugin(&dest_dir);

    println!("cargo:rerun-if-changed=build.rs");
}
