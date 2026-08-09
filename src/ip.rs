use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::{BufReader, BufWriter};
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::HttpRequest;
use ipgeolocate::{Locator, Service};
use tracing::{error, info};
use tracing::log::warn;
use serde::{Deserialize, Serialize};

#[derive(Serialize,Deserialize)]
struct IpCache{
    found:HashMap<String,IpData>,
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
    hits:Vec<u64> //timestamp
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
        Err(error) => {error!("Error: {}", error); return},
    };
    info!("Geolocated ip: {} - {}, {} ({}) @ {}N {}W isp:{}", data.ip, data.city, data.region, data.country, data.latitude, data.longitude,data.isp);
    match update_cache(data){
        Ok(_) => {},
        Err(error) => {error!("Error: {}", error); return},
    }
    println!("{:?}", ip);
}

pub fn update_cache(data:Locator) -> Result<(), Box<dyn Error>>{
    let file = File::open("./ip_cache.json")?;
    let reader = BufReader::new(file);

    let mut cache:IpCache = serde_json::from_reader(reader)?;
    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs();
    match cache.found.get(&data.ip) {

        Some(data) => {
            let mut hits = data.hits.clone();

            hits.push(current_time);
            cache.found.insert(data.ip.clone(), IpData{
                ip:data.ip.clone(),
                city:data.city.clone(),
                region:data.region.clone(),
                country:data.country.clone(),
                latitude:data.latitude.clone(),
                longitude:data.longitude.clone(),
                isp: data.isp.clone(),

                count:data.count+1,
                hits
            });
        }
        None => {
            cache.found.insert(data.ip.clone(), IpData {
                ip: data.ip.clone(),
                city: data.city.clone(),
                region: data.region.clone(),
                country: data.country.clone(),
                latitude: data.latitude.clone(),
                longitude: data.longitude.clone(),
                isp: data.isp.clone(),

                count: 1,
                hits: vec![current_time]
            });
        }
    }
    let file = File::create("./ip_cache.json")?;
    let writer = BufWriter::new(file);
    serde_json::to_writer(writer, &cache)?;

    Ok(())
}