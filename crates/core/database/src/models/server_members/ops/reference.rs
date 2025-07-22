use std::collections::HashSet;

use iso8601_timestamp::Timestamp;
use revolt_result::Result;

use crate::ReferenceDb;
use crate::{FieldsMember, Member, MemberCompositeKey, PartialMember};

use super::{AbstractServerMembers, ChunkedServerMembersGenerator};

#[async_trait]
impl AbstractServerMembers for ReferenceDb {
    /// Insert a new server member into the database
    async fn insert_or_merge_member(&self, member: &Member) -> Result<Option<Member>> {
        let mut server_members = self.server_members.lock().await;
        if server_members.contains_key(&member.id) {
            Err(create_database_error!("insert", "member"))
        } else {
            server_members.insert(member.id.clone(), member.clone());
            Ok(None)
        }
    }

    /// Fetch a server member by their id
    async fn fetch_member(&self, server_id: &str, user_id: &str) -> Result<Member> {
        let server_members = self.server_members.lock().await;
        server_members
            .get(&MemberCompositeKey {
                server: server_id.to_string(),
                user: user_id.to_string(),
            })
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch all members in a server
    async fn fetch_all_members(&self, server_id: &str) -> Result<Vec<Member>> {
        let server_members = self.server_members.lock().await;
        Ok(server_members
            .values()
            .filter(|member| member.id.server == server_id)
            .cloned()
            .collect())
    }

    /// Fetch all members in a server as an iterator
    async fn fetch_all_members_chunked(
        &self,
        server_id: &str,
    ) -> Result<ChunkedServerMembersGenerator> {
        let server_members = self.server_members.lock().await;

        let members = server_members
            .clone()
            .into_values()
            .filter(move |member| member.id.server == server_id)
            .collect();

        // this is inefficient as shit but its the reference db so its fine
        Ok(ChunkedServerMembersGenerator::new_reference(members))
    }

    /// Fetch all members that have any of the roles given
    async fn fetch_all_members_with_roles(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<Vec<Member>> {
        let server_members = self.server_members.lock().await;

        Ok(server_members
            .clone()
            .into_values()
            .filter(|member| {
                member.id.server == server_id
                    && !member
                        .roles
                        .iter()
                        .filter(|p| roles.contains(*p))
                        .collect::<Vec<&String>>()
                        .is_empty()
            })
            .collect())
    }

    async fn fetch_all_members_with_roles_chunked(
        &self,
        server_id: &str,
        roles: &[String],
    ) -> Result<ChunkedServerMembersGenerator> {
        let server_members = self.server_members.lock().await;

        let resp = server_members
            .clone()
            .into_values()
            .filter(|member| {
                member.id.server == server_id
                    && !member
                        .roles
                        .iter()
                        .filter(|p| roles.contains(*p))
                        .collect::<Vec<&String>>()
                        .is_empty()
            })
            .collect();

        return Ok(ChunkedServerMembersGenerator::new_reference(resp));
    }

    /// Fetch all memberships for a user
    async fn fetch_all_memberships(&self, user_id: &str) -> Result<Vec<Member>> {
        let server_members = self.server_members.lock().await;
        Ok(server_members
            .values()
            .filter(|member| member.id.user == user_id)
            .cloned()
            .collect())
    }

    /// Fetch multiple members by their ids
    async fn fetch_members(&self, server_id: &str, ids: &[String]) -> Result<Vec<Member>> {
        let server_members = self.server_members.lock().await;
        Ok(ids
            .iter()
            .filter_map(|id| {
                server_members
                    .get(&MemberCompositeKey {
                        server: server_id.to_string(),
                        user: id.to_string(),
                    })
                    .cloned()
            })
            .collect())
    }

    /// Fetch member count of a server
    async fn fetch_member_count(&self, server_id: &str) -> Result<usize> {
        let server_members = self.server_members.lock().await;
        Ok(server_members
            .values()
            .filter(|member| member.id.server == server_id)
            .count())
    }

    /// Fetch server count of a user
    async fn fetch_server_count(&self, user_id: &str) -> Result<usize> {
        let server_members = self.server_members.lock().await;
        Ok(server_members
            .values()
            .filter(|member| member.id.user == user_id)
            .count())
    }

    /// Update information for a server member
    async fn update_member(
        &self,
        id: &MemberCompositeKey,
        partial: &PartialMember,
        remove: Vec<FieldsMember>,
    ) -> Result<()> {
        let mut server_members = self.server_members.lock().await;
        if let Some(member) = server_members.get_mut(id) {
            for field in remove {
                #[allow(clippy::disallowed_methods)]
                member.remove_field(&field);
            }

            member.apply_options(partial.clone());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Soft delete a member
    async fn soft_delete_member(&self, id: &MemberCompositeKey) -> Result<()> {
        let mut server_members = self.server_members.lock().await;

        let member = server_members.get_mut(id);
        if let Some(member) = member {
            if member.in_timeout() {
                panic!("Soft deletion is not implemented.")
            } else if server_members.remove(id).is_some() {
                Ok(())
            } else {
                Err(create_error!(NotFound))
            }
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Delete a server member by their id
    async fn force_delete_member(&self, id: &MemberCompositeKey) -> Result<()> {
        let mut server_members = self.server_members.lock().await;
        if server_members.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    async fn remove_dangling_members(&self) -> Result<()> {
        todo!()
    }

    async fn fetch_server_participants(
        &self,
        server_id: &str,
    ) -> Result<Vec<(crate::User, Option<Member>)>> {
        let servers = self.servers.lock().await;
        let server = servers.get(server_id).ok_or(create_error!(NotFound))?;

        let mut hash = HashSet::new();
        server.channels.iter().for_each(|c| {
            hash.insert(c.clone());
        });

        drop(servers);

        let messages = self.messages.lock().await;
        let userids = messages
            .iter()
            .filter(|(_, message)| hash.contains(&message.channel))
            .map(|(_, message)| message.author.clone())
            .collect::<HashSet<String>>();

        drop(messages);

        let users = self.users.lock().await;
        let members = self.server_members.lock().await;

        let resp_users: Vec<(crate::User, Option<crate::Member>)> = users
            .iter()
            .filter(|(uid, _)| userids.contains(*uid))
            .map(|(uid, user)| {
                let key = MemberCompositeKey {
                    server: server_id.to_string(),
                    user: uid.clone(),
                };
                // fuck it we ball
                (user.clone(), members.get(&key).map(|f| f.clone()))
            })
            .collect();

        Ok(resp_users)
    }

    /// Fetch all members of a server
    async fn fetch_server_members(
        &self,
        server_id: &str,
        page_size: u8,
        after: Option<usize>,
    ) -> Result<Vec<(crate::User, Member)>> {
        let members = self.server_members.lock().await;
        let users = self.users.lock().await;

        let mut server_members: Vec<&Member> = members
            .iter()
            .filter(|(_, f)| f.id.server == server_id)
            .map(|(_, m)| m)
            .collect();

        server_members.sort_by(|a, b| a.joined_at.cmp(&b.joined_at));
        let iterator = server_members.iter();

        let mut resp: Vec<&&Member> = if let Some(after) = after {
            iterator
                .skip_while(|f| {
                    f.joined_at
                        .duration_since(Timestamp::UNIX_EPOCH)
                        .whole_milliseconds()
                        <= after as i128
                })
                .collect()
        } else {
            iterator.collect()
        };

        resp.truncate(page_size as usize);

        Ok(resp
            .iter()
            .map(|f| {
                (
                    users.get(&f.id.user).unwrap().clone(),
                    f.clone().clone().clone(),
                )
            })
            .collect())
    }
}
