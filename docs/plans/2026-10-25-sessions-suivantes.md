# Sessions suivantes — prompts prêts à l'emploi

**Date :** 2026-10-25 — Claude. **Statut :** prêt, non lancé.
Une session par travail. Chaque prompt suit la méthode du skill `ambition` (profondeur :
matière brute, outils précis, juge extérieur) et les règles d'`AGENTS.md`.

**Ordre conseillé :**
1. Analyse complète : `docs/plans/2026-10-25-analyse-complete-app.md` (préalable aux autres).
2. Lot A — export partiel.
3. Lot B — QA lisible.
4. Lot C — navigation de recherche.
5. Étude REA de RuneTranslate : `RAE-PLAN/runetranslate/PLAN.md` (local).
6. Modèles et fournisseurs IA (peut tourner en parallèle des lots, autre zone du code).
7. Préparation de release.

Lots A à C : relire d'abord les conclusions de l'analyse complète (`docs/audit/`), qui peuvent en
changer l'ordre ou le contenu. Pour lancer une session :
`@docs/plans/2026-10-25-sessions-suivantes.md` puis « Exécute le prompt du lot X ».

Règles communes aux sessions de code :
- une branche + une PR par lot, jamais de commit direct sur `main` ;
- plan vérifiable dans `tasks/todo.md` avant le code ;
- gate complet avant la PR : `pnpm typecheck && pnpm lint && pnpm test && cargo fmt --check
  && cargo clippy --all-targets -- -D warnings && cargo test` ;
- vérification dans l'app réelle avec le MCP `tauri` (`pnpm tauri:linux`), captures à l'appui ;
- `tasks/handoff.md` et un journal `docs/journal/` mis à jour en cours de route.

---

## Lot A — Export partiel (priorité 1)

**Juge extérieur :** un projet MV partiellement traduit s'exporte, le jeu se lance, et les lignes
non traduites restent identiques à l'original (comparaison octet pour octet des fichiers non touchés).

> Lis `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`, la section « Retours utilisateur
> sportsfan30 » de `tasks/todo.md`, et le rapport d'analyse dans `docs/audit/` s'il existe.
>
> Objectif : un traducteur qui travaille par petits lots et teste en jeu doit pouvoir exporter
> sans tout traduire. Contexte : jeu MV, v0.4.10 ; l'interface bloque l'export tant qu'il reste
> des lignes non traduites (`exportAllBlockedDesc`), alors que le backend ignore déjà les segments
> vides (`commands/export.rs`, filtres `target_text != ''`).
>
> 1. Vérifie ce constat dans le code actuel, puis écris un plan dans `tasks/todo.md`.
> 2. Implémente : export d'un projet partiel avec avertissement clair (nombre de lignes restées en
>    langue source) ; export d'un ou plusieurs fichiers choisis ; statut « fichier ignoré »
>    (ex. `MapInfos.json`) exclu des stats, du QA et de l'export. Migration SQL si nécessaire,
>    numérotée après la dernière existante.
> 3. Preuve : tests Rust d'export partiel (fichiers non touchés identiques octet pour octet),
>    tests frontend de l'avertissement, essai réel via le MCP `tauri` sur un projet MV.
> 4. Gate complet, branche `feat/partial-export`, PR avec résultats.

## Lot B — QA lisible (priorité 2)

**Juge extérieur :** sur un vrai projet, le filtre « Score < 100 » de la grille liste les mêmes
segments que le rapport QA, et le compteur « X/Y ok » correspond au nombre de segments à 100.

> Lis `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`, la section sportsfan30 de
> `tasks/todo.md`, et `docs/audit/` s'il existe.
>
> Objectif : voir d'un coup d'œil les lignes qui demandent encore du travail. Constats :
> le score QA n'est enregistré qu'à l'audit d'export (`core/report.rs`), donc la colonne QA de la
> grille affiche « — » et les filtres QA sont vides ; une ligne trop large ne coûte que 10 points,
> donc elle n'apparaît jamais sous « Score < 70 » ; le compteur de projet et le panneau du segment
> peuvent se contredire (masquage `line_too_long` quand le text fitting est actif).
>
> 1. Vérifie ces trois constats dans le code actuel, puis écris un plan dans `tasks/todo.md`.
> 2. Implémente : score QA calculé et enregistré dès qu'une cible change (traduction LLM, édition,
>    import de pack) et recalcul possible pour un projet existant ; filtres « Score < 100 » nommé
>    clairement et « par type d'erreur » (dont `line_too_long`) ; compteur cohérent avec le panneau.
> 3. Preuve : tests Rust et frontend, mesure du temps de recalcul sur un projet de 15 000 lignes,
>    essai réel via le MCP `tauri`.
> 4. Gate complet, branche `feat/qa-filters`, PR avec résultats.

## Lot C — Navigation de recherche (priorité 3)

**Juge extérieur :** dans un CommonEvents de 15 000 lignes, un clic sur un résultat de recherche
globale affiche et sélectionne la bonne ligne à l'écran (capture via MCP `tauri`).

> Lis `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`, la section sportsfan30 de
> `tasks/todo.md`, et `docs/audit/` s'il existe.
>
> Objectif : retrouver une réplique dans un gros fichier. Constats : le clic sur un résultat global
> ouvre le fichier et sélectionne le segment sans faire défiler la grille
> (`GlobalSearchResults.tsx`) ; l'emplacement s'affiche en pointeur JSON brut tronqué ; la
> recherche dans le fichier renumérote les résultats et perd la vraie position.
>
> 1. Vérifie ces constats dans le code actuel, puis écris un plan dans `tasks/todo.md`.
> 2. Implémente : défilement jusqu'au segment au clic ; emplacement lisible (nom et ID de
>    l'événement, numéro de ligne) pour MV/MZ et Wolf ; vrai numéro de ligne conservé quand on filtre.
> 3. Preuve : tests frontend, essai réel avec captures via le MCP `tauri` sur un gros fichier.
> 4. Gate complet, branche `feat/search-navigation`, PR avec résultats.

## Modèles et fournisseurs IA

**Juge extérieur :** un jeu de test fixe tiré de vrais jeux, traduit par chaque modèle candidat,
noté par des contrôles automatiques (placeholders, QA, sortie structurée valide) et par une relecture
humaine à l'aveugle de l'utilisateur ; coût réel mesuré par `ProviderCallMetrics`.

> Lis `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`, puis
> `docs/research/rapports/Alternatives Ollama et crates Rust.md` et les réponses reçues de
> sportsfan30 et ArcoFrio sur leurs modèles (si l'utilisateur les a collées).
>
> Objectif : que Hoshi2Star traduise bien sans dépendre d'un seul fournisseur ni d'un PC puissant.
> Contexte : PC de développement faible ; un seul adaptateur `OpenAiCompatibleProvider` ; JSON
> structuré et `reasoning_effort` activés seulement pour Ollama ; un refus du modèle déclenche
> probablement des retries puis un split sur le même fournisseur.
>
> 1. Vérifie ces constats dans le code actuel (`src-tauri/src/llm/`), puis écris un plan dans `tasks/todo.md`.
> 2. Construis d'abord le banc d'essai : 50 à 100 segments réels (dialogues, choix, termes système avec
>    `%1`, codes de contrôle), un script qui les traduit avec plusieurs modèles et sort un tableau
>    (validité, QA, tokens, coût, temps). Commence par 3–4 modèles cloud peu chers via OpenRouter ;
>    les clés API sont fournies par l'utilisateur et ne sont jamais écrites dans le dépôt.
> 3. Ensuite seulement, propose les changements de code : erreur `Refused` distincte avec repli vers
>    un autre fournisseur, JSON structuré pour les fournisseurs compatibles, préréglages de
>    fournisseurs (Ollama, LM Studio, llama-server, OpenRouter, DeepSeek, Gemini), et Anthropic/Gemini
>    natifs via `genai` si le banc d'essai le justifie.
> 4. Preuve : le tableau du banc d'essai avant/après, tests Rust, gate complet, branche
>    `feat/llm-providers`, PR avec résultats.

## Préparation de release

**Juge extérieur :** le build de release se construit, et l'export fonctionne sur 2–3 jeux MV/MZ
réels testés en jeu par l'utilisateur.

> Lis `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`, `CHANGELOG.md` et
> `.github/workflows/release.yml`.
>
> Objectif : préparer la prochaine version publique.
> 1. Liste ce qui a changé depuis v0.4.10 (`git log v0.4.10..main`) et mets à jour `CHANGELOG.md`
>    en français et anglais, en créditant ArcoFrio (PR #5 et #6) comme promis.
> 2. Vérifie que le build de release se construit (`pnpm tauri build`) et que le plugin
>    `mcp-bridge` en est absent.
> 3. Prépare une checklist de test manuel pour l'utilisateur : export sur 2–3 jeux MV/MZ, dont un
>    ancien projet, et lancement en jeu.
> 4. Ne crée ni tag ni release sans mon accord explicite.
