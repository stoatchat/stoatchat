use revolt_database::{
    Database, MemberSearchQuery, User,
    util::{permissions::DatabasePermissionQuery, reference::Reference},
};
use revolt_models::v0;
use revolt_permissions::PermissionQuery;
use revolt_result::{Result, create_error};
use rocket::{State, serde::json::Json};
use validator::Validate;

/// # Search Members
///
/// Search server members by nickname, display name or username, and/or by roles.
#[openapi(tag = "Server Members")]
#[get("/<target>/members/search?<options..>", rank = 1)]
pub async fn search(
    db: &State<Database>,
    user: User,
    target: Reference<'_>,
    options: v0::OptionsMemberSearch,
) -> Result<Json<v0::MemberSearchResponse>> {
    options.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let server = target.as_server(db).await?;

    let mut query = DatabasePermissionQuery::new(db, &user).server(&server);
    if !query.are_we_a_member().await {
        return Err(create_error!(NotFound));
    }

    let v0::OptionsMemberSearch {
        query: text,
        roles,
        after,
        limit,
    } = options;

    let limit = limit.unwrap_or(25);

    // Fetch one extra member to find out whether there is another page.
    let mut members = db
        .search_members(
            &server.id,
            MemberSearchQuery {
                query: text,
                roles,
                after,
                limit: limit + 1,
            },
        )
        .await?;

    let next = if members.len() as i64 > limit {
        members.truncate(limit as usize);
        members.last().map(|member| member.id.user.clone())
    } else {
        None
    };

    let user_ids: Vec<String> = members
        .iter()
        .map(|member| member.id.user.clone())
        .collect();

    let users = User::fetch_many_ids_as_mutuals(db, &user, &user_ids).await?;

    Ok(Json(v0::MemberSearchResponse {
        members: members.into_iter().map(Into::into).collect(),
        users,
        next,
    }))
}

#[cfg(test)]
mod test {
    use revolt_database::{Member, PartialMember, PartialUser, Server, User};
    use revolt_models::v0;
    use rocket::http::{Header, Status};

    use crate::util::test::TestHarness;

    struct Fixture {
        harness: TestHarness,
        server: Server,
        token: String,
        alice: User,
        bob: User,
        carol: User,
        role_a: String,
        role_b: String,
    }

    /// Owner plus three members:
    /// - alice: roles A and B
    /// - bob: display name "Bobby Tables", role A
    /// - carol: nickname "Moderator Carol", no roles, no permissions
    async fn fixture() -> Fixture {
        let harness = TestHarness::new().await;
        let (_, session, owner) = harness.new_user().await;
        let (server, _) = harness.new_server(&owner).await;
        Member::create(&harness.db, &server, &owner, None)
            .await
            .unwrap();

        let role_a = harness.new_role(&server, 1, None).await.id;
        let role_b = harness.new_role(&server, 2, None).await.id;

        let alice = User::create(&harness.db, "alice".to_string(), None, None)
            .await
            .unwrap();
        let bob = User::create(
            &harness.db,
            "bob".to_string(),
            None,
            PartialUser {
                display_name: Some("Bobby Tables".to_string()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        let carol = User::create(&harness.db, "carol".to_string(), None, None)
            .await
            .unwrap();

        for (user, partial) in [
            (
                &alice,
                PartialMember {
                    roles: Some(vec![role_a.clone(), role_b.clone()]),
                    ..Default::default()
                },
            ),
            (
                &bob,
                PartialMember {
                    roles: Some(vec![role_a.clone()]),
                    ..Default::default()
                },
            ),
            (
                &carol,
                PartialMember {
                    nickname: Some("Moderator Carol".to_string()),
                    ..Default::default()
                },
            ),
        ] {
            let (member, _) = Member::create(&harness.db, &server, user, None)
                .await
                .unwrap();
            harness
                .db
                .update_member(&member.id, &partial, vec![])
                .await
                .unwrap();
        }

        Fixture {
            harness,
            server,
            token: session.token,
            alice,
            bob,
            carol,
            role_a,
            role_b,
        }
    }

    impl Fixture {
        async fn get(
            &self,
            token: &str,
            params: &str,
        ) -> (Status, Option<v0::MemberSearchResponse>) {
            let response = self
                .harness
                .client
                .get(format!(
                    "/servers/{}/members/search?{params}",
                    self.server.id
                ))
                .header(Header::new("x-session-token", token.to_string()))
                .dispatch()
                .await;

            let status = response.status();
            (status, response.into_json().await)
        }

        async fn search(&self, params: &str) -> v0::MemberSearchResponse {
            let (status, body) = self.get(&self.token, params).await;
            assert_eq!(status, Status::Ok, "search `{params}` failed");
            body.expect("Failed to deserialise search response")
        }

        async fn search_ids(&self, params: &str) -> Vec<String> {
            let mut ids: Vec<String> = self
                .search(params)
                .await
                .members
                .into_iter()
                .map(|member| member.id.user)
                .collect();
            ids.sort();
            ids
        }
    }

    fn sorted(users: &[&User]) -> Vec<String> {
        let mut ids: Vec<String> = users.iter().map(|user| user.id.clone()).collect();
        ids.sort();
        ids
    }

    #[rocket::async_test]
    async fn no_filters_returns_everyone() {
        let f = fixture().await;
        let response = f.search("").await;

        assert_eq!(response.members.len(), 4);
        assert_eq!(response.users.len(), 4);
        assert!(response.next.is_none());
    }

    #[rocket::async_test]
    async fn matches_username_display_name_and_nickname() {
        let f = fixture().await;

        // username, case-insensitive partial
        assert_eq!(f.search_ids("query=ALI").await, sorted(&[&f.alice]));
        // display name
        assert_eq!(f.search_ids("query=tables").await, sorted(&[&f.bob]));
        // nickname
        assert_eq!(f.search_ids("query=moderator").await, sorted(&[&f.carol]));
    }

    #[rocket::async_test]
    async fn discriminator_must_match_exactly() {
        let f = fixture().await;

        let query = format!("query=alice%23{}", f.alice.discriminator);
        assert_eq!(f.search_ids(&query).await, sorted(&[&f.alice]));

        let wrong = if f.alice.discriminator == "0001" {
            "0002"
        } else {
            "0001"
        };
        assert!(
            f.search_ids(&format!("query=alice%23{wrong}"))
                .await
                .is_empty()
        );
    }

    #[rocket::async_test]
    async fn regex_characters_are_literal() {
        let f = fixture().await;
        assert!(f.search_ids("query=.%2A").await.is_empty());
        assert!(f.search_ids("query=%28").await.is_empty());
    }

    #[rocket::async_test]
    async fn role_filter_requires_all_roles() {
        let f = fixture().await;

        assert_eq!(
            f.search_ids(&format!("roles={}&roles={}", f.role_a, f.role_b))
                .await,
            sorted(&[&f.alice])
        );
        assert_eq!(
            f.search_ids(&format!("roles={}", f.role_a)).await,
            sorted(&[&f.alice, &f.bob])
        );
        // combined with a string query
        assert_eq!(
            f.search_ids(&format!("roles={}&query=bob", f.role_a)).await,
            sorted(&[&f.bob])
        );
    }

    #[rocket::async_test]
    async fn pagination_walks_every_member_once() {
        let f = fixture().await;

        let mut seen = vec![];
        let mut after: Option<String> = None;

        loop {
            let params = match &after {
                Some(after) => format!("limit=1&after={after}"),
                None => "limit=1".to_string(),
            };

            let response = f.search(&params).await;
            assert_eq!(response.members.len(), 1);
            seen.push(response.members[0].id.user.clone());

            match response.next {
                Some(next) => {
                    assert_eq!(&next, &response.members[0].id.user);
                    after = Some(next);
                }
                None => break,
            }
        }

        assert_eq!(seen.len(), 4);
        let mut unique = seen.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique, seen, "pages must be ordered with no duplicates");
    }

    #[rocket::async_test]
    async fn requires_membership() {
        let f = fixture().await;

        // carol is a regular member without any moderation permission
        let (_, carol_session) = f.harness.account_from_user(f.carol.id.clone()).await;
        let (status, _) = f.get(&carol_session.token, "").await;
        assert_eq!(status, Status::Ok);

        // a user outside the server can't tell it exists
        let (_, stranger_session, _) = f.harness.new_user().await;
        let (status, _) = f.get(&stranger_session.token, "").await;
        assert_eq!(status, Status::NotFound);
    }

    #[rocket::async_test]
    async fn rejects_invalid_options() {
        let f = fixture().await;

        let (status, _) = f.get(&f.token, "limit=0").await;
        assert_eq!(status, Status::BadRequest);

        let (status, _) = f.get(&f.token, "limit=101").await;
        assert_eq!(status, Status::BadRequest);
    }
}
