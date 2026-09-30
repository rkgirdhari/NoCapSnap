<!-- Owner-supplied specification, received 2026-09-30. Stored verbatim (tables
     reformatted to Markdown). Gates cite sections of this file as "Spec §n". -->

# Operations and Technical Specification: CapSnap (NoCapSnap)

## 1. Executive Overview and Strategic Product Identity

CapSnap is a high-integrity, private feedback infrastructure designed for the hospitality industry. It functions as a closed-loop system, allowing restaurant staff to document the specific dish prepared for a guest and providing that guest a secure channel for neutral communication. Unlike public-facing aggregators that expose establishments to reputational volatility via public scraping or review gating, CapSnap preserves data as a constructive internal tool. By focusing on private dish-level feedback, the platform protects the restaurant's reputation while adhering to a "neutrality mandate" that forbids the filtration of negative responses or redirection to third-party marketing platforms.

Product Identity Matrix

| Attribute | Specification |
|---|---|
| Official Display Name | CapSnap |
| Internal Project Slug | NoCapSnap / nocapsnap |
| Package Identity | com.hammurabicoding.nocapsnap |
| Ownership | Hammurabi Coding Company LLC (Founder: R. K. Girdhari) |

The core value proposition lies in its strategic independence. CapSnap rejects the industry trend of converting private feedback into public Google Reviews, an action that frequently violates Google Maps UGC policies regarding fake or automated contributions. Instead, it provides a high-entropy, 1–5 rating system that is strictly for internal quality control. This privacy-first mission is underpinned by a specialized, self-hosted technical stack designed to eliminate external service dependencies and third-party data leakage.

## 2. Modern Architecture and Technical Stack Rationalization

The project has undergone a fundamental architectural shift, moving away from the legacy Expo/React Native skeleton toward a high-performance Rust ecosystem. This transition resolves significant risks regarding memory safety, cross-platform performance, and native bridge limitations inherent in older JavaScript-based mobile frameworks.

Target Technology Stack

1. Frontend Framework: SvelteKit 2.x – Configured as a static Single Page Application (SPA). This ensures the staff UI can be packaged within Tauri's binary and the guest portal can be served efficiently by the Rust backend.
2. Mobile Wrapper: Tauri 2.5+ – Utilizes Rust commands for native bridge functionality, providing low-level access to the Android camera and filesystem while maintaining a minimal binary footprint.
3. Backend Monolith: Rust/Axum – Chosen for high-concurrency, memory-safe API operations. This service handles staff authentication, tenant-aware data logic, and the guest feedback exchange.
4. Persistence Layer: SQLite – Implemented as a "local-first" storage on the device and a durable server-side store. This eliminates the operational overhead and external costs associated with PostgreSQL or Redis.

This architecture explicitly rejects AWS (S3/CloudFront), Redis, and managed cloud services in favor of a "self-hosted first" mandate. By utilizing owner-operated vServers, CapSnap minimizes recurring operational costs and third-party account risks. This lean stack enables the robust, disconnected workflows required in professional kitchens.

## 3. Staff Capture Workflow and Local-First Image Processing

In high-pressure restaurant environments, network reliability is a secondary concern to operational speed. CapSnap employs a local-first capture model, ensuring that staff can document dishes regardless of connectivity states.

Image Processing Lifecycle

* On-Demand Permissions: The application requests camera access only at the point of use. It explicitly forbids the collection of background location data, contacts, or microphone access.
* Client-Side Hardening: Images are resized to 2048px on the longest edge on-device. All EXIF and GPS metadata is stripped prior to synchronization to prevent data leakage.
* Atomic Persistence: Every capture is committed to local SQLite storage using an outbox pattern. To prevent staff confusion during synchronization delays, the UI must explicitly display "saved offline / QR not ready" for pending items, transitioning to "Synced / QR ready" only after server acknowledgement.

The "Sync Protocol" requires the server to acknowledge receipt and validate the media before a guest QR can be generated. This prevents "dead links" and ensures data integrity before the guest interaction begins.

## 4. High-Entropy QR Capabilities and Guest Feedback Protocol

Guest feedback is managed via cryptographically secure capability tokens. This architecture ensures that links are one-time-use, authentic, and inherently private.

Guest Link Architecture

* Token Entropy: Links are derived from 256-bit random capability tokens.
* Fragment-Based Security: The token resides in the URL fragment (e.g., https://domain.com/g/#token). Fragments are not transmitted to the server in the initial HTTP request, preventing the token from appearing in standard server logs.
* Session Exchange & History Cleanup: The guest page exchanges the fragment token for a short-lived session via a POST /api/v1/guest/session request. Following this exchange, the application must use history.replaceState to immediately remove the token from the browser history.

The "Neutrality Mandate" ensures that every guest is presented with the same 1–5 rating and optional comment field. No "review gating" or satisfaction-based redirection is permitted. Furthermore, CapSnap responses are never converted into public reviews, maintaining compliance with Google Maps UGC standards and ensuring the feedback loop remains a private, professional communication channel.

## 5. Multi-Tenant Data Model and API Architecture

The system enforces strict tenant isolation at the service level. The server must derive the Organization ID, Location ID, and role from the authenticated session; it never trusts tenant identifiers supplied in the request body by a client.

Core Entity Schema

| Entity | Key Fields |
|---|---|
| Organizations | ID, Name, Slug, Status |
| Locations | ID, Org_ID, Name, IANA Timezone |
| Staff (RBAC) | Identity, Password Hash, Role (Admin, Manager, Chef, Server) |
| Captures | ID, Client_ID, Staff_ID, Media_ID, Sync State, Capture Time (UTC) |
| Media Assets | Opaque Media ID, SHA-256 Digest, MIME, Private Storage Key |
| Guest Feedback | Link_ID, Integer Rating (1–5), Comment (Bounded 2k Unicode) |

API Route Map (/api/v1)

* Public: GET /health (minimal liveness), POST /auth/sessions (rate-limited login).
* Staff (Session Required): POST /media (streamed binary), POST /captures (idempotent metadata), GET /locations/{id}/menu-items.
* Guest (Token/Session): POST /guest/session (fragment exchange), POST /guest/feedback (one-time submission).

Media uploads require server-side validation of file signatures, MIME types, and SHA-256 digests. The server must verify decode success before accepting the upload to prevent corrupted or malicious payloads from entering the storage layer.

## 6. Security, Privacy, and Google Play Compliance (API 36+)

CapSnap targets Android API 36+ (Android 16) to ensure long-term viability and strict adherence to modern security standards.

Privacy Guardrail Checklist

* No Guest PII: Collection of guest names, emails, or phone numbers is strictly forbidden.
* Asset Isolation: Third-party trackers, external fonts, and remote analytics are prohibited to prevent leakage to third-party providers.
* Permission Minimization: Only CAMERA is permitted. LOCATION, MICROPHONE, and CONTACTS are explicitly excluded.

Hard-Coded Retention Defaults

To maintain data hygiene and legal compliance, the following hard-coded defaults apply:

* Server Original Images: 30 days maximum after successful sync.
* Guest Feedback Records: 12 months for operational reporting, then de-identified or deleted.
* Active Guest Links: 30 days maximum or until first submission.
* Operational Logs: 30 days maximum; redacted of tokens, comments, and image bytes.

## 7. Implementation Roadmap and Operations Runbook

The implementation follows a phased de-risking strategy, prioritizing the technical hurdles of the Tauri-Android environment.

7-Phase Release Plan

1. Feasibility Spike: Hard Gate. Prove cross-compilation of the SQLite crate for Android 16 and verify Tauri-Android camera access before any UI development.
2. Local Vertical Slice: Build the offline-capable staff UI, including local persistence and the "Saved Offline" UI states.
3. Hosted Feedback Slice: Deploy the Axum/SQLite backend to a verified vServer and implement the fragment-based guest portal.
4. Operations Hardening: Execute the 3-copy encrypted backup plan and perform a full restoration drill.
5. Beta & Play Readiness: Finalize the Data Safety declarations, ensuring they reflect actual behavior (no tracking/fonts).
6. Production Rollout: Manual AAB upload via Play Console for initial release.
7. Windows 11 Target: Defer desktop packaging until mobile stability is verified; requires separate Windows/MSVC environment.

Critical Incident Runbook

| Scenario | Immediate Response | Recovery / Verification |
|---|---|---|
| Disk Nearly Full | Stop accepting server media; alert admin. | Run retention job; verify backup before manual cleanup. |
| Device Lost | Revoke device session and staff account server-side. | Re-enroll replacement; reconcile synced records. |
| Backup Failure | Pause destructive retention or migration tasks. | Repair destination; rerun encrypted snapshot and verify hash. |
| Zap Host Conflict | Verify plan: vServer vs. Static Web Space. | Hard Gate. Axum requires a long-running Linux process. |

Strategic Final Statement: The project remains in the architectural phase until the feasibility spike confirms Android SQLite compatibility and the Zap hosting plan is verified for long-running Linux processes. Legal approval of the publisher identity and the physical device camera spike are absolute blocking decisions for public release.
