mod mongodb;
mod reference;
use revolt_result::Result;

use crate::models::admin_cases::{AdminCase, PartialAdminCase};

#[async_trait]
pub trait AbstractAdminCases: Sync + Send {
    async fn admin_case_create(&self, case: AdminCase) -> Result<()>;

    async fn admin_case_assign_report(&self, case_id: &str, report_id: &str) -> Result<()>;

    async fn admin_case_edit(&self, case_id: &str, partial: &PartialAdminCase) -> Result<()>;

    async fn admin_case_fetch(&self, case_id: &str) -> Result<AdminCase>;

    async fn admin_case_fetch_from_shorthand(&self, short_id: &str) -> Result<AdminCase>;

    /// title is fuzzy, the rest of the arguments are direct matches
    /// before_id and limit are for paginating
    async fn admin_case_search(
        &self,
        title: Option<&str>,
        status: Option<&str>,
        owner_id: Option<&str>,
        tags: Option<Vec<String>>,
        before_id: Option<&str>,
        limit: i64,
    ) -> Result<Vec<AdminCase>>;
}
