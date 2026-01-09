use revolt_result::Result;

use crate::ReferenceDb;
use crate::{FieldsRole, FieldsServer, PartialRole, PartialServer, Role, Server};

use super::AbstractServers;

#[async_trait]
impl AbstractServers for ReferenceDb {
    /// Insert a new server into database
    async fn insert_server(&self, server: &Server) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if servers.contains_key(&server.id) {
            Err(create_database_error!("insert", "server"))
        } else {
            servers.insert(server.id.to_string(), server.clone());
            Ok(())
        }
    }

    /// Fetch a server by its id
    async fn fetch_server(&self, id: &str) -> Result<Server> {
        let servers = self.servers.lock().await;
        servers
            .get(id)
            .cloned()
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Fetch a servers by their ids
    async fn fetch_servers<'a>(&self, ids: &'a [String]) -> Result<Vec<Server>> {
        let servers = self.servers.lock().await;
        ids.iter()
            .map(|id| {
                servers
                    .get(id)
                    .cloned()
                    .ok_or_else(|| create_error!(NotFound))
            })
            .collect()
    }

    /// Update a server with new information
    async fn update_server(
        &self,
        id: &str,
        partial: &PartialServer,
        remove: Vec<FieldsServer>,
    ) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if let Some(server) = servers.get_mut(id) {
            for field in remove {
                #[allow(clippy::disallowed_methods)]
                server.remove_field(&field);
            }

            server.apply_options(partial.clone());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Delete a server by its id
    async fn delete_server(&self, id: &str) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if servers.remove(id).is_some() {
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Insert a new role into server object
    async fn insert_role(&self, server_id: &str, role: &Role) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if let Some(server) = servers.get_mut(server_id) {
            server.roles.insert(role.id.clone(), role.clone());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Update an existing role on a server
    async fn update_role(
        &self,
        server_id: &str,
        role_id: &str,
        partial: &PartialRole,
        remove: Vec<FieldsRole>,
    ) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if let Some(server) = servers.get_mut(server_id) {
            if let Some(role) = server.roles.get_mut(role_id) {
                for field in remove {
                    #[allow(clippy::disallowed_methods)]
                    role.remove_field(&field);
                }

                role.apply_options(partial.clone());
                Ok(())
            } else {
                Err(create_error!(NotFound))
            }
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Delete a role from a server
    ///
    /// Also updates channels and members.
    async fn delete_role(&self, server_id: &str, role_id: &str) -> Result<()> {
        let mut servers = self.servers.lock().await;
        if let Some(server) = servers.get_mut(server_id) {
            if server.roles.remove(role_id).is_some() {
                Ok(())
            } else {
                Err(create_error!(NotFound))
            }
        } else {
            Err(create_error!(NotFound))
        }
    }
}
