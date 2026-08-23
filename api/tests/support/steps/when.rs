//! Steps that perform the action under test.
//!
//! Paths and bodies go through the world's placeholder expansion, so a feature
//! can write `/trips/{trip:morning ride}` or `"start_location_id": {location:Gare de Lyon}`
//! instead of hard-coding identifiers no one can predict.

use axum::http::Method;
use cucumber::when;

use crate::support::world::{FIXTURE_PASSWORD, World};

#[when(expr = "I GET {string}")]
async fn i_get(world: &mut World, path: String) {
    world.send(Method::GET, &path, None, true).await;
}

#[when(expr = "I DELETE {string}")]
async fn i_delete(world: &mut World, path: String) {
    world.send(Method::DELETE, &path, None, true).await;
}

#[when(expr = "I POST {string} with:")]
async fn i_post(world: &mut World, path: String, step: &cucumber::gherkin::Step) {
    let body = docstring(step);
    world.send(Method::POST, &path, Some(body), true).await;
}

#[when(expr = "I PATCH {string} with:")]
async fn i_patch(world: &mut World, path: String, step: &cucumber::gherkin::Step) {
    let body = docstring(step);
    world.send(Method::PATCH, &path, Some(body), true).await;
}

#[when(expr = "I POST {string} with no body")]
async fn i_post_without_body(world: &mut World, path: String) {
    world
        .send(Method::POST, &path, Some("{}".to_string()), true)
        .await;
}

#[when(expr = "I anonymously POST {string} with no body")]
async fn anonymous_post_without_body(world: &mut World, path: String) {
    world
        .send(Method::POST, &path, Some("{}".to_string()), false)
        .await;
}

/// Same as `I POST ... with:`, with the bearer token deliberately left out.
#[when(expr = "I anonymously POST {string} with:")]
async fn anonymous_post(world: &mut World, path: String, step: &cucumber::gherkin::Step) {
    let body = docstring(step);
    world.send(Method::POST, &path, Some(body), false).await;
}

#[when(expr = "I anonymously PATCH {string} with:")]
async fn anonymous_patch(world: &mut World, path: String, step: &cucumber::gherkin::Step) {
    let body = docstring(step);
    world.send(Method::PATCH, &path, Some(body), false).await;
}

#[when(expr = "I anonymously GET {string}")]
async fn anonymous_get(world: &mut World, path: String) {
    world.send(Method::GET, &path, None, false).await;
}

#[when(expr = "I anonymously DELETE {string}")]
async fn anonymous_delete(world: &mut World, path: String) {
    world.send(Method::DELETE, &path, None, false).await;
}

#[when(expr = "I sign in as {string} with password {string}")]
async fn i_sign_in(world: &mut World, email: String, password: String) {
    let body = serde_json::json!({ "email": email, "password": password }).to_string();
    world
        .send(Method::POST, "/users/login", Some(body), false)
        .await;
}

#[when(expr = "I sign in as {string}")]
async fn i_sign_in_with_fixture_password(world: &mut World, email: String) {
    i_sign_in(world, email, FIXTURE_PASSWORD.to_string()).await;
}

/// Signs in and keeps the issued tokens, so later steps act as that user.
#[when(expr = "{string} signs in again")]
async fn signs_in_again(world: &mut World, email: String) {
    let password = world.user(&email).password.clone();
    i_sign_in(world, email.clone(), password).await;

    if let Some(body) = world.last().body.clone()
        && let Some(user) = world.users.get_mut(&email)
    {
        user.access_token = body["access_token"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        user.refresh_token = body["refresh_token"]
            .as_str()
            .unwrap_or_default()
            .to_string();
    }

    world.current_user = Some(email);
}

/// Swaps the current user's refresh token for the freshly issued pair, so a
/// following step can prove the old one no longer works.
#[when("I refresh my session")]
async fn i_refresh_my_session(world: &mut World) {
    let body = serde_json::json!({ "refresh_token": "{token:refresh}" }).to_string();
    world
        .send(Method::POST, "/users/token/refresh", Some(body), false)
        .await;

    if let (Some(body), Some(alias)) = (world.last().body.clone(), world.current_user.clone())
        && let Some(user) = world.users.get_mut(&alias)
        && let Some(refresh) = body["refresh_token"].as_str()
    {
        user.access_token = body["access_token"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        user.refresh_token = refresh.to_string();
    }
}

fn docstring(step: &cucumber::gherkin::Step) -> String {
    step.docstring
        .as_ref()
        .expect("this step needs a docstring holding the request body")
        .trim()
        .to_string()
}
