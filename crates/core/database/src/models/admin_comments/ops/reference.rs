use iso8601_timestamp::Timestamp;
use revolt_result::Result;

use crate::ReferenceDb;
use crate::{AdminComment, PartialAdminComment};

use super::AbstractAdminComments;

#[async_trait]
impl AbstractAdminComments for ReferenceDb {
    async fn admin_comment_insert(&self, comment: AdminComment) -> Result<()> {
        let mut admin_comments = self.admin_comments.lock().await;
        if admin_comments.contains_key(&comment.id) {
            Err(create_database_error!("insert", "admin_comments"))
        } else {
            admin_comments.insert(comment.id.to_string(), comment.clone());
            Ok(())
        }
    }

    async fn admin_comment_update(
        &self,
        comment_id: &str,
        partial: &PartialAdminComment,
    ) -> Result<()> {
        let mut admin_comments = self.admin_comments.lock().await;
        if let Some(comment) = admin_comments.get_mut(comment_id) {
            comment.apply_options(partial.clone());
            comment.edited_at = Some(Timestamp::now_utc().format().to_string());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    async fn admin_comment_fetch(&self, id: &str) -> Result<AdminComment> {
        let admin_comments = self.admin_comments.lock().await;
        admin_comments
            .get(id)
            .map_or(Err(create_error!(NotFound)), |ac| Ok(ac.clone()))
    }

    async fn admin_comment_fetch_related_case(&self, case_id: &str) -> Result<Vec<AdminComment>> {
        let admin_comments = self.admin_comments.lock().await;
        Ok(admin_comments
            .iter()
            .filter_map(|(_, c)| {
                if c.case_id.as_ref().is_some_and(|c| c == case_id) {
                    Some(c.clone())
                } else {
                    None
                }
            })
            .collect())
    }

    async fn admin_comment_fetch_object_comments(
        &self,
        object_id: &str,
    ) -> Result<Vec<AdminComment>> {
        let admin_comments = self.admin_comments.lock().await;
        Ok(admin_comments
            .iter()
            .filter_map(|(_, c)| {
                if c.object_id == object_id {
                    Some(c.clone())
                } else {
                    None
                }
            })
            .collect())
    }
}
