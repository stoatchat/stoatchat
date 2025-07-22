mod mongodb;
mod reference;

use revolt_result::Result;

use crate::{models::admin_comments::AdminComment, PartialAdminComment};

#[async_trait]
pub trait AbstractAdminComments: Sync + Send {
    async fn admin_comment_insert(&self, comment: AdminComment) -> Result<()>;

    async fn admin_comment_update(
        &self,
        comment_id: &str,
        partial: &PartialAdminComment,
    ) -> Result<()>;

    async fn admin_comment_fetch(&self, id: &str) -> Result<AdminComment>;

    /// Fetch all comments related to the case. This includes comments made on other objects.
    async fn admin_comment_fetch_related_case(&self, case_id: &str) -> Result<Vec<AdminComment>>;

    /// Fetch all comments on an object
    async fn admin_comment_fetch_object_comments(
        &self,
        object_id: &str,
    ) -> Result<Vec<AdminComment>>;
}
