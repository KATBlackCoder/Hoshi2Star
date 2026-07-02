# Prompt — Remédiation audit 2026-07-01

> Prompt prêt à coller dans une **nouvelle session** Claude Code sur ce dépôt
> (contexte frais entre deux phases). Objectif : lire les 3 documents d'audit,
> puis exécuter la prochaine phase non terminée.
>
> **Avancement** (source de vérité : `tasks/todo.md` + `docs/journal/`) :
> - Phase 1 (🔴 P0 — réinjection Common Events Wolf) : ✅ **terminée** (2026-07-01,
>   mergée sur `main`).
> - Prochaine : **Phase 2** — 🟠 paniques des parsers binaires Wolf.
> - Ordre restant : 2 → 3 → 4 → 5 → 7 → 6 → 8 → 9.

---

```
CONTEXTE
--------
Le projet Hoshi2Star a fait l'objet d'un audit d'architecture complet le 2026-07-01.
Trois documents en constituent le résultat et la feuille de route. Ils sont la source
de vérité de ta mission.

MISSION
-------
Corriger le projet en implémentant, une par une, les phases du plan de remédiation issu
de l'audit — en respectant strictement les conventions du dépôt.

ÉTAPE 0 — LECTURE OBLIGATOIRE (avant tout code)
Lis, dans cet ordre et intégralement :
  1. docs/audit-architecture-2026-07-01.md            (audit initial : périmètre, notes /10, DRY/SOLID)
  2. docs/audit-architecture-2026-07-01-complement.md  (analyses détaillées + corrections + décision #8 = Option 2)
  3. docs/audit-remediation-plan-2026-07-01.md         (les 9 phases, étape par étape — TON PLAN)
Lis aussi CONTEXT.md, CLAUDE.md, ROADMAP.md, docs/architecture.md et tasks/lessons.md
(conventions et architecture réelle du projet).
Ne présuppose rien : chaque conclusion des docs est justifiée par fichier:ligne — vérifie
sur le code réel avant d'agir, car le code a pu évoluer depuis l'audit.

RÈGLES D'EXÉCUTION (non négociables)
  - Une seule phase à la fois, dans l'ordre recommandé du plan : 1 → 2 → 3 → 4 → 5 → 7 → 6 → 8 → 9.
  - Pour CHAQUE phase :
      a. Écris le plan détaillé de la phase dans tasks/todo.md avec des items cochables
         AVANT de toucher au code (plan mode).
      b. Pour un bug : écris d'abord un test qui échoue (rouge) reproduisant le défaut,
         puis corrige jusqu'au vert.
      c. Reste strictement dans le périmètre de la phase — aucun refacto opportuniste
         (log-le comme tâche séparée dans tasks/todo.md).
      d. Gate de vérification obligatoire avant de déclarer la phase terminée :
             pnpm typecheck && \
             cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && \
             cargo test --manifest-path src-tauri/Cargo.toml
         Ne déclare jamais « fini » sans preuve que ça marche.
      e. Mets à jour la documentation impactée :
           - docs/architecture.md DÈS QU'IL Y A UNE MODIFICATION AU NIVEAU ARCHITECTURE
             (nouveau module/fonction publique, nouveau champ de type IPC, nouvelle
             commande Tauri, nouveau flux de données, changement de dispatch/couches,
             nouvelle migration SQL). Garde-le fidèle au code réel et mets à jour sa date.
           - CHANGELOG.md (skill update-changelog) à chaque fix/amélioration.
           - ROADMAP.md si l'échéance F4/F5 est touchée.
           - Un ADR (docs/adr/) si une décision d'architecture structurante est prise.
      f. Coche les items dans tasks/todo.md au fur et à mesure.
  - Utilise des sous-agents pour l'exploration/l'analyse parallèle afin de garder le
    contexte propre (cf. CLAUDE.md).
  - Après toute correction que je te donne, ajoute la leçon à tasks/lessons.md.

POINTS D'ATTENTION SPÉCIFIQUES
  - Fixtures Wolf réelles disponibles dans test/ (gitignoré) sous les noms
    Densyanai_Inko_ver2.0 (v3.5) et 月咲流ホノカver1.03 (v2) — utilise-les pour
    vérifier les fixes Wolf sur données réelles (round-trips byte-exacts).
  - Phase 2 (🟠 crash) : borner legacy_xor.rs (racine L731 orig_size, slices
    L591/1214+), passer les maths d'offset en checked_add, borner les vec!/with_capacity
    de dat_parser.rs. NE PAS toucher les catch_unwind (extractor.rs, garde-fou voulu).
    Un test « entrée malformée → Err, pas panic » par site.
  - Phase 5 : la décision produit #8 est déjà tranchée — Option 2 (exposer
    needs_review_count par fichier + reviewed_count projet). Applique-la telle quelle,
    ne rouvre pas le débat. Elle ajoute des champs aux types IPC (SourceFile,
    ProjectStats) → reflète-les dans docs/architecture.md (section domain/types.rs).
  - Phase 8 : refonte du dispatch moteur (enum FileType, découpage open_project) →
    changement d'architecture majeur, docs/architecture.md doit être remis à jour.

DÉMARRAGE
Commence par l'Étape 0 (lecture). Identifie la PROCHAINE phase non terminée dans
tasks/todo.md (les phases faites y sont marquées « livrée » ; voir aussi docs/journal/).
Présente-moi un résumé de cette phase (ce que tu as vérifié sur le code réel + ton plan
tasks/todo.md) et attends ma validation avant de coder.
```
