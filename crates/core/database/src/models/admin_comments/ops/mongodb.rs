use revolt_result::Result;

use crate::MongoDb;
use crate::{AdminComment, PartialAdminComment};

use super::AbstractAdminComments;

static COL: &str = "admin_comments";

#[async_trait]
impl AbstractAdminComments for MongoDb {
    async fn admin_comment_insert(&self, comment: AdminComment) -> Result<()> {
        query!(self, insert_one, COL, comment).map(|_| ())
    }

    async fn admin_comment_update(
        &self,
        comment_id: &str,
        partial: &PartialAdminComment,
    ) -> Result<()> {
        query!(
            self,
            update_one_by_id,
            COL,
            comment_id,
            partial,
            vec![],
            None
        )
        .map(|_| ())
    }

    async fn admin_comment_fetch(&self, id: &str) -> Result<AdminComment> {
        query!(self, find_one, COL, doc! {"_id": id})?.ok_or_else(|| create_error!(NotFound))
    }

    async fn admin_comment_fetch_related_case(&self, case_id: &str) -> Result<Vec<AdminComment>> {
        query!(self, find, COL, doc! {"case_id": case_id})
    }

    async fn admin_comment_fetch_object_comments(
        &self,
        object_id: &str,
    ) -> Result<Vec<AdminComment>> {
        query!(self, find, COL, doc! {"object_id": object_id})
    }
}
