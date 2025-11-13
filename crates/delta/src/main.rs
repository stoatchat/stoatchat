#[macro_use]
extern crate rocket;
#[macro_use]
extern crate revolt_rocket_okapi;
#[macro_use]
extern crate serde_json;

pub mod routes;
pub mod util;

use revolt_config::config;
use revolt_database::events::client::EventV1;
use revolt_database::AMQP;
use revolt_ratelimits::rocket as ratelimiter;
use rocket::{Build, Rocket};
use rocket_cors::{AllowedOrigins, CorsOptions};
use rocket_prometheus::PrometheusMetrics;
use std::net::Ipv4Addr;
use std::str::FromStr;

use amqprs::{
    channel::ExchangeDeclareArguments,
    connection::{Connection, OpenConnectionArguments},
};
use async_std::channel::unbounded;
use authifier::AuthifierEvent;
use rocket::data::ToByteUnit;
use revolt_database::voice::VoiceClient;

pub async fn web() -> Rocket<Build> {
    // Get settings
    let config = config().await;

    // Ensure environment variables are present
    config.preflight_checks();

    // Setup database
    let db = revolt_database::DatabaseInfo::Auto.connect().await.unwrap();
    log::info!("database_here {db:?}");
    db.migrate_database().await.unwrap();

    // Setup Authifier event channel
    let (_, receiver) = unbounded();

    // Setup Authifier
    let authifier = db.clone().to_authifier().await;

    // Launch a listener for Authifier events
    async_std::task::spawn(async move {
        while let Ok(event) = receiver.recv().await {
            match &event {
                AuthifierEvent::CreateSession { .. } | AuthifierEvent::CreateAccount { .. } => {
                    EventV1::Auth(event).global().await
                }
                AuthifierEvent::DeleteSession { user_id, .. }
                | AuthifierEvent::DeleteAllSessions { user_id, .. } => {
                    let id = user_id.to_string();
                    EventV1::Auth(event).private(id).await
                }
            }
        }
    });

    // Configure CORS
    let cors = CorsOptions {
        allowed_origins: AllowedOrigins::All,
        allowed_methods: [
            "Get", "Put", "Post", "Delete", "Options", "Head", "Trace", "Connect", "Patch",
        ]
        .iter()
        .map(|s| FromStr::from_str(s).unwrap())
        .collect(),
        ..Default::default()
    }
    .to_cors()
    .expect("Failed to create CORS.");

    // Configure Swagger
    let swagger = revolt_rocket_okapi::swagger_ui::make_swagger_ui(
        &revolt_rocket_okapi::swagger_ui::SwaggerUIConfig {
            url: "/openapi.json".to_owned(),
            ..Default::default()
        },
    )
    .into();

    let swagger_0_8 = revolt_rocket_okapi::swagger_ui::make_swagger_ui(
        &revolt_rocket_okapi::swagger_ui::SwaggerUIConfig {
            url: "/0.8/openapi.json".to_owned(),
            ..Default::default()
        },
    )
    .into();

    let swagger_0_8 = revolt_rocket_okapi::swagger_ui::make_swagger_ui(
        &revolt_rocket_okapi::swagger_ui::SwaggerUIConfig {
            url: "/0.8/openapi.json".to_owned(),
            ..Default::default()
        },
    )
    .into();

    // Voice handler
    let voice_client = VoiceClient::new(config.api.livekit.nodes.clone());
    // Configure Rabbit
    let connection = Connection::open(&OpenConnectionArguments::new(
        &config.rabbit.host,
        config.rabbit.port,
        &config.rabbit.username,
        &config.rabbit.password,
    ))
    .await
    .expect("Failed to connect to RabbitMQ");

    let channel = connection
        .open_channel(None)
        .await
        .expect("Failed to open RabbitMQ channel");

    channel
        .exchange_declare(
            ExchangeDeclareArguments::new(&config.pushd.exchange, "direct")
                .durable(true)
                .finish(),
        )
        .await
        .expect("Failed to declare exchange");

    let amqp = AMQP::new(connection, channel);

    // Launch background task workers
    revolt_database::tasks::start_workers(db.clone(), amqp.clone());

    // Configure Rocket
    let rocket = rocket::build();
    let prometheus = PrometheusMetrics::new();

    // Ratelimits
    let ratelimits = ratelimiter::RatelimitStorage::new(util::ratelimits::DeltaRatelimits);

    routes::mount(config, rocket)
        .attach(prometheus.clone())
        .mount("/metrics", prometheus)
        .mount("/", rocket_cors::catch_all_options_routes())
        .mount("/", ratelimiter::routes())
        .mount("/swagger/", swagger)
        .mount("/0.8/swagger/", swagger_0_8)
        .manage(authifier)
        .manage(db)
        .manage(amqp)
        .manage(cors.clone())
        .manage(voice_client)
        .manage(ratelimits)
        .attach(ratelimiter::RatelimitFairing)
        .attach(cors)
        .configure(rocket::Config {
            limits: rocket::data::Limits::default().limit("string", 5.megabytes()),
            address: Ipv4Addr::new(0, 0, 0, 0).into(),
            port: 14702,
            ..Default::default()
        })
}

#[launch]
async fn rocket() -> _ {
    // Configure logging and environment
    revolt_config::configure!(api);

    // Start web server
    web().await
}
