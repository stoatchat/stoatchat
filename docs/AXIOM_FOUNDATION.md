# Axiom backend foundation

Axiom is built primarily on the existing Stoat-derived Rust backend. The Nexus prototype is a **visual/interaction design reference only**, not a replacement backend.

## Rules for future changes
- Preserve Stoat's authentication/session handling, API semantics, permission enforcement, real-time events, storage architecture and database migrations.
- All direct-message, friendship/block, server moderation, and attachment authorization checks must remain enforced server-side.
- Do not migrate the Nexus prototype's local JSON account data or Python HTTP server into production.
- Keep the frontend (`axiom_frontend`) and backend API versions compatible; document intentional breaking changes.
- Preserve upstream license notices, maintain an upstream sync path, and avoid introducing secrets or production data into the repository.

## Production readiness gates
1. Establish a reproducible local development environment from the existing compose and Rust workspace manifests.
2. Validate authentication, authorization, blocking, and file access with integration tests.
3. Verify file-size controls, streaming behavior, storage quotas, abuse protection and cleanup paths.
4. Add observability, backup/restore exercises, dependency checks and deployment documentation.
5. Audit any newly introduced functionality against existing Stoat permissions rather than trusting only frontend checks.

## Initial milestone
No production service code is changed in this phase. The next backend changes should be driven by confirmed frontend requirements and accompanied by tests.
