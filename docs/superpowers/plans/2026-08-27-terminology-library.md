# Hoshi2Star Terminology Library Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Keep the existing uncommitted QA contract work isolated and run every gate before moving to the next phase.

**Goal:** Remplacer le glossaire actuel par une base terminologique évolutive, globale et multilingue qui analyse localement les textes japonais extraits, classe les termes avec la logique propre au moteur, aide le modèle uniquement avec les termes pertinents et contrôle leur cohérence sans alourdir Tauri, React, SQLite ni les prompts.

**Architecture:** SQLite conserve une entrée source globale, ses traductions par langue et portée, ses variantes, ses occurrences par segment et l'état des scans incrémentaux. Un service Rust borné charge l'analyseur japonais une seule fois, combine les entités sûres fournies par l'adaptateur MV/MZ avec les lemmes Lindera, puis écrit par petits lots. Le frontend React affiche une page `Terminologie` paginée côté serveur avec TanStack Query/Table. Au moment de chaque véritable requête LLM, le pipeline résout les termes à partir des IDs de segments du lot; il n'envoie jamais une liste arbitraire. La QA réutilise le même résolveur dans un mode pur et non-mutant.

**Tech Stack:** Rust 2021, Tauri v2, SQLx/SQLite, Lindera 5.1/IPADIC, Tokio, React 19, TypeScript, TanStack Query/Table/Virtual, Zustand limité à l'état d'interface, shadcn/Radix, Vitest, Cargo tests, MCP Tauri.

---

## Principes non négociables

- Les entrées source sont globales et survivent à la suppression d'un projet; seules leurs occurrences et leurs traductions propres à ce projet sont supprimées en cascade.
- Une traduction est une donnée distincte de l'entrée source. Une entrée non traduite n'a pas de ligne vide dans `terminology_translations`.
- `part_of_speech` décrit la grammaire (`noun`, `verb`, `adjective`); `semantic_type` décrit le sens moteur (`character`, `item`, `weapon`, `skill`, etc.). Les deux ne doivent pas être fusionnés.
- MV/MZ décide comment transformer ses `segment_kind` en types sémantiques. Le noyau terminologique ne contient aucun `match` propre à RPG Maker.
- Lindera est un analyseur local et ne remplace pas l'adaptateur moteur. Les noms/items/armes/classes reconnus structurellement ont priorité sur une inférence morphologique.
- Aucun terme n'est automatiquement `locked`. Une traduction générée par modèle commence en `proposed`.
- Les verbes et adjectifs sont injectés comme indications contextuelles par défaut. Seules les entités stables et explicitement verrouillées sont contrôlées par correspondance exacte.
- L'absence de scan ne bloque pas la traduction du jeu; elle signifie seulement qu'aucune aide terminologique non résolue ne sera injectée.
- Le scan ne contacte aucun fournisseur. La traduction des termes est une action séparée et explicite.
- Les listes complètes ne traversent jamais l'IPC: pagination, filtres et agrégations restent côté SQLite.
- Le code historique `glossary` reste une façade temporaire uniquement pendant la migration; il ne doit pas devenir un second système synchronisé.

## Découpage de livraison

| Jalon | Contenu                                        | Dépend de        | Critère de sortie                                                     |
| ----- | ---------------------------------------------- | ---------------- | --------------------------------------------------------------------- |
| T0    | Stabiliser les contrats QA déjà présents       | commit `7c1b6b9` | worktree propre et suite actuelle verte                               |
| T1    | Spike analyseur + budget ressources            | T0               | choix documenté embedded ou ressource mmap, budgets tenus             |
| T2    | Schéma et repository terminologique            | T1               | migration d'une DB existante sans perte et CRUD testé                 |
| T3    | Adaptateur MV/MZ + scan local incrémental      | T2               | fixture ja analysée de façon déterministe, annulation/reprise testées |
| T4    | API Tauri + page Terminologie                  | T3               | scan/filtre/édition utilisables sans provider                         |
| T5    | Traduction assistée des termes                 | T4               | ja→en et ja→fr, résultats `proposed`, validation manuelle             |
| T6    | Résolution par requête et intégration pipeline | T5               | zéro fallback arbitraire, contexte borné et pertinent                 |
| T7    | QA terminologique, packs et durcissement       | T6               | tests complets, MCP Tauri et jeu pilote satisfaisants                 |

Ne pas commencer T2 si T1 dépasse le budget sans avoir testé le mode dictionnaire externe mmap. Ne pas commencer T6 tant que les traductions proposées/validées ne sont pas distinguées dans la base et l'UI.

---

### Task 0: Isoler et valider le socle QA déjà en cours

**Files:**

- Modify only if failing: `src-tauri/tests/e2e_project_flow.rs`
- Existing untracked fixtures: `src-tauri/tests/fixtures/mv_mz/`
- Existing untracked contract: `src-tauri/tests/qa_v2_contract.rs`

- [x] **Step 1: Examiner les changements existants sans les reformater globalement**

Run: `git diff -- src-tauri/tests/e2e_project_flow.rs && git status --short`

Expected: seulement les fixtures/contrats QA de la phase précédente sont présents; aucun fichier applicatif inattendu.

- [x] **Step 2: Exécuter les contrats ciblés**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test qa_v2_contract --test e2e_project_flow`

Expected: PASS ou tests explicitement `ignored` avec le motif d'implémentation déjà documenté; aucune erreur de fixture.

- [x] **Step 3: Exécuter la base de régression avant la nouvelle fonctionnalité**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `pnpm typecheck && pnpm test && pnpm build`

Expected: PASS; le warning de taille de chunk Vite déjà connu est acceptable, pas une nouvelle erreur.

- [x] **Step 4: Créer un commit séparé pour ce socle**

Run: `git add src-tauri/tests/e2e_project_flow.rs src-tauri/tests/fixtures/mv_mz src-tauri/tests/qa_v2_contract.rs`

Run: `git commit -m "test(qa): add multilingual MV/MZ quality contracts"`

Expected: le travail QA est indépendant de la terminologie et le worktree redevient propre.

---

### Task 1: Mesurer Lindera avant de l'intégrer à l'application

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Create: `src-tauri/benches/terminology_analyzer.rs`
- Create: `src-tauri/tests/fixtures/terminology/ja_tokens.json`
- Create: `docs/benchmarks/terminology-analyzer-2026-08-27.md`
- Conditional if embedded exceeds the gate: `src-tauri/tauri.conf.json`
- Conditional if embedded exceeds the gate: `src-tauri/resources/lindera-ipadic/`

- [x] **Step 1: Écrire le corpus attendu avant l'adaptateur**

Créer une fixture UTF-8 avec au minimum: nom propre en kanji, nom en katakana, verbe conjugué, adjectif en `い`, adjectif en `な`, particules, auxiliaires, nombre, code RPG Maker (`\\V[1]`, `\\N[2]`, `\\C[3]`), nom d'item composé et deux phrases provenant des fixtures MV/MZ. Pour chaque cas, stocker le lemme attendu, la catégorie générale attendue et les tokens à exclure.

- [x] **Step 2: Ajouter Lindera 5.1 dans une configuration mesurable**

Commencer avec `lindera = { version = "5.1", features = ["embed-ipadic"] }`. Ajouter un bench `harness = false` qui utilise `std::time::Instant` et la mesure RSS native de la plateforme afin de ne pas ajouter Criterion au binaire applicatif. Ne pas activer UniDic, NEologd ou CJK dans le premier build.

Run: `cargo check --manifest-path src-tauri/Cargo.toml`

Expected: le crate et IPADIC se compilent sur la toolchain du dépôt.

- [x] **Step 3: Écrire un benchmark reproductible**

Le benchmark doit mesurer séparément: temps de construction de l'analyseur, temps froid, temps chaud, débit sur 1 191 segments, RSS avant/après, taille du binaire release et taille imputable au dictionnaire. Répéter cinq fois et publier médiane/p95; ne pas utiliser le réseau pendant la mesure.

Run: `cargo bench --manifest-path src-tauri/Cargo.toml --bench terminology_analyzer`

Expected: un rapport contient toutes les mesures, la version Lindera, l'OS, l'architecture et le hash du corpus.

- [x] **Step 4: Appliquer le gate de ressources**

Le mode embedded est accepté si: analyse chaude de 1 191 segments ≤ 5 s, re-scan sans changement ≤ 1 s, hausse RSS après initialisation ≤ 180 MiB, ajout au paquet compressé ≤ 25 MiB et CPU au repos revient à 0 %. Sinon, retirer `embed-ipadic`, empaqueter IPADIC 5.1 sous `src-tauri/resources/lindera-ipadic/`, déclarer cette ressource dans `tauri.conf.json`, résoudre son chemin avec l'API Tauri et la charger en mmap, puis répéter exactement le benchmark. Si les deux modes échouent, arrêter ce plan avant migration et comparer Vibrato sur le même trait/corpus.

- [x] **Step 5: Vérifier licences et distribution hors ligne**

Documenter les licences de Lindera et du dictionnaire IPADIC retenu. Confirmer que l'analyse fonctionne dans un build release sans téléchargement au premier lancement.

- [x] **Step 6: Commit**

Run: `git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/benches/terminology_analyzer.rs src-tauri/tests/fixtures/terminology/ja_tokens.json docs/benchmarks/terminology-analyzer-2026-08-27.md`

Run: `git commit -m "perf(terminology): validate Japanese analyzer budgets"`

---

### Task 2: Créer le schéma SQLite et migrer le glossaire sans perte

**Files:**

- Create: `src-tauri/migrations/0008_terminology.sql`
- Modify: `src-tauri/src/db/pool.rs`
- Create: `src-tauri/tests/terminology_migration.rs`

- [x] **Step 1: Écrire d'abord les tests de migration**

Les tests doivent créer une DB au niveau 0007 avec: un terme global manuel, un terme projet auto-généré, deux paires `ja-en`/`ja-fr` et une valeur vide historique. Après 0008, vérifier:

- conservation des traductions non vides;
- traduction manuelle en `approved/required`, traduction auto en `proposed/preferred`;
- ligne vide migrée comme entrée sans traduction;
- portée `project_id` conservée sur la traduction;
- suppression d'un projet: occurrences/traductions locales supprimées, entrée globale conservée;
- migration idempotente dans une DB neuve.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test terminology_migration`

Expected: FAIL car les tables n'existent pas.

- [x] **Step 2: Ajouter le schéma 0008**

Le schéma doit créer exactement ces responsabilités:

```sql
CREATE TABLE terminology_entries (
  id TEXT PRIMARY KEY NOT NULL,
  source_language TEXT NOT NULL,
  canonical_text TEXT NOT NULL,
  normalized_text TEXT NOT NULL,
  reading TEXT,
  part_of_speech TEXT NOT NULL CHECK(part_of_speech IN
    ('noun','proper_noun','verb','adjective','adverb','expression','unknown')),
  semantic_type TEXT NOT NULL,
  sense_key TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active','ignored','archived')),
  origin TEXT NOT NULL CHECK(origin IN
    ('engine','lindera','manual','legacy_glossary','import')),
  confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0.0 AND confidence <= 1.0),
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(source_language, normalized_text, semantic_type, sense_key)
);

CREATE TABLE terminology_translations (
  id TEXT PRIMARY KEY NOT NULL,
  entry_id TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
  target_language TEXT NOT NULL,
  project_id TEXT REFERENCES projects(id) ON DELETE CASCADE,
  target_text TEXT NOT NULL CHECK(length(trim(target_text)) > 0),
  review_status TEXT NOT NULL CHECK(review_status IN ('proposed','approved','locked')),
  enforcement TEXT NOT NULL CHECK(enforcement IN ('contextual','preferred','required')),
  confidence REAL NOT NULL DEFAULT 1.0 CHECK(confidence >= 0.0 AND confidence <= 1.0),
  provider_id TEXT,
  model TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE UNIQUE INDEX idx_term_translation_global
  ON terminology_translations(entry_id, target_language)
  WHERE project_id IS NULL;
CREATE UNIQUE INDEX idx_term_translation_project
  ON terminology_translations(entry_id, target_language, project_id)
  WHERE project_id IS NOT NULL;

CREATE TABLE terminology_source_variants (
  id TEXT PRIMARY KEY NOT NULL,
  entry_id TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
  surface_text TEXT NOT NULL,
  normalized_text TEXT NOT NULL,
  variant_kind TEXT NOT NULL CHECK(variant_kind IN ('inflection','alias','orthography','reading')),
  UNIQUE(entry_id, normalized_text, variant_kind)
);

CREATE TABLE terminology_target_variants (
  id TEXT PRIMARY KEY NOT NULL,
  translation_id TEXT NOT NULL REFERENCES terminology_translations(id) ON DELETE CASCADE,
  text TEXT NOT NULL,
  normalized_text TEXT NOT NULL,
  UNIQUE(translation_id, normalized_text)
);

CREATE TABLE terminology_occurrences (
  entry_id TEXT NOT NULL REFERENCES terminology_entries(id) ON DELETE CASCADE,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  segment_id TEXT NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
  surface_text TEXT NOT NULL,
  engine_kind TEXT NOT NULL,
  occurrence_count INTEGER NOT NULL DEFAULT 1 CHECK(occurrence_count > 0),
  PRIMARY KEY(entry_id, segment_id, surface_text)
);

CREATE TABLE terminology_segment_scans (
  segment_id TEXT PRIMARY KEY NOT NULL REFERENCES segments(id) ON DELETE CASCADE,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  source_hash TEXT NOT NULL,
  analyzer_version TEXT NOT NULL,
  scanned_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE terminology_scans (
  id TEXT PRIMARY KEY NOT NULL,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  source_language TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('running','completed','cancelled','failed')),
  analyzer_version TEXT NOT NULL,
  processed_segments INTEGER NOT NULL DEFAULT 0,
  total_segments INTEGER NOT NULL DEFAULT 0,
  discovered_entries INTEGER NOT NULL DEFAULT 0,
  started_at TEXT NOT NULL DEFAULT (datetime('now')),
  finished_at TEXT,
  error TEXT
);
```

Ajouter les index de lecture: entrées par `(source_language, normalized_text)`, traductions par `(target_language, review_status)`, occurrences par `(project_id, segment_id)` et scans de segments par `project_id`.

- [x] **Step 3: Migrer les données historiques dans la même migration**

Découper `lang_pair` au premier tiret uniquement pour les valeurs historiques connues (`ja-en`, `ja-fr`). Utiliser des IDs déterministes dérivés de l'ancien ID afin que la migration soit vérifiable. Ne pas supprimer `glossary_terms` en 0008; arrêter simplement toute nouvelle écriture après la bascule applicative.

- [x] **Step 4: Mettre à jour la validation de schéma**

Dans `validate_patch_schema`, remplacer l'exigence applicative `glossary_terms` par les tables minimales `terminology_entries`, `terminology_translations` et `terminology_occurrences`. Garder la table historique tolérée, mais non requise pour une future DB.

- [x] **Step 5: Vérifier migration et contraintes**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test terminology_migration db::pool`

Expected: PASS, y compris cascades, index partiels et réouverture de la DB.

- [x] **Step 6: Commit**

Run: `git add src-tauri/migrations/0008_terminology.sql src-tauri/src/db/pool.rs src-tauri/tests/terminology_migration.rs`

Run: `git commit -m "feat(terminology): add multilingual termbase schema"`

---

### Task 3: Construire un noyau terminologique petit et sans dépendance UI

**Files:**

- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Create: `src-tauri/src/core/terminology/mod.rs`
- Create: `src-tauri/src/core/terminology/types.rs`
- Create: `src-tauri/src/core/terminology/normalize.rs`
- Create: `src-tauri/src/core/terminology/repository.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Test: modules ci-dessus

- [x] **Step 1: Écrire les types et tests de sérialisation**

Définir des enums Rust sérialisés en `snake_case` pour POS, type sémantique, statut, origine, review et enforcement. Définir `TerminologyEntryView`, `TerminologyQuery`, `PaginatedTerminology`, `CreateTermInput`, `UpdateTermInput` et `TermStats`. Vérifier les noms camelCase des champs IPC.

- [x] **Step 2: Centraliser la normalisation**

Ajouter `unicode-normalization = "0.1"`. `normalize.rs` doit faire NFKC, trim, espaces internes stables et casse pour les langues qui en ont une. La normalisation japonaise ne doit pas convertir kanji en lecture. Toutes les contraintes d'unicité, recherches et variantes appellent cette seule fonction.

- [x] **Step 3: Écrire les tests du repository avant les requêtes**

Couvrir pagination, recherche, filtres POS/type/statut/présence dans projet, vue globale, override projet, traduction manquante, création, édition, archivage, verrouillage et variantes acceptées. Vérifier qu'une page de 100 entrées n'effectue pas de requête N+1.

Run: `cargo test --manifest-path src-tauri/Cargo.toml core::terminology`

Expected: FAIL avant implémentation.

- [x] **Step 4: Implémenter les requêtes SQL bornées**

Une seule requête paginée retourne l'entrée, la traduction effective (override projet avant global), le nombre d'occurrences du projet et jusqu'à trois contextes représentatifs via une seconde requête groupée par les IDs de la page. `page_size` est borné à `1..=200`. Les modifications utilisent des transactions courtes et `archived` plutôt qu'un hard delete pour une entrée déjà référencée.

- [x] **Step 5: Ajouter une mesure de requêtes et un test 10k**

Dans une DB de test de 10 000 entrées, recherche/tri/page doivent rester sous 100 ms p95 sur la machine de référence et ne retourner que la page demandée.

- [x] **Step 6: Commit**

Run: `git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/core/mod.rs src-tauri/src/core/terminology`

Run: `git commit -m "feat(terminology): add paginated termbase repository"`

---

### Task 4: Ajouter l'interface d'analyse et l'adaptateur japonais Lindera

**Files:**

- Create: `src-tauri/src/core/terminology/analyzer/mod.rs`
- Create: `src-tauri/src/core/terminology/analyzer/japanese.rs`
- Create: `src-tauri/src/core/terminology/analyzer/filters.rs`
- Modify: `src-tauri/src/core/terminology/mod.rs`
- Test: modules ci-dessus

- [x] **Step 1: Écrire le contrat indépendant de Lindera**

```rust
pub struct LinguisticToken {
    pub surface: String,
    pub lemma: String,
    pub reading: Option<String>,
    pub part_of_speech: PartOfSpeech,
    pub pos_detail: Option<String>,
    pub conjugation: Option<String>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub is_unknown: bool,
}

pub trait MorphologicalAnalyzer: Send + Sync {
    fn version(&self) -> &str;
    fn analyze(&self, text: &str) -> Result<Vec<LinguisticToken>, TerminologyError>;
}
```

Les tests du scanner utiliseront un faux analyseur; aucun test métier ne doit dépendre directement de Lindera.

- [x] **Step 2: Protéger les codes moteur avant tokenisation**

Réutiliser la logique MV/MZ existante de protection des codes via une entrée fournie par l'adaptateur moteur. Ne pas recopier les regex de placeholders dans `japanese.rs`. Vérifier que `\\V[1]`, `\\N[2]`, balises de couleur et marqueurs de nom n'engendrent aucun terme.

- [x] **Step 3: Mapper IPADIC vers les catégories stables Hoshi2Star**

Mapper noms généraux/propres, verbes, adjectifs, adverbes et expressions. Exclure particules, auxiliaires, symboles, espaces, ponctuation, nombres seuls et tokens vides. Conserver la forme de base comme `canonical_text`, la surface comme variante et la lecture si disponible.

- [x] **Step 4: Tester la fixture linguistique**

Run: `cargo test --manifest-path src-tauri/Cargo.toml core::terminology::analyzer`

Expected: PASS pour toutes les formes grammaticales et tous les placeholders de la fixture.

- [x] **Step 5: Commit**

Run: `git add src-tauri/src/core/terminology/analyzer src-tauri/src/core/terminology/mod.rs`

Run: `git commit -m "feat(terminology): add bounded Japanese morphology analyzer"`

---

### Task 5: Faire appartenir la sémantique MV/MZ à l'adaptateur moteur

**Files:**

- Create: `src-tauri/src/engines/terminology.rs`
- Create: `src-tauri/src/engines/mv_mz/terminology.rs`
- Modify: `src-tauri/src/engines/mod.rs`
- Modify: `src-tauri/src/engines/mv_mz/mod.rs`
- Test: `src-tauri/src/engines/mv_mz/terminology.rs`

- [x] **Step 1: Définir le contrat moteur**

Le contrat retourne `EngineTermSeed { source_text, semantic_type, part_of_speech, confidence }` et une version de texte sûre pour l'analyse morphologique. Il reçoit `Segment` avec le `file_type` de son `SourceFile`, pas seulement une chaîne, afin d'utiliser `segment_kind`, fichier, clé JSON, intervenant et contexte.

- [x] **Step 2: Écrire la table de vérité MV/MZ**

Tester au minimum les mappings: `actor_name/actor_nickname→character`, `speaker→speaker`, `class_name→class`, `item_name + Items.json→item`, `item_name + Weapons.json→weapon`, `item_name + Armors.json→armor`, `skill_name→skill`, `enemy_name→enemy`, `state_name→state`, `map_name→place`, `common_event_name→event`, `system_term→system`, `game_title→title`. Les profils, descriptions, messages, dialogues et choix ne créent pas d'entité structurée mais restent analysables par Lindera.

- [x] **Step 3: Implémenter sans logique générique RPG Maker dans le core**

Les textes structurés sont insérés même s'ils n'apparaissent qu'une fois. Une entrée structurelle l'emporte sur une entrée Lindera identique grâce à une confiance supérieure et à son `semantic_type` spécifique; aucun écrasement destructif d'un sens différent.

- [x] **Step 4: Vérifier les fixtures MV et MZ**

Run: `cargo test --manifest-path src-tauri/Cargo.toml engines::mv_mz::terminology`

Expected: mêmes résultats pour les variantes MV et MZ, et aucun classement universel appliqué aux autres moteurs.

- [x] **Step 5: Commit**

Run: `git add src-tauri/src/engines/terminology.rs src-tauri/src/engines/mod.rs src-tauri/src/engines/mv_mz/mod.rs src-tauri/src/engines/mv_mz/terminology.rs`

Run: `git commit -m "feat(mv-mz): classify engine-owned terminology"`

---

### Task 6: Scanner les projets de façon incrémentale, annulable et peu coûteuse

**Files:**

- Create: `src-tauri/src/core/terminology/scanner.rs`
- Create: `src-tauri/src/core/terminology/service.rs`
- Modify: `src-tauri/src/core/terminology/mod.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/core/terminology/scanner.rs`
- Create: `src-tauri/tests/terminology_scan.rs`

- [x] **Step 1: Écrire les tests avec un faux analyseur compté**

Vérifier: premier scan, second scan sans appel analyseur, modification d'un seul segment, suppression d'un segment, fusion des occurrences, priorité aux seeds moteur, annulation après un chunk, reprise, erreur analyseur et deux demandes simultanées pour le même projet.

- [x] **Step 2: Implémenter le service long-vivant**

Ajouter `Arc<TerminologyService>` à `AppState`. Le service possède l'analyseur initialisé une fois, un sémaphore bornant les scans et une map `scan_id → AtomicBool` pour l'annulation. Le travail CPU s'exécute dans `spawn_blocking`; aucun appel Lindera ne bloque le runtime async Tauri.

- [x] **Step 3: Lire et écrire par chunks**

Lire les segments triés par ID par pages de 250. Calculer `SHA-256(segment_id + source_text + segment_kind + analyzer_version)`. Ignorer les hashes inchangés. Pour un segment modifié, supprimer ses anciennes occurrences, analyser, puis upsert entrées/variantes/occurrences et scan du segment dans une transaction. Commit après au plus 250 segments pour borner WAL/RAM et permettre l'annulation.

- [x] **Step 4: Publier des événements compacts**

Émettre `h2s://terminology/scan-progress` au plus dix fois par seconde avec `{scanId, projectId, processed, total, discovered}` et un unique `h2s://terminology/scan-done`. Ne jamais émettre les tokens ou la liste complète des termes dans les événements.

- [x] **Step 5: Ajuster SQLite seulement après mesure**

Mesurer WAL, `busy_timeout` et `synchronous=NORMAL` avec scan + édition simultanée. Les activer dans `db/pool.rs` seulement si le test démontre moins de contention sans perte aux tests de crash transactionnel. Garder `max_connections=5` tant qu'une mesure ne justifie pas plus.

- [x] **Step 6: Vérifier performance et inactivité**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test terminology_scan core::terminology::scanner`

Expected: PASS; mémoire bornée par chunk, un seul scan actif par projet et deuxième scan no-op.

- [x] **Step 7: Commit**

Run: `git add src-tauri/src/core/terminology src-tauri/src/state.rs src-tauri/src/lib.rs src-tauri/tests/terminology_scan.rs src-tauri/src/db/pool.rs`

Run: `git commit -m "feat(terminology): scan projects incrementally"`

---

### Task 7: Exposer une API Tauri étroite et testable

**Files:**

- Create: `src-tauri/src/commands/terminology.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src-tauri/tests/terminology_commands.rs`

- [x] **Step 1: Définir les commandes**

Ajouter uniquement: `list_terminology`, `get_terminology_stats`, `create_terminology_entry`, `update_terminology_entry`, `archive_terminology_entry`, `upsert_terminology_translation`, `start_terminology_scan`, `cancel_terminology_scan`. Les inputs portent `sourceLanguage`, `targetLanguage` et `projectId` séparément; ne plus faire parser `lang_pair` par les nouvelles API.

- [x] **Step 2: Borner et valider chaque input**

Refuser page négative, page > 200, texte vide, langue vide, statut/enforcement inconnu et projet inexistant. Une commande ne contient pas de SQL métier: elle valide, appelle le service/repository et convertit l'erreur en message stable.

- [x] **Step 3: Tester avec l'app Tauri de test**

Vérifier les noms camelCase, la pagination, l'override projet, le démarrage asynchrone et l'annulation. Les tests ne chargent pas le vrai dictionnaire: injecter le faux analyseur dans `TerminologyService`.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test terminology_commands`

Expected: PASS.

- [x] **Step 4: Commit**

Run: `git add src-tauri/src/commands/terminology.rs src-tauri/src/commands/mod.rs src-tauri/src/lib.rs src-tauri/tests/terminology_commands.rs`

Run: `git commit -m "feat(tauri): expose terminology commands"`

---

### Task 8: Construire la page React Terminologie sans dupliquer l'état serveur

**Files:**

- Modify: `src/stores/ui.ts`
- Modify: `src/stores/ui.test.ts`
- Modify: `src/components/shell/ModeNavigation.tsx`
- Modify: `src/components/shell/ModeNavigation.test.tsx`
- Modify: `src/App.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/locales/en.json`
- Modify: `src/locales/fr.json`
- Create: `src/components/ui/dialog.tsx`
- Create: `src/components/ui/table.tsx`
- Create: `src/features/terminology/api.ts`
- Create: `src/features/terminology/queryKeys.ts`
- Create: `src/features/terminology/TerminologyWorkspace.tsx`
- Create: `src/features/terminology/TerminologyToolbar.tsx`
- Create: `src/features/terminology/TerminologyTable.tsx`
- Create: `src/features/terminology/TermEditorDialog.tsx`
- Create: `src/features/terminology/ScanProgress.tsx`
- Create: `src/features/terminology/terminologyUiStore.ts`
- Test: fichiers `*.test.tsx` adjacents

- [x] **Step 1: Ajouter le mode `terminology`**

Le libellé français est `Terminologie`, pas `Bibliothèque`, car `library` désigne déjà les projets. La page fonctionne en vue globale sans projet et se filtre automatiquement sur le projet actif s'il existe. La source/cible par défaut vient du projet actif.

- [x] **Step 2: Séparer état serveur et état d'interface**

TanStack Query conserve pages, stats et mutations/invalidation. Zustand conserve uniquement recherche saisie, filtres, lignes sélectionnées et portée global/projet. Ne jamais recopier les entrées reçues dans Zustand.

- [x] **Step 3: Écrire les tests d'interface avant les composants**

Couvrir: page vide, chargement, erreur IPC, pagination, filtre POS/type/statut, changement de langue cible, sélection multiple, scan indisponible si source non japonaise, progression/annulation, création manuelle, édition/approbation/verrouillage et archivage confirmé.

Run: `pnpm test src/features/terminology src/components/shell/ModeNavigation.test.tsx src/stores/ui.test.ts`

Expected: FAIL avant implémentation.

- [x] **Step 4: Implémenter la table compacte**

Colonnes: terme source, lecture, POS, type moteur, occurrence projet, cible effective, portée, statut de révision, enforcement, actions. Utiliser TanStack Table avec pagination serveur de 100 lignes et rendu virtualisé uniquement si la mesure DOM le justifie. Les mutations optimistes sont interdites pour les changements de verrouillage; attendre la confirmation SQLite.

- [x] **Step 5: Implémenter le scan explicite**

Le bouton `Analyser le vocabulaire` affiche le périmètre et rappelle que l'opération est locale. Pendant le scan, montrer compteurs et annulation sans bloquer navigation/édition. À `scan-done`, invalider stats et page courante, pas toutes les queries de l'application.

- [x] **Step 6: Vérifier accessibilité et charge UI**

Navigation clavier, focus du dialog, labels de boutons, état `aria-busy`, annonces de progression modérées et aucun rendu de 10 000 lignes. Une page/filtre doit rester interactive sous 100 ms hors IPC.

- [x] **Step 7: Commit**

Run: `git add src/stores/ui.ts src/stores/ui.test.ts src/components/shell/ModeNavigation.tsx src/components/shell/ModeNavigation.test.tsx src/App.tsx src/lib/types.ts src/locales/en.json src/locales/fr.json src/components/ui/dialog.tsx src/components/ui/table.tsx src/features/terminology`

Run: `git commit -m "feat(ui): add project-aware terminology workspace"`

---

### Task 9: Traduire les termes avec le modèle sans les approuver automatiquement

**Files:**

- Create: `src-tauri/prompts/terminology/default.toml`
- Modify: `src-tauri/src/llm/prompts.rs`
- Create: `src-tauri/src/core/terminology/translator.rs`
- Modify: `src-tauri/src/commands/terminology.rs`
- Modify: `src-tauri/src/lib.rs`
- Create: `src/features/terminology/TermTranslateDialog.tsx`
- Modify: `src/features/terminology/api.ts`
- Modify: `src/features/terminology/TerminologyWorkspace.tsx`
- Test: modules et composants adjacents

- [ ] **Step 1: Écrire le protocole strict par ID**

Entrée provider par terme: ID, source, POS, type sémantique, lecture et 1 à 3 contextes représentatifs bornés. Sortie JSON stricte: `{id, target, confidence}`. Refuser ID manquant/inconnu/dupliqué, cible vide, texte source copié pour ja→en/fr et réponse libre. Ne jamais persister une réponse partielle silencieusement.

- [ ] **Step 2: Adapter le prompt aux capacités sans créer un prompt par fournisseur**

Réutiliser les politiques de provider/modèle existantes: petit modèle reçoit moins de termes/contextes; modèle robuste peut traiter un lot plus grand. La destination (`en` ou `fr`) et la catégorie sont explicites. Le prompt demande un lemme ou nom canonique, pas une phrase traduite.

- [ ] **Step 3: Respecter les profils de ressources**

Eco: 8 termes/1 contexte; Balanced: 20 termes/2 contextes; Fast: maximum 50/3 contextes, toujours une requête active à la fois pour les providers locaux. Émettre les métriques existantes `h2s://llm/metrics` et une progression terminologique séparée.

- [ ] **Step 4: Persister en `proposed`**

Chaque résultat valide crée/remplace la traduction pour langue+portée avec `review_status=proposed`, `enforcement=contextual` pour verbes/adjectifs/adverbes et `preferred` pour noms/entités. Seule une action utilisateur passe à `approved` ou `locked/required`.

- [ ] **Step 5: Ajouter le dialogue de sélection**

Permettre: sélection visible, sélection manuelle, uniquement non traduits, cible en/fr, portée globale/projet et provider/modèle courant. Afficher avant envoi le nombre de termes et une estimation de tokens.

- [ ] **Step 6: Tester ja→en et ja→fr avec provider mock**

Run: `cargo test --manifest-path src-tauri/Cargo.toml core::terminology::translator`

Run: `pnpm test src/features/terminology/TermTranslateDialog.test.tsx`

Expected: résultats proposés corrects, aucune auto-approbation, erreurs protocolaires visibles et métriques émises.

- [ ] **Step 7: Commit**

Run: `git add src-tauri/prompts/terminology/default.toml src-tauri/src/llm/prompts.rs src-tauri/src/core/terminology/translator.rs src-tauri/src/commands/terminology.rs src-tauri/src/lib.rs src/features/terminology`

Run: `git commit -m "feat(terminology): translate term candidates with review states"`

---

### Task 10: Résoudre les termes pour chaque vraie requête LLM

**Files:**

- Create: `src-tauri/src/core/terminology/resolver.rs`
- Modify: `src-tauri/src/core/terminology/mod.rs`
- Modify: `src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/llm/split.rs`
- Modify: `src-tauri/src/commands/translate.rs`
- Modify: `src-tauri/prompts/translate/default.toml`
- Test: modules ci-dessus

- [ ] **Step 1: Écrire les tests qui reproduisent le défaut actuel**

Vérifier qu'un terme sans occurrence dans le lot n'est jamais envoyé, que zéro correspondance produit une liste vide, qu'un override projet masque le global, qu'une traduction `proposed` n'est pas `required`, et qu'un lot de dialogue séparé d'un lot canonique reçoit des hints différents.

- [ ] **Step 2: Remplacer le tuple de glossaire par un type riche**

```rust
pub struct TerminologyHint {
    pub source: String,
    pub target: String,
    pub semantic_type: String,
    pub part_of_speech: PartOfSpeech,
    pub enforcement: Enforcement,
    pub accepted_targets: Vec<String>,
}
```

`TranslationContext` porte `terminology_hints`. Supprimer `glossary_terms` après adaptation de tous les tests.

- [ ] **Step 3: Résoudre après le regroupement interne du pipeline**

Pour chaque appel réel à `provider.translate`, passer les IDs de segments du sous-lot à `resolver`. Une requête SQL récupère uniquement les entrées actives ayant une occurrence sur ces segments et une traduction effective non vide dans la cible. Ordonner `required`, `preferred`, `contextual`, puis fréquence; dédupliquer et borner à 20 termes et 10 % du budget estimé de prompt.

- [ ] **Step 4: Supprimer le fallback arbitraire**

Retirer `core::glossary::relevant_terms`, notamment les dix termes les plus courts quand rien ne correspond. Retirer le chargement par fichier dans `commands/translate.rs`. Aucun fallback global n'est autorisé.

- [ ] **Step 5: Rendre le prompt explicite**

Présenter `required` comme obligatoire si le sens correspond, `preferred` comme traduction cohérente recommandée et `contextual` comme aide non littérale. Préciser qu'un terme ne doit jamais être inséré si le texte source du segment ne l'emploie pas.

- [ ] **Step 6: Mesurer les tokens**

Sur la fixture 1 191 segments, comparer tokens de prompts avec/sans terminologie. Le surcoût médian doit rester ≤ 10 %, aucun appel ne doit dépasser la fenêtre configurée et le nombre de hints doit apparaître dans les métriques.

- [ ] **Step 7: Vérifier le pipeline complet**

Run: `cargo test --manifest-path src-tauri/Cargo.toml llm::pipeline llm::provider core::terminology::resolver commands::translate`

Expected: PASS; zéro terme sans occurrence et zéro fallback arbitraire.

- [ ] **Step 8: Commit**

Run: `git add src-tauri/src/core/terminology src-tauri/src/llm/provider.rs src-tauri/src/llm/pipeline.rs src-tauri/src/llm/split.rs src-tauri/src/commands/translate.rs src-tauri/prompts/translate/default.toml`

Run: `git commit -m "feat(localization): resolve terminology per provider request"`

---

### Task 11: Unifier la QA terminologique et remplacer l'interface Glossaire

**Files:**

- Modify: `src-tauri/src/core/qa.rs`
- Modify: `src-tauri/src/commands/qa.rs`
- Modify: `src-tauri/src/core/report.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/components/editor/QAPanel.tsx`
- Modify: `src/components/editor/QAPanel.test.tsx`
- Create: `src/components/editor/TerminologyInspector.tsx`
- Modify: `src/components/shell/InspectorRail.tsx`
- Delete: `src/components/editor/GlossaryPanel.tsx`
- Modify: `src/hooks/useAppHandlers.ts`
- Modify: `src/stores/editor.ts`
- Modify: `src/stores/project.ts`
- Modify: `src/App.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/fr.json`

- [ ] **Step 1: Faire du rapport QA un évaluateur pur**

Activer et satisfaire le contrat `qa_report_preview_is_read_only_and_export_runs_one_explicit_audit`. La prévisualisation ne modifie ni statut ni score; l'audit explicite avant export peut persister les résultats dans une transaction contrôlée.

- [ ] **Step 2: Écrire les règles terminologiques**

- `locked + required` sur entité stable: erreur critique si aucune cible/variante acceptée n'apparaît;
- `approved + preferred`: warning de cohérence;
- `proposed` ou `contextual`: information de révision, jamais blocage exact;
- verbes/adjectifs: jamais correspondance exacte critique sans variante cible explicitement ajoutée;
- terme absent de la source/occurrence: aucune règle.

Renommer l'erreur API en `TerminologyMismatch` et accepter temporairement `GlossaryMismatch` en désérialisation des anciens rapports si nécessaire.

- [ ] **Step 3: Réutiliser le même resolver**

QA reçoit les IDs de segments et demande les hints au resolver; elle ne recharge pas toute la base. Une incohérence répétée est groupée par `entry_id` dans le rapport pour permettre d'ouvrir la page Terminologie filtrée sur le terme.

- [ ] **Step 4: Remplacer l'inspecteur sans créer un deuxième CRUD**

Le nouvel inspecteur montre seulement les termes correspondant au segment actif et ouvre la page complète pour l'édition. Supprimer l'ancien panneau React et ses handlers/stores. Garder provisoirement la façade Rust `glossary` en lecture uniquement parce que les packs v1 l'utilisent encore; sa suppression appartient à Task 12. Vérifier:

Run: `rg -n "Glossary|glossary|get_glossary|glossary_terms" src src-tauri/src src-tauri/prompts`

Expected: aucune UI/store active de glossaire; seulement la façade backend historique, les packs, la migration et le prompt historique en attente de Task 12.

- [ ] **Step 5: Vérifier QA et frontend**

Run: `cargo test --manifest-path src-tauri/Cargo.toml core::qa commands::qa core::report`

Run: `pnpm test src/components/editor/QAPanel.test.tsx src/components/editor/TerminologyInspector.test.tsx`

Expected: QA pure en preview, audit explicite, règles d'enforcement correctes.

- [ ] **Step 6: Commit**

Run: `git add -A src-tauri/src/core/qa.rs src-tauri/src/commands/qa.rs src-tauri/src/core/report.rs src/components/editor src/components/shell/InspectorRail.tsx src/hooks/useAppHandlers.ts src/stores/editor.ts src/stores/project.ts src/App.tsx src/lib/types.ts src/locales/en.json src/locales/fr.json`

Run: `git commit -m "refactor(qa): enforce reviewed terminology consistently"`

---

### Task 12: Préserver les packs `.h2s` v1 et la restauration

**Files:**

- Modify: `src-tauri/src/core/h2s_pack.rs`
- Modify: `src-tauri/src/commands/pack.rs`
- Modify: `src-tauri/src/commands/terminology.rs`
- Modify: `src-tauri/src/llm/prompts.rs`
- Modify: `src-tauri/src/core/mod.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Delete: `src-tauri/src/core/glossary.rs`
- Delete: `src-tauri/src/commands/glossary.rs`
- Delete: `src-tauri/prompts/glossary/default.toml`
- Modify: `src/components/PackExportDialog.tsx`
- Modify: `src/components/PackImportWizard.tsx`
- Test: modules et composants adjacents

- [ ] **Step 1: Écrire les tests de compatibilité avant modification**

Tester: export d'un pack v1 depuis la nouvelle base, import d'un ancien `glossary.json`, conflit global/projet, ja-en et ja-fr, terme non traduit, terme proposé, terme verrouillé et suppression du projet après import.

- [ ] **Step 2: Garder le format v1**

À l'export, mapper uniquement les traductions `approved` ou `locked` visibles pour le projet vers le champ historique `glossary` (`source`, `target`, `domain`). Ne pas exporter les candidats non traduits/proposés. À l'import, créer entrée+traduction `approved/preferred`, origine `import`, portée projet.

- [ ] **Step 3: Afficher les conséquences dans l'assistant**

Le preview indique créations, mises à jour et conflits terminologiques. Aucun pack importé ne doit écraser silencieusement une traduction `locked`.

- [ ] **Step 4: Retirer la façade historique après bascule des packs**

Avant suppression, déplacer `extract_wolf_speakers` vers `commands/terminology.rs` comme wrapper de compatibilité qui utilise `extract_wolf_speaker_names` mais écrit les entrées `speaker` et leurs occurrences dans la nouvelle base; conserver son nom IPC pour ne pas casser le parcours Wolf existant. Supprimer ensuite les autres commandes, le core et le prompt glossary, puis retirer leurs exports et leur enregistrement Tauri. Ne pas supprimer la table SQLite `glossary_terms` en 0008: elle reste une donnée de rollback historique, mais aucune requête applicative ne la lit ou l'écrit.

Run: `rg -n "Glossary|glossary|get_glossary|glossary_terms" src src-tauri/src src-tauri/prompts`

Expected: uniquement le champ sérialisé `glossary` du format pack v1, les tests de compatibilité et les mentions explicites de migration; aucun CRUD, store, commande ou prompt actif.

- [ ] **Step 5: Vérifier les archives réelles**

Run: `cargo test --manifest-path src-tauri/Cargo.toml commands::pack core::h2s_pack`

Run: `pnpm test src/components/PackImportWizard.test.tsx`

Expected: anciens packs lisibles, nouveaux packs v1 lisibles par la version précédente pour la partie glossaire.

- [ ] **Step 6: Commit**

Run: `git add -A src-tauri/src/core src-tauri/src/commands src-tauri/src/lib.rs src-tauri/src/llm/prompts.rs src-tauri/prompts src/components/PackExportDialog.tsx src/components/PackImportWizard.tsx`

Run: `git commit -m "feat(pack): map terminology to h2s v1 glossary"`

---

### Task 13: Durcir, documenter et valider dans la vraie application Tauri

**Files:**

- Create: `docs/terminology.md`
- Create: `docs/architecture/terminology.md`
- Modify: `docs/architecture.md`
- Create: `src-tauri/tests/terminology_e2e.rs`
- Modify only for discovered defects: files des tâches précédentes

- [ ] **Step 1: Ajouter l'E2E automatisé synthétique**

Flux: ouvrir fixture MV/MZ ja-en, extraire, scanner, vérifier catégories, traduire deux termes via mock, approuver/verrouiller, traduire segments, contrôler prompt/QA, exporter ZIP, supprimer projet, vérifier que l'entrée globale survit et que les occurrences locales disparaissent. Répéter le chemin cible ja-fr.

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test terminology_e2e`

Expected: PASS et ZIP JSON valide.

- [ ] **Step 2: Exécuter tous les contrôles statiques**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `pnpm lint && pnpm typecheck && pnpm test && pnpm build`

Expected: aucune erreur; tous les tests terminologiques et QA actifs.

- [ ] **Step 3: Tester le parcours visible avec MCP Tauri**

Lancer le build debug avec le bridge déjà configuré, puis utiliser MCP Tauri pour: ouvrir la fixture, aller dans Terminologie, scanner, filtrer noms/verbes/adjectifs, éditer une cible, lancer traduction de termes mock/local, verrouiller, revenir au Patch, traduire et afficher QA. Capturer console, erreurs IPC, screenshots et événements. Aucune action ne doit exiger DevTools manuel.

- [ ] **Step 4: Rejouer le pilote StandGirl sur une copie temporaire**

Baseline acquise: 1 191 textes traduits, 0 erreur critique finale, 205 warnings de largeur, ZIP valide. Mesurer maintenant: temps scan froid/chaud, termes par catégorie, taille DB, RAM/CPU, tokens de traduction, hints par appel, incohérences terminologiques, erreurs critiques, warnings et ZIP. L'original du jeu reste inchangé et hash-identique.

- [ ] **Step 5: Appliquer les gates finaux**

Acceptation:

- scan froid 1 191 segments ≤ 5 s et re-scan no-op ≤ 1 s;
- aucun provider contacté pendant le scan;
- mémoire supplémentaire dans le budget retenu en Task 1 et CPU idle à 0 %;
- DB de 10k termes paginée sans chargement complet, requêtes p95 ≤ 100 ms;
- au plus 20 hints et ≤ 10 % de tokens de prompt terminologique par appel;
- mêmes IDs de segment/placeholder avant et après;
- ja→en et ja→fr utilisables;
- 0 erreur critique à l'export pilote;
- ZIP valide et projet original intact;
- suppression d'un jeu enlève ses données projet mais pas la base globale;
- aucune clé API, texte ou terme envoyé hors du provider explicitement sélectionné;
- licence Lindera/IPADIC incluse au packaging.

- [ ] **Step 6: Documenter l'usage et les limites**

Expliquer scan local, catégories, portées, review/enforcement, projets supprimés, langues supportées, sauvegarde DB, import/export v1, limites de la morphologie japonaise et fait que l'utilisateur reste responsable de la validation sémantique.

- [ ] **Step 7: Commit**

Run: `git add docs/terminology.md docs/architecture/terminology.md docs/architecture.md src-tauri/tests/terminology_e2e.rs`

Run: `git commit -m "docs(terminology): document and verify the end-to-end workflow"`

---

## Hors périmètre de cette livraison

- Analyseurs morphologiques chinois, coréen ou langues européennes. Le registre d'analyseurs les permettra, mais seul `ja` est activé.
- Adaptateurs terminologiques VX Ace, Wolf, Unity, Bakin et autres. Chacun devra implémenter le contrat moteur après stabilisation MV/MZ.
- OCR, traduction d'images, lecteur intégré et traitement de ressources censurées. Ils ne doivent pas retarder le pipeline MV/MZ.
- Synchronisation cloud publique de la base terminologique. La base reste locale et personnelle.
- Auto-verrouillage par confiance du modèle. Toute contrainte `required` demeure une décision utilisateur.
- Suppression physique automatique des termes devenus orphelins. Une commande d'entretien explicite pourra être conçue après observation de plusieurs projets réels.

## Ordre de démarrage recommandé

Commencer par Task 0, puis Task 1. La première ligne de code fonctionnelle de la bibliothèque ne doit pas être l'UI ni la migration: c'est le corpus japonais et le benchmark Lindera. Ce gate évite de figer une architecture qui augmenterait excessivement RAM, taille du paquet ou temps de démarrage. Une fois T1 validé, T2 et T3 constituent le premier vrai incrément livrable: base migrée, repository paginé et aucun changement visible pour l'utilisateur tant que les invariants de données ne sont pas sûrs.
