use actix_web::{
    dev::ServiceRequest, get, http::StatusCode, post, web, App, Error, HttpResponse,
    HttpResponseBuilder, HttpServer, Responder,
};
use actix_web_httpauth::{
    extractors::{
        bearer::{self, BearerAuth},
        AuthenticationError,
    },
    middleware::HttpAuthentication,
};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use std::time::Duration;

const TOPIC: &str = "edi_15";

#[derive(serde::Deserialize)]
pub struct FormData {
    username: String,
    password: String,
}

async fn validator(
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (Error, ServiceRequest)> {
    if credentials.token() == "SECRET_KEY" {
        Ok(req)
    } else {
        let config = req
            .app_data::<bearer::Config>()
            .cloned()
            .unwrap_or_default()
            .scope("urn:example:channel=HBO&urn:example:rating=G,PG-13");

        Err((AuthenticationError::from(config).into(), req))
    }
}

#[get("/off", wrap = "HttpAuthentication::bearer(validator)")]
async fn turn_off(data: web::Data<AsyncClient>) -> impl Responder {
    data.publish(TOPIC, QoS::AtMostOnce, false, "OFF")
        .await
        .ok();
    web::Json("Turned motor OFF\n")
}

#[get("/on", wrap = "HttpAuthentication::bearer(validator)")]
async fn turn_on(data: web::Data<AsyncClient>) -> impl Responder {
    data.publish(TOPIC, QoS::AtMostOnce, false, "ON").await.ok();
    web::Json("Turned motor ON\n")
}

#[get("/readings")]
async fn readings(data: web::Data<Vec<u8>>) -> impl Responder {
    web::Json(data)
}

// IMP: REMOVE THIS SMH
const USERNAME: &str = "dx";
const PASSWD: &str = "secure123";

#[post("/login")]
async fn login(form: web::Form<FormData>) -> impl Responder {
    if USERNAME == form.username && PASSWD == form.password {
        return HttpResponse::Ok().body("SECRET_KEY");
    }
    HttpResponseBuilder::new(StatusCode::UNAUTHORIZED).json("Invalid credentials")
}

fn get_data() -> std::io::Result<Vec<u8>> {
    // TODO: Read data from sensors
    Ok([0, 10, 50, 10, 30].to_vec())
}

fn get_mqtt() -> AsyncClient {
    let mut mqttoptions = MqttOptions::new("my-client-x123", "broker.hivemq.com", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(10));

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    tokio::spawn(async move {
        loop {
            let event = eventloop.poll().await;
            match &event {
                // Ok(rumqttc::Event::Outgoing(rumqttc::Outgoing::Publish(v))) => {
                //     println!("Publishing {v:?}");
                // }
                Ok(v) => {
                    println!("Event: {v:?}");
                }
                Err(e) => {
                    println!("Error = {e:?}");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    return;
                }
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    });

    client
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(
                get_data().expect("Failed to get sensor data"),
            ))
            .app_data(web::Data::new(get_mqtt()))
            .service(
                web::scope("/api")
                    .service(turn_off)
                    .service(turn_on)
                    .service(login)
                    .service(readings),
            )
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
