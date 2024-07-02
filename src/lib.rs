mod ping;

use faststr::FastStr;
use log::{info, warn};
use crate::ping::*;
use volo_grpc::{ response, Request, Response, Status };
use std::result::Result;
use volo_gen::cfst_rpc::*;
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;

static BOOTSTRAPLOCK: Lazy<Arc<Mutex<bool>>> = Lazy::new(|| Arc::new(Mutex::new(false)));
static BOOTSTRAPCONFIG: Lazy<Arc<Mutex<BootstrapRequest>>> = Lazy::new(|| Arc::new(Mutex::new(BootstrapRequest { province: FastStr::new(""), isp:  FastStr::new(""), maximum_mbps: 100, client_version:  FastStr::empty(), bootstrap_token:  FastStr::empty(), node_id:  FastStr::new("") })));

pub struct S;

impl CloudflareSpeedtest for S {
    async fn bootstrap(&self, req: Request<BootstrapRequest>) -> Result<Response<BootstrapResponse>, Status> {
        let mut bootstrap_look = BOOTSTRAPLOCK.try_lock().unwrap();

        let mut response = BootstrapResponse { success: false, should_upgrade: false, message: FastStr::empty(), session_token: FastStr::empty() };

        if *bootstrap_look {
            response = BootstrapResponse {
                success: false,
                should_upgrade: false,
                message: "You have already bootstrapped".into(),
                session_token: String::new().into(),
            };

            drop(bootstrap_look);

            warn!("接收到 BootStrap 初始化信息，但已经完成初始化");
        } else {
            *bootstrap_look = true;
            drop(bootstrap_look);

            let mut bootstarp_config = BOOTSTRAPCONFIG.lock().unwrap();

            *bootstarp_config = req.get_ref().clone();

            drop(bootstarp_config);

            let mut should_upgrade = false;
            if req.get_ref().client_version != env!("CARGO_PKG_VERSION"){
                should_upgrade = true;
            }

            response = BootstrapResponse {
                success: true,
                should_upgrade: should_upgrade,
                message: "Done".into(),
                session_token: req.get_ref().bootstrap_token.clone(),
            };

            info!("接收到 BootStrap 初始化信息，成功完成初始化");
        }

        Result::Ok(Response::new(response))
    }

    async fn speedtest(&self, req: Request<SpeedtestRequest>) -> Result<Response<SpeedtestResponse>, Status> {
        Result::Ok(Response::new(Default::default()))
    }

    async fn upgrade(&self, req: Request<UpgradeRequest>) -> Result<Response<UpgradeResponse>, Status> {
        Result::Ok(Response::new(Default::default()))
    }

    async fn alive(&self, req: Request<Ping>) -> Result<Response<Pong>, Status> {
        println!("a");
        Result::Ok(Response::new(Default::default()))
    }
}
