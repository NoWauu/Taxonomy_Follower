//! Steps for the routing providers.
//!
//! Routing is not exposed over HTTP yet, so these scenarios drive the provider
//! the application state was built with, through the same trait the endpoints
//! will use once they are wired up.

use axum::response::IntoResponse as _;
use cucumber::{then, when};

use api::routing::{Coordinates, RouteProfile, RouteRequest};

use crate::support::world::World;

#[when(expr = "I ask for a {word} route from {string} to {string}")]
async fn route_between(world: &mut World, profile: String, from: String, to: String) {
    let waypoints = vec![point(world, &from), point(world, &to)];
    compute(world, &profile, waypoints).await;
}

#[when(expr = "I ask for a {word} route from {string} through {string} to {string}")]
async fn route_through(world: &mut World, profile: String, from: String, via: String, to: String) {
    let waypoints = vec![point(world, &from), point(world, &via), point(world, &to)];
    compute(world, &profile, waypoints).await;
}

#[when(expr = "I ask for a {word} route through {int} identical waypoints")]
async fn route_through_many(world: &mut World, profile: String, count: usize) {
    let waypoints = vec![
        Coordinates {
            latitude: 48.844_444,
            longitude: 2.373_611,
        };
        count
    ];
    compute(world, &profile, waypoints).await;
}

#[then("the route is an estimate")]
async fn route_is_an_estimate(world: &mut World) {
    assert!(
        route(world).estimated,
        "the route should be flagged as estimated"
    );
}

#[then("the route is not an estimate")]
async fn route_is_not_an_estimate(world: &mut World) {
    assert!(
        !route(world).estimated,
        "the route should not be flagged as estimated"
    );
}

#[then(expr = "the route has {int} legs")]
async fn route_has_legs(world: &mut World, expected: usize) {
    assert_eq!(route(world).legs.len(), expected);
}

#[then(expr = "the route is about {float} km long")]
async fn route_is_about(world: &mut World, kilometres: f64) {
    let actual = route(world).distance_meters / 1_000.0;
    let tolerance = (kilometres * 0.05).max(1.0);

    assert!(
        (actual - kilometres).abs() <= tolerance,
        "expected about {kilometres} km, got {actual:.1} km"
    );
}

#[then(expr = "the route geometry is {string}")]
async fn route_geometry_is(world: &mut World, expected: String) {
    assert_eq!(route(world).geometry.as_deref(), Some(expected.as_str()));
}

#[then("the route has no geometry")]
async fn route_has_no_geometry(world: &mut World) {
    assert!(route(world).geometry.is_none());
}

#[then(expr = "routing fails with {string}")]
async fn routing_fails_with(world: &mut World, fragment: String) {
    let error = world
        .route
        .as_ref()
        .expect("a step must ask for a route first")
        .as_ref()
        .err()
        .unwrap_or_else(|| panic!("the route was computed when a failure was expected"));

    assert!(
        error.contains(&fragment),
        "expected the failure to mention `{fragment}`, got `{error}`"
    );
}

/// A provider failure the caller cannot do anything about must not surface as a
/// 4xx: it has to stay an internal error, which is what hides the raw Mapbox
/// message from the client.
#[then("the failure is ours, not the caller's")]
async fn the_failure_is_ours(world: &mut World) {
    let outcome = world
        .routing
        .route(RouteRequest::new(
            vec![
                Coordinates {
                    latitude: 48.844_444,
                    longitude: 2.373_611,
                },
                Coordinates {
                    latitude: 50.637_9,
                    longitude: 3.070_6,
                },
            ],
            RouteProfile::Driving,
        ))
        .await;

    let status = outcome
        .expect_err("the call fails")
        .into_response()
        .status();

    assert_eq!(status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
}

async fn compute(world: &mut World, profile: &str, waypoints: Vec<Coordinates>) {
    let profile: RouteProfile = profile.parse().expect("a known routing profile");
    let outcome = world
        .routing
        .route(RouteRequest::new(waypoints, profile))
        .await
        .map_err(|error| error.to_string());

    world.route = Some(outcome);
}

fn point(world: &World, name: &str) -> Coordinates {
    let (latitude, longitude) = world.coordinates_of(name);

    Coordinates {
        latitude,
        longitude,
    }
}

fn route(world: &World) -> &api::routing::Route {
    world
        .route
        .as_ref()
        .expect("a step must ask for a route first")
        .as_ref()
        .unwrap_or_else(|error| panic!("the route failed: {error}"))
}
