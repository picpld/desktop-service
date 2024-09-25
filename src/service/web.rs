use super::data::*;
use anyhow::{bail, Context, Ok, Result};
use once_cell::sync::OnceCell;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
#[derive(Debug, Default)]
pub struct ServerStatus {
    pub info: Option<StartBody>,
    pub pid: u32,
}
#[allow(dead_code)]
#[derive(Debug, Default)]
pub struct DNSStatus {
    pub dns: Option<String>,
}

impl ServerStatus {
    pub fn global() -> &'static Arc<Mutex<ServerStatus>> {
        static CLASHSTATUS: OnceCell<Arc<Mutex<ServerStatus>>> = OnceCell::new();

        CLASHSTATUS.get_or_init(|| Arc::new(Mutex::new(ServerStatus::default())))
    }
}

#[allow(dead_code)]
impl DNSStatus {
    pub fn global() -> &'static Arc<Mutex<DNSStatus>> {
        static DNSSTAUS: OnceCell<Arc<Mutex<DNSStatus>>> = OnceCell::new();

        DNSSTAUS.get_or_init(|| Arc::new(Mutex::new(DNSStatus::default())))
    }
}

/// GET /version
/// 获取服务进程的版本
pub fn version() -> Result<HashMap<String, String>> {
    let version = env!("CARGO_PKG_VERSION");

    let mut map = HashMap::new();

    map.insert("service".into(), "Desktop Service".into());
    map.insert("version".into(), version.into());

    Ok(map)
}

/// POST /start
/// 启动进程
pub fn start(body: StartBody) -> Result<()> {
    // stop the old server
    let _ = stop();

    let body_cloned = body.clone();

    let log = File::create(body.log_file).context("failed to open log")?;
    let result = Command::new(body.bin_path)
        .args(body.args)
        .stdout(log)
        .spawn()?;

    let mut arc = ServerStatus::global().lock();
    arc.info = Some(body_cloned);
    arc.pid = result.id();

    Ok(())
}

/// POST /stop
/// 停止 server 进程
pub fn stop() -> Result<()> {
    let mut arc = ServerStatus::global().lock();

    if arc.info.is_none() {
        // 没有进程在运行
        return Ok(());
    }

    let system = System::new_with_specifics(
        RefreshKind::new().with_processes(ProcessRefreshKind::everything()),
    );
    let bin_path = arc.info.clone().unwrap().bin_path;
    let filename = Path::new(&bin_path).file_stem().unwrap().to_str().unwrap();
    let procs = system.processes_by_name(filename);
    for proc in procs {
        if proc.pid().as_u32() == arc.pid {
            proc.kill();
        }
    }
    arc.info = None;
    arc.pid = 0;
    Ok(())
}

/// GET /info
/// 获取 server 当前执行信息
pub fn info() -> Result<StartBody> {
    let arc = ServerStatus::global().lock();

    match arc.info.clone() {
        Some(info) => Ok(info),
        None => bail!("server not executed"),
    }
}

/// POST /set_dns
/// 设置DNS
pub fn set_dns(_body: DnsBody) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let service = default_network_service().or_else(|_e| default_network_service_by_ns());
        if let Err(e) = service {
            return Err(e);
        }
        let service = service.unwrap();
        let mut arc = DNSStatus::global().lock();
        if arc.dns.is_none() {
            let output = networksetup()
                .arg("-getdnsservers")
                .arg(&service)
                .output()?;
            let mut origin_dns = String::from_utf8(output.stdout)?.trim().replace("\n", " ");
            if origin_dns
                .trim()
                .starts_with("There aren't any DNS Servers set on")
            {
                origin_dns = "Empty".to_string();
            }
            arc.dns = Some(origin_dns);
        }

        networksetup()
            .arg("-setdnsservers")
            .arg(&service)
            .arg(_body.dns)
            .output()?;
    }

    Ok(())
}

/// POST /unset_dns
/// 还原DNS
pub fn unset_dns() -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        let mut arc = DNSStatus::global().lock();

        let origin_dns = match arc.dns.clone() {
            Some(dns) => dns,
            None => "".to_string(),
        };
        if !origin_dns.is_empty() {
            let service = default_network_service().or_else(|_e| default_network_service_by_ns());
            if let Err(e) = service {
                return Err(e);
            }
            let service = service.unwrap();
            networksetup()
                .arg("-setdnsservers")
                .arg(service)
                .arg(origin_dns)
                .output()?;

            arc.dns = None;
        }
    }

    Ok(())
}
#[cfg(target_os = "macos")]
fn networksetup() -> Command {
    Command::new("networksetup")
}

#[cfg(target_os = "macos")]
fn default_network_service() -> Result<String> {
    use std::net::{SocketAddr, UdpSocket};
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect("1.1.1.1:80")?;
    let ip = socket.local_addr()?.ip();
    let addr = SocketAddr::new(ip, 0);

    let interfaces = interfaces::Interface::get_all()?;
    let interface = interfaces
        .into_iter()
        .find(|i| i.addresses.iter().find(|a| a.addr == Some(addr)).is_some())
        .map(|i| i.name.to_owned());

    match interface {
        Some(interface) => {
            let service = get_server_by_order(interface)?;
            Ok(service)
        }
        None => anyhow::bail!("No network service found"),
    }
}

#[cfg(target_os = "macos")]
fn default_network_service_by_ns() -> Result<String> {
    let output = networksetup().arg("-listallnetworkservices").output()?;
    let stdout = String::from_utf8(output.stdout)?;
    let mut lines = stdout.split('\n');
    lines.next(); // ignore the tips

    // get the first service
    match lines.next() {
        Some(line) => Ok(line.into()),
        None => anyhow::bail!("No network service found"),
    }
}

#[cfg(target_os = "macos")]
fn get_server_by_order(device: String) -> Result<String> {
    let services = listnetworkserviceorder()?;
    let service = services
        .into_iter()
        .find(|(_, _, d)| d == &device)
        .map(|(s, _, _)| s);
    match service {
        Some(service) => Ok(service),
        None => anyhow::bail!("No network service found"),
    }
}

#[cfg(target_os = "macos")]
fn listnetworkserviceorder() -> Result<Vec<(String, String, String)>> {
    let output = networksetup().arg("-listnetworkserviceorder").output()?;
    let stdout = String::from_utf8(output.stdout)?;

    let mut lines = stdout.split('\n');
    lines.next(); // ignore the tips

    let mut services = Vec::new();
    let mut p: Option<(String, String, String)> = None;

    for line in lines {
        if !line.starts_with('(') {
            continue;
        }

        if p.is_none() {
            let ri = line.find(')');
            if ri.is_none() {
                continue;
            }
            let ri = ri.unwrap();
            let service = line[ri + 1..].trim();
            p = Some((service.into(), "".into(), "".into()));
        } else {
            let line = &line[1..line.len() - 1];
            let pi = line.find("Port:");
            let di = line.find(", Device:");
            if pi.is_none() || di.is_none() {
                continue;
            }
            let pi = pi.unwrap();
            let di = di.unwrap();
            let port = line[pi + 5..di].trim();
            let device = line[di + 9..].trim();
            let (service, _, _) = p.as_mut().unwrap();
            *p.as_mut().unwrap() = (service.to_owned(), port.into(), device.into());
            services.push(p.take().unwrap());
        }
    }

    Ok(services)
}
