use std::collections::HashMap;

use revolt_models::v0;
use revolt_permissions::OverrideField;

use crate::types as discord;
use revolt_database::{
    Category, Channel, Database, PartialChannel, PartialRole, PartialServer, Role, Server,
    SystemMessageChannels,
};
use revolt_result::{Result, ToRevoltError};

pub async fn fetch_template(template_id: &str) -> Result<discord::GuildTemplate> {
    reqwest::get(format!(
        "https://discord.com/api/v10/guilds/templates/{template_id}"
    ))
    .await
    .to_internal_error()?
    .json()
    .await
    .to_internal_error()
}

pub async fn import_template(
    db: &Database,
    server: &mut Server,
    template: discord::GuildTemplate,
) -> Result<Vec<Channel>> {
    let mut role_mapping = HashMap::new();
    let mut channel_mapping = HashMap::new();
    let mut categories = HashMap::new();

    for discord_channel in &template.serialized_source_guild.channels {
        if discord_channel.channel_type == discord::ChannelType::Category as u32 {
            categories.insert(discord_channel.id, discord_channel.clone());
        };
    }

    let mut role_positions = template.serialized_source_guild.roles
        .iter()
        .filter(|r| r.id != 0)
        .collect::<Vec<_>>();

    role_positions.sort_by(|a, b| b.id.cmp(&a.id));

    let role_positions = role_positions
        .into_iter()
        .enumerate()
        .map(|(i, r)| (r.id, i as i64))
        .collect::<HashMap<_, _>>();

    for discord_role in template.serialized_source_guild.roles.clone() {
        if discord_role.id == 0 {
            continue;
        };

        role_mapping.insert(
            discord_role.id,
            convert_role(db, server, role_positions[&discord_role.id], discord_role).await?,
        );
    };

    for discord_channel in template.serialized_source_guild.channels.clone() {
        if discord_channel.channel_type == discord::ChannelType::Category as u32 {
            continue;
        };

        channel_mapping.insert(
            discord_channel.id,
            convert_channel(db, server, &categories, &role_mapping, discord_channel).await?,
        );
    }

    convert_server(db, server, &channel_mapping, template.serialized_source_guild).await?;

    Ok(channel_mapping.into_values().collect())
}

pub async fn convert_role(db: &Database, server: &mut Server, rank: i64, role: discord::Role) -> Result<Role> {
    let mut role_payload = Role::create(db, server, role.name).await?;
    let mut partial = PartialRole {
        rank: Some(rank),
        ..Default::default()
    };

    if role.hoist {
        partial.hoist = Some(true)
    };

    if let Some(colors) = role.colors {
        if colors.primary_color != 0 {
            partial.colour = Some(format!("#{:x}", colors.primary_color))
        };
    };

    if &role.permissions != "0" {
        let allow = convert_permissions(&role.permissions);
        partial.permissions = Some(OverrideField { a: allow, d: 0 });
    };

    role_payload
        .update(db, &server.id, partial, Vec::new())
        .await?;

    server.roles.insert(role_payload.id.clone(), role_payload.clone());

    Ok(role_payload)
}

static PERMISSION_MAPPING: &[(i64, i64)] = &[];

pub fn convert_permissions(permissions: &str) -> i64 {
    let value = permissions.parse::<i64>().unwrap();
    let mut output = 0;

    for (discord_perm, perm) in PERMISSION_MAPPING {
        if (value & *discord_perm) == *discord_perm {
            output |= *perm
        };
    }

    output
}

pub async fn convert_channel(
    db: &Database,
    server: &mut Server,
    categories: &HashMap<u32, discord::Channel>,
    role_mapping: &HashMap<u32, Role>,
    channel: discord::Channel,
) -> Result<Channel> {
    let mut channel_payload = Channel::create_server_channel(
        db,
        server,
        v0::DataCreateServerChannel {
            channel_type: v0::LegacyServerChannelType::Text,
            name: channel.name,
            description: channel.topic,
            nsfw: channel.nsfw,
            voice: if channel.channel_type == discord::ChannelType::Voice as u32
                || channel.channel_type == discord::ChannelType::StageVoice as u32
            {
                Some(v0::VoiceInformation {
                    max_users: channel.user_limit.map(|limit| limit as usize),
                })
            } else {
                None
            },
        },
        true,
    )
    .await?;

    let mut partial = PartialChannel::default();
    let mut role_permissions = HashMap::new();

    if let Some(parent_id) = channel.parent_id {
        let category = categories.get(&parent_id).unwrap();

        for discord_overwrite in category.permission_overwrites.iter().flatten() {
            if discord_overwrite.overwrite_type == discord::OverwriteType::Member as u32 {
                continue;
            };

            let overwrite = OverrideField {
                a: convert_permissions(&discord_overwrite.allow),
                d: convert_permissions(&discord_overwrite.deny),
            };

            if discord_overwrite.id == 0 {
                partial.default_permissions = Some(overwrite)
            } else {
                role_permissions.insert(
                    role_mapping.get(&discord_overwrite.id).unwrap().id.clone(),
                    overwrite,
                );
            };
        }
    };

    for discord_overwrite in channel.permission_overwrites.iter().flatten() {
        if discord_overwrite.overwrite_type == discord::OverwriteType::Member as u32 {
            continue;
        };

        let overwrite = OverrideField {
            a: convert_permissions(&discord_overwrite.allow),
            d: convert_permissions(&discord_overwrite.deny),
        };

        if discord_overwrite.id == 0 {
            if let Some(default_permissions) = &mut partial.default_permissions {
                default_permissions.a |= overwrite.a;
                default_permissions.d |= overwrite.d;
            } else {
                partial.default_permissions = Some(overwrite)
            }
        } else {
            let id = &role_mapping.get(&discord_overwrite.id).unwrap().id;

            if let Some(role_permission) = role_permissions.get_mut(id) {
                role_permission.a |= overwrite.a;
                role_permission.d |= overwrite.d;
            } else {
                role_permissions.insert(id.clone(), overwrite);
            };
        };
    }

    partial.role_permissions = Some(role_permissions);

    channel_payload.update(db, partial, Vec::new()).await?;

    Ok(channel_payload)
}

pub async fn convert_server(
    db: &Database,
    server: &mut Server,
    channel_mapping: &HashMap<u32, Channel>,
    guild: discord::Guild,
) -> Result<()> {
    let mut partial = PartialServer {
        categories: Some(
            guild
                .channels
                .iter()
                .filter(|c| c.channel_type == discord::ChannelType::Category as u32)
                .map(|category| {
                    let mut channels = guild
                        .channels
                        .iter()
                        .filter(|c| c.parent_id.as_ref() == Some(&category.id))
                        .collect::<Vec<_>>();

                    channels.sort_by(|a, b| a.position.cmp(&b.position));

                    Category {
                        id: category.id.to_string(),
                        title: category.name.clone(),
                        channels: channels
                            .into_iter()
                            .map(|c| channel_mapping.get(&c.id).unwrap().id().to_string())
                            .collect(),
                    }
                })
                .collect(),
        ),
        ..Default::default()
    };

    // TODO: banner

    if let Some(system_channel_id) = &guild.system_channel_id {
        let channel_id = channel_mapping
            .get(system_channel_id)
            .unwrap()
            .id()
            .to_string();

        partial.system_messages = Some(SystemMessageChannels {
            user_banned: Some(channel_id.clone()),
            user_joined: Some(channel_id.clone()),
            user_kicked: Some(channel_id.clone()),
            user_left: Some(channel_id),
        });
    };

    let default_role = guild.roles.iter().find(|r| r.id == 0).unwrap();
    partial.default_permissions = Some(convert_permissions(&default_role.permissions));

    server.update(db, partial, Vec::new()).await?;

    Ok(())
}
