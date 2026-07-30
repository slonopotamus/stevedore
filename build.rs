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

const DOCKER_VERSION: &str = "29.6.2";
const DOCKER_URL: &str =
    formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_SHA: &str = "c790bfcc9e8b227173b20f04f4ddaae6e997eac6d4aeaf7a850e2874d9222944";

const DOCKER_BUILDX_VERSION: &str = "0.36.0";
const DOCKER_BUILDX_URL: &str = formatcp!(
    "https://github.com/docker/buildx/releases/download/v{DOCKER_BUILDX_VERSION}/buildx-v{DOCKER_BUILDX_VERSION}.windows-amd64.exe"
);
const DOCKER_BUILDX_SHA: &str = "ce84699d1d93a67d25888b9e6a717862a71b20305602f96d44770b1c12dfcdd8";

const DOCKER_COMPOSE_VERSION: &str = "5.3.1";
const DOCKER_COMPOSE_URL: &str = formatcp!(
    "https://github.com/docker/compose/releases/download/v{DOCKER_COMPOSE_VERSION}/docker-compose-windows-x86_64.exe"
);
const DOCKER_COMPOSE_SHA: &str = "6d36cc701393c066d67ebc77773b718d8c738bc4ccb350fbf1dc0e6a09f44cb9";

const WINCRED_VERSION: &str = "0.9.8";
const WINCRED_URL: &str = formatcp!(
    "https://github.com/docker/docker-credential-helpers/releases/download/v{WINCRED_VERSION}/docker-credential-wincred-v{WINCRED_VERSION}.windows-amd64.exe"
);
const WINCRED_SHA: &str = "fec0aa06335bbefc7f515630c9368afaa1423fa9fa585112f123b6625f72d560";

const CONTAINERD_VERSION: &str = "2.3.3-2-gd27132612";
const CONTAINERD_URL: &str = formatcp!(
    "https://github.com/slonopotamus/containerd/releases/download/v{CONTAINERD_VERSION}/containerd-{CONTAINERD_VERSION}-windows-amd64.tar.gz"
);
const CONTAINERD_SHA: &str = "981356144f9e8ecc10b3193b915ca3866030e5a0c7a1ec77a70f504a06785c82";

const NERDCTL_VERSION: &str = "2.3.5";
const NERDCTL_URL: &str = formatcp!(
    "https://github.com/containerd/nerdctl/releases/download/v{NERDCTL_VERSION}/nerdctl-{NERDCTL_VERSION}-windows-amd64.tar.gz"
);
const NERDCTL_SHA: &str = "eda2a08d8a5ea97443c73fed1a827fbc385d4f7847486721899cd7e2c2f9dc36";

const BUILDKIT_VERSION: &str = "0.32.0";
const BUILDKIT_URL: &str = formatcp!(
    "https://github.com/moby/buildkit/releases/download/v{BUILDKIT_VERSION}/buildkit-v{BUILDKIT_VERSION}.windows-amd64.tar.gz"
);
const BUILDKIT_SHA: &str = "820d3354d83d48fb4ab185292269c118cb885cb49a9c657fb4a189f38b8efdc7";

const CNI_VERSION: &str = "0.3.3";
const CNI_URL: &str = formatcp!(
    "https://github.com/microsoft/windows-container-networking/releases/download/v{CNI_VERSION}/windows-container-networking-cni-amd64-v{CNI_VERSION}.zip"
);
const CNI_SHA: &str = "12ce767c9bc5b8088021339bedd119a85d40bc2e18df5c55ca629b1ef4d346c5";

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
