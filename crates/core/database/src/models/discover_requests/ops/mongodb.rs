use revolt_result::Result;

use crate::DiscoverBan;
use crate::DiscoverRequest;
use crate::DiscoverRequestStatus;
use crate::DiscoverRequestType;
use crate::MongoDb;

use super::AbstractDiscoverRequest;

static DISCOVER_COL: &str = "discover_requests";
static DISCOVER_BANS_COL: &str = "discover_bans";

#[async_trait]
impl AbstractDiscoverRequest for MongoDb {
    /// Insert request into database.
    async fn insert_discover_request(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<DiscoverRequest> {
        if let Ok(mut prev) = self
            .fetch_discover_request_by_item_id(request_type.clone(), item)
            .await
        {
            match prev.status {
                DiscoverRequestStatus::Approved(_)
                | DiscoverRequestStatus::Pending
                | DiscoverRequestStatus::UnderReview => return Err(create_error!(NoEffect)),
                _ => Ok(()),
            }?;
            self.col::<DiscoverRequest>(DISCOVER_COL).update_one(
                doc! {"request_type": bson::to_bson(&request_type).expect("failed to serialize"), "request_id": item},
                doc! {"$set": {"status": bson::to_bson(&DiscoverRequestStatus::Pending).expect("failed to serialize")}},
            ).await.map_err(|_| create_database_error!("update_one", DISCOVER_COL))?;

            prev.status = DiscoverRequestStatus::Pending;
            Ok(prev)
        } else {
            let ret = DiscoverRequest {
                request_type,
                request_id: item.to_string(),
                status: DiscoverRequestStatus::Pending,
            };
            self.col::<DiscoverRequest>(DISCOVER_COL)
                .insert_one(ret.clone())
                .await
                .map_err(|_| create_database_error!("insert_one", DISCOVER_COL))?;
            Ok(ret)
        }
    }

    /// Fetch emoji by their parent id
    async fn fetch_discover_request_by_item_id(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<DiscoverRequest> {
        query!(
            self,
            find_one,
            DISCOVER_COL,
            doc! {"request_type": bson::to_bson(&request_type).expect("failed to serialize"), "request_id": item}
        )?.ok_or_else(|| create_database_error!("find_one", DISCOVER_COL))
    }

    /// Update emoji with new information
    async fn delete_discover_request(
        &self,
        request_type: DiscoverRequestType,
        item: &str,
    ) -> Result<()> {
        query!(
            self,
            delete_one,
            DISCOVER_COL,
            doc! {"request_type": bson::to_bson(&request_type).expect("failed to serialize"), "request_id": item}
        ).map(|_| ())
    }

    /// Fetch if the item is banned from being requested
    async fn get_discover_ban(&self, item_type: DiscoverRequestType, item: &str) -> Result<bool> {
        query!(
            self,
            find_one,
            DISCOVER_BANS_COL,
            doc! {"request_type": bson::to_bson(&item_type).expect("failed to serialize"), "request_id": item}
        )?.ok_or_else(|| create_database_error!("find_one", DISCOVER_COL)).map(|_: DiscoverBan| true)
    }
}
