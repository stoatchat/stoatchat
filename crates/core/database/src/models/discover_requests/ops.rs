use revolt_result::Result;

use crate::{DiscoverRequest, DiscoverRequestType};

#[cfg(feature = "mongodb")]
mod mongodb;
mod reference;

#[async_trait]
pub trait AbstractDiscoverRequest: Sync + Send {
    /// Insert discover request into database.
    /// Update an existing one if it was previously denied
    async fn insert_discover_request(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<DiscoverRequest>;

    /// Fetch Discover request by their parent id
    async fn fetch_discover_request_by_item_id(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<DiscoverRequest>;

    /// Remove Discover request
    async fn delete_discover_request(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<()>;

    /// Fetch if the item is banned from being requested. If the item is Some, then the item is banned.
    async fn get_discover_ban(&self, item_type: DiscoverRequestType, item: &str) -> Result<bool>;
}
