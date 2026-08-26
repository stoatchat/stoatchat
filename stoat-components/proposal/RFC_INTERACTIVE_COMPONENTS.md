# RFC: Interactive Components for Bots

**Status:** Proposal
**Author:** Stoat Contributors
**Date:** 2026-08-26

## Summary

This proposal adds support for interactive components (buttons and select menus) for bots on Stoat, similar to Discord and other chat platforms.

## Motivation

Currently, bots on Stoat can only send text messages, embeds, and reactions. This limits user interaction. Interactive components allow:

- Creating forms and polls
- Confirmation/cancel buttons
- Selection menus for settings
- Richer and more intuitive interfaces

## Proposed Components

### 1. Action Row

Container to group components (maximum 5 per row).

```json
{
  "type": 1,
  "components": [...]
}
```

### 2. Button

Clickable button with different styles.

```json
{
  "type": 2,
  "style": 1,
  "label": "Confirm",
  "custom_id": "confirm_btn",
  "disabled": false,
  "emoji": {"name": "check"}
}
```

**Styles:**
| Value | Name | Color |
|-------|------|-------|
| 1 | PRIMARY | Blue |
| 2 | SECONDARY | Gray |
| 3 | SUCCESS | Green |
| 4 | DANGER | Red |
| 5 | LINK | No color (opens URL) |

### 3. Select Menu

Dropdown menu for option selection.

```json
{
  "type": 3,
  "custom_id": "role_select",
  "options": [
    {"label": "Option 1", "value": "opt1", "description": "Description"},
    {"label": "Option 2", "value": "opt2"}
  ],
  "placeholder": "Select...",
  "min_values": 1,
  "max_values": 1,
  "disabled": false
}
```

## API Schema

### DataMessageSend (updated)

```json
{
  "type": "object",
  "properties": {
    "content": {"type": "string"},
    "attachments": {"type": "array"},
    "embeds": {"type": "array"},
    "components": {
      "type": "array",
      "items": {"$ref": "#/components/schemas/ActionRow"},
      "nullable": true
    }
  }
}
```

### ActionRow

```json
{
  "type": "object",
  "required": ["type", "components"],
  "properties": {
    "type": {"type": "integer", "enum": [1]},
    "components": {
      "type": "array",
      "items": {
        "oneOf": [
          {"$ref": "#/components/schemas/Button"},
          {"$ref": "#/components/schemas/SelectMenu"}
        ]
      },
      "maxItems": 5
    }
  }
}
```

### Button

```json
{
  "type": "object",
  "required": ["type", "style", "label"],
  "properties": {
    "type": {"type": "integer", "enum": [2]},
    "style": {"type": "integer", "enum": [1, 2, 3, 4, 5]},
    "label": {"type": "string", "maxLength": 80},
    "custom_id": {"type": "string", "maxLength": 100},
    "disabled": {"type": "boolean"},
    "emoji": {"$ref": "#/components/schemas/PartialEmoji"},
    "url": {"type": "string", "format": "uri"}
  }
}
```

### SelectMenu

```json
{
  "type": "object",
  "required": ["type", "custom_id", "options"],
  "properties": {
    "type": {"type": "integer", "enum": [3]},
    "custom_id": {"type": "string", "maxLength": 100},
    "options": {
      "type": "array",
      "items": {"$ref": "#/components/schemas/SelectOption"},
      "minItems": 1,
      "maxItems": 25
    },
    "placeholder": {"type": "string", "maxLength": 150},
    "min_values": {"type": "integer", "minimum": 0, "maximum": 25},
    "max_values": {"type": "integer", "minimum": 1, "maximum": 25},
    "disabled": {"type": "boolean"}
  }
}
```

### SelectOption

```json
{
  "type": "object",
  "required": ["label", "value"],
  "properties": {
    "label": {"type": "string", "maxLength": 100},
    "value": {"type": "string", "maxLength": 100},
    "description": {"type": "string", "maxLength": 100},
    "emoji": {"$ref": "#/components/schemas/PartialEmoji"},
    "default": {"type": "boolean"}
  }
}
```

## Events

### interaction_create

When a user interacts with a component.

```json
{
  "type": "interaction_create",
  "id": "interaction_id",
  "token": "interaction_token",
  "type": 3,
  "message_id": "message_id",
  "channel_id": "channel_id",
  "guild_id": "guild_id",
  "author": {"$ref": "#/components/schemas/User"},
  "data": {
    "custom_id": "confirm_btn",
    "component_type": 2
  }
}
```

## Responses

### Responding to Interactions

```json
{
  "type": 4,
  "data": {
    "content": "Action confirmed!",
    "flags": 64
  }
}
```

```json
{
  "type": 7,
  "data": {
    "content": "Message edited",
    "components": []
  }
}
```

```json
{
  "type": 7,
  "data": {
    "content": "Action processed",
    "components": []
  }
}
```

## Usage Examples

### Confirmation Buttons

```json
{
  "content": "Do you want to delete this message?",
  "components": [
    {
      "type": 1,
      "components": [
        {"type": 2, "style": 3, "label": "Yes", "custom_id": "delete_yes"},
        {"type": 2, "style": 4, "label": "No", "custom_id": "delete_no"}
      ]
    }
  ]
}
```

### Selection Menu

```json
{
  "content": "Select your role:",
  "components": [
    {
      "type": 1,
      "components": [
        {
          "type": 3,
          "custom_id": "role_select",
          "options": [
            {"label": "Admin", "value": "admin", "description": "Full access"},
            {"label": "Mod", "value": "mod", "description": "Moderation"},
            {"label": "Member", "value": "member"}
          ],
          "placeholder": "Choose a role..."
        }
      ]
    }
  ]
}
```

## Limits

- Maximum **5** Action Rows per message
- Maximum **5** components per Action Row
- Maximum **25** options per Select Menu
- **100** characters for custom_id
- **80** characters for button label
- **100** characters for option label

## References

- [Discord Interactions](https://discord.com/developers/docs/interactions/receiving-and-responding)
- [Stoat API Current](https://developers.stoat.chat/api-reference)
