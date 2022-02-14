#![windows_subsystem = "windows"]

use std::error::Error;
use std::io;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;

use directories::BaseDirs;
use named_lock::NamedLock;
use trayicon::{Icon, MenuBuilder, TrayIconBuilder};
use winit::event_loop::EventLoopProxy;
use winit::{
    event::Event,
    event_loop::{ControlFlow, EventLoop},
};

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
        if !wsl.is_distribution_registered(self.name) {
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
                .output()?;
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
    _tray_icon: trayicon::TrayIcon<Events>,
}

impl Application {
    fn new(
        event_loop_proxy: EventLoopProxy<Events>,
        app_dir: PathBuf,
    ) -> Result<Application, Box<dyn Error>> {
        let icon_loading = Icon::from_buffer(
            include_bytes!("../resources/stevedore_grey.ico"),
            None,
            None,
        )
            .map_err(Box::new)?;

        let icon = Icon::from_buffer(include_bytes!("../resources/stevedore.ico"), None, None)
            .map_err(Box::new)?;

        let mut tray_icon = TrayIconBuilder::new()
            .sender_winit(event_loop_proxy)
            .icon(icon_loading)
            .tooltip("Stevedore")
            .menu(MenuBuilder::new().item("Quit", Events::Quit))
            .build()
            .map_err(Box::new)?;

        let wsl_distribution = WslDistribution::new(app_dir, "stevedore")?;

        tray_icon.set_icon(&icon).map_err(Box::new)?;

        Ok(Application {
            _wsl_distribution: wsl_distribution,
            _tray_icon: tray_icon,
        })
    }

    fn move_helper(&self) {}
}

fn do_main() -> Result<(), Box<dyn Error>> {
    let app_dir = PathBuf::from(std::env::current_exe()?.parent().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "Cannot resolve application directory",
        )
    })?);

    let named_lock = NamedLock::create("stevedore")?;
    // Lock to prevent multiple app instances from running
    // TODO: Provide human-readable error message
    let _guard = named_lock.try_lock().map_err(Box::new)?;

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

fn main() -> Result<(), Box<dyn Error>> {
    do_main().map_err(|error| {
        let _ = msgbox::create("Error", error.to_string().as_str(), msgbox::IconType::Error);
        error
    })
}
