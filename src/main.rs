mod config;
mod error;
mod file;

use config::Config;
use config::Place;
use config::User;
use error::ServerResult;
use file::LocationSnapshot;
use file::LocationWriter;
use file::UserDataSnapshot;
use file::UserDataSnapshotLocation;
use file::UserDataWriter;
use geoutils::Distance;
use geoutils::Location;
use rocket::http::Status;
use rocket::{get, routes, State};
use std::time::Duration;

#[rocket::launch]
async fn rocket() -> _ {
    let config = match Config::load().await {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    let figment = rocket::Config::figment().merge(("port", config.port));
    rocket::custom(figment)
        .manage(LocationWriter::new(
            config.file_locations.location.clone(),
            std::time::Duration::from_millis(1000 * 60 * 60 * 12),
        ))
        .manage(UserDataWriter::new(
            config.file_locations.data.clone(),
            Duration::from_millis(config.file_duration),
        ))
        .manage(config)
        .mount("/", routes![location_update, data_update])
}

#[get("/data-update/<pwd>/<user_id>/<lat>/<long>/<battery>")]
async fn data_update(
    pwd: &str,
    user_id: &str,
    lat: f64,
    long: f64,
    battery: u8,
    writer: &State<UserDataWriter>,
    config: &State<Config>,
) -> Status {
    if pwd != config.password {
        return Status::Unauthorized;
    }

    let user = match get_user(config, user_id) {
        Some(v) => v,
        None => {
            println!("Got a request with an invalid user id: {}", user_id);
            return Status::NotImplemented;
        }
    };

    if battery > 100 {
        return Status::BadRequest;
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

    match writer
        .data_update(
            String::from(user_id),
            UserDataSnapshot::new(
                UserDataSnapshotLocation::new(&LocationSnapshot::new(lat, long, place), battery),
                user,
            ),
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

#[get("/location-update/<pwd>/<user_id>/<lat>/<long>")]
async fn location_update(
    pwd: &str,
    user_id: &str,
    lat: f64,
    long: f64,
    writer: &State<LocationWriter>,
    config: &State<Config>,
) -> Status {
    if pwd != config.password {
        return Status::Unauthorized;
    }

    if let None = get_user(config, user_id) {
        println!("Got a request with an invalid user id: {}", user_id);
        return Status::NotImplemented;
    };

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

fn is_at_place(config: &State<Config>, lat: f64, long: f64) -> ServerResult<Option<Place>> {
    let current_location = Location::new(lat, long);
    for (name, place) in config.places.iter() {
        if current_location.is_in_circle(
            &Location::new(place.lat, place.long),
            Distance::from_meters(place.radius),
        )? {
            let mut place = place.clone();
            place.name = Some(name.clone());
            return Ok(Some(place));
        }
    }

    Ok(None)
}

fn get_user<'a>(config: &'a State<Config>, user_id: &str) -> Option<&'a User> {
    return config.users.get(user_id);
}
