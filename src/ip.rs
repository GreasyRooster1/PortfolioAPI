use actix_web::HttpRequest;
use tracing::log::warn;

pub(crate) fn track_ip(req: HttpRequest){
    let client_ip = req
        .headers()
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok());
    let ip = match client_ip {
        Some(addr) => addr,
        None => {warn!("could not find ip");return}
    };
    println!("{:?}", ip);
}