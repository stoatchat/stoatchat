# Authentication

To authenticate with the API, you must first acquire a bot token or user token:

- **Bot:** create one from user settings in the client
- **User:** copy one from client or authenticate through API

Then you may provide these through either:

| Type  |      Header       |
| :---: | :---------------: |
|  Bot  |   `X-Bot-Token`   |
| User  | `X-Session-Token` |

When dealing with an authenticated route.
