Feature: Locations
  Locations are the shared reference data trips point at: a departure, an
  arrival, and the stops in between. Anyone signed in can add one; anyone at
  all can read them.

  Background:
    Given I am signed in as "ada@example.com"

  Scenario: Creating a location
    When I POST "/locations" with:
      """
      {"name": "Gare de Lyon", "latitude": 48.844444, "longitude": 2.373611}
      """
    Then the response status is 201
    And the response field "name" is "Gare de Lyon"
    And the response field "latitude" is "48.844444"
    And the response field "longitude" is "2.373611"
    And the response field "distance_meters" is absent
    And the database holds 1 locations

  Scenario: Creating a location needs an account
    When I anonymously POST "/locations" with:
      """
      {"name": "Gare de Lyon", "latitude": 48.844444, "longitude": 2.373611}
      """
    Then the response status is 401
    And the database holds 0 locations

  Scenario: Coordinates outside the globe are refused
    When I POST "/locations" with:
      """
      {"name": "Nowhere", "latitude": 91.0, "longitude": 2.373611}
      """
    Then the response status is 400
    And the validation details mention "latitude"

  Scenario: A nameless location is refused
    When I POST "/locations" with:
      """
      {"name": "", "latitude": 48.844444, "longitude": 2.373611}
      """
    Then the response status is 400
    And the validation details mention "name"

  Scenario: Reading a location back
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I anonymously GET "/locations/{location:Gare de Lyon}"
    Then the response status is 200
    And the response field "name" is "Gare de Lyon"

  Scenario: Reading a location that does not exist
    When I anonymously GET "/locations/424242"
    Then the response status is 404
    And the error code is "not_found"

  Scenario: Listing every location
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Gare de Lille" at 50.637900, 3.070600
    When I anonymously GET "/locations"
    Then the response status is 200
    And the response holds 2 items

  Scenario: Filtering locations by name
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Aéroport d'Orly" at 48.723300, 2.379400
    When I anonymously GET "/locations?q=gare"
    Then the response status is 200
    And the response holds 1 items
    And the response field "0.name" is "Gare de Lyon"

  Scenario: The name filter treats wildcards as plain characters
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I anonymously GET "/locations?q=%25"
    Then the response status is 200
    And the response holds 0 items

  Scenario: Searching around a point keeps the neighbours and reports distances
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Gare de Lille" at 50.637900, 3.070600
    When I anonymously GET "/locations?latitude=48.85&longitude=2.37&radius_meters=20000"
    Then the response status is 200
    And the response holds 1 items
    And the response field "0.name" is "Gare de Lyon"
    And the response field "0.distance_meters" is at most 2000.0

  Scenario: Half a point is not a point
    When I anonymously GET "/locations?latitude=48.85"
    Then the response status is 400
    And the error message contains "must be provided together"

  Scenario: A radius without a point is refused
    When I anonymously GET "/locations?radius_meters=5000"
    Then the response status is 400
    And the error message contains "requires `latitude` and `longitude`"

  Scenario: Paging through locations
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Gare de Lille" at 50.637900, 3.070600
    When I anonymously GET "/locations?limit=1&offset=1"
    Then the response status is 200
    And the response holds 1 items

  Scenario: Renaming a location
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I PATCH "/locations/{location:Gare de Lyon}" with:
      """
      {"name": "Paris Gare de Lyon"}
      """
    Then the response status is 200
    And the response field "name" is "Paris Gare de Lyon"
    And the response field "latitude" is "48.844444"

  Scenario: Moving a location needs both coordinates
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I PATCH "/locations/{location:Gare de Lyon}" with:
      """
      {"latitude": 48.85}
      """
    Then the response status is 400
    And the error message contains "must be updated together"

  Scenario: An empty update is refused
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I PATCH "/locations/{location:Gare de Lyon}" with:
      """
      {}
      """
    Then the response status is 400
    And the error message contains "nothing to update"

  Scenario: Updating needs an account
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I anonymously PATCH "/locations/{location:Gare de Lyon}" with:
      """
      {"name": "Paris Gare de Lyon"}
      """
    Then the response status is 401

  Scenario: Deleting a location
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    When I DELETE "/locations/{location:Gare de Lyon}"
    Then the response status is 204
    And the response body is empty
    And the database holds 0 locations

  Scenario: Deleting a location that does not exist
    When I DELETE "/locations/424242"
    Then the response status is 404

  Scenario: A location a trip depends on cannot be deleted
    Given a location "Gare de Lyon" at 48.844444, 2.373611
    And a location "Gare de Lille" at 50.637900, 3.070600
    And a trip "morning ride" published by "ada@example.com" from "Gare de Lyon" to "Gare de Lille" in 3 days with 3 seats
    When I DELETE "/locations/{location:Gare de Lyon}"
    Then the response status is 409
    And the error code is "conflict"
    And the database holds 2 locations
