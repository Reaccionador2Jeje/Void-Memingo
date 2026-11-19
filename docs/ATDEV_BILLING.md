# AtDev Billing — Hybrid plans (Void-Memingo)

**Status:** proposal only. Local/open-source development workflows remain free.

## Hybrid model

Keep the core toolchain, local execution, and self-hosted workflows free. If a hosted companion service is developed, it may offer optional hosted builds, collaboration, or artifact storage. Do not charge for local use or claim hosted features before they exist.

## Proposed tiers (CLP/month; exploratory)

| Plan | Reference price | Scope |
| --- | ---: | --- |
| Community | $0 | Local and self-hosted functionality |
| Supporter | Voluntary | Optional contribution |
| Developer | $990 | Future hosted build/storage allowance, if implemented |

Suggested future entitlements: `void_memingo.hosted_builds.monthly`, `void_memingo.artifacts.storage_bytes`, `void_memingo.team.seats`.

No checkout is active. Hosted quotas must be enforced server-side, with clear export/deletion and outage behavior. Adult account-owner review is required before real-money launch.
