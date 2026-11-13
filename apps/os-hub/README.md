# Atdev OS Hub

Atdev OS Hub is the web catalogue and publishing community for operating-system projects.

## Local development

Requirements: Node.js 20.19+ or 22.12+.

```sh
cp .env.example .env.local
# Fill VITE_SUPABASE_URL and VITE_SUPABASE_PUBLISHABLE_KEY from an active Supabase project.
npm install
npm run dev
```

Open the URL printed by Vite (default: http://127.0.0.1:4173).

## Authentication setup

In Supabase Dashboard → Authentication → Sign In / Providers:
1. Enable GitHub and configure a GitHub OAuth application.
2. Enable Google and configure its OAuth client.
3. Add the local and production OS Hub origins to the allowed redirect URLs.
4. Set the Supabase callback URL in each OAuth provider console.

The frontend uses Supabase Auth OAuth. It does not receive or store provider client secrets. Use only a publishable key in the browser; never put a service-role key in `.env.local`.

## Database

Review `../../supabase/schema-draft.sql` and apply it only after selecting an active project. The connected Atdev OpenWeb Linux project was reported INACTIVE during initial setup, so this application has not been connected to a live database or deployed.

## Current MVP

- Public OS catalogue and search.
- GitHub and Google sign-in buttons.
- Authenticated project metadata publishing.
- Source repository links.
- Green/blue pond-inspired Atdev visual identity.

Release management, screenshot uploads, stars, comments, moderation, and verified build provenance are planned next. ISO binaries are linked from release hosts rather than uploaded into the database service.
