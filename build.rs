use std::env;
use std::fs::{create_dir_all, File};
use std::io;
use std::io::Cursor;
use std::path::PathBuf;

use const_format::formatcp;
use http_req::request;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

const DOCKER_VERSION: &'static str = "20.10.7";
const DOCKER_URL: &'static str = formatcp!("https://download.docker.com/win/static/stable/x86_64/docker-{DOCKER_VERSION}.zip");
const DOCKER_SHA256: &'static str = "c3bd22dab5f9ece41c2f496b4551b54f823625a85d8e4789d762a2d249d8b3b2";

fn get_dest_dir() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    return PathBuf::from(manifest_dir_string).join("target").join(build_type);
}

fn main() {
    let mut compressed_data = Vec::new();
    let response = request::get(DOCKER_URL, &mut compressed_data).unwrap();
    if !response.status_code().is_success() {
        panic!("Failed to download {}: HTTP {}", DOCKER_URL, response.status_code());
    }

    let hash = Sha256::digest(&compressed_data);
    if &format!("{:x}", hash) != DOCKER_SHA256 {
        panic!("Checksum mismatch: expected {} but got {:x}", DOCKER_SHA256, hash);
    }

    let mut zip_archive = ZipArchive::new(Cursor::new(compressed_data)).unwrap();
    let dest_dir = get_dest_dir();

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

    println!("cargo:rerun-if-changed=build.rs");
}
