# Brief — session cloud du plan DD (à coller au lancement)

Tu exécutes le plan « Plan — DD : mise en commun des doublons, front et back », en tête de `tasks/todo.md`, du début à la fin, sans fusionner.

1. Applique d'abord la reprise obligatoire d'`AGENTS.md` : `tasks/handoff.md`, `tasks/lessons.md`, `CONTEXT.md`. Lis ensuite le plan DD en entier et le rapport `docs/audit/2026-11-17-audit-doublons.md`. Réponds en français et tutoie l'utilisateur.
2. Le plan est validé par l'utilisateur, avec les décisions DD-D1 à DD-D8 telles qu'écrites, y compris les quatre « Points tranchés par l'utilisateur le 2026-11-17 » intégrés au plan. Tu es autorisé à créer les branches nommées dans le plan, à commiter, à pousser et à ouvrir les PR. Interdit : fusionner, pousser en force, écrire sur `main`.
3. Ordre : chaîne back L4 → L2 → L1 → L5 → L3 (L4 vers `main`, puis chaque PR vers la branche du lot précédent) ; chaîne front L6 → L7 (L6 vers `main`). Commence par la chaîne back.
4. Pour chaque lot : étapes dans l'ordre, gate tel qu'écrit dans « Règles communes » (gate complet à chaque lot back ; gate front à chaque lot front, puis gate complet en fin de chaîne front), preuves du lot, puis PR décrite en français (contenu, preuves, nombres du gate, limites).
5. Respecte les « Règles d'arrêt ». En cas d'arrêt, passe à l'autre chaîne si elle n'est pas finie, puis arrête-toi.
6. Ne modifie ni `tasks/todo.md` ni `tasks/handoff.md`. Tiens `docs/journal/2026-11-17-dd-back.md` (sur la chaîne back) et `docs/journal/2026-11-17-dd-front.md` (sur la chaîne front), à partir de `docs/journal/TEMPLATE.md`, avec une section par lot : ce qui est fait, preuves, nombres du gate, écarts au plan, constats « supposés » retirés.
7. À la fin, écris un récapitulatif : pour chaque lot, la branche, le dernier commit, le lien de la PR, l'état du gate, et ce qui reste ou ce qui a bloqué.
