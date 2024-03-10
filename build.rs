use std::env;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

use const_format::formatcp;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const DOCKER_VERSION: &str = "25.0.4";
const DOCKER_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_SHA: &str = "7ef75de357525b19c6f60098cba75571aa286eb56206cda121c5693d63d28386";

const DOCKER_COMPOSE_VERSION: &str = "2.24.7";
const DOCKER_COMPOSE_URL: &str = formatcp!("https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_VERSION}/docker-compose-windows-x86_64.exe");
const DOCKER_COMPOSE_SHA: &str = "1a5ffa12cff51a65f4e5e8874ed46b0783cfbc8f5ef837f5b9523decf1afd1d0";

const WINCRED_VERSION: &str = "0.8.1";
const WINCRED_URL: &str = formatcp!("https://github.com/docker/docker-credential-helpers/releases/download/v{WINCRED_VERSION}/docker-credential-wincred-v{WINCRED_VERSION}.windows-amd64.exe");
const WINCRED_SHA: &str = "86c3aa9120ad136e5f7d669267a8e271f0d9ec2879c75908f20a769351043a28";

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
            create_dir_all(p).unwrap();
        }

        let mut outfile = File::create(&path).unwrap();
        io::copy(&mut file, &mut outfile).unwrap();
    }
}

fn build_docker(dest_dir: &Path) {
    let compressed_path = dest_dir.join("docker.zip");
    download_file(DOCKER_URL, DOCKER_SHA, &compressed_path);
    unzip(&compressed_path, dest_dir);
}

fn build_wincred(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-credential-wincred.exe");
    download_file(WINCRED_URL, WINCRED_SHA, &dest_path);
}

fn download_file(uri: &str, expected_sha: &str, dest: &Path) {
    if let Ok(mut file) = File::open(dest) {
        let mut digest = Sha256::new();
        io::copy(&mut file, &mut digest).unwrap();
        let actual_sha = digest.finalize();
        if expected_sha == format!("{actual_sha:x}") {
            return;
        }
    }

    let data = reqwest::blocking::get(uri).unwrap().bytes().unwrap();
    let actual_sha = Sha256::digest(&data);
    if format!("{actual_sha:x}") != expected_sha {
        panic!("Checksum mismatch for {uri}: expected {expected_sha} but got {actual_sha:x}");
    }
    let mut outfile = File::create(dest).unwrap();
    outfile.write_all(&data).unwrap();
}

fn build_docker_compose(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-compose.exe");
    download_file(DOCKER_COMPOSE_URL, DOCKER_COMPOSE_SHA, &dest_path);
}

fn main() {
    let dest_dir = get_dest_dir();

    build_docker(&dest_dir);
    build_docker_compose(&dest_dir);
    build_wincred(&dest_dir);

    println!("cargo:rerun-if-changed=build.rs");
}
