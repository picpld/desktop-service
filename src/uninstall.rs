#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
fn main() {
    panic!("This program is not intended to run on this platform.");
}

const SERVICE_NAME: &str = "desktop-service";

#[cfg(not(windows))]
use anyhow::Error;

#[cfg(target_os = "macos")]
fn main() -> Result<(), Error> {
    use regex::Regex;
    use std::{fs::remove_file, path::Path};

    let dot_name = Regex::new(r"[\-_]").unwrap().replace_all(SERVICE_NAME, ".");

    let plist_file = format!("/Library/LaunchDaemons/{}.helper.plist", dot_name);
    let plist_file = &plist_file;

    // Unload the service.
    std::process::Command::new("launchctl")
        .arg("unload")
        .arg(plist_file)
        .output()
        .expect("Failed to unload service.");

    // Remove the service file.
    let target_binary_path = format!("/Library/PrivilegedHelperTools/{}.helper", dot_name);
    let target_binary_path = &target_binary_path;
    let service_file = Path::new(target_binary_path);
    if service_file.exists() {
        remove_file(service_file).expect("Failed to remove service file.");
    }

    // Remove the plist file.
    let plist_file = Path::new(plist_file);
    if plist_file.exists() {
        remove_file(plist_file).expect("Failed to remove plist file.");
    }
    Ok(())
}
#[cfg(target_os = "linux")]
fn main() -> Result<(), Error> {
    use std::{fs::remove_file, path::Path};

    // Disable the service
    std::process::Command::new("systemctl")
        .arg("disable")
        .arg(SERVICE_NAME)
        .arg("--now")
        .output()
        .expect("Failed to disable service.");

    // Remove the unit file.
    let unit_file = format!("/etc/systemd/system/{}.service", SERVICE_NAME);
    let unit_file = Path::new(&unit_file);
    if unit_file.exists() {
        remove_file(unit_file).expect("Failed to remove unit file.");
    }

    std::process::Command::new("systemctl")
        .arg("daemon-reload")
        .output()
        .expect("Failed to reload systemd daemon.");
    Ok(())
}

/// stop and uninstall the service
#[cfg(windows)]
fn main() -> windows_service::Result<()> {
    use std::{thread, time::Duration};
    use windows_service::{
        service::{ServiceAccess, ServiceState},
        service_manager::{ServiceManager, ServiceManagerAccess},
    };

    let manager_access = ServiceManagerAccess::CONNECT;
    let service_manager = ServiceManager::local_computer(None::<&str>, manager_access)?;

    let service_access = ServiceAccess::QUERY_STATUS | ServiceAccess::STOP | ServiceAccess::DELETE;
    let service = service_manager.open_service(SERVICE_NAME, service_access)?;

    let service_status = service.query_status()?;
    if service_status.current_state != ServiceState::Stopped {
        if let Err(err) = service.stop() {
            eprintln!("{err}");
        }
        // Wait for service to stop
        thread::sleep(Duration::from_secs(1));
    }

    service.delete()?;
    Ok(())
}
