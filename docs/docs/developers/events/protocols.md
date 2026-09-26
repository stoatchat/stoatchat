# Events 1.0.0 documentation

This page documents various incoming and outgoing events.

## Table of Contents

* [Operations](#operations)
  * [RECEIVE Authenticate](#receive-authenticate-operation)
  * [RECEIVE BeginTyping](#receive-begintyping-operation)
  * [RECEIVE EndTyping](#receive-endtyping-operation)
  * [RECEIVE Ping](#receive-ping-operation)
  * [RECEIVE Subscribe](#receive-subscribe-operation)
  * [SEND Error](#send-error-operation)
  * [SEND Authenticated](#send-authenticated-operation)
  * [SEND Logout](#send-logout-operation)
  * [SEND Bulk](#send-bulk-operation)
  * [SEND Pong](#send-pong-operation)
  * [SEND Ready](#send-ready-operation)
  * [SEND Message](#send-message-operation)
  * [SEND MessageUpdate](#send-messageupdate-operation)
  * [SEND MessageAppend](#send-messageappend-operation)
  * [SEND MessageDelete](#send-messagedelete-operation)
  * [SEND MessageReact](#send-messagereact-operation)
  * [SEND MessageUnreact](#send-messageunreact-operation)
  * [SEND MessageRemoveReaction](#send-messageremovereaction-operation)
  * [SEND ChannelCreate](#send-channelcreate-operation)
  * [SEND ChannelUpdate](#send-channelupdate-operation)
  * [SEND ChannelDelete](#send-channeldelete-operation)
  * [SEND ChannelGroupJoin](#send-channelgroupjoin-operation)
  * [SEND ChannelGroupLeave](#send-channelgroupleave-operation)
  * [SEND ChannelStartTyping](#send-channelstarttyping-operation)
  * [SEND ChannelStopTyping](#send-channelstoptyping-operation)
  * [SEND ChannelAck](#send-channelack-operation)
  * [SEND ServerCreate](#send-servercreate-operation)
  * [SEND ServerUpdate](#send-serverupdate-operation)
  * [SEND ServerDelete](#send-serverdelete-operation)
  * [SEND ServerMemberUpdate](#send-servermemberupdate-operation)
  * [SEND ServerMemberJoin](#send-servermemberjoin-operation)
  * [SEND ServerMemberLeave](#send-servermemberleave-operation)
  * [SEND ServerRoleUpdate](#send-serverroleupdate-operation)
  * [SEND ServerRoleDelete](#send-serverroledelete-operation)
  * [SEND UserUpdate](#send-userupdate-operation)
  * [SEND UserRelationship](#send-userrelationship-operation)
  * [SEND UserPlatformWipe](#send-userplatformwipe-operation)
  * [SEND EmojiCreate](#send-emojicreate-operation)
  * [SEND EmojiUpdate](#send-emojiupdate-operation)
  * [SEND EmojiDelete](#send-emojidelete-operation)
  * [SEND Auth](#send-auth-operation)

## Operations

### RECEIVE `Authenticate` Operation

* Operation ID: `Authenticate.publish`

#### Message Authenticate `Authenticate`

*Authenticate with the server.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Authenticate"`) | - | **required** |
| token | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Authenticate",
  "token": "string"
}
```



### RECEIVE `BeginTyping` Operation

* Operation ID: `BeginTyping.publish`

#### Message BeginTyping `BeginTyping`

*Tell other users that you have begun typing in a channel.*

Must be in the specified channel or nothing will happen.

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"BeginTyping"`) | - | **required** |
| channel | string | channel_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "BeginTyping",
  "channel": "string"
}
```



### RECEIVE `EndTyping` Operation

* Operation ID: `EndTyping.publish`

#### Message EndTyping `EndTyping`

*Tell other users that you have stopped typing in a channel.*

Must be in the specified channel or nothing will happen.

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"EndTyping"`) | - | **required** |
| channel | string | channel_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "EndTyping",
  "channel": "string"
}
```



### RECEIVE `Ping` Operation

* Operation ID: `Ping.publish`

#### Message Ping `Ping`

*Ping the server, you can specify a timestamp that you'll receive back.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Ping"`) | - | **required** |
| data | number | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Ping",
  "data": 0
}
```



### RECEIVE `Subscribe` Operation

* Operation ID: `Subscribe.publish`

#### Message Subscribe `Subscribe`

*Subscribe to a server's UserUpdate events.*

Implementation notes:
- Subscriptions automatically expire within 15 minutes.
- A client may have up to 5 active subscriptions.
- This has no effect on bot sessions.
- This event should only be sent **iff** app/client is in focus.
- You should aim to send this event at most every 10 minutes per server.


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Subscribe"`) | - | **required** |
| server_id | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Subscribe",
  "server_id": "string"
}
```



### SEND `Error` Operation

* Operation ID: `Error.subscribe`

#### Message Error `Error`

*An error occurred which meant you couldn't authenticate.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Error"`) | - | **required** |
| error | string | One of: - `LabelMe`: uncategorised error - `InternalError`: the server ran into an issue - `InvalidSession`: authentication details are incorrect - `OnboardingNotFinished`: user has not chosen a username - `AlreadyAuthenticated`: this connection is already authenticated | allowed (`"LabelMe"`, `"InternalError"`, `"InvalidSession"`, `"OnboardingNotFinished"`, `"AlreadyAuthenticated"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Error",
  "error": "LabelMe"
}
```



### SEND `Authenticated` Operation

* Operation ID: `Authenticated.subscribe`

#### Message Authenticated `Authenticated`

*The server has authenticated your connection and you will shortly start receiving data.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Authenticated"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Authenticated"
}
```



### SEND `Logout` Operation

* Operation ID: `Logout.subscribe`

#### Message Logged Out `Logout`

*The current user session has been invalidated or the bot token has been reset.*

Your connection will be closed shortly after.

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Logout"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Logout"
}
```



### SEND `Bulk` Operation

* Operation ID: `Bulk.subscribe`

#### Message Bulk `Bulk`

*Several events have been sent, process each item of `v` as its own event.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Bulk"`) | - | **required** |
| v | array&lt;any&gt; | - | - | - | **required** |
| v (single item) | any | - | - | - | **additional properties are allowed** |

> Examples of payload _(generated)_

```json
{
  "type": "Bulk",
  "v": [
    null
  ]
}
```



### SEND `Pong` Operation

* Operation ID: `Pong.subscribe`

#### Message Pong `Pong`

*Ping response from the server.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Pong"`) | - | **required** |
| data | number | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Pong",
  "data": 0
}
```



### SEND `Ready` Operation

* Operation ID: `Ready.subscribe`

#### Message Ready `Ready`

*Data for use by client, data structures match the API specification.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Ready"`) | - | **required** |
| users | array&lt;any&gt; | - | - | - | - |
| users (single item) | any | - | - | - | **additional properties are allowed** |
| servers | array&lt;any&gt; | - | - | - | - |
| servers (single item) | any | - | - | - | **additional properties are allowed** |
| channels | array&lt;any&gt; | - | - | - | - |
| channels (single item) | any | - | - | - | **additional properties are allowed** |
| members | array&lt;any&gt; | - | - | - | - |
| members (single item) | any | - | - | - | **additional properties are allowed** |
| emojis | array&lt;any&gt; | - | - | - | - |
| emojis (single item) | any | - | - | - | **additional properties are allowed** |
| user_settings | array&lt;any&gt; | - | - | - | - |
| user_settings (single item) | any | - | - | - | **additional properties are allowed** |
| channel_unreads | array&lt;any&gt; | - | - | - | - |
| channel_unreads (single item) | any | - | - | - | **additional properties are allowed** |
| policy_changes | array&lt;any&gt; | - | - | - | - |
| policy_changes (single item) | any | - | - | - | **additional properties are allowed** |

> Examples of payload _(generated)_

```json
{
  "type": "Ready",
  "users": [
    null
  ],
  "servers": [
    null
  ],
  "channels": [
    null
  ],
  "members": [
    null
  ],
  "emojis": [
    null
  ],
  "user_settings": [
    null
  ],
  "channel_unreads": [
    null
  ],
  "policy_changes": [
    null
  ]
}
```



### SEND `Message` Operation

* Operation ID: `Message.subscribe`

#### Message Message `Message`

*Message received, the event object has the same schema as the Message object in the API with the addition of an event type.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Message"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "Message"
}
```



### SEND `MessageUpdate` Operation

* Operation ID: `MessageUpdate.subscribe`

#### Message MessageUpdate `MessageUpdate`

*Message edited or otherwise updated.*

`data` field contains a partial Message object.

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"MessageUpdate"`) | - | **required** |
| id | string | message_id | - | - | **required** |
| channel | string | channel_id | - | - | **required** |
| data | object | - | - | - | **required**, **additional properties are allowed** |

> Examples of payload _(generated)_

```json
{
  "type": "MessageUpdate",
  "id": "string",
  "channel": "string",
  "data": {}
}
```



### SEND `MessageAppend` Operation

* Operation ID: `MessageAppend.subscribe`

#### Message MessageAppend `MessageAppend`

*Message has data being appended to it.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"MessageAppend"`) | - | **required** |
| id | string | message_id | - | - | **required** |
| channel | string | channel_id | - | - | **required** |
| append | object | - | - | - | **required**, **additional properties are allowed** |
| append.embeds | array&lt;any&gt; | - | - | - | - |
| append.embeds (single item) | any | - | - | - | **additional properties are allowed** |

> Examples of payload _(generated)_

```json
{
  "type": "MessageAppend",
  "id": "string",
  "channel": "string",
  "append": {
    "embeds": [
      null
    ]
  }
}
```



### SEND `MessageDelete` Operation

* Operation ID: `MessageDelete.subscribe`

#### Message MessageDelete `MessageDelete`

*Message has been deleted.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"MessageDelete"`) | - | **required** |
| id | string | message_id | - | - | **required** |
| channel | string | channel_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "MessageDelete",
  "id": "string",
  "channel": "string"
}
```



### SEND `MessageReact` Operation

* Operation ID: `MessageReact.subscribe`

#### Message MessageReact `MessageReact`

*A reaction has been added to a message.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"MessageReact"`) | - | **required** |
| id | string | message_id | - | - | **required** |
| channel_id | string | - | - | - | **required** |
| user_id | string | - | - | - | **required** |
| emoji_id | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "MessageReact",
  "id": "string",
  "channel_id": "string",
  "user_id": "string",
  "emoji_id": "string"
}
```



### SEND `MessageUnreact` Operation

* Operation ID: `MessageUnreact.subscribe`

#### Message MessageUnreact `MessageUnreact`

*A reaction has been removed from a message.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"MessageUnreact"`) | - | **required** |
| id | string | message_id | - | - | **required** |
| channel_id | string | - | - | - | **required** |
| user_id | string | - | - | - | **required** |
| emoji_id | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "MessageUnreact",
  "id": "string",
  "channel_id": "string",
  "user_id": "string",
  "emoji_id": "string"
}
```



### SEND `MessageRemoveReaction` Operation

* Operation ID: `MessageRemoveReaction.subscribe`

#### Message MessageRemoveReaction `MessageRemoveReaction`

*A certain reaction has been removed from the message.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"MessageRemoveReaction"`) | - | **required** |
| id | string | message_id | - | - | **required** |
| channel_id | string | - | - | - | **required** |
| emoji_id | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "MessageRemoveReaction",
  "id": "string",
  "channel_id": "string",
  "emoji_id": "string"
}
```



### SEND `ChannelCreate` Operation

* Operation ID: `ChannelCreate.subscribe`

#### Message ChannelCreate `ChannelCreate`

*Channel created, the event object has the same schema as the Channel object in the API with the addition of an event type.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelCreate"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelCreate"
}
```



### SEND `ChannelUpdate` Operation

* Operation ID: `ChannelUpdate.subscribe`

#### Message ChannelUpdate `ChannelUpdate`

*Channel details updated.*

- `data` field contains a partial Channel object.
- `clear` field lists fields to remove: `Icon`, `Description`


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelUpdate"`) | - | **required** |
| id | string | channel_id | - | - | **required** |
| data | object | - | - | - | **required**, **additional properties are allowed** |
| clear | array&lt;string&gt; | - | - | - | - |
| clear (single item) | string | - | allowed (`"Icon"`, `"Description"`) | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelUpdate",
  "id": "string",
  "data": {},
  "clear": [
    "Icon"
  ]
}
```



### SEND `ChannelDelete` Operation

* Operation ID: `ChannelDelete.subscribe`

#### Message ChannelDelete `ChannelDelete`

*Channel has been deleted.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelDelete"`) | - | **required** |
| id | string | channel_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelDelete",
  "id": "string"
}
```



### SEND `ChannelGroupJoin` Operation

* Operation ID: `ChannelGroupJoin.subscribe`

#### Message ChannelGroupJoin `ChannelGroupJoin`

*A user has joined the group.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelGroupJoin"`) | - | **required** |
| id | string | channel_id | - | - | **required** |
| user | string | user_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelGroupJoin",
  "id": "string",
  "user": "string"
}
```



### SEND `ChannelGroupLeave` Operation

* Operation ID: `ChannelGroupLeave.subscribe`

#### Message ChannelGroupLeave `ChannelGroupLeave`

*A user has left the group.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelGroupLeave"`) | - | **required** |
| id | string | channel_id | - | - | **required** |
| user | string | user_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelGroupLeave",
  "id": "string",
  "user": "string"
}
```



### SEND `ChannelStartTyping` Operation

* Operation ID: `ChannelStartTyping.subscribe`

#### Message ChannelStartTyping `ChannelStartTyping`

*A user has started typing in this channel.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelStartTyping"`) | - | **required** |
| id | string | channel_id | - | - | **required** |
| user | string | user_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelStartTyping",
  "id": "string",
  "user": "string"
}
```



### SEND `ChannelStopTyping` Operation

* Operation ID: `ChannelStopTyping.subscribe`

#### Message ChannelStopTyping `ChannelStopTyping`

*A user has stopped typing in this channel.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelStopTyping"`) | - | **required** |
| id | string | channel_id | - | - | **required** |
| user | string | user_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelStopTyping",
  "id": "string",
  "user": "string"
}
```



### SEND `ChannelAck` Operation

* Operation ID: `ChannelAck.subscribe`

#### Message ChannelAck `ChannelAck`

*You have acknowledged new messages in this channel up to this message ID.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ChannelAck"`) | - | **required** |
| id | string | channel_id | - | - | **required** |
| user | string | user_id | - | - | **required** |
| message_id | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ChannelAck",
  "id": "string",
  "user": "string",
  "message_id": "string"
}
```



### SEND `ServerCreate` Operation

* Operation ID: `ServerCreate.subscribe`

#### Message ServerCreate `ServerCreate`

*Server created, the event object has the same schema as the SERVER object in the API with the addition of an event type.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerCreate"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ServerCreate"
}
```



### SEND `ServerUpdate` Operation

* Operation ID: `ServerUpdate.subscribe`

#### Message ServerUpdate `ServerUpdate`

*Server details updated.*

- `data` field contains a partial Server object.
- `clear` field lists fields to remove: `Icon`, `Banner`, `Description`


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerUpdate"`) | - | **required** |
| id | string | server_id | - | - | **required** |
| data | object | - | - | - | **required**, **additional properties are allowed** |
| clear | array&lt;string&gt; | - | - | - | - |
| clear (single item) | string | - | allowed (`"Icon"`, `"Banner"`, `"Description"`) | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "ServerUpdate",
  "id": "string",
  "data": {},
  "clear": [
    "Icon"
  ]
}
```



### SEND `ServerDelete` Operation

* Operation ID: `ServerDelete.subscribe`

#### Message ServerDelete `ServerDelete`

*Server has been deleted.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerDelete"`) | - | **required** |
| id | string | server_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ServerDelete",
  "id": "string"
}
```



### SEND `ServerMemberUpdate` Operation

* Operation ID: `ServerMemberUpdate.subscribe`

#### Message ServerMemberUpdate `ServerMemberUpdate`

*Server member details updated.*

- `data` field contains a partial Server Member object.
- `clear` field lists fields to remove: `Nickname`, `Avatar`


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerMemberUpdate"`) | - | **required** |
| id | object | - | - | - | **required**, **additional properties are allowed** |
| id.server | string | server_id | - | - | - |
| id.user | string | user_id | - | - | - |
| data | object | - | - | - | **required**, **additional properties are allowed** |
| clear | array&lt;string&gt; | - | - | - | - |
| clear (single item) | string | - | allowed (`"Nickname"`, `"Avatar"`) | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "ServerMemberUpdate",
  "id": {
    "server": "string",
    "user": "string"
  },
  "data": {},
  "clear": [
    "Nickname"
  ]
}
```



### SEND `ServerMemberJoin` Operation

* Operation ID: `ServerMemberJoin.subscribe`

#### Message ServerMemberJoin `ServerMemberJoin`

*A user has joined the server.*

`member` field contains a Member object.

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerMemberJoin"`) | - | **required** |
| id | string | server_id | - | - | **required** |
| user | string | user_id | - | - | **required** |
| member | object | - | - | - | **required**, **additional properties are allowed** |

> Examples of payload _(generated)_

```json
{
  "type": "ServerMemberJoin",
  "id": "string",
  "user": "string",
  "member": {}
}
```



### SEND `ServerMemberLeave` Operation

* Operation ID: `ServerMemberLeave.subscribe`

#### Message ServerMemberLeave `ServerMemberLeave`

*A user has left the server.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerMemberLeave"`) | - | **required** |
| id | string | server_id | - | - | **required** |
| user | string | user_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ServerMemberLeave",
  "id": "string",
  "user": "string"
}
```



### SEND `ServerRoleUpdate` Operation

* Operation ID: `ServerRoleUpdate.subscribe`

#### Message ServerRoleUpdate `ServerRoleUpdate`

*Server role has been updated or created.*

- `data` field contains a partial Server Role object.
- `clear` field lists fields to remove: `Colour`


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerRoleUpdate"`) | - | **required** |
| id | string | server_id | - | - | **required** |
| role_id | string | - | - | - | **required** |
| data | object | - | - | - | **required**, **additional properties are allowed** |
| clear | array&lt;string&gt; | - | - | - | - |
| clear (single item) | string | - | allowed (`"Colour"`) | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "ServerRoleUpdate",
  "id": "string",
  "role_id": "string",
  "data": {},
  "clear": [
    "Colour"
  ]
}
```



### SEND `ServerRoleDelete` Operation

* Operation ID: `ServerRoleDelete.subscribe`

#### Message ServerRoleDelete `ServerRoleDelete`

*Server role has been deleted.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"ServerRoleDelete"`) | - | **required** |
| id | string | server_id | - | - | **required** |
| role_id | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "ServerRoleDelete",
  "id": "string",
  "role_id": "string"
}
```



### SEND `UserUpdate` Operation

* Operation ID: `UserUpdate.subscribe`

#### Message UserUpdate `UserUpdate`

*User has been updated.*

- `data` field contains a partial User object.
- `clear` field lists fields to remove: `ProfileContent`, `ProfileBackground`, `StatusText`, `Avatar`, `DisplayName`


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"UserUpdate"`) | - | **required** |
| id | string | user_id | - | - | **required** |
| data | object | - | - | - | **required**, **additional properties are allowed** |
| clear | array&lt;string&gt; | - | - | - | - |
| clear (single item) | string | - | allowed (`"ProfileContent"`, `"ProfileBackground"`, `"StatusText"`, `"Avatar"`, `"DisplayName"`) | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "UserUpdate",
  "id": "string",
  "data": {},
  "clear": [
    "ProfileContent"
  ]
}
```



### SEND `UserRelationship` Operation

* Operation ID: `UserRelationship.subscribe`

#### Message UserRelationship `UserRelationship`

*Your relationship with another user has changed.*

- `user` field contains a User object.
- `status` field matches Relationship Status in API.


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"UserRelationship"`) | - | **required** |
| id | string | your_user_id | - | - | **required** |
| user | object | - | - | - | **required**, **additional properties are allowed** |
| status | string | - | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "UserRelationship",
  "id": "string",
  "user": {},
  "status": "string"
}
```



### SEND `UserPlatformWipe` Operation

* Operation ID: `UserPlatformWipe.subscribe`

#### Message UserPlatformWipe `UserPlatformWipe`

*User has been platform banned or deleted their account.*

Clients should remove the following associated data:
- Messages
- DM Channels
- Relationships
- Server Memberships

User flags are specified to explain why a wipe is occurring though not all reasons will necessarily ever appear.


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"UserPlatformWipe"`) | - | **required** |
| user_id | string | - | - | - | **required** |
| flags | string | user_flags | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "UserPlatformWipe",
  "user_id": "string",
  "flags": "string"
}
```



### SEND `EmojiCreate` Operation

* Operation ID: `EmojiCreate.subscribe`

#### Message EmojiCreate `EmojiCreate`

*Emoji created, the event object has the same schema as the Emoji object in the API with the addition of an event type.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"EmojiCreate"`) | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "EmojiCreate"
}
```



### SEND `EmojiUpdate` Operation

* Operation ID: `EmojiUpdate.subscribe`

#### Message EmojiUpdate `EmojiUpdate`

*Emoji has been updated.*

`data` field contains a partial Emoji object.

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"EmojiUpdate"`) | - | **required** |
| id | string | emoji_id | - | - | **required** |
| data | object | - | - | - | **required**, **additional properties are allowed** |
| data.name | string | emoji_name | - | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "EmojiUpdate",
  "id": "string",
  "data": {
    "name": "string"
  }
}
```



### SEND `EmojiDelete` Operation

* Operation ID: `EmojiDelete.subscribe`

#### Message EmojiDelete `EmojiDelete`

*Emoji has been deleted.*

##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"EmojiDelete"`) | - | **required** |
| id | string | emoji_id | - | - | **required** |

> Examples of payload _(generated)_

```json
{
  "type": "EmojiDelete",
  "id": "string"
}
```



### SEND `Auth` Operation

* Operation ID: `Auth.subscribe`

#### Message Auth `Auth`

*Forwarded events from Authifier, currently only session deletion events are forwarded.*

`event_type` may be one of:

**DeleteSession** - A session has been deleted.
```json
{ "event_type": "DeleteSession", "user_id": "{user_id}", "session_id": "{session_id}" }
```

**DeleteAllSessions** - All sessions for this account have been deleted, optionally excluding a given ID.
```json
{ "event_type": "DeleteAllSessions", "user_id": "{user_id}", "exclude_session_id": "{session_id}" }
```


##### Payload

| Name | Type | Description | Value | Constraints | Notes |
|---|---|---|---|---|---|
| (root) | object | - | - | - | **additional properties are allowed** |
| type | string | - | const (`"Auth"`) | - | **required** |
| event_type | string | - | allowed (`"DeleteSession"`, `"DeleteAllSessions"`) | - | **required** |
| user_id | string | - | - | - | **required** |
| session_id | string | - | - | - | - |
| exclude_session_id | string | - | - | - | - |

> Examples of payload _(generated)_

```json
{
  "type": "Auth",
  "event_type": "DeleteSession",
  "user_id": "string",
  "session_id": "string",
  "exclude_session_id": "string"
}
```



