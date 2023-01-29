#![windows_subsystem = "windows"]

mod error;

use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use std::{io, slice};

use directories::BaseDirs;
use named_lock::Error::WouldBlock;
use named_lock::NamedLock;
use trayicon::{Icon, MenuBuilder, TrayIconBuilder};
use winit::event_loop::EventLoopProxy;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

fn parse_wsl_output(vec: Vec<u8>) -> String {
    let words = unsafe {
        #[allow(clippy::cast_ptr_alignment)]
        slice::from_raw_parts(vec.as_ptr() as *const u16, vec.len() / 2)
    };
    String::from_utf16_lossy(words)
}

#[derive(Clone, Eq, PartialEq, Debug)]
enum Events {
    Quit,
}

struct ChildDrop {
    child: std::process::Child,
}

impl Drop for ChildDrop {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct WslDistribution {
    name: &'static str,
    app_dir: PathBuf,
    processes: Vec<ChildDrop>,
}

impl Drop for WslDistribution {
    fn drop(&mut self) {
        let _ = self.terminate();
    }
}

const DEFAULT_DOCKER_CONTEXT_NAME: &str = "default";
const LINUX_DOCKER_CONTEXT_HOST: &str = "npipe:////./pipe/dockerDesktopLinuxEngine";
const WINDOWS_DOCKER_CONTEXT_HOST: &str = "npipe:////./pipe/dockerDesktopWindowsEngine";

fn update_docker_context(name: &str, host: &str) -> std::io::Result<std::process::Output> {
    let output = Command::new("docker")
        .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
        .arg("context")
        .arg("update")
        .arg(name)
        .arg("--docker")
        .arg(format!("host={host}"))
        .output()?;
    if output.status.success() {
        return Ok(output);
    }

    Command::new("docker")
        .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
        .arg("context")
        .arg("create")
        .arg(name)
        .arg("--docker")
        .arg(format!("host={host}"))
        .output()
}

fn use_docker_context(name: &str) -> std::io::Result<std::process::Output> {
    Command::new("docker")
        .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
        .arg("context")
        .arg("use")
        .arg(name)
        .output()
}

impl WslDistribution {
    fn new(app_dir: PathBuf, name: &'static str) -> std::io::Result<WslDistribution> {
        let mut distribution = WslDistribution {
            name,
            app_dir,
            processes: vec![],
        };

        // Just in case something hanged from the previous run
        let _ = distribution.terminate();

        distribution.register()?;
        distribution.start()?;

        Ok(distribution)
    }

    fn command(&self) -> std::process::Command {
        let mut command = Command::new("wsl");
        command
            .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
            .arg("--distribution")
            .arg(self.name);
        command
    }

    fn register(&self) -> std::io::Result<()> {
        let wsl = wslapi::Library::new()?;

        // TODO(https://github.com/slonopotamus/stevedore/issues/24): we need to store docker data in a separate wsl distribution so it isn't wiped away during upgrades

        // TODO(https://github.com/slonopotamus/stevedore/issues/25): we need to re-register in case wsl distribution is outdated
        let output = if wsl.is_distribution_registered(self.name) {
            self.command().arg("--exec").arg("echo").output()?
        } else {
            let base_dirs = BaseDirs::new().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    "Cannot resolve user data directory",
                )
            })?;
            let tarball = self.app_dir.join("stevedore.tar.gz");
            let data_dir = base_dirs.data_local_dir().join("Stevedore");
            Command::new("wsl")
                .arg("--import")
                .arg(self.name)
                .arg(data_dir)
                .arg(tarball)
                .arg("--version")
                .arg("2")
                .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
                .output()?
        };

        if !output.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                format!(
                    "Stevedore failed to start: {}",
                    parse_wsl_output(output.stdout)
                ),
            ));
        }

        Ok(())
    }

    fn start(&mut self) -> std::io::Result<()> {
        self.processes = vec![
            ChildDrop {
                child: self.command().arg("--exec").arg("dockerd").spawn()?,
            },
            ChildDrop {
                child: Command::new(self.app_dir.join("docker-wsl-proxy.exe"))
                    .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
                    .arg("-c")
                    .arg(format!("wsl://{}/var/run/docker.sock", self.name))
                    .arg("-l")
                    .arg(LINUX_DOCKER_CONTEXT_HOST)
                    .spawn()?,
            },
        ];
        Ok(())
    }

    fn terminate(&mut self) -> std::io::Result<std::process::Output> {
        self.processes.clear();
        let _ = self
            .command()
            .arg("--exec")
            .arg("rm")
            .arg("-f")
            .arg("/var/run/docker*")
            .output();

        Command::new("wsl")
            .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
            .arg("--terminate")
            .arg(self.name)
            .output()
    }
}

struct Application {
    _wsl_distribution: WslDistribution,
    tray_icon: trayicon::TrayIcon<Events>,
}

impl Application {
    fn new(
        event_loop_proxy: EventLoopProxy<Events>,
        app_dir: PathBuf,
    ) -> Result<Application, Box<dyn std::error::Error>> {
        let icon_loading = Icon::from_buffer(
            include_bytes!("../resources/stevedore_grey.ico"),
            None,
            None,
        )
        .map_err(Box::new)?;

        let icon = Icon::from_buffer(include_bytes!("../resources/stevedore.ico"), None, None)
            .map_err(Box::new)?;

        let tray_icon = TrayIconBuilder::new()
            .sender_winit(event_loop_proxy)
            .icon(icon_loading)
            .tooltip("Stevedore")
            .menu(MenuBuilder::new().item("Quit", Events::Quit))
            .build()
            .map_err(Box::new)?;

        let wsl_distribution = WslDistribution::new(app_dir, "stevedore")?;

        let mut result = Application {
            _wsl_distribution: wsl_distribution,
            tray_icon,
        };

        update_docker_context("desktop-linux", LINUX_DOCKER_CONTEXT_HOST).map_err(Box::new)?;
        update_docker_context("desktop-windows", WINDOWS_DOCKER_CONTEXT_HOST).map_err(Box::new)?;
        use_docker_context("desktop-linux").map_err(Box::new)?;

        result.tray_icon.set_icon(&icon).map_err(Box::new)?;

        Ok(result)
    }

    fn move_helper(&self) {}
}

impl Drop for Application {
    fn drop(&mut self) {
        // Restore default Docker context
        let _ = use_docker_context(DEFAULT_DOCKER_CONTEXT_NAME);
    }
}

fn do_main() -> Result<(), Box<dyn std::error::Error>> {
    let app_dir = PathBuf::from(std::env::current_exe()?.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Cannot resolve application directory",
        )
    })?);

    let named_lock = NamedLock::create("stevedore")?;
    // Lock to prevent multiple app instances from running
    let guard = named_lock.try_lock();
    if let Err(err) = guard {
        return match err {
            WouldBlock => Err(Box::new(error::Error::AlreadyRunning)),
            _ => Err(Box::new(err)),
        };
    }

    let event_loop = EventLoop::<Events>::with_user_event();
    let application = Application::new(event_loop.create_proxy(), app_dir)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // See https://github.com/rust-windowing/winit/issues/1472
        // We need to move everything to event loop, otherwise they will never be dropped
        // TODO: why does not `let _ = application;` work?
        application.move_helper();

        if let Event::UserEvent(e) = event {
            match e {
                Events::Quit => {
                    *control_flow = ControlFlow::Exit;
                }
            }
        }
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    do_main().map_err(|error| {
        let _ = msgbox::create("Error", error.to_string().as_str(), msgbox::IconType::Error);
        error
    })
}
