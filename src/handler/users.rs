use actix_web::{web, HttpResponse, Scope};
use validator::Validate;

use crate::{
    auth::{Authenticated, RequireAuth}, 
    db::UserExt, 
    dtos::{FilterUserDto, RequestQueryDto, UserData, UserListResponseDto, UserResponseDto}, 
    error::HttpError, 
    AppState,
    models::UserRole,
};

pub fn users_handler() -> Scope {
    web::scope("/api/users")
        .route(
            "", 
            web::get()
            .to(get_users)
            .wrap(RequireAuth::allowed_roles(vec![UserRole::Admin])),
        )
        .route(
            "/me", 
            web::get().to(get_me).wrap(RequireAuth::allowed_roles(vec![
                UserRole::User,
                UserRole::Moderator,
                UserRole::Admin,
            ])),
        )
}

#[utoipa::path(
    get,
    path = "/api/users/me",
    tag = "Get Authenticated User Endpoint",
    responses(
        (status = 200, description= "Authenticated User", body = UserResponseDto),
        (status= 500, description= "Internal Server Error", body = Response )
       
    ),
    security(
       ("token" = [])
   )
)]
pub async fn get_me(user: Authenticated) -> Result<HttpResponse, HttpError> {
    let filtered_user = FilterUserDto::filter_user(&user);

    let response_data = UserResponseDto {
        status: "success".to_string(),
        data: UserData { 
            user: filtered_user, 
        }
    };

    Ok(HttpResponse::Ok().json(response_data))
}


#[utoipa::path(
    get,
    path = "/api/users",
    tag = "Get All Users Endpoint",
    params(
        RequestQueryDto
    ),
    responses(
        (status = 200, description= "All Users", body = [UserResponseDto]),
        (status=401, description= "Authentication Error", body= Response),
        (status=403, description= "Permission Denied Error", body= Response),
        (status= 500, description= "Internal Server Error", body = Response )
       
    ),
    security(
       ("token" = [])
   )
)]
pub async fn get_users(
    query: web::Query<RequestQueryDto>,
    app_state: web::Data<AppState>,
) -> Result<HttpResponse, HttpError> {
    let query_params: RequestQueryDto = query.into_inner();

    query_params
        .validate()
        .map_err(|e| HttpError::bat_request(e.to_string()))?;

    let page = query_params.page.unwrap_or(1);
    let limit = query_params.limit.unwrap_or(10);

    let users = app_state
            .db_client
            .get_users(page as u32, limit)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(HttpResponse::Ok().json(UserListResponseDto {
        status: "success".to_string(),
        users: FilterUserDto::filter_users(&users),
        results: users.len(),
    }))
}