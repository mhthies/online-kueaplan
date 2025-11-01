
# Authorization

This application does **not** have a concept of user accounts or identities for authorization, so there is no authentization or authentication.
(There may be some kind of user name of device identifiers to track changes and even authorize changes to KüA entries, but this will not be bound to user identities either.)
Instead, each event has multiple pre-shared passphrases, which grant a certain access role for this event.
The access roles are mapped to grated privileges for this role.

For example, we may have the passphrase "Buxtehude", granting orga access (role) for event 1 for all time and the passphrase "Foo", granting user access (role) for event 2.
The orga access role qualifies for retrieving and editing entries, categories and rooms; the user access role only qualifies for retrieving the data.

Passphrases are stored in the database as plain text and have a global unique id, used for session token storage (see below).


## Session / Authorization storage

We don't want clients to store the passphrases entered by the user.
Thus, we use a "session token" for authorization of requests.
The session token consists of a list of unique ids of passphrases, plus a timestamp and a message authentication signature (HMAC), based on a hidden secret from the application config file, to avoid tempering.


## Procedure

- The user tries to access an event, but the client receives a 403 reponse.
- The client displays a passphrase prompt
- The user enters a passphrase and the client sends the passphrase to the /event/<event>/login endpoint, together with the existing session token
- The server checks the passphrase, adds the passphrase's unique id to the session token, re-signs it and sends it back to the client.
- The client sends the session token with each following API request
- Upon every request, the server
  - checks the HMAC and session token timeout
  - retrieves the list of passphrase unique ids
  - checks the access role for the requested event for the requested event, granted by the given list of passphrases
  - checks that the access role qualifies for the privilege that is required for the request


## Derivable Passphrases

### Problem & Requirements

- We want to generate shareable links like https://kueaplan.de/events/2/ical?token=... that can be used with calendar applications (etc.) and don't require an addition authentication mechanism
- The shareable link should be creatable based on user or orga authentication
- The token in shareable link should only provide the privileges required by the specific link
- The token in shareable link must offer the same security features as the session token (HMAC, timeout)
- We want to be able to disable all link tokens, generated from a specific passphrase, in case of compromising (passphrase leaking)


### Solution

- We re-use the session token format for the link token
- We have a special access role `SharableViewLink`, granting read privileges *plus* the privilege to use the special endpoints that take the token as URL query parameter.
- We allow passphrases to be derived from other passphrases.
  That means, a client, hat is authenticated for a given passphrase is allowed *derive* authentication for another passphrase, with a different access role, without actually submitting the passphrase.
- We create derivable passphrases with the SharableViewLink access role, derivable from each user and orga passphrase.

Now, we can create shareable link for authenticated users and orgas, with a token which is only authenticated for a single passphrase—the respective SharableViewLink passphrase which is derivable from the user's authenticated passphrase.
