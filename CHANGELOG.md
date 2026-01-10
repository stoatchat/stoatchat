# Changelog

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
