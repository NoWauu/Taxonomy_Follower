Feature: Accounts and sessions
  Signing up, signing in, keeping a session alive and getting back in after
  forgetting a password.

  Scenario: Signing up returns the account and a token pair
    When I anonymously POST "/users/register" with:
      """
      {
        "email": "ada@example.com",
        "password": "correct-horse-battery-42",
        "display_name": "Ada Lovelace"
      }
      """
    Then the response status is 201
    And the response field "user.email" is "ada@example.com"
    And the response field "user.display_name" is "Ada Lovelace"
    And the response field "user.email_verified" is "false"
    And the response field "token_type" is "Bearer"
    And the response field "access_token" is present
    And the response field "refresh_token" is present
    And the database holds 1 users

  Scenario: An email can only be registered once
    Given a registered user "ada@example.com"
    When I anonymously POST "/users/register" with:
      """
      {"email": "ada@example.com", "password": "another-decent-passphrase"}
      """
    Then the response status is 409
    And the error code is "conflict"
    And the database holds 1 users

  Scenario: Emails are matched case insensitively
    Given a registered user "ada@example.com"
    When I anonymously POST "/users/register" with:
      """
      {"email": "ADA@Example.com", "password": "another-decent-passphrase"}
      """
    Then the response status is 409

  Scenario: A short password is refused
    When I anonymously POST "/users/register" with:
      """
      {"email": "ada@example.com", "password": "too-short"}
      """
    Then the response status is 400
    And the error code is "validation_error"
    And the validation details mention "password"
    And the database holds 0 users

  Scenario: A password built from the email address is refused
    When I anonymously POST "/users/register" with:
      """
      {"email": "ada@example.com", "password": "ada-ada-ada-ada"}
      """
    Then the response status is 400
    And the error message contains "must not contain your email address"

  Scenario: A password with too few distinct characters is refused
    When I anonymously POST "/users/register" with:
      """
      {"email": "ada@example.com", "password": "abababababab"}
      """
    Then the response status is 400
    And the error message contains "5 different characters"

  Scenario: A malformed email is refused
    When I anonymously POST "/users/register" with:
      """
      {"email": "not-an-email", "password": "correct-horse-battery-42"}
      """
    Then the response status is 400
    And the validation details mention "email"

  Scenario: Signing in with the right password
    Given a registered user "ada@example.com"
    When I sign in as "ada@example.com"
    Then the response status is 200
    And the response field "user.email" is "ada@example.com"
    And the response field "access_token" is present

  Scenario: Signing in with the wrong password
    Given a registered user "ada@example.com"
    When I sign in as "ada@example.com" with password "not-the-password"
    Then the response status is 401
    And the error message contains "invalid email or password"

  Scenario: Signing in with an unknown email says the same thing
    When I sign in as "nobody@example.com" with password "correct-horse-battery-42"
    Then the response status is 401
    And the error message contains "invalid email or password"

  Scenario: The current user is behind the access token
    Given I am signed in as "ada@example.com"
    When I GET "/users/me"
    Then the response status is 200
    And the response field "email" is "ada@example.com"

  Scenario: No token, no current user
    Given a registered user "ada@example.com"
    When I anonymously GET "/users/me"
    Then the response status is 401
    And the error code is "unauthorized"

  Scenario: A forged token is refused
    Given I am signed in as "ada@example.com"
    When I anonymously POST "/users/token/verify" with:
      """
      {"token": "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJub3BlIn0.nope"}
      """
    Then the response status is 401

  Scenario: A valid token verifies
    Given I am signed in as "ada@example.com"
    When I POST "/users/token/verify" with:
      """
      {"token": "{token:access}"}
      """
    Then the response status is 200
    And the response field "valid" is "true"
    And the response field "email" is "ada@example.com"

  Scenario: Refreshing rotates the refresh token
    Given I am signed in as "ada@example.com"
    When I refresh my session
    Then the response status is 200
    And the response field "access_token" is present
    And the response field "refresh_token" is present

  Scenario: A refresh token cannot be replayed
    Given I am signed in as "ada@example.com"
    When I POST "/users/token/refresh" with:
      """
      {"refresh_token": "{token:refresh}"}
      """
    Then the response status is 200
    When I POST "/users/token/refresh" with:
      """
      {"refresh_token": "{token:refresh}"}
      """
    Then the response status is 401
    And the error message contains "invalid or expired refresh token"

  Scenario: Logging out kills the refresh token
    Given I am signed in as "ada@example.com"
    When I POST "/users/logout" with:
      """
      {"refresh_token": "{token:refresh}"}
      """
    Then the response status is 200
    When I POST "/users/token/refresh" with:
      """
      {"refresh_token": "{token:refresh}"}
      """
    Then the response status is 401

  Scenario: Logging out everywhere needs an access token
    Given a registered user "ada@example.com"
    When I anonymously POST "/users/logout/all" with no body
    Then the response status is 401

  Scenario: Logging out everywhere kills every session
    Given I am signed in as "ada@example.com"
    When I POST "/users/logout/all" with no body
    Then the response status is 200
    When I POST "/users/token/refresh" with:
      """
      {"refresh_token": "{token:refresh}"}
      """
    Then the response status is 401

  Scenario: Asking for a reset link sends an email
    Given a registered user "ada@example.com"
    When I anonymously POST "/users/password/forgot" with:
      """
      {"email": "ada@example.com"}
      """
    Then the response status is 200
    And a password reset email is sent to "ada@example.com"
    And the email to "ada@example.com" is about "password"

  Scenario: Asking for a reset link on an unknown address gives nothing away
    When I anonymously POST "/users/password/forgot" with:
      """
      {"email": "nobody@example.com"}
      """
    Then the response status is 200
    And no email is sent to "nobody@example.com"

  Scenario: Resetting the password with the emailed token
    Given a registered user "ada@example.com"
    And "ada@example.com" asked for a password reset
    When I anonymously POST "/users/password/reset" with:
      """
      {"token": "{token:reset}", "password": "brand-new-passphrase-77"}
      """
    Then the response status is 200
    When I sign in as "ada@example.com" with password "brand-new-passphrase-77"
    Then the response status is 200
    When I sign in as "ada@example.com" with password "correct-horse-battery-42"
    Then the response status is 401

  Scenario: A reset token only works once
    Given a registered user "ada@example.com"
    And "ada@example.com" asked for a password reset
    When I anonymously POST "/users/password/reset" with:
      """
      {"token": "{token:reset}", "password": "brand-new-passphrase-77"}
      """
    Then the response status is 200
    When I anonymously POST "/users/password/reset" with:
      """
      {"token": "{token:reset}", "password": "yet-another-passphrase-88"}
      """
    Then the response status is 400
    And the error message contains "invalid, already used, or expired"

  Scenario: An expired reset token is refused
    Given a registered user "ada@example.com"
    And "ada@example.com" asked for a password reset
    And "ada@example.com" waited 2 hour for the reset link
    When I anonymously POST "/users/password/reset" with:
      """
      {"token": "{token:reset}", "password": "brand-new-passphrase-77"}
      """
    Then the response status is 400

  Scenario: Resetting a password revokes the sessions it protected
    Given I am signed in as "ada@example.com"
    And "ada@example.com" asked for a password reset
    When I anonymously POST "/users/password/reset" with:
      """
      {"token": "{token:reset}", "password": "brand-new-passphrase-77"}
      """
    Then the response status is 200
    When I POST "/users/token/refresh" with:
      """
      {"refresh_token": "{token:refresh}"}
      """
    Then the response status is 401
