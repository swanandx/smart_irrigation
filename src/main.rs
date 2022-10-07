use actix_web::{dev::ServiceRequest, get, post, web, App, Error, HttpServer, Responder, HttpResponse, HttpResponseBuilder, http::StatusCode};
use actix_web_httpauth::{
    extractors::{
        bearer::{self, BearerAuth},
        AuthenticationError,
    },
    middleware::HttpAuthentication,
};

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
async fn turn_off() -> impl Responder {
    web::Json("Turned motor OFF\n")
}

#[get("/on", wrap = "HttpAuthentication::bearer(validator)")]
async fn turn_on() -> impl Responder {
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

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .app_data(web::Data::new(
                get_data().expect("Failed to get sensor data"),
            ))
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
