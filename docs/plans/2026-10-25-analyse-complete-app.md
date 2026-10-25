# Analyse complète de Hoshi2Star — préparation de session

**Date :** 2026-10-25 — Claude. **Statut :** prêt, non lancé. Outillage à jour au 2026-10-25.
**Méthode :** prompt « deeper, not bigger » de Felix Rieseberg (ingénieur Claude, *How to be more
ambitious: a tutorial for humans with models*, 13/10/2026). Avant d'écrire la demande, on répond à
4 questions, puis on réécrit la demande pour exiger exactement ça :

1. Que ferait ici un professionnel de haut niveau, que je ne demande pas ?
2. De quelle matière brute partirait-il, au lieu de ce qu'on m'a déjà résumé ?
3. Quelles méthodes et quels outils précis utiliserait-il ?
4. Qu'est-ce qui, à part toi ou moi, montrerait que le résultat est juste ?

Demande initiale : « analyse de façon complète mon app ».

## 1. Ce qu'un professionnel ferait en plus

- Une analyse **orientée risques**, classée par impact utilisateur, pas un inventaire du code :
  perte de données et corruption du jeu à l'export d'abord, puis sécurité, qualité de traduction,
  ergonomie, maintenabilité.
- Un **modèle de menaces** des entrées non fiables : fichiers de jeu, `plugins.js`, archives Wolf,
  packs `.h2s`, `.hoshi2star.json`, réponses du LLM.
- Le **parcours réel complet** par moteur (MV, MZ, Wolf v2, Wolf v3) : ouvrir → extraire → traduire
  → QA → exporter → appliquer au jeu, avec un projet neuf et un projet ancien.
- Un rapprochement **produit** : les retours utilisateurs (sportsfan30, ArcoFrio, F95zone) et la
  ROADMAP comparés à ce que le code fait réellement.
- Des **recommandations chiffrées** : effort, risque, ordre, et ce qu'on ne fait pas.

## 2. Matière brute à utiliser (pas les résumés)

- Le **code** de `main` (Rust ~30k lignes, TS ~12k), pas `CONTEXT.md` ni `ROADMAP.md`, qui peuvent
  être en retard (cf. `tasks/lessons.md` du 2026-09-21).
- L'**historique Git** et `docs/journal/` : fichiers les plus corrigés, régressions passées.
- Des **jeux réels** MV, MZ et Wolf v2/v3, plus les fixtures `src-tauri/tests/` ; une **copie** de la
  base locale (`~/.local/share/com.blackat.hoshi2star/hoshi2star.db`, jamais l'original).
- Les **sorties brutes** : logs de l'app, métriques `ProviderCallMetrics`, résultats de tests,
  y compris les 2 tests Windows qui échouent.
- Les **retours utilisateurs** bruts : captures et messages de sportsfan30 (`tasks/todo.md`).
- L'audit du 2026-09-05 et les journaux des PR #5/#6, à **revérifier**, pas à recopier.

## 3. Méthodes et outils

| Besoin | Outil |
|---|---|
| Carte de l'architecture et des flux | sous-agents en parallèle par couche (engines, llm, core, export, UI) ; skill `diagram` |
| Qualité Rust | `cargo clippy -- -W clippy::pedantic` (lecture seule), `cargo audit`, `cargo deny`, `cargo llvm-cov` |
| Qualité TS | `pnpm audit`, couverture Vitest, recherche de code mort |
| Sécurité | skills `security-review`, `cso` ; plugin `finecomb` (à installer, optionnel) |
| Revue de code | skill `code-review`, `bmad-code-review` ; plugin officiel `/code-review:code-review` (installé ; publie un commentaire sur la PR) |
| Santé globale | skill `health` |
| Exactitude des formats | tests aller-retour : `inject(extract(jeu))` identique octet par octet sans traduction |
| App réelle | MCP `tauri` (`@hypothesi/tauri-mcp-server@0.13.2`) : piloter l'app lancée par `pnpm tauri:linux`, captures, IPC |
| Docs à jour (Tauri v2, sqlx, React 19) | MCP `context7` (connecté) |
| Coût LLM pour les essais | modèle cloud peu cher via OpenRouter (voir `docs/research/rapports/`) |

## 4. Ce qui prouvera que c'est juste

- Chaque constat donne `fichier:ligne` **et** une reproduction : test qui échoue, commande, ou capture.
- Aller-retour octet pour octet sur au moins un jeu réel par moteur.
- Un export réel lancé dans le jeu par l'utilisateur (seul juge final de la stabilité du jeu).
- Les compteurs se recoupent : segments extraits = segments en base = segments exportés.
- Les 4 retours de sportsfan30 sont reproduits ou réfutés.
- Une seconde passe de relecture indépendante sur les constats bloquants.
- Chaque point de l'audit du 2026-09-05 : corrigé / toujours présent / non vérifiable, avec preuve.

## Outillage disponible et à corriger

- **Déjà disponibles** : skills `code-review`, `security-review`, `cso`, `health`, `investigate`,
  `diagram`, `bmad-code-review`, `bmad-review`, `bmad-deep-recon`, `make-interfaces-feel-better`,
  `accessibility`, `ambition` ; MCP `tauri`, `context7`, `filesystem`, `rea`.
- **Corrigé le 2026-10-25** :
  - `.mcp.json` : `filesystem` pointe vers `/home/blackat/Bureau/Hoshi2Star` ;
  - MCP `tauri` réparé : version fixée `@hypothesi/tauri-mcp-server@0.13.2`, plugin de l'app
    passé en 0.13.2 avec Tauri 2.12 (PR #9) ; connexion vérifiée ;
  - **Context7** ajouté en MCP utilisateur (HTTP, connecté) : docs à jour de Tauri v2, sqlx, React 19 ;
  - les ~35 skills Tauri déjà installés pour Codex (`.agents/skills/*tauri*`, `rust-skills`,
    `zustand-state-management`, `tanstack-table`, `shadcn`) sont rendus disponibles à Claude par
    liens symboliques locaux dans `.claude/skills/` (ignorés par Git, cf. `.gitignore`).
    Utiles pour l'analyse : `understanding-tauri-ipc`, `understanding-tauri-runtime-authority`,
    `configuring-tauri-capabilities`, `configuring-tauri-csp`, `understanding-tauri-ecosystem-security`,
    `testing-tauri-apps`, `rust-skills`.
- **Plugin officiel `code-review`** : installé et activé (`/code-review:code-review`). Aucun plugin
  Tauri dédié dans l'annuaire.
- **Optionnels, non installés** : `Rust CLI Skills`, `finecomb`.
- Mode auto : le classifieur a bloqué Bash à plusieurs reprises le 2026-10-25 (incident
  transitoire côté service).

## Découpage en 3 sessions

Le travail est trop gros pour une seule session (contexte saturé avant le rapport). Une session
par phase ; chacune s'arrête à son livrable, met à jour `tasks/handoff.md` et la case ci-dessous.
Livrable unique enrichi au fil des sessions : `docs/audit/2026-10-analyse-complete.md`.

- [ ] **Session 1 — Carte et contrôles automatiques** (aucun jeu réel nécessaire)
  - Cartographie par couche (sous-agents en parallèle : engines, llm, core, export, UI) et modèle de
    menaces des entrées non fiables.
  - Contrôles : clippy pedantic, `cargo audit`, `cargo deny`, couverture Rust/TS, `pnpm audit`,
    `security-review`. Installation d'outils manquants : accord utilisateur.
  - Livrable : sections « Architecture », « Menaces », « Constats statiques » du rapport + liste
    des points à confirmer sur jeux réels.
  - Arrêt : quand ces sections sont écrites. Ne pas commencer les jeux réels.
- [ ] **Session 2 — Parcours réels sur jeux** (demande à l'utilisateur les chemins des jeux et le
      modèle de traduction des essais)
  - Ordre : MV (cas de sportsfan30), MZ, Wolf v2, Wolf v3 ; pour chacun un projet neuf et un ancien.
  - Aller-retour octet pour octet sans traduction ; traduction d'un petit échantillon ; QA ; export.
  - Point d'arrêt possible après chaque moteur (consigner l'avancement dans `tasks/handoff.md`).
  - Livrable : section « Parcours réels » avec preuves ; exports prêts à tester en jeu par l'utilisateur.
- [ ] **Session 3 — Vérifications ciblées et rapport final**
  - Reproduire ou réfuter les 4 retours de sportsfan30 et les points A–F de l'audit du 2026-09-05.
  - Seconde relecture indépendante des constats bloquants.
  - Livrable : synthèse d'une page, liste priorisée (gravité, effort, preuve), ce qu'on ne fait
    pas et pourquoi, proposition de lots pour `tasks/todo.md`.

Pour lancer une session : joindre ce plan et écrire
« Exécute la session N du plan, avec le prompt réécrit ci-dessous. »

## Prompts prêts à coller, un par session

**Session 1**

> @docs/plans/2026-10-25-analyse-complete-app.md
> Exécute la **session 1** de ce plan. Lis d'abord `AGENTS.md`, `tasks/handoff.md` et
> `tasks/lessons.md`. Ne modifie aucun code applicatif.
> Fais la carte du code par couche avec des sous-agents en parallèle (engines, llm, core, export, UI),
> le modèle de menaces des entrées non fiables, puis les contrôles automatiques : clippy pedantic,
> `cargo audit`, `cargo deny`, couverture Rust et TS, `pnpm audit`, `security-review`.
> Demande-moi avant d'installer un outil. Chaque constat : `fichier:ligne`, gravité, reproduction,
> classé vérifié / déduit / inconnu.
> Crée `docs/audit/2026-10-analyse-complete.md` avec les sections « Architecture », « Menaces »,
> « Constats statiques » et « À confirmer sur jeux réels ». Arrête-toi là, sans toucher aux jeux réels.
> Coche la session 1 dans le plan, mets à jour `tasks/handoff.md` et crée l'entrée de journal.

**Session 2**

> @docs/plans/2026-10-25-analyse-complete-app.md
> Exécute la **session 2** de ce plan. Lis d'abord `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`
> et `docs/audit/2026-10-analyse-complete.md`. Ne modifie aucun code applicatif.
> Avant de commencer, demande-moi les chemins de mes jeux de test (MV, MZ, Wolf v2, Wolf v3) et le
> modèle à utiliser pour les traductions d'essai. Travaille sur une copie de ma base, jamais l'originale.
> Ordre : MV, MZ, Wolf v2, Wolf v3. Pour chaque moteur, avec un projet neuf et un ancien : ouvrir,
> extraire, réinjecter sans traduction (fichiers identiques octet pour octet), traduire un petit
> échantillon, QA, exporter. Utilise le MCP `tauri` sur l'app lancée par `pnpm tauri:linux`.
> Vérifie les points « À confirmer sur jeux réels » de la session 1.
> Ajoute la section « Parcours réels » au rapport, avec preuves. Prépare les exports que je testerai
> moi-même dans le jeu. Après chaque moteur, consigne l'avancement dans `tasks/handoff.md` ;
> si la session doit s'arrêter, la suivante reprendra au moteur suivant. Coche la session 2 à la fin.

**Session 3**

> @docs/plans/2026-10-25-analyse-complete-app.md
> Exécute la **session 3** de ce plan. Lis d'abord `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`
> et `docs/audit/2026-10-analyse-complete.md`. Ne modifie aucun code applicatif.
> Reproduis ou réfute, avec preuve, les 4 retours de sportsfan30 (`tasks/todo.md`) et les points A–F
> de `docs/audit-mv-mz-ia-2026-09-05.md`. Fais relire les constats bloquants par une seconde passe
> indépendante (sous-agent, ou skill `code-review` en local sur les fichiers concernés ; pas le plugin
> `/code-review:code-review`, qui publie sur GitHub).
> Termine le rapport : synthèse d'une page, liste priorisée (gravité, effort, preuve), ce qu'on ne fait
> pas et pourquoi, proposition de lots pour `tasks/todo.md` (un plan détaillé par lot sera écrit plus tard).
> Tiens compte des résultats de mes tests en jeu s'ils sont consignés dans `tasks/handoff.md`.
> Coche la session 3, mets à jour `tasks/handoff.md` et le journal.

## Prompt réécrit pour la nouvelle session

> Lis `AGENTS.md`, `tasks/handoff.md`, `tasks/lessons.md`, puis ce plan
> (`docs/plans/2026-10-25-analyse-complete-app.md`). Ne modifie aucun code applicatif.
>
> Je veux une analyse **plus profonde, pas plus large** de Hoshi2Star, comme la ferait un
> ingénieur senior chargé de décider quoi corriger avant la prochaine release publique.
>
> 1. Pars du code de `main`, de l'historique Git, des journaux, des tests et de jeux réels, pas des
>    documents de synthèse. Utilise une copie de ma base, jamais l'originale.
> 2. Classe les risques par impact utilisateur : corruption du jeu ou perte de traduction, sécurité
>    des entrées non fiables, qualité de traduction, ergonomie, maintenabilité.
> 3. Pour chaque moteur supporté, déroule le parcours réel complet, avec un projet neuf et un ancien.
>    Vérifie l'aller-retour octet pour octet sans traduction.
> 4. Reproduis ou réfute les 4 retours de sportsfan30 et chaque point de l'audit du 2026-09-05.
> 5. Utilise les outils listés dans le plan (sous-agents par couche, clippy pedantic, cargo audit,
>    couverture, security-review, tauri MCP pour l'app réelle). Les ajouts d'outils demandent mon accord.
> 6. Chaque constat : `fichier:ligne`, gravité, reproduction (test, commande ou capture) et effort
>    estimé. Sépare vérifié, déduit et inconnu. Fais relire les constats bloquants par une seconde passe.
>
> Livrable : `docs/audit/2026-10-analyse-complete.md`, avec une synthèse d'une page, la liste
> priorisée, ce qu'on ne fait pas et pourquoi, et une proposition de lots pour `tasks/todo.md`.
> Mets à jour `tasks/handoff.md` et le journal en cours de route.
