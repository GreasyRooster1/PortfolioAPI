use actix_web::HttpRequest;
use tracing::log::warn;

pub(crate) fn track_ip(req: HttpRequest){
    let ip = match req.peer_addr().map(|addr| addr.ip()){
        Some(addr) => addr,
        None => {warn!("could not find ip");return}
    };
    println!("{:?}", ip);
}