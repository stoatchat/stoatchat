---
title: Interactive Components
description: Buttons and select menus for Stoat bots
sidebar_position: 1
---

# Interactive Components

Interactive components allow you to create rich interfaces in your bots, such as buttons, select menus, and forms.

:::caution
This feature is currently in development. Full support will be added soon.
:::

## Component Types

| Component | Type | Description |
|-----------|------|-------------|
| Action Row | 1 | Container to group components |
| Button | 2 | Clickable button |
| Select Menu | 3 | Dropdown menu |

## Action Row

Action Row is a container that groups components. Each message can have up to **5 Action Rows**.

```json
{
  "type": 1,
  "components": [...]
}
```

## Button

Buttons allow users to interact with messages.

### Styles

| Style | Value | Color | Use |
|-------|-------|-------|-----|
| PRIMARY | 1 | Blue | Main action |
| SECONDARY | 2 | Gray | Secondary action |
| SUCCESS | 3 | Green | Confirmation |
| DANGER | 4 | Red | Delete/Cancel |
| LINK | 5 | No color | Open URL |

### Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| type | integer | Yes | Always 2 |
| style | integer | Yes | Button style |
| label | string | Yes | Button text (max 80) |
| custom_id | string | Yes* | Unique ID (max 100) |
| disabled | boolean | No | Disabled state |
| emoji | object | No | Emoji next to label |
| url | string | Yes* | URL (LINK only) |

*\* custom_id is required except for LINK. url is required only for LINK.*

### Examples

#### Primary Button

```json
{
  "type": 2,
  "style": 1,
  "label": "Click Here",
  "custom_id": "primary_btn"
}
```

#### Button with Emoji

```json
{
  "type": 2,
  "style": 3,
  "label": "Like",
  "custom_id": "like_btn",
  "emoji": {"name": "heart"}
}
```

#### Link Button

```json
{
  "type": 2,
  "style": 5,
  "label": "Visit Site",
  "url": "https://stoat.chat"
}
```

#### Disabled Button

```json
{
  "type": 2,
  "style": 2,
  "label": "Unavailable",
  "custom_id": "disabled_btn",
  "disabled": true
}
```

## Select Menu

Select menus allow choosing one or more options from a list.

### Properties

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| type | integer | Yes | Always 3 |
| custom_id | string | Yes | Unique ID (max 100) |
| options | array | Yes | Options list (1-25) |
| placeholder | string | No | Placeholder text (max 150) |
| min_values | integer | No | Minimum selections (default: 1) |
| max_values | integer | No | Maximum selections (default: 1) |
| disabled | boolean | No | Disabled state |

### Options

| Property | Type | Required | Description |
|----------|------|----------|-------------|
| label | string | Yes | Option text (max 100) |
| value | string | Yes | Returned value (max 100) |
| description | string | No | Description (max 100) |
| emoji | object | No | Option emoji |
| default | boolean | No | Pre-selected |

### Example

```json
{
  "type": 3,
  "custom_id": "color_select",
  "options": [
    {"label": "Red", "value": "red", "emoji": {"name": "red_circle"}},
    {"label": "Green", "value": "green", "emoji": {"name": "green_circle"}},
    {"label": "Blue", "value": "blue", "emoji": {"name": "blue_circle"}}
  ],
  "placeholder": "Choose a color...",
  "min_values": 1,
  "max_values": 1
}
```

## Messages with Components

### Complete Example

```json
{
  "content": "Select an action:",
  "components": [
    {
      "type": 1,
      "components": [
        {
          "type": 3,
          "custom_id": "action_select",
          "options": [
            {"label": "Ban", "value": "ban"},
            {"label": "Mute", "value": "mute"},
            {"label": "Kick", "value": "kick"}
          ],
          "placeholder": "Action..."
        }
      ]
    },
    {
      "type": 1,
      "components": [
        {"type": 2, "style": 3, "label": "Confirm", "custom_id": "confirm"},
        {"type": 2, "style": 4, "label": "Cancel", "custom_id": "cancel"}
      ]
    }
  ]
}
```

## Limits

- **5** Action Rows per message
- **5** components per Action Row
- **25** options per Select Menu
- **100** characters for custom_id
- **80** characters for button label
- **100** characters for option label

## Next Steps

- [Interaction Events](/developers/events/interactions)
- [Interaction Responses](/developers/api/interaction-responses)
