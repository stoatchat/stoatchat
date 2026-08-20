use revolt_result::Result;

use crate::ReferenceDb;
use crate::{DiscoverRequest, DiscoverRequestStatus, DiscoverRequestType};

use super::AbstractDiscoverRequest;

#[async_trait]
impl AbstractDiscoverRequest for ReferenceDb {
    /// Insert request into database.
    async fn insert_discover_request(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<DiscoverRequest> {
        let ret = DiscoverRequest {
            request_type: request_type.clone(),
            request_id: item.to_string(),
            status: DiscoverRequestStatus::Pending,
        };

        let mut discover = self.discover_requests.lock().await;
        if let std::collections::hash_map::Entry::Vacant(e) =
            discover.entry((request_type, item.to_string()))
        {
            e.insert(ret.clone());
            Ok(ret)
        } else {
            Err(create_database_error!("insert", "discover_requests"))
        }
    }

    /// Fetch emoji by their parent id
    async fn fetch_discover_request_by_item_id(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<DiscoverRequest> {
        let discover = self.discover_requests.lock().await;
        discover
            .iter()
            .find(|(_, d)| d.request_id == item && d.request_type == request_type)
            .map(|(_, d)| d.clone())
            .ok_or_else(|| create_error!(NotFound))
    }

    /// Update emoji with new information
    async fn delete_discover_request(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<()> {
        let discover = self.discover_requests.lock().await;
        let req = discover
            .iter()
            .find(|(_, d)| d.request_id == item && d.request_type == request_type)
            .map(|(id, _)| id);

        if let Some(req) = req {
            let mut discover = self.discover_requests.lock().await;
            discover.remove_entry(req);
            Ok(())
        } else {
            Err(create_error!(NotFound))
        }
    }

    /// Fetch if the item is banned from being requested
    async fn get_discover_ban(&self, item_type: DiscoverRequestType, item: &str) -> Result<bool> {
        let discover = self.discover_bans.lock().await;
        discover
            .iter()
            .find(|(_, d)| d.item_id == item && d.item_type == item_type)
            .map(|(_, _)| true)
            .ok_or_else(|| create_error!(NotFound))
    }
}
