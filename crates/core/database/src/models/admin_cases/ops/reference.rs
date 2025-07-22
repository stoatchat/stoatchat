use revolt_result::Result;

use crate::ReferenceDb;
use crate::{AdminCase, PartialAdminCase};

use super::AbstractAdminCases;

#[async_trait]
impl AbstractAdminCases for ReferenceDb {
    async fn admin_case_create(&self, case: AdminCase) -> Result<()> {
        let mut admin_cases = self.admin_cases.lock().await;
        if admin_cases.contains_key(&case.id) {
            Err(create_database_error!("insert", "admin_cases"))
        } else {
            admin_cases.insert(case.id.to_string(), case.clone());
            Ok(())
        }
    }

    async fn admin_case_assign_report(&self, case_id: &str, report_id: &str) -> Result<()> {
        let mut reports = self.safety_reports.lock().await;
        if let Some(report) = reports.get_mut(report_id) {
            report.case_id = Some(case_id.to_string());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    async fn admin_case_edit(&self, case_id: &str, partial: &PartialAdminCase) -> Result<()> {
        let mut admin_cases = self.admin_cases.lock().await;
        if let Some(case) = admin_cases.get_mut(case_id) {
            case.apply_options(partial.clone());
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    async fn admin_case_fetch(&self, case_id: &str) -> Result<AdminCase> {
        let admin_cases = self.admin_cases.lock().await;
        if let Some(case) = admin_cases.get(case_id) {
            Ok(case.clone())
        } else {
            Err(create_error!(NotFound))
        }
    }

    async fn admin_case_fetch_from_shorthand(&self, short_id: &str) -> Result<AdminCase> {
        let admin_cases = self.admin_cases.lock().await;
        if let Some((_, case)) = admin_cases.iter().find(|(_, c)| c.short_id == short_id) {
            Ok(case.clone())
        } else {
            Err(create_error!(NotFound))
        }
    }

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
    ) -> Result<Vec<AdminCase>> {
        let admin_cases = self.admin_cases.lock().await;
        Ok(admin_cases
            .iter()
            .filter(|(_, case)| {
                if let Some(title) = title {
                    case.title.to_lowercase().contains(title)
                } else {
                    true
                }
            })
            .filter(|(_, case)| {
                if let Some(status) = status {
                    case.status == status
                } else {
                    true
                }
            })
            .filter(|(_, case)| {
                if let Some(owner) = owner_id {
                    case.owner_id == owner
                } else {
                    true
                }
            })
            .filter(|(_, case)| {
                if let Some(tags) = &tags {
                    tags.iter().filter(|p| case.tags.contains(p)).count() > 0
                } else {
                    true
                }
            })
            .skip_while(|(id, _)| {
                if let Some(before) = before_id {
                    id.as_str() <= before
                } else {
                    false
                }
            })
            .by_ref()
            .take(limit as usize)
            .map(|(_, case)| case.clone())
            .collect())
    }
}
