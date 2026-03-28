# Changelog

## [0.12.0](https://github.com/stoatchat/stoatchat/compare/v0.11.5...v0.12.0) (2026-03-28)


### Features

* add bug report template for issue tracking ([#627](https://github.com/stoatchat/stoatchat/issues/627)) ([f777e28](https://github.com/stoatchat/stoatchat/commit/f777e2863c6ca50057c8b5d0a5be14915d287724))
* Add slowmode functionality to text channels ([#680](https://github.com/stoatchat/stoatchat/issues/680)) ([6107f24](https://github.com/stoatchat/stoatchat/commit/6107f242fd3ebaff71a15f9a16330ffbcb4f2d7b))
* Allow restricting server creation to specific users ([#685](https://github.com/stoatchat/stoatchat/issues/685)) ([edfa97d](https://github.com/stoatchat/stoatchat/commit/edfa97db108c9c81828547f98a1db5315cb5ba4a))
* compute thumbhash for images ([#596](https://github.com/stoatchat/stoatchat/issues/596)) ([c2d4369](https://github.com/stoatchat/stoatchat/commit/c2d4369e160f32d79bce0a0b0f14677f89de3669))
* Detect animation in image files for fetch_preview ([#574](https://github.com/stoatchat/stoatchat/issues/574)) ([3fa0abf](https://github.com/stoatchat/stoatchat/commit/3fa0abf47f5f42ddd8ee041fe4c44fbc5ba800c1))
* expose global and user limits in root API response ([#644](https://github.com/stoatchat/stoatchat/issues/644)) ([0b522eb](https://github.com/stoatchat/stoatchat/commit/0b522ebddc17f2e3f792ff5e2347793e9849fa23))
* implement time based message sweep on user ban ([#670](https://github.com/stoatchat/stoatchat/issues/670)) ([98c7b1b](https://github.com/stoatchat/stoatchat/commit/98c7b1b5a5b9fdac5c0ab83be10f0e23114dbfc9))
* load config from env vars ([#576](https://github.com/stoatchat/stoatchat/issues/576)) ([5191bd1](https://github.com/stoatchat/stoatchat/commit/5191bd16b2a905b8409838e34eb0baca96f08580))
* parse message push notification content and replace internal formatting ([#693](https://github.com/stoatchat/stoatchat/issues/693)) ([d1e72ce](https://github.com/stoatchat/stoatchat/commit/d1e72cee42c54e16f4e49af569897528b10a28ca))
* Transfer ownership ([#396](https://github.com/stoatchat/stoatchat/issues/396)) ([735d644](https://github.com/stoatchat/stoatchat/commit/735d644e043793cb86e74aab5b88bb4b8bc17ba2))
* update livekit ([#698](https://github.com/stoatchat/stoatchat/issues/698)) ([f181edc](https://github.com/stoatchat/stoatchat/commit/f181edc8f2ff3ce4b6d48938dfc73931ecfa2279))


### Bug Fixes

* add flag for disabling events instead of commenting them out ([#695](https://github.com/stoatchat/stoatchat/issues/695)) ([a5cd08a](https://github.com/stoatchat/stoatchat/commit/a5cd08a655dece4269f3ac84fa2387ae356709a5))
* add masquerade permission to default direct message settings ([#665](https://github.com/stoatchat/stoatchat/issues/665)) ([ab52569](https://github.com/stoatchat/stoatchat/commit/ab525699bd6663333f0e9fed6d2455e482e6a09f))
* Check for appropriate permission for removing other users avatar ([#657](https://github.com/stoatchat/stoatchat/issues/657)) ([d56135e](https://github.com/stoatchat/stoatchat/commit/d56135e0cbc713884c9378832952f7ad490fa315))
* default video resolution is a non-existent size ([#601](https://github.com/stoatchat/stoatchat/issues/601)) ([0698e11](https://github.com/stoatchat/stoatchat/commit/0698e115e8d003d615e468c4fb9654e6bbc9107f)), closes [#588](https://github.com/stoatchat/stoatchat/issues/588)
* **docs:** Update GitHub links ([#647](https://github.com/stoatchat/stoatchat/issues/647)) ([b830631](https://github.com/stoatchat/stoatchat/commit/b830631bd25a546844b7bdd30386084bb365e4de))
* don't use a bitop for OR ([#676](https://github.com/stoatchat/stoatchat/issues/676)) ([5701b5c](https://github.com/stoatchat/stoatchat/commit/5701b5c18c513f796af365169ceaea372a22638c))
* Fix typo for p256dh in vapid notification flow ([#622](https://github.com/stoatchat/stoatchat/issues/622)) ([a80ad1c](https://github.com/stoatchat/stoatchat/commit/a80ad1cbe58b8af5e45751e51d94d93c1cea1c9f))
* improve generated openapi.json ([#584](https://github.com/stoatchat/stoatchat/issues/584)) ([52ed510](https://github.com/stoatchat/stoatchat/commit/52ed5100c2446e0b261085639e123e7e124cab2c))
* no node state set on channel creation ([#653](https://github.com/stoatchat/stoatchat/issues/653)) ([24d0d2b](https://github.com/stoatchat/stoatchat/commit/24d0d2b7266f6f8a692d0a52704acfecf517674c))
* only show first line on commit messages ([#696](https://github.com/stoatchat/stoatchat/issues/696)) ([91783b9](https://github.com/stoatchat/stoatchat/commit/91783b906697fc85305dee683f7c15dda55f0c50))
* pass &str to Reference ([#697](https://github.com/stoatchat/stoatchat/issues/697)) ([ccda6f5](https://github.com/stoatchat/stoatchat/commit/ccda6f5c53ee043705f7ff6b5f6c393f020781de))
* redis_url vs redis_uri in config ([#666](https://github.com/stoatchat/stoatchat/issues/666)) ([b0b728f](https://github.com/stoatchat/stoatchat/commit/b0b728fb0dbc9ee28360301de1c3ea501bbbff1d))
* replace some links and Revolt mentions to current Stoat ([#515](https://github.com/stoatchat/stoatchat/issues/515)) ([d629e89](https://github.com/stoatchat/stoatchat/commit/d629e89304be2f0011e189293b278f07d346aa7d))
* send push notifications for DM and group messages ([#660](https://github.com/stoatchat/stoatchat/issues/660)) ([52c0d2f](https://github.com/stoatchat/stoatchat/commit/52c0d2f266b76d8975bba2d5e75c62bb30149c45))
* store server id in redis and in room metadata to be able to delete voice state in all scenarios ([#656](https://github.com/stoatchat/stoatchat/issues/656)) ([49c6289](https://github.com/stoatchat/stoatchat/commit/49c628958070e4f0a5edc764d3b48158589219d9))
* uname is missing from crond ([#675](https://github.com/stoatchat/stoatchat/issues/675)) ([dc4438b](https://github.com/stoatchat/stoatchat/commit/dc4438bc3c7b2cad8d442b3cd438afb9ed566a5e))

## [0.11.5](https://github.com/stoatchat/stoatchat/compare/v0.11.4...v0.11.5) (2026-02-17)


### Reverts

* disable user update events ([#593](https://github.com/stoatchat/stoatchat/issues/593)) ([1c98ead](https://github.com/stoatchat/stoatchat/commit/1c98ead69579b4700be0b51c9020bb8402336cc6))

## [0.11.4](https://github.com/stoatchat/stoatchat/compare/v0.11.3...v0.11.4) (2026-02-16)


### Bug Fixes

* add separate config option for redis events replica url ([#590](https://github.com/stoatchat/stoatchat/issues/590)) ([a75e4ea](https://github.com/stoatchat/stoatchat/commit/a75e4eabfc4b34aba7620c82ba77558a32d9e10a))

## [0.11.3](https://github.com/stoatchat/stoatchat/compare/v0.11.2...v0.11.3) (2026-02-13)


### Bug Fixes

* cut presence traffic too while we engineer a new events architecture ([#561](https://github.com/stoatchat/stoatchat/issues/561)) ([1f8ea96](https://github.com/stoatchat/stoatchat/commit/1f8ea963ad742f693f405e6438f1c343c81e6579))

## [0.11.2](https://github.com/stoatchat/stoatchat/compare/v0.11.1...v0.11.2) (2026-02-13)


### Bug Fixes

* cut events traffic while we engineer a new events architecture ([#559](https://github.com/stoatchat/stoatchat/issues/559)) ([a11986b](https://github.com/stoatchat/stoatchat/commit/a11986ba1ad16b672ff1080913a684567d88adbb))

## [0.11.1](https://github.com/stoatchat/stoatchat/compare/v0.11.0...v0.11.1) (2026-02-13)


### Bug Fixes

* bots in multiple voice channel logic ([#544](https://github.com/stoatchat/stoatchat/issues/544)) ([94cb916](https://github.com/stoatchat/stoatchat/commit/94cb916231b9b8befb2e94065917ff40815bec52))

## [0.11.0](https://github.com/stoatchat/stoatchat/compare/v0.10.3...v0.11.0) (2026-02-10)


### Features

* appeal to the almighty Spamhaus ([#524](https://github.com/stoatchat/stoatchat/issues/524)) ([5132270](https://github.com/stoatchat/stoatchat/commit/5132270f2edd6df25ce414daa42ed1b2aa6fa7a9))

## [0.10.3](https://github.com/stoatchat/stoatchat/compare/v0.10.2...v0.10.3) (2026-02-07)


### Bug Fixes

* update `Revolt` -&gt; `Stoat` in email titles/desc. ([#508](https://github.com/stoatchat/stoatchat/issues/508)) ([84483ce](https://github.com/stoatchat/stoatchat/commit/84483cee7af3e5dfa16f7fe13e334c4d9f5abd60))

## [0.10.2](https://github.com/stoatchat/stoatchat/compare/v0.10.1...v0.10.2) (2026-01-25)


### Bug Fixes

* thumbnailification requires rgb8/rgba8 ([#505](https://github.com/stoatchat/stoatchat/issues/505)) ([413aa04](https://github.com/stoatchat/stoatchat/commit/413aa04dcaf8bff3935ed1e5f31432e11a03ce6f))

## [0.10.1](https://github.com/stoatchat/stoatchat/compare/v0.10.0...v0.10.1) (2026-01-25)


### Bug Fixes

* use Rust 1.92.0 for Docker build ([#503](https://github.com/stoatchat/stoatchat/issues/503)) ([98da8a2](https://github.com/stoatchat/stoatchat/commit/98da8a28a0aa2fee4e8eee1d86bd7c49e3187477))

## [0.10.0](https://github.com/stoatchat/stoatchat/compare/v0.9.4...v0.10.0) (2026-01-25)


### Features

* allow kicking members from voice channels ([#495](https://github.com/stoatchat/stoatchat/issues/495)) ([0dc5442](https://github.com/stoatchat/stoatchat/commit/0dc544249825a49c793309edee5ec1838458a6da))
* repository architecture for files crate w. added tests ([#498](https://github.com/stoatchat/stoatchat/issues/498)) ([01ded20](https://github.com/stoatchat/stoatchat/commit/01ded209c62208fc906d6aab9b08c04e860e10ef))


### Bug Fixes

* expose ratelimit headers via cors ([#496](https://github.com/stoatchat/stoatchat/issues/496)) ([a1a2125](https://github.com/stoatchat/stoatchat/commit/a1a21252d0ad58937e41f16e5fb86f96bebd2a51))

## [0.9.4](https://github.com/stoatchat/stoatchat/compare/v0.9.3...v0.9.4) (2026-01-10)


### Bug Fixes

* checkout repo. before bumping lock ([#490](https://github.com/stoatchat/stoatchat/issues/490)) ([b2da2a8](https://github.com/stoatchat/stoatchat/commit/b2da2a858787853be43136fd526a0bd72baf78ef))
* persist credentials for git repo ([#492](https://github.com/stoatchat/stoatchat/issues/492)) ([c674a9f](https://github.com/stoatchat/stoatchat/commit/c674a9fd4e0abbd51569870e4b38074d4a1de03c))

## [0.9.3](https://github.com/stoatchat/stoatchat/compare/v0.9.2...v0.9.3) (2026-01-10)


### Bug Fixes

* pipeline fixes ([#487](https://github.com/stoatchat/stoatchat/issues/487)) ([aeeafeb](https://github.com/stoatchat/stoatchat/commit/aeeafebefc36a43a656cf797c9251ca50292733c))

## [0.9.2](https://github.com/stoatchat/stoatchat/compare/v0.9.1...v0.9.2) (2026-01-10)


### Bug Fixes

* disable publish for services ([#485](https://github.com/stoatchat/stoatchat/issues/485)) ([d13609c](https://github.com/stoatchat/stoatchat/commit/d13609c37279d6a40445dcd99564e5c3dd03bac1))

## [0.9.1](https://github.com/stoatchat/stoatchat/compare/v0.9.0...v0.9.1) (2026-01-10)


### Bug Fixes

* **ci:** pipeline fixes (marked as fix to force release) ([#483](https://github.com/stoatchat/stoatchat/issues/483)) ([303e52b](https://github.com/stoatchat/stoatchat/commit/303e52b476585eea81c33837f1b01506ce387684))

## [0.9.0](https://github.com/stoatchat/stoatchat/compare/v0.8.8...v0.9.0) (2026-01-10)


### Features

* add id field to role ([#470](https://github.com/stoatchat/stoatchat/issues/470)) ([2afea56](https://github.com/stoatchat/stoatchat/commit/2afea56e56017f02de98e67316b4457568ad5b26))
* add ratelimits to gifbox ([1542047](https://github.com/stoatchat/stoatchat/commit/154204742d21cbeff6e2577b00f50b495ea44631))
* include groups and dms in fetch mutuals ([caa8607](https://github.com/stoatchat/stoatchat/commit/caa86074680d46223cebc20f41e9c91c41ec825d))
* include member payload in ServerMemberJoin event ([480f210](https://github.com/stoatchat/stoatchat/commit/480f210ce85271e13d1dac58a5dae08de108579d))
* initial work on tenor gif searching ([b0c977b](https://github.com/stoatchat/stoatchat/commit/b0c977b324b8144c1152589546eb8fec5954c3e7))
* make message lexer use unowned string ([1561481](https://github.com/stoatchat/stoatchat/commit/1561481eb4cdc0f385fbf0a81e4950408050e11f))
* ready payload field customisation ([db57706](https://github.com/stoatchat/stoatchat/commit/db577067948f13e830b5fb773034e9713a1abaff))
* require auth for search ([b5cd5e3](https://github.com/stoatchat/stoatchat/commit/b5cd5e30ef7d5e56e8964fb7c543965fa6bf5a4a))
* trending and categories routes ([5885e06](https://github.com/stoatchat/stoatchat/commit/5885e067a627b8fff1c8ce2bf9e852ff8cf3f07a))
* voice chats v2 ([#414](https://github.com/stoatchat/stoatchat/issues/414)) ([d567155](https://github.com/stoatchat/stoatchat/commit/d567155f124e4da74115b1a8f810062f7c6559d9))


### Bug Fixes

* add license to revolt-parser ([5335124](https://github.com/stoatchat/stoatchat/commit/53351243064cac8d499dd74284be73928fa78a43))
* allow for disabling default features ([65fbd36](https://github.com/stoatchat/stoatchat/commit/65fbd3662462aed1333b79e59155fa6377e83fcc))
* apple music to use original url instead of metadata url ([bfe4018](https://github.com/stoatchat/stoatchat/commit/bfe4018e436a4075bae780dd4d35a9b58315e12f))
* apply uname fix to january and autumn ([8f9015a](https://github.com/stoatchat/stoatchat/commit/8f9015a6ff181d208d9269ab8691bd417d39811a))
* **ci:** publish images under stoatchat and remove docker hub ([d65c1a1](https://github.com/stoatchat/stoatchat/commit/d65c1a1ab3bdc7e5684b03f280af77d881661a3d))
* correct miniz_oxide in lockfile ([#478](https://github.com/stoatchat/stoatchat/issues/478)) ([5d27a91](https://github.com/stoatchat/stoatchat/commit/5d27a91e901dd2ea3e860aeaed8468db6c5f3214))
* correct shebang for try-tag-and-release ([050ba16](https://github.com/stoatchat/stoatchat/commit/050ba16d4adad5d0fb247867aa3e94e3d42bd12d))
* correct string_cache in lockfile ([#479](https://github.com/stoatchat/stoatchat/issues/479)) ([0b178fc](https://github.com/stoatchat/stoatchat/commit/0b178fc791583064bf9ca94b1d39b42d021e1d79))
* don't remove timeouts when a member leaves a server ([#409](https://github.com/stoatchat/stoatchat/issues/409)) ([e635bc2](https://github.com/stoatchat/stoatchat/commit/e635bc23ec857d648d5705e1a3875d7bc3402b0d))
* don't update the same field while trying to remove it ([f4ee35f](https://github.com/stoatchat/stoatchat/commit/f4ee35fb093ca49f0a64ff4b17fd61587df28145)), closes [#392](https://github.com/stoatchat/stoatchat/issues/392)
* github webhook incorrect payload and formatting ([#468](https://github.com/stoatchat/stoatchat/issues/468)) ([dc9c82a](https://github.com/stoatchat/stoatchat/commit/dc9c82aa4e9667ea6639256c65ac8de37a24d1f7))
* implement Serialize to ClientMessage ([dea0f67](https://github.com/stoatchat/stoatchat/commit/dea0f675dde7a63c7a59b38d469f878b7a8a3af4))
* newly created roles should be ranked the lowest ([947eb15](https://github.com/stoatchat/stoatchat/commit/947eb15771ed6785b3dcd16c354c03ded5e4cbe0))
* permit empty `remove` array in edit requests ([6ad3da5](https://github.com/stoatchat/stoatchat/commit/6ad3da5f35f989a2e7d8e29718b98374248e76af))
* preserve order of replies in message ([#447](https://github.com/stoatchat/stoatchat/issues/447)) ([657a3f0](https://github.com/stoatchat/stoatchat/commit/657a3f08e5d652814bbf0647e089ed9ebb139bbf))
* prevent timing out members which have TimeoutMembers permission ([e36fc97](https://github.com/stoatchat/stoatchat/commit/e36fc9738bac0de4f3fcbccba521f1e3754f7ae7))
* relax settings name regex ([3a34159](https://github.com/stoatchat/stoatchat/commit/3a3415915f0d0fdce1499d47a2b7fa097f5946ea))
* remove authentication tag bytes from attachment download ([32e6600](https://github.com/stoatchat/stoatchat/commit/32e6600272b885c595c094f0bc69459250220dcb))
* rename openapi operation ids ([6048587](https://github.com/stoatchat/stoatchat/commit/6048587d348fbca0dc3a9b47690c56df8fece576)), closes [#406](https://github.com/stoatchat/stoatchat/issues/406)
* respond with 201 if no body in requests ([#465](https://github.com/stoatchat/stoatchat/issues/465)) ([24fedf8](https://github.com/stoatchat/stoatchat/commit/24fedf8c4d9cd3160bdec97aa451520f8beaa739))
* swap to using reqwest for query building ([38dd4d1](https://github.com/stoatchat/stoatchat/commit/38dd4d10797b3e6e397fc219e818f379bdff19f2))
* use `trust_cloudflare` config value instead of env var ([cc7a796](https://github.com/stoatchat/stoatchat/commit/cc7a7962a882e1627fcd0bc75858a017415e8cfc))
* use our own result types instead of tenors types ([a92152d](https://github.com/stoatchat/stoatchat/commit/a92152d86da136997817e797c7af8e38731cdde8))
