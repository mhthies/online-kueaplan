
# Authorization

This application does **not** have a concept of user accounts or identities for authorization, so there is no authentization or authentication.
(There may be some kind of user name of device identifiers to track changes and even authorize changes to KüA entries, but this will not be bound to user identities either.)
Instead, each event has multiple pre-shared passphrases, which grant a certain set of privileges for a certain datetime interval.
The possible privileges are mapped to access roles.

For example, we may have the passphrase "Buxtehude", granting orga access (role) for event 1 for all time and the passphrase "Foo", granting user access (role) for event 2.
The orga access role qualifies for retrieving and editing entries, categories and rooms; the user access role only qualifies for retrieving the data.

Passphrases are stored in the database as plain text and have a global unique id, used for session token storage (see below).


## Session / Authorization storage

We don't want clients to store the passphrases entered by the user.
Thus, we use a "session token" for authorization of requests.
The session token consists of a list of unique ids of passphrases, plus a message authentication signature (HMAC), based on a hidden secret from the application config file, to avoid tempering.


## Procedure

- The user tries to access an event, but the client receives a 403 reponse.
- The client displays a passphrase prompt
- The user enters a passphrase and the client sends the passphrase to the /event/<event>/login endpoint, together with the existing session token
- The server checks the passphrase, adds the passphrase's unique id to the session token, re-signs it and sends it back to the client.
- The client sends the session token with each following API request
- Upon every request, the server
  - checks the HMAC
  - retrieves the list of passphrase unique ids
  - checks the access role for the requested event for the requested event, granted by the given list of passphrases
  - checks that the access role qualifies for the privilege that is required for the request
