use actix_web::{
    cookie::time::Duration as ActixWebDuration, cookie::Cookie, web, HttpResponse, Responder, Scope,
};
use serde_json::json;
use validator::Validate;

use crate::{
    db::UserExt,
    dtos::{
        FilterUserDto, LoginUserDto, RegisterUserDto, UserData, UserLoginResponseDto,
        UserResponseDto,
    },
    error::{ErrorMessage, HttpError},
    auth::RequireAuth,
    models::UserRole,
    utils::{password, token},
    AppState,
};

pub fn auth_handler() -> Scope {
    web::scope("/api/auth")
        .route("/register", web::post().to(register))
        .route("/login", web::post().to(login))
        .route(
            "/logout",
            web::post().to(logout).wrap(RequireAuth::allowed_roles(vec![
                UserRole::User,
                UserRole::Moderator,
                UserRole::Admin,
            ])) 
        )
}

#[utoipa::path(
    post,
    path = "/api/auth/register",
    tag = "Register Account Endpoint",
    request_body(content = RegisterUserDto, description = "Credentials to create account", example = json!({"email": "johndoe@example.com","name": "John Doe","password": "password123","passwordConfirm": "password123"})),
    responses(
        (status=201, description= "Account created successfully", body= UserResponseDto ),
        (status=400, description= "Validation Errors", body= Response),
        (status=409, description= "User with email already exists", body= Response),
        (status=500, description= "Internal Server Error", body= Response ),
    )
)]
pub async fn register(
    app_state: web::Data<AppState>,
    body: web::Json<RegisterUserDto>,
) -> Result<HttpResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bat_request(e.to_string()))?;

    let hashed_password = 
        password::hash(&body.password).map_err(|e| HttpError::server_error(e.to_string()))?;

    let result = app_state
            .db_client
            .save_user(&body.name, &body.email, &hashed_password)
            .await;

    match result {
        Ok(user) => Ok(HttpResponse::Created().json(UserResponseDto {
            status: "success".to_string(),
            data: UserData {
                user: FilterUserDto::filter_user(&user),
            }
        })),
        Err(sqlx::Error::Database(db_err)) => {
            if db_err.is_unique_violation() {
                Err(HttpError::unique_constraint_voilation(
                    ErrorMessage::EmailExist,
                ))
            } else {
                Err(HttpError::server_error(db_err.to_string()))
            }
        }
        Err(e) => Err(HttpError::server_error(e.to_string())),
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/login",
    tag = "Login Endpoint",
    request_body(content = LoginUserDto, description = "Credentials to log in to your account", example = json!({"email": "johndoe@example.com","password": "password123"})),
    responses(
        (status=200, description= "Login successfull", body= UserLoginResponseDto ),
        (status=400, description= "Validation Errors", body= Response ),
        (status=500, description= "Internal Server Error", body= Response ),
    )
)]
pub async fn login(
    app_state: web::Data<AppState>,
    body: web::Json<LoginUserDto>,
) -> Result<HttpResponse, HttpError> {
    body.validate()
       .map_err(|e| HttpError::bat_request(e.to_string()))?;

    let result = app_state
                    .db_client
                    .get_user(None, None, Some(&body.email))
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;
    let user = result.ok_or(HttpError::unauthorized(ErrorMessage::WrongCredentials))?;

    let password_matches = password::compare(&body.password, &user.password)
        .map_err(|_| HttpError::unauthorized(ErrorMessage::WrongCredentials))?;

    if password_matches {
        let token = token::create_token(
                &user.id.to_string(), 
                &app_state.env.jwt_secret.as_bytes(), 
                app_state.env.jwt_maxage
            )
            .map_err(|e| HttpError::server_error(e.to_string()))?;

        let cookie = Cookie::build("token", token.to_owned())
                    .path("/")
                    .max_age(ActixWebDuration::new(60 * &app_state.env.jwt_maxage, 0))
                    .http_only(true)
                    .finish();
        Ok(HttpResponse::Ok()
            .cookie(cookie)
            .json(UserLoginResponseDto {
                status: "success".to_string(),
                token,
            })
        )
    } else {
        Err(HttpError::unauthorized(ErrorMessage::WrongCredentials))
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "Logout Endpoint",
    responses(
        (status=200, description= "Logout successfull" ),
        (status=400, description= "Validation Errors", body= Response ),
        (status=401, description= "Unauthorize Error", body= Response),
        (status=500, description= "Internal Server Error", body= Response ),
    ),
    security(
       ("token" = [])
   )
)]
pub async fn logout() -> impl Responder {
    let cookie = Cookie::build("token", "")
            .path("/")
            .max_age(ActixWebDuration::new(-1, 0))
            .http_only(true)
            .finish();
    
    HttpResponse::Ok()
        .cookie(cookie)
        .json(json!({"status": "success"}))
}