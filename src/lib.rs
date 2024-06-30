use volo_grpc::{ Request, Response, Status };
use std::result::Result;
use volo_gen::cfst_rpc::*;

pub struct S;

impl CloudflareSpeedtest for S {
    async fn bootstrap(&self, req: Request<BootstrapRequest>) -> Result<Response<BootstrapResponse>, Status> {
        Result::Ok(Response::new(Default::default()))
    }

    async fn speedtest(&self, req: Request<SpeedtestRequest>) -> Result<Response<SpeedtestResponse>, Status> {
        Result::Ok(Response::new(Default::default()))
    }

    async fn upgrade(&self, req: Request<UpgradeRequest>) -> Result<Response<UpgradeResponse>, Status> {
        Result::Ok(Response::new(Default::default()))
    }

    async fn alive(&self, req: Request<Ping>) -> Result<Response<Pong>, Status> {
        Result::Ok(Response::new(Default::default()))
    }
}
