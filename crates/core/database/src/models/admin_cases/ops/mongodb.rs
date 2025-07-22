use async_std::stream::StreamExt;
use bson::Document;
use revolt_result::Result;

use crate::MongoDb;
use crate::{AdminCase, PartialAdminCase};

use super::AbstractAdminCases;

static COL: &str = "admin_cases";

#[async_trait]
impl AbstractAdminCases for MongoDb {
    async fn admin_case_create(&self, case: AdminCase) -> Result<()> {
        query!(self, insert_one, COL, case).map(|_| ())
    }

    async fn admin_case_assign_report(&self, case_id: &str, report_id: &str) -> Result<()> {
        self.col::<Document>("safety_reports")
            .update_one(
                doc! {"_id": { "$regex": format!("{}$", report_id)}},
                doc! {"case_id": case_id},
            )
            .await
            .map(|_| ())
            .map_err(|_| create_database_error!("safety_reports", "update_one"))
    }

    async fn admin_case_edit(&self, case_id: &str, partial: &PartialAdminCase) -> Result<()> {
        query!(self, update_one_by_id, COL, case_id, partial, vec![], None).map(|_| ())
    }

    async fn admin_case_fetch(&self, case_id: &str) -> Result<AdminCase> {
        query!(self, find_one_by_id, COL, case_id)?
            .ok_or_else(|| create_database_error!("find_one", COL))
    }

    async fn admin_case_fetch_from_shorthand(&self, short_id: &str) -> Result<AdminCase> {
        query!(self, find_one, COL, doc! {"short_id": short_id})?
            .ok_or_else(|| create_database_error!("find_one", COL))
    }

    /// title is fuzzy, the rest of the arguments are direct matches
    async fn admin_case_search(
        &self,
        title: Option<&str>,
        status: Option<&str>,
        owner_id: Option<&str>,
        tags: Option<Vec<String>>,
        before_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AdminCase>> {
        let mut query = Document::new();

        if let Some(title) = title {
            query.insert("$text", doc! {"$search": title});
        }

        if let Some(status) = status {
            query.insert("status", status);
        }

        if let Some(owner) = owner_id {
            query.insert("owner_id", owner);
        }

        if let Some(tags) = tags {
            query.insert("tags", doc! {"$elemMatch": {"$in": tags}});
        }

        if let Some(before) = before_id {
            query.insert("_id", doc! {"$lt": before});
        }

        Ok(self
            .col::<AdminCase>(COL)
            .find(query)
            .limit(limit)
            .await
            .map_err(|_| create_database_error!("find", COL))?
            .filter_map(|f| f.ok())
            .collect()
            .await)
    }
}
