//! Assertions on what the API answered.

use std::time::Duration;

use cucumber::then;
use serde_json::Value;

use crate::support::world::World;

#[then(expr = "the response status is {int}")]
async fn status_is(world: &mut World, expected: u16) {
    let recorded = world.last();
    assert_eq!(
        recorded.status.as_u16(),
        expected,
        "unexpected status, body was: {}",
        recorded.raw
    );
}

#[then(expr = "the error code is {string}")]
async fn error_code_is(world: &mut World, expected: String) {
    let actual = world
        .field("error")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    assert_eq!(actual, expected, "body was: {}", world.last().raw);
}

#[then(expr = "the error message contains {string}")]
async fn error_message_contains(world: &mut World, fragment: String) {
    let message = world
        .field("message")
        .and_then(Value::as_str)
        .unwrap_or_default();

    assert!(
        message.contains(&fragment),
        "expected the message to contain `{fragment}`, got `{message}`"
    );
}

#[then(expr = "the validation details mention {string}")]
async fn details_mention(world: &mut World, field: String) {
    let details = world
        .field("details")
        .expect("a validation error carries details");

    assert!(
        details.get(&field).is_some(),
        "expected `{field}` among the validation details, got {details}"
    );
}

/// Compares a field to an expected value written in the feature file.
///
/// The expectation is parsed as JSON first, so `3`, `true` and `"Gare de Lyon"`
/// all compare the way a reader expects; anything else falls back to a string
/// comparison.
#[then(expr = "the response field {string} is {string}")]
async fn field_is(world: &mut World, path: String, expected: String) {
    let actual = world
        .field(&path)
        .unwrap_or_else(|| panic!("no `{path}` in the response: {}", world.last().raw));

    let matches = match serde_json::from_str::<Value>(&expected) {
        Ok(value) if !value.is_string() => &value == actual,
        _ => actual.as_str() == Some(expected.as_str()),
    };

    assert!(
        matches,
        "expected `{path}` to be `{expected}`, got `{actual}`"
    );
}

#[then(expr = "the response field {string} is the id of {string}")]
async fn field_is_the_id_of(world: &mut World, path: String, name: String) {
    let expected = world.resolve(&format!("{{location:{name}}}"));
    let actual = world
        .field(&path)
        .unwrap_or_else(|| panic!("no `{path}` in the response: {}", world.last().raw));

    assert_eq!(actual.to_string(), expected);
}

#[then(expr = "the response field {string} is the user {string}")]
async fn field_is_the_user(world: &mut World, path: String, email: String) {
    let expected = world.user(&email).id.to_string();
    let actual = world
        .field(&path)
        .and_then(Value::as_str)
        .unwrap_or_default();

    assert_eq!(actual, expected);
}

#[then(expr = "the response field {string} is absent")]
async fn field_is_absent(world: &mut World, path: String) {
    assert!(
        world.field(&path).is_none(),
        "expected no `{path}` in {}",
        world.last().raw
    );
}

#[then(expr = "the response field {string} is present")]
async fn field_is_present(world: &mut World, path: String) {
    assert!(
        world.field(&path).is_some(),
        "expected a `{path}` in {}",
        world.last().raw
    );
}

#[then(expr = "the response field {string} is at most {float}")]
async fn field_is_at_most(world: &mut World, path: String, ceiling: f64) {
    let actual = world
        .field(&path)
        .and_then(Value::as_f64)
        .unwrap_or_else(|| panic!("no numeric `{path}` in {}", world.last().raw));

    assert!(
        actual <= ceiling,
        "expected `{path}` <= {ceiling}, got {actual}"
    );
}

#[then(expr = "the response holds {int} items")]
async fn holds_items(world: &mut World, expected: usize) {
    let items = world
        .last()
        .body
        .as_ref()
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("the response is not a list: {}", world.last().raw));

    assert_eq!(
        items.len(),
        expected,
        "unexpected item count, body was: {}",
        world.last().raw
    );
}

/// Counts the items of a list nested in the response, e.g. the stops of a trip.
#[then(expr = "the response holds {int} items in {string}")]
async fn holds_items_in(world: &mut World, expected: usize, path: String) {
    let items = world
        .field(&path)
        .and_then(Value::as_array)
        .unwrap_or_else(|| panic!("`{path}` is not a list: {}", world.last().raw));

    assert_eq!(items.len(), expected, "body was: {}", world.last().raw);
}

#[then(expr = "the response body is empty")]
async fn body_is_empty(world: &mut World) {
    assert!(
        world.last().raw.trim().is_empty(),
        "expected an empty body, got: {}",
        world.last().raw
    );
}

#[then(expr = "the database holds {int} {word}")]
async fn database_holds(world: &mut World, expected: i64, table: String) {
    let actual = world.count(&table).await;
    assert_eq!(actual, expected, "unexpected row count in `{table}`");
}

#[then(expr = "a password reset email is sent to {string}")]
async fn a_reset_email_is_sent(world: &mut World, email: String) {
    let mail = world.mails.wait_for(&email, Duration::from_secs(2)).await;

    assert!(
        mail.is_some(),
        "no password reset email reached `{email}` within two seconds"
    );
}

#[then(expr = "the email to {string} is about {string}")]
async fn the_email_is_about(world: &mut World, email: String, fragment: String) {
    let mail = world
        .mails
        .wait_for(&email, Duration::from_secs(2))
        .await
        .unwrap_or_else(|| panic!("no email reached `{email}`"));

    assert!(
        mail.subject.contains(&fragment),
        "expected the subject to mention `{fragment}`, got `{}`",
        mail.subject
    );
}

#[then(expr = "no email is sent to {string}")]
async fn no_email_is_sent(world: &mut World, email: String) {
    // Nothing to wait for, but the sending task is spawned: give it the same
    // chance to deliver as the positive assertion does.
    tokio::time::sleep(Duration::from_millis(200)).await;

    assert!(
        world.mails.to(&email).is_empty(),
        "an email reached `{email}` when none was expected"
    );
}
