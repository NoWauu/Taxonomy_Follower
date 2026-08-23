Feature: Health probe
  Monitoring needs one endpoint that answers only when the API can actually
  reach its database.

  Scenario: The API and its database are up
    When I anonymously GET "/health"
    Then the response status is 200

  Scenario: The OpenAPI document is served
    When I anonymously GET "/openapi.json"
    Then the response status is 200
    And the response field "info.title" is "Taxonomy Follower API"
