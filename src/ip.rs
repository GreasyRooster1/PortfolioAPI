use actix_web::HttpRequest;
use ipgeolocate::{Locator, Service};
use tracing::{error, info};
use tracing::log::warn;
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize)]
struct IpCache{
    found:Vec<IpData>,
}

#[derive(Serialize,Deserialize)]
struct IpData{
    ip:String,
    city:String,
    region:String,
    country:String,
    latitude:String,
    longitude:String,
    isp: String,

    count:u64,
}

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

    let data = match Locator::get(ip, service).await {
        Ok(ip) => ip,
        Err(error) => error!("Error: {}", error),
    };
    info!("Geolocated ip: {} - {}, {} ({}) @ {}N {}W isp:{}", data.ip, data.city, data.region, data.country, data.latitude, data.longitude,data.isp);

    println!("{:?}", ip);
}

pub fn update_cache(data:Locator){

}