Feature: Trips
  A trip is a ride someone offers: when it leaves, where from, where to, the
  stops it makes, and how many seats are left. Only the driver who published it
  may change or cancel it.

  Background:
    Given I am signed in as "ada@example.com"
    And a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Gare de Lille" at 50.637900, 3.070600
    And a location "Arras" at 50.287000, 2.781700

  Scenario: Publishing a trip
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": 3
      }
      """
    Then the response status is 201
    And the response field "created_by" is the user "ada@example.com"
    And the response field "start_location.name" is "Gare de Lyon"
    And the response field "end_location.name" is "Gare de Lille"
    And the response field "available_seats" is "3"
    And the response holds 0 items in "stops"
    And the database holds 1 trips

  Scenario: Publishing a trip with stops keeps their order
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": 2,
        "stop_location_ids": [{location:Arras}]
      }
      """
    Then the response status is 201
    And the response field "stops.0.stop_order" is "0"
    And the response field "stops.0.location.name" is "Arras"

  Scenario: A round trip leaves from and returns to the same place
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lyon},
        "available_seats": 4,
        "stop_location_ids": [{location:Arras}]
      }
      """
    Then the response status is 201
    And the response field "start_location.name" is "Gare de Lyon"
    And the response field "end_location.name" is "Gare de Lyon"

  Scenario: Publishing needs an account
    When I anonymously POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": 3
      }
      """
    Then the response status is 401
    And the database holds 0 trips

  Scenario: A trip cannot leave in the past
    When I POST "/trips" with:
      """
      {
        "start_date": "{ago:1 day}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": 3
      }
      """
    Then the response status is 400
    And the error message contains "must be in the future"

  Scenario: A trip cannot point at a location that does not exist
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": 424242,
        "available_seats": 3
      }
      """
    Then the response status is 400
    And the error message contains "unknown location ids: 424242"

  Scenario: A stop cannot repeat the departure
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": 3,
        "stop_location_ids": [{location:Gare de Lyon}]
      }
      """
    Then the response status is 400
    And the error message contains "already the departure or the arrival"

  Scenario: The same stop cannot appear twice
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": 3,
        "stop_location_ids": [{location:Arras}, {location:Arras}]
      }
      """
    Then the response status is 400
    And the error message contains "appears twice"

  Scenario: Seats cannot be negative
    When I POST "/trips" with:
      """
      {
        "start_date": "{in:3 days}",
        "start_location_id": {location:Gare de Lyon},
        "end_location_id": {location:Gare de Lille},
        "available_seats": -1
      }
      """
    Then the response status is 400
    And the validation details mention "available_seats"

  Scenario: Reading a trip back
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And the trip "morning ride" stops at "Arras"
    When I anonymously GET "/trips/{trip:morning ride}"
    Then the response status is 200
    And the response field "start_location.name" is "Gare de Lyon"
    And the response field "stops.0.location.name" is "Arras"

  Scenario: Reading a trip that does not exist
    When I anonymously GET "/trips/424242"
    Then the response status is 404

  Scenario: Listing trips
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And a trip "evening ride" published by "ada@example.com" from "Gare de Lille" to "Gare de Lyon" in 4 days with 1 seats
    When I anonymously GET "/trips"
    Then the response status is 200
    And the response holds 2 items

  Scenario: Filtering trips by departure location
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And a trip "evening ride" published by "ada@example.com" from "Gare de Lille" to "Gare de Lyon" in 4 days with 1 seats
    When I anonymously GET "/trips?start_location_id={location:Gare de Lyon}"
    Then the response status is 200
    And the response holds 1 items
    And the response field "0.end_location.name" is "Gare de Lille"

  Scenario: Filtering trips by remaining seats
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And a trip "full ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 4 days with 0 seats
    When I anonymously GET "/trips?min_available_seats=1"
    Then the response status is 200
    And the response holds 1 items
    And the response field "0.available_seats" is "3"

  Scenario: Filtering trips by driver
    Given a registered user "grace@example.com"
    And a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And a trip "other ride" published by "grace@example.com" from "Gare de Lille" to "Gare de Lyon" in 3 days with 3 seats
    When I anonymously GET "/trips?created_by={user:grace@example.com}"
    Then the response status is 200
    And the response holds 1 items
    And the response field "0.created_by" is the user "grace@example.com"

  Scenario: Filtering trips by departure window
    Given a trip "soon ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 2 days with 3 seats
    And a trip "later ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 20 days with 3 seats
    When I anonymously GET "/trips?departing_before={in:5 days}"
    Then the response status is 200
    And the response holds 1 items

  Scenario: A backwards departure window is refused
    When I anonymously GET "/trips?departing_after={in:5 days}&departing_before={in:1 days}"
    Then the response status is 400
    And the error message contains "must not be later than"

  Scenario: Searching trips around a departure point
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And a trip "northern ride" published by "ada@example.com" from "Gare de Lille" to "Gare de Lyon" in 3 days with 3 seats
    When I anonymously GET "/trips?latitude=48.85&longitude=2.37&radius_meters=20000"
    Then the response status is 200
    And the response holds 1 items
    And the response field "0.start_location.name" is "Gare de Lyon"

  Scenario: The driver changes the seat count
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    When I PATCH "/trips/{trip:morning ride}" with:
      """
      {"available_seats": 1}
      """
    Then the response status is 200
    And the response field "available_seats" is "1"

  Scenario: Replacing the stops of a trip
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And the trip "morning ride" stops at "Arras"
    When I PATCH "/trips/{trip:morning ride}" with:
      """
      {"stop_location_ids": []}
      """
    Then the response status is 200
    And the response holds 0 items in "stops"

  Scenario: An empty update is refused
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    When I PATCH "/trips/{trip:morning ride}" with:
      """
      {}
      """
    Then the response status is 400
    And the error message contains "nothing to update"

  Scenario: An update is checked against the route it would leave behind
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And the trip "morning ride" stops at "Arras"
    When I PATCH "/trips/{trip:morning ride}" with:
      """
      {"end_location_id": {location:Arras}}
      """
    Then the response status is 400
    And the error message contains "already the departure or the arrival"

  Scenario: Another user cannot change a trip
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And I am signed in as "grace@example.com"
    When I PATCH "/trips/{trip:morning ride}" with:
      """
      {"available_seats": 0}
      """
    Then the response status is 403
    And the error code is "forbidden"

  Scenario: Another user cannot cancel a trip
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And I am signed in as "grace@example.com"
    When I DELETE "/trips/{trip:morning ride}"
    Then the response status is 403
    And the database holds 1 trips

  Scenario: The driver cancels the trip
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    And the trip "morning ride" stops at "Arras"
    When I DELETE "/trips/{trip:morning ride}"
    Then the response status is 204
    And the database holds 0 trips
    And the database holds 0 trip_stops

  Scenario: Cancelling needs an account
    Given a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    When I anonymously DELETE "/trips/{trip:morning ride}"
    Then the response status is 401

  Scenario: Cancelling a trip that does not exist
    When I DELETE "/trips/424242"
    Then the response status is 404
