mod config;
mod error;
mod file;

use config::Config;
use config::Place;
use error::ServerResult;
use file::LocationSnapshot;
use file::LocationWriter;
use geoutils::Distance;
use geoutils::Location;
use rocket::http::Status;
use rocket::{get, routes, State};
use std::path::Path;

#[rocket::main]
async fn main() {
    let config = match Config::load().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    rocket::build()
        .manage(LocationWriter::new(
            Path::new("example/locations"),
            std::time::Duration::from_secs(1000 * 60 * 60 * 12),
        ))
        .manage(config)
        .mount("/", routes![location_update])
        .launch()
        .await
        .unwrap();
}

#[get("/location-update/<pwd>/<user_id>/<lat>/<long>")]
async fn location_update(
    pwd: &str,
    user_id: &str,
    lat: f64,
    long: f64,
    writer: &State<LocationWriter<'_>>,
    config: &State<Config>,
) -> Status {
    if pwd != config.password {
        return Status::Unauthorized;
    }

    let place = match is_at_place(config, lat.clone(), long.clone()) {
        Ok(v) => v,
        Err(e) => {
            println!(
                "A server error occurred while figuring out if there were any nearby places: {}",
                e
            );
            return Status::InternalServerError;
        }
    };

    let place = match place {
        Some(v) => Some(v.clone()),
        None => None,
    };

    match writer
        .location_update(
            String::from(user_id),
            LocationSnapshot::new(lat, long, place),
        )
        .await
    {
        Ok(_) => {}
        Err(e) => {
            println!("A file_error occurred while writing a new location: {}", e);
            return Status::InternalServerError;
        }
    }

    Status::Ok
}

fn is_at_place(config: &State<Config>, lat: f64, long: f64) -> ServerResult<Option<&Place>> {
    let current_location = Location::new(lat, long);
    for place in config.places.iter() {
        if current_location.is_in_circle(
            &Location::new(place.lat, place.long),
            Distance::from_meters(place.radius),
        )? {
            return Ok(Some(place));
        }
    }

    Ok(None)
}
