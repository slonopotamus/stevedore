use std::env;
use std::fs::{File, create_dir_all};
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};

use const_format::formatcp;
use flate2::read::GzDecoder;
use sha2::{Digest, Sha256};
use tar::Archive;
use zip::ZipArchive;

const DOCKER_VERSION: &str = "28.2.2";
const DOCKER_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_SHA: &str = "e24b4ae2dd8b0dbcfa11384d29c366ef864bef1f5810d83f0e9ef2984367a8b6";

const DOCKER_BUILDX_VERSION: &str = "0.24.0";
const DOCKER_BUILDX_URL: &str = formatcp!(
    "https://github.com/docker/buildx/releases/download/v{DOCKER_BUILDX_VERSION}/buildx-v{DOCKER_BUILDX_VERSION}.windows-amd64.exe"
);
const DOCKER_BUILDX_SHA: &str = "8dec102c8eb14f434707cc05a8f0e366c090ded6ad74d9c5f8a64a9c0b766140";

const DOCKER_COMPOSE_VERSION: &str = "2.36.2";
const DOCKER_COMPOSE_URL: &str = formatcp!(
    "https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_VERSION}/docker-compose-windows-x86_64.exe"
);
const DOCKER_COMPOSE_SHA: &str = "82ebec0022949087f883b3dffa0d7e57a2a141203ad31c012381d2754962c905";

const WINCRED_VERSION: &str = "0.9.3";
const WINCRED_URL: &str = formatcp!(
    "https://github.com/docker/docker-credential-helpers/releases/download/v{WINCRED_VERSION}/docker-credential-wincred-v{WINCRED_VERSION}.windows-amd64.exe"
);
const WINCRED_SHA: &str = "deaa1206069dd3bf68d65b0a5c71d0ac87f63663b31221082ea035e5dde0d174";

const CONTAINERD_VERSION: &str = "2.1.1";
const CONTAINERD_URL: &str = formatcp!(
    "https://github.com/containerd/containerd/releases/download/v{CONTAINERD_VERSION}/containerd-{CONTAINERD_VERSION}-windows-amd64.tar.gz"
);
const CONTAINERD_SHA: &str = "1d07f8f9d8c20376613795c11df8f88d32b7fc8408d77ade95beb004053e3e8b";

const NERDCTL_VERSION: &str = "2.1.2";
const NERDCTL_URL: &str = formatcp!(
    "https://github.com/containerd/nerdctl/releases/download/v{NERDCTL_VERSION}/nerdctl-{NERDCTL_VERSION}-windows-amd64.tar.gz"
);
const NERDCTL_SHA: &str = "43822c3d18ce0f029dd4aeb5ab1ba296071b7e95278d70cc6699ef652d83c98d";

const BUILDKIT_VERSION: &str = "0.22.0";
const BUILDKIT_URL: &str = formatcp!(
    "https://github.com/moby/buildkit/releases/download/v{BUILDKIT_VERSION}/buildkit-v{BUILDKIT_VERSION}.windows-amd64.tar.gz"
);
const BUILDKIT_SHA: &str = "e76584227535814b25be9bb202ef400fd43c32b74c3ed3a0a05e9ceee40b3f66";

const CNI_VERSION: &str = "0.3.1";
const CNI_URL: &str = formatcp!(
    "https://github.com/microsoft/windows-container-networking/releases/download/v{CNI_VERSION}/windows-container-networking-cni-amd64-v{CNI_VERSION}.zip"
);
const CNI_SHA: &str = "4f36ee6905ada238ca2a9e1bfb8a1fb2912c2d88c4b6e5af4c41a42db70d7d68";

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

fn untar(file: &Path, dest_dir: &Path) {
    let tar_gz = File::open(file).unwrap();
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);
    archive.unpack(dest_dir).unwrap();
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

fn build_containerd(dest_dir: &Path) {
    let compressed_path = dest_dir.join("containerd.tar.gz");
    download_file(CONTAINERD_URL, CONTAINERD_SHA, &compressed_path);
    untar(&compressed_path, dest_dir);
}

fn build_nerdctl(dest_dir: &Path) {
    let compressed_path = dest_dir.join("nerdctl.tar.gz");
    download_file(NERDCTL_URL, NERDCTL_SHA, &compressed_path);
    untar(&compressed_path, dest_dir);
}

fn build_buildkit(dest_dir: &Path) {
    let compressed_path = dest_dir.join("buildkit.tar.gz");
    download_file(BUILDKIT_URL, BUILDKIT_SHA, &compressed_path);
    untar(&compressed_path, dest_dir);
}

fn build_cni(dest_dir: &Path) {
    let compressed_path = dest_dir.join("cni.zip");
    download_file(CNI_URL, CNI_SHA, &compressed_path);
    unzip(&compressed_path, dest_dir);
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

fn build_docker_buildx(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-buildx.exe");
    download_file(DOCKER_BUILDX_URL, DOCKER_BUILDX_SHA, &dest_path);
}

fn build_docker_compose(dest_dir: &Path) {
    let dest_path = dest_dir.join("docker-compose.exe");
    download_file(DOCKER_COMPOSE_URL, DOCKER_COMPOSE_SHA, &dest_path);
}

fn main() {
    let dest_dir = get_dest_dir();

    build_docker(&dest_dir);
    build_docker_buildx(&dest_dir);
    build_docker_compose(&dest_dir);
    build_wincred(&dest_dir);
    build_containerd(&dest_dir);
    build_nerdctl(&dest_dir);
    build_buildkit(&dest_dir);
    build_cni(&dest_dir);

    println!("cargo:rerun-if-changed=build.rs");
}
