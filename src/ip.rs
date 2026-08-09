use actix_web::HttpRequest;
use ipgeolocate::{Locator, Service};
use tracing::log::warn;

pub(crate) async fn track_ip(req: HttpRequest){
    let client_ip = req
        .headers()
        .get("CF-Connecting-IP")
        .and_then(|v| v.to_str().ok());
    let ip = match client_ip {
        Some(addr) => addr,
        None => {warn!("could not find ip");return}
    };
    let service = Service::IpApi;

    match Locator::get(ip, service).await {
        Ok(ip) => println!("{} - {}, {} ({}) @ {}N {}W isp:{}", ip.ip, ip.city, ip.region, ip.country, ip.latitude, ip.longitude,ip.isp),
        Err(error) => println!("Error: {}", error),
    };
    println!("{:?}", ip);
}