# Voice & Audio

Stoat supports real-time voice and video in voice channels and DMs, powered by [LiveKit](https://livekit.io).

## Overview

The voice system consists of three parts:

| Component | Role |
|---|---|
| **Stoat backend (`delta`)** | Issues LiveKit tokens, manages voice state in Redis |
| **LiveKit server** | Handles the real-time media transport (WebRTC) |
| **Voice ingress daemon** | Receives LiveKit webhooks and translates them into Stoat WebSocket events |

```
Client ──join_call──► delta ──token──► Client
                        │
                        └──create room──► LiveKit
                                            │
                                         (media)
                                            │
Client ◄──WS events──── bonfire ◄── Redis ◄── voice-ingress ◄──webhook── LiveKit
```

## Joining a Voice Channel

### `POST /channels/:id/join_call`

Request a LiveKit token to join a voice channel.

**Requires:** `Connect` channel permission.

#### Request Body

```json
{
  "node": "worldwide",
  "force_disconnect": false,
  "recipients": ["01HHXYZABCDEF0123456"]
}
```

| Field | Type | Description |
|---|---|---|
| `node` | `string?` | Name of the LiveKit node to join. Required when the channel has no existing call; optional if a call is already in progress (the existing node is used). |
| `force_disconnect` | `bool?` | Disconnect any other existing voice sessions for this user before joining. Useful for switching devices. Bots may not use this field. |
| `recipients` | `string[]?` | User IDs to notify of the call starting. Only used when the user is the first participant in the call. |

#### Response

```json
{
  "token": "<livekit-jwt>",
  "url": "wss://livekit.example.com"
}
```

| Field | Type | Description |
|---|---|---|
| `token` | `string` | Short-lived JWT (10 seconds) for authenticating with the LiveKit server. |
| `url` | `string` | WebSocket URL of the LiveKit server. Pass this to the LiveKit SDK. |

#### Error Codes

| Error | Description |
|---|---|
| `LiveKitUnavailable` | Voice is not configured on this instance. |
| `NotAVoiceChannel` | The target channel does not support voice. |
| `AlreadyConnected` | User is already connected to a voice channel and `force_disconnect` was not set. |
| `CannotJoinCall` | The voice channel is full (at `max_users` capacity). |
| `UnknownNode` | The requested node does not exist in the server configuration. |

#### Flow

1. Call `POST /channels/:id/join_call` to receive a `token` and `url`.
2. Connect to the LiveKit server using the LiveKit client SDK, passing the `token` and `url`.
3. LiveKit notifies the voice ingress daemon when you join the room.
4. The voice ingress daemon publishes a `VoiceChannelJoin` WebSocket event to all subscribers.

## Data Models

### `UserVoiceState`

Represents the voice state of a single user in a channel.

```json
{
  "id": "01HHXYZABCDEF0123456",
  "joined_at": "2024-01-15T12:34:56.000Z",
  "is_receiving": true,
  "is_publishing": false,
  "screensharing": false,
  "camera": false
}
```

| Field | Type | Description |
|---|---|---|
| `id` | `string` | The user's ID. |
| `joined_at` | `string` | ISO 8601 timestamp of when the user joined the voice channel. |
| `is_receiving` | `bool` | Whether the user is receiving (listening to) audio. |
| `is_publishing` | `bool` | Whether the user is publishing (transmitting) a microphone track. |
| `screensharing` | `bool` | Whether the user is sharing their screen. |
| `camera` | `bool` | Whether the user has their camera enabled. |

### `PartialUserVoiceState`

A partial `UserVoiceState` used in `UserVoiceStateUpdate` events. All fields except `id` are optional; only changed fields are included.

```json
{
  "id": "01HHXYZABCDEF0123456",
  "is_publishing": true
}
```

### `ChannelVoiceState`

Represents the full voice state of a channel, including all connected participants.

```json
{
  "id": "01HHXYZABCDEF0123456",
  "participants": [
    {
      "id": "01HHUSER000000000001",
      "joined_at": "2024-01-15T12:34:56.000Z",
      "is_receiving": true,
      "is_publishing": true,
      "screensharing": false,
      "camera": false
    }
  ]
}
```

| Field | Type | Description |
|---|---|---|
| `id` | `string` | The channel's ID. |
| `participants` | `UserVoiceState[]` | List of voice states for all connected participants. |

Voice states for all voice channels the user is subscribed to are delivered in the `voice_states` array of the `Ready` event.

## Voice Permissions

The following channel permissions control voice access:

| Permission | Description |
|---|---|
| `Connect` | Required to join a voice channel. |
| `Speak` | Allows publishing a microphone track. Without this, `is_publishing` is forced to `false`. |
| `Video` | Allows publishing camera and screen share tracks. Without this, `camera` and `screensharing` are forced to `false`. Also subject to per-user limits set by the instance. |
| `Listen` | Allows subscribing to (receiving) other participants' audio and video. Without this, `is_receiving` is forced to `false`. |
| `ManageChannel` | Allows joining a full voice channel that has reached its `max_users` limit. |
| `MoveMembers` | Allows moving another user to a different voice channel via the member edit endpoint. |

When a role permission is changed that affects voice, the server automatically syncs permissions for all affected participants currently in voice. Each affected participant receives a `UserVoiceStateUpdate` event reflecting their new effective capabilities.

## WebSocket Events

See [Events Protocol](./events/protocol.md) for the full list of voice-related WebSocket events:

- [`VoiceChannelJoin`](./events/protocol.md#voicechanneljoin) — a user connected to a voice channel
- [`VoiceChannelLeave`](./events/protocol.md#voicechannelleave) — a user disconnected from a voice channel
- [`VoiceChannelMove`](./events/protocol.md#voicechannelmove) — a user was moved between voice channels
- [`UserVoiceStateUpdate`](./events/protocol.md#uservoicestateupdate) — a user's microphone/camera/screenshare state changed
- [`UserMoveVoiceChannel`](./events/protocol.md#usermovevoicechannel) — private event sent to you when a moderator moves you to another channel

## Configuration

Voice requires a running LiveKit instance. Configure it in `Revolt.toml`:

```toml
[api.livekit.nodes.worldwide]
url = "https://livekit.example.com"      # HTTP API URL
key = "your-livekit-api-key"
secret = "your-livekit-api-secret"
private = false                           # hide from nodes list if true

[hosts.livekit.worldwide]
# WebSocket URL returned to clients when they join this node
# This is what clients pass to the LiveKit SDK
"wss://livekit.example.com"
```

The voice ingress daemon (`revolt-voice-ingress`) must be reachable by the LiveKit server for webhooks. It listens on port `8500` by default. See `livekit.example.yml` for the LiveKit webhook configuration.
