use actix_web::{web, HttpResponse, Scope};
use validator::Validate;

use crate::{
    auth::{Authenticated, RequireAuth}, db::UserExt, dtos::{FilterUserDto, NameUpdateDto, RequestQueryDto, Response, RoleUpdateDto, UserData, UserListResponseDto, UserPasswordUpdateDto, UserResponseDto}, error::HttpError, models::UserRole, utils::password, AppState
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
        .route(
            "/me/name",
            web::put().to(update_user_name).wrap(RequireAuth::allowed_roles(vec![
                UserRole::User,
                UserRole::Moderator,
                UserRole::Admin,
            ])) 
        )
        .route(
            "/me/role",
            web::put().to(update_user_role).wrap(RequireAuth::allowed_roles(vec![
                UserRole::User,
                UserRole::Moderator,
                UserRole::Admin,
            ])) 
        )
        .route(
            "/me/password", 
            web::put().to(update_user_password).wrap(RequireAuth::allowed_roles(vec![
                UserRole::User,
                UserRole::Moderator,
                UserRole::Admin,
            ]))
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

    let users_count = app_state
            .db_client
            .get_user_count()
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    Ok(HttpResponse::Ok().json(UserListResponseDto {
        status: "success".to_string(),
        users: FilterUserDto::filter_users(&users),
        results: users_count,
    }))
}


#[utoipa::path(
    put,
    path = "/api/users/me/name",
    tag = "Update User Name Endpoint",
    request_body(content = NameUpdateDto, example = json!({"name": "john doe"})),
    responses(
        (status = 200, description = "User name updated successfully", body = UserResponseDto),
        (status = 400, description = "Invalid request data", body = Response),
        (status = 401, description = "Unauthorized", body = Response),
        (status = 403, description = "Forbidden", body = Response),
        (status = 500, description = "Internal server error", body = Response)
    ),
    security(
        ("token" = [])
    )
)]
pub async fn update_user_name(
    user: Authenticated,
    body: web::Json<NameUpdateDto>,
    app_state: web::Data<AppState>
) -> Result<HttpResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bat_request(e.to_string()))?;

    let result = app_state.db_client
            .update_user_name(user.id, &body.name)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&result);

    Ok(HttpResponse::Ok().json(UserResponseDto {
        status: "success".to_string(),
        data: UserData { 
            user: filtered_user, 
        }
    }))
}

#[utoipa::path(
    put,
    path = "/api/users/me/role",
    tag = "Update User Role Endpoint",
    request_body(content = RoleUpdateDto, example = json!({"role": "User"})),
    responses(
        (status = 200, description = "User role updated successfully", body = UserResponseDto),
        (status = 400, description = "Invalid request data", body = Response),
        (status = 401, description = "Unauthorized", body = Response),
        (status = 403, description = "Forbidden", body = Response),
        (status = 500, description = "Internal server error", body = Response)
    ),
    security(
        ("token" = [])
    )
)]
pub async fn update_user_role(
    user: Authenticated,
    body: web::Json<RoleUpdateDto>,
    app_state: web::Data<AppState>
) -> Result<HttpResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bat_request(e.to_string()))?;

    let result = app_state.db_client
            .update_user_role(user.id, body.role)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    let filtered_user = FilterUserDto::filter_user(&result);

    Ok(HttpResponse::Ok().json(UserResponseDto {
        status: "success".to_string(),
        data: UserData { 
            user: filtered_user, 
        }
    }))
}

#[utoipa::path(
    put,
    path = "/api/users/me/password",
    tag = "Update User Password Endpoint",
    request_body(content = UserPasswordUpdateDto, example = json!({
        "new_password": "password1234",
        "new_password_confirm": "password1234",
        "old_password": "password123",
    })),
    responses(
        (status = 200, description = "Password updated successfully", body = Response),
        (status = 400, description = "Invalid request data", body = Response),
        (status = 401, description = "Unauthorized", body = Response),
        (status = 403, description = "Forbidden", body = Response),
        (status = 500, description = "Internal server error", body = Response)
    ),
    security(
        ("token" = [])
    )
)]
pub async fn update_user_password(
    user: Authenticated,
    body: web::Json<UserPasswordUpdateDto>,
    app_state: web::Data<AppState>
) -> Result<HttpResponse, HttpError> {
    body.validate()
        .map_err(|e| HttpError::bat_request(e.to_string()))?;

    let user_id = uuid::Uuid::parse_str(&user.id.to_string()).unwrap();

    let result = app_state
                    .db_client
                    .get_user(Some(user_id.clone()), None, None)
                    .await
                    .map_err(|e| HttpError::server_error(e.to_string()))?;

    let user = result.ok_or(HttpError::bat_request("Server error"))?;

    let password_match = password::compare(&body.old_password, &user.password)
                .map_err(|e| HttpError::bat_request(e.to_string()))?;

    if !password_match {
        return Err(HttpError::server_error("Old password is incorrect".to_string()))?;
    } 

    let hashed_password = password::hash(&body.new_password)
                .map_err(|e| HttpError::server_error(e.to_string()))?;

    app_state.db_client
            .update_user_password(user_id.clone(), hashed_password)
            .await
            .map_err(|e| HttpError::server_error(e.to_string()))?;

    let response = Response {
        message: "Password updated successfull".to_string(),
        status: "success",
    };

    Ok(HttpResponse::Ok().json(response))
}