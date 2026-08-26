# Implementation Guide - Backend (Rust)

This guide describes how to implement interactive components in the Stoat backend.

## Data Structures

### Add to `revolt-models`

```rust
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComponentType {
    ActionRow = 1,
    Button = 2,
    SelectMenu = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ButtonStyle {
    Primary = 1,
    Secondary = 2,
    Success = 3,
    Danger = 4,
    Link = 5,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Component {
    #[serde(rename = 1)]
    ActionRow(ActionRow),
    #[serde(rename = 2)]
    Button(Button),
    #[serde(rename = 3)]
    SelectMenu(SelectMenu),
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct ActionRow {
    #[validate(length(max = 5))]
    pub components: Vec<Component>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct Button {
    pub style: ButtonStyle,
    #[validate(length(max = 80))]
    pub label: String,
    #[validate(length(max = 100))]
    pub custom_id: Option<String>,
    #[validate(length(max = 255))]
    pub url: Option<String>,
    pub disabled: Option<bool>,
    pub emoji: Option<PartialEmoji>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SelectMenu {
    #[validate(length(max = 100))]
    pub custom_id: String,
    #[validate(length(min = 1, max = 25))]
    pub options: Vec<SelectOption>,
    #[validate(length(max = 150))]
    pub placeholder: Option<String>,
    pub min_values: Option<u32>,
    pub max_values: Option<u32>,
    pub disabled: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct SelectOption {
    #[validate(length(max = 100))]
    pub label: String,
    #[validate(length(max = 100))]
    pub value: String,
    #[validate(length(max = 100))]
    pub description: Option<String>,
    pub emoji: Option<PartialEmoji>,
    pub default: Option<bool>,
}
```

### Update DataMessageSend

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct DataMessageSend {
    #[validate(length(min = 0, max = 5))]
    pub components: Option<Vec<Component>>,
}
```

## Validation

### Add validation

```rust
pub fn validate_components(components: &[Component]) -> Result<(), Error> {
    if components.len() > 5 {
        return Err(Error::InvalidComponents {
            message: "Maximum 5 action rows".into(),
        });
    }
    
    for component in components {
        match component {
            Component::ActionRow(row) => {
                if row.components.len() > 5 {
                    return Err(Error::InvalidComponents {
                        message: "Maximum 5 components per row".into(),
                    });
                }
                
                for inner in &row.components {
                    if matches!(inner, Component::ActionRow(_)) {
                        return Err(Error::InvalidComponents {
                            message: "Cannot nest action rows".into(),
                        });
                    }
                }
            }
            Component::Button(btn) => {
                if btn.style == ButtonStyle::Link && btn.url.is_none() {
                    return Err(Error::InvalidComponents {
                        message: "Link buttons must have URL".into(),
                    });
                }
                if btn.style != ButtonStyle::Link && btn.custom_id.is_none() {
                    return Err(Error::InvalidComponents {
                        message: "Non-link buttons must have custom_id".into(),
                    });
                }
            }
            Component::SelectMenu(menu) => {
                if menu.options.is_empty() {
                    return Err(Error::InvalidComponents {
                        message: "Select menu must have options".into(),
                    });
                }
            }
        }
    }
    
    Ok(())
}
```

## Events

### Create interaction event

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    InteractionCreate(InteractionCreate),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionCreate {
    pub id: String,
    pub token: String,
    pub interaction_type: InteractionType,
    pub message_id: Option<String>,
    pub channel_id: String,
    pub guild_id: Option<String>,
    pub member: Option<Member>,
    pub author: User,
    pub data: InteractionData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    #[serde(rename = 2)]
    MessageComponent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "component_type")]
pub enum InteractionData {
    #[serde(rename = 2)]
    Button { custom_id: String },
    #[serde(rename = 3)]
    SelectMenu { custom_id: String, values: Vec<String> },
}
```

## Responses

### Response endpoint

```rust
pub fn routes() -> Router {
    Router::new()
        .route("/:id/callback", post(interaction_callback))
}

async fn interaction_callback(
    Path(id): Path<String>,
    Json(data): Json<InteractionResponse>,
) -> Result<impl IntoResponse, Error> {
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionResponse {
    pub r#type: ResponseType,
    pub data: Option<ResponseData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseType {
    #[serde(rename = 4)]
    ChannelMessageWithSource = 4,
    #[serde(rename = 7)]
    UpdateMessage = 7,
}
```

## Tests

### Unit tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_serialization() {
        let button = Button {
            style: ButtonStyle::Primary,
            label: "Click me".into(),
            custom_id: Some("btn_1".into()),
            url: None,
            disabled: None,
            emoji: None,
        };
        
        let json = serde_json::to_string(&button).unwrap();
        assert!(json.contains("\"type\":2"));
        assert!(json.contains("\"style\":1"));
    }

    #[test]
    fn test_validation() {
        let row = ActionRow {
            components: vec![
                Component::Button(Button {
                    style: ButtonStyle::Link,
                    label: "Link".into(),
                    custom_id: None,
                    url: None,
                    disabled: None,
                    emoji: None,
                })
            ],
        };
        
        assert!(validate_components(&[Component::ActionRow(row)]).is_err());
    }
}
```

## References

- [Revolt Models](https://github.com/stoatchat/stoat/tree/main/crates/models)
- [Revolt Delta](https://github.com/stoatchat/stoat/tree/main/crates/delta)
- [API OpenAPI Spec](https://stoat.chat/api/openapi.json)
