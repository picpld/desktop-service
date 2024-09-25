mod data;
mod web;

use self::data::*;
use self::web::*;
use tokio::runtime::Runtime;
use warp::Filter;

#[cfg(windows)]
const SERVICE_NAME: &str = "desktop-service";
const LISTEN_PORT: u16 = 27247;

#[cfg(windows)]
use std::{ffi::OsString, time::Duration};
#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher, Result,
};

#[cfg(windows)]
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

macro_rules! wrap_response {
    ($expr: expr) => {
        match $expr {
            Ok(data) => warp::reply::json(&JsonResponse {
                code: 0,
                msg: "ok".into(),
                data: Some(data),
            }),
            Err(err) => warp::reply::json(&JsonResponse {
                code: 400,
                msg: format!("{err}"),
                data: Option::<()>::None,
            }),
        }
    };
}

/// The Service
pub async fn run_service() -> anyhow::Result<()> {
    // 开启服务 设置服务状态
    #[cfg(windows)]
    let status_handle = service_control_handler::register(
        SERVICE_NAME,
        move |event| -> ServiceControlHandlerResult {
            match event {
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                ServiceControl::Stop => std::process::exit(0),
                _ => ServiceControlHandlerResult::NotImplemented,
            }
        },
    )?;
    #[cfg(windows)]
    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: Duration::default(),
        process_id: None,
    })?;

    let api_version = warp::get()
        .and(warp::path("version"))
        .map(move || wrap_response!(version()));

    let api_start = warp::post()
        .and(warp::path("start"))
        .and(warp::body::json())
        .map(move |body: StartBody| wrap_response!(start(body)));

    let api_stop = warp::post()
        .and(warp::path("stop"))
        .map(move || wrap_response!(stop()));

    let api_info = warp::get()
        .and(warp::path("info"))
        .map(move || wrap_response!(info()));

    let api_set_dns = warp::post()
        .and(warp::path("set_dns"))
        .and(warp::body::json())
        .map(move |body: DnsBody| wrap_response!(set_dns(body)));

    let api_unset_dns = warp::post()
        .and(warp::path("unset_dns"))
        .map(|| wrap_response!(unset_dns()));

    warp::serve(
        api_version
            .or(api_start)
            .or(api_stop)
            .or(api_info)
            .or(api_set_dns)
            .or(api_unset_dns),
    )
    .run(([127, 0, 0, 1], LISTEN_PORT))
    .await;

    Ok(())
}

/// Service Main function
#[cfg(windows)]
pub fn main() -> Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

#[cfg(not(windows))]
pub fn main() {
    if let Ok(rt) = Runtime::new() {
        rt.block_on(async {
            let _ = run_service().await;
        });
    }
}

#[cfg(windows)]
define_windows_service!(ffi_service_main, my_service_main);

#[cfg(windows)]
pub fn my_service_main(_arguments: Vec<OsString>) {
    if let Ok(rt) = Runtime::new() {
        rt.block_on(async {
            let _ = run_service().await;
        });
    }
}
