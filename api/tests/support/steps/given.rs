//! Steps that put the world into a known state before the request under test.

use std::time::Duration;

use axum::http::Method;
use chrono::Utc;
use cucumber::given;

use crate::support::world::{FIXTURE_PASSWORD, World};

#[given(expr = "a registered user {string}")]
async fn a_registered_user(world: &mut World, email: String) {
    world
        .register(&email, FIXTURE_PASSWORD)
        .await
        .expect("the fixture account is created");
}

#[given(expr = "a registered user {string} with password {string}")]
async fn a_registered_user_with_password(world: &mut World, email: String, password: String) {
    world
        .register(&email, &password)
        .await
        .expect("the fixture account is created");
}

#[given(expr = "I am signed in as {string}")]
async fn i_am_signed_in_as(world: &mut World, email: String) {
    if !world.users.contains_key(&email) {
        world
            .register(&email, FIXTURE_PASSWORD)
            .await
            .expect("the fixture account is created");
    }

    world.current_user = Some(email);
}

#[given("I am not signed in")]
async fn i_am_not_signed_in(world: &mut World) {
    world.current_user = None;
}

#[given(expr = "a location {string} at {float}, {float}")]
async fn a_location(world: &mut World, name: String, latitude: f64, longitude: f64) {
    world
        .insert_location(&name, latitude, longitude)
        .await
        .expect("the fixture location is inserted");
}

#[given(
    expr = "a trip {string} published by {string} from {string} to {string} in {int} days with {int} seats"
)]
async fn a_trip(
    world: &mut World,
    name: String,
    owner: String,
    start: String,
    end: String,
    days: i64,
    seats: i32,
) {
    let owner_id = world.user(&owner).id;
    let start_id = world.location_id(&start);
    let end_id = world.location_id(&end);

    world
        .insert_trip(
            &name,
            owner_id,
            Utc::now() + chrono::Duration::days(days),
            start_id,
            end_id,
            seats,
        )
        .await
        .expect("the fixture trip is inserted");
}

#[given(expr = "the trip {string} stops at {string}")]
async fn the_trip_stops_at(world: &mut World, trip: String, location: String) {
    let trip_id = world.trip_id(&trip);
    let location_id = world.location_id(&location);
    let stop_order: i32 = sqlx::query_scalar(
        "SELECT coalesce(max(stop_order) + 1, 0) FROM trip_stops WHERE trip_id = $1",
    )
    .bind(trip_id)
    .fetch_one(&world.db)
    .await
    .expect("the next stop order is readable");

    world
        .insert_stop(trip_id, location_id, stop_order)
        .await
        .expect("the fixture stop is inserted");
}

/// Points the routing provider at the stub and arms it with a canned answer.
#[given("the Mapbox Directions API answers with:")]
async fn mapbox_answers_with(world: &mut World, step: &cucumber::gherkin::Step) {
    let body = step
        .docstring
        .as_ref()
        .expect("this step needs a docstring holding the canned response")
        .trim()
        .to_string();

    world.mapbox.answer_with(axum::http::StatusCode::OK, body);
    world.use_mapbox();
}

#[given(expr = "the Mapbox Directions API answers HTTP {int} with:")]
async fn mapbox_answers_status_with(
    world: &mut World,
    status: u16,
    step: &cucumber::gherkin::Step,
) {
    let body = step
        .docstring
        .as_ref()
        .expect("this step needs a docstring holding the canned response")
        .trim()
        .to_string();

    world.mapbox.answer_with(
        axum::http::StatusCode::from_u16(status).expect("a valid status code"),
        body,
    );
    world.use_mapbox();
}

#[given("no routing provider is configured")]
async fn no_routing_provider(world: &mut World) {
    world.use_fallback_routing();
}

/// A reset link the user never asked for, to check it is refused.
#[given(expr = "{string} waited {int} hour for the reset link")]
async fn waited(world: &mut World, email: String, hours: i64) {
    let user_id = world.user(&email).id;
    sqlx::query("UPDATE password_reset_tokens SET expires_at = $2 WHERE user_id = $1")
        .bind(user_id)
        .bind(Utc::now() - chrono::Duration::hours(hours))
        .execute(&world.db)
        .await
        .expect("the reset token is aged");
}

#[given(expr = "{string} asked for a password reset")]
async fn asked_for_a_password_reset(world: &mut World, email: String) {
    let body = serde_json::json!({ "email": email }).to_string();
    world
        .send(Method::POST, "/users/password/forgot", Some(body), false)
        .await;

    let mail = world
        .mails
        .wait_for(&email, Duration::from_secs(2))
        .await
        .expect("a password reset email reaches the mailbox");

    world.reset_token = Some(extract_reset_token(&mail.text_body));
}

/// Digs the token out of the `?token=` of the reset link in the email body.
fn extract_reset_token(body: &str) -> String {
    body.split("token=")
        .nth(1)
        .expect("the reset email carries a link with a token")
        .split(|c: char| c.is_whitespace() || c == '"' || c == '>' || c == '&')
        .next()
        .expect("the token is not empty")
        .to_string()
}
