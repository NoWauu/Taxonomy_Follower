Feature: Routing
  Turning the points of a trip into a distance, a duration and a line to draw.
  With a Mapbox token the real Directions API answers; without one the API
  falls back to straight lines, which it flags as estimates.

  Background:
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Gare de Lille" at 50.637900, 3.070600
    And a location "Arras" at 50.287000, 2.781700

  Scenario: Without a token, routes are straight-line estimates
    Given no routing provider is configured
    When I ask for a driving route from "Gare de Lyon" to "Gare de Lille"
    Then the route is an estimate
    And the route has 1 legs
    And the route is about 204.0 km long
    And the route has no geometry

  Scenario: A stop adds a leg
    Given no routing provider is configured
    When I ask for a driving route from "Gare de Lyon" through "Arras" to "Gare de Lille"
    Then the route has 2 legs
    And the route is about 207.0 km long

  Scenario: Mapbox answers with the real itinerary
    Given the Mapbox Directions API answers with:
      """
      {
        "code": "Ok",
        "routes": [
          {
            "distance": 225300.4,
            "duration": 8100.2,
            "geometry": "ynh~Fk`nSaAbCdEjB",
            "legs": [
              {"distance": 120000.0, "duration": 4300.0},
              {"distance": 105300.4, "duration": 3800.2}
            ]
          }
        ]
      }
      """
    When I ask for a driving route from "Gare de Lyon" through "Arras" to "Gare de Lille"
    Then the route is not an estimate
    And the route has 2 legs
    And the route is about 225.3 km long
    And the route geometry is "ynh~Fk`nSaAbCdEjB"

  Scenario: Mapbox knows the waypoints but finds no road between them
    Given the Mapbox Directions API answers with:
      """
      {"code": "NoRoute", "message": "No route found"}
      """
    When I ask for a driving route from "Gare de Lyon" to "Gare de Lille"
    Then routing fails with "no route connects these locations"

  Scenario: A rejected token is our problem, not the caller's
    Given the Mapbox Directions API answers HTTP 401 with:
      """
      {"message": "Not Authorized - Invalid Token"}
      """
    When I ask for a driving route from "Gare de Lyon" to "Gare de Lille"
    Then routing fails with "HTTP 401"
    And the failure is ours, not the caller's

  Scenario: Traffic-aware routing is capped at three waypoints
    Given no routing provider is configured
    When I ask for a driving-traffic route through 4 identical waypoints
    Then routing fails with "at most 3 waypoints"

  Scenario: A route needs somewhere to go
    Given no routing provider is configured
    When I ask for a driving route through 1 identical waypoints
    Then routing fails with "at least a departure and an arrival"
