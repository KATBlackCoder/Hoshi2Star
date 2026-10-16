# RPG Maker MV/MZ — Textes optionnels (plugins, listes System, note-tags)

> Branche `feat/mv-mz-plugin-text` (2026-10-14). Jeu pilote : 魔術師「恵」と禁断の魔術 (MZ).
> Code : `src-tauri/src/engines/mv_mz/{coverage/, profiles.rs, keke.rs, plugins_js.rs,
> json_span.rs, originals.rs, integrity.rs, locale.rs}`, commandes `commands/coverage.rs`
> et `commands/export.rs`, UI `TextCoverageDialog`, `ExportLocaleOption`, `ExportIntegrityDialog`.

## Pourquoi

L'extraction de base ne lit que `data/*.json` et laisse donc du japonais visible en jeu :

| Source | Visible dans | Risque si traduit naïvement |
|---|---|---|
| Paramètres de `js/plugins.js` | Menus, Options, bestiaire, popups | Le même plugin mélange libellés, énumérations comparées littéralement, clés de note-tag, noms de touches |
| Listes System (`skillTypes`, `equipTypes`, `elements`, `weaponTypes`, `armorTypes`) | Menus compétences, équipement… | Des plugins/scripts cherchent ces noms (ex. Keke `スキル/魔術`) |
| Arguments de commandes plugin MZ (357) | Log de combat, popups | Certains arguments sont des identifiants |
| Libellés de note-tags Keke `<コマンド:…>` | Commandes de combat | Grammaire stricte (espaces supprimés, séparateurs `,`) |
| `displayName` des maps | Popup de nom de map | Dessiné avec `drawText` : pas de codes d'échappement |

Principe : **rien n'est proposé sans preuve que c'est de l'affichage**, chaque option affiche ses
conséquences avant activation, et l'export **refuse** toute modification non planifiée.

## Modèle de catégories

Un segment optionnel existe **si et seulement si** sa catégorie est activée
(`project_text_categories`, migration `0010`). Désactiver une catégorie « parke » ses traductions
(`parked_segments`), restaurées si la source n'a pas changé. `segments.text_category` vaut `NULL`
pour l'extraction de base.

| État | Signification | Activation |
|---|---|---|
| **Verified** | Profil vérifié + aucune référence par nom ailleurs | Opt-in, pré-cochée dans le dialogue |
| **Linked** | Référencé par nom, mais Hoshi2Star sait réécrire *toutes* les références | Opt-in, confirmation des conséquences (`accept_consequences`) |
| **Blocked** | Référence non couverte, ou champ identifiant | Jamais ; raison + emplacements affichés |
| **Not covered** | Plugin sans profil vérifié | Rapport seulement (indices statiques, jamais extrait) |

Le scan tourne toujours sur les **originaux vierges** et `set_text_categories` re-scanne côté Rust
(l'UI n'est jamais crue sur parole).

Rien n'est activé sans l'utilisateur. Après une extraction neuve, le scan (le snapshot vient d'être
pris) propose seulement une notification « Examiner » s'il existe une catégorie activable. Un projet
restauré n'est **pas** scanné à l'ouverture, car le scan pourrait devoir snapshotter tout le jeu.
`get_text_coverage_status` (sans scan) indique si une décision existe déjà ; sinon la même
notification est proposée.

### Sites de référence scannés

Frontières de mots Unicode, sur : notes (texte lisible des tags seulement, positions Keke
masquées), scripts (355/655, 111 type 12, 122 op 4, move routes code 45, formules de dégâts),
commentaires 356, arguments 357, paramètres et littéraux JS des plugins **activés** (hors source du
plugin propriétaire). Exclus car éditeur seulement : 108/408 (commentaires), 657 (résumé 357),
champs `memo` des profils.

Les références à des textes de base (ex. nom d'acteur comparé dans un script) donnent des
avertissements non bloquants dans le dialogue de couverture.

## Profils de plugins

TOML embarqués (`src-tauri/profiles/mv_mz/*.toml`, `deny_unknown_fields`), épinglés par
`verified_sha256` (SHA-256 du source, CRLF → LF). Le format accepte aussi `verified_versions`
(`@version` du source). Les profils livrés n'utilisent que le hash, car une copie modifiée garde
le même en-tête (vérifié par `embedded_profiles_are_valid`). Source différente, plugin désactivé
ou source illisible ⇒ rapport seulement. Les noms de plugins de `plugins.js` (dossier du jeu, non
fiable) contenant `..`, `\`, `:` ou un segment vide sont ignorés avant toute lecture de
`js/plugins/`.

| Règle | Rôle |
|---|---|
| `[[display]]` | Chemin de paramètre affiché (`[]` = tous les éléments d'une liste). Flags : `message`, `template` (`%1`… à garder), `escape_codes` (rendu `drawTextEx`) |
| `[[fill_in]]` | Paramètre **vide** dont le défaut est codé en dur (choisi par `key_field`) : écrire un libellé remplace ce défaut (ex. NUUN `DateSelect` → `倒した数`) |
| `[[blocked]]` | Jamais traduisible, avec raison (énumérations, clés de note-tag, touches) |
| `[[memo]]` | Champ éditeur jamais lu : ni proposé, ni site de référence |
| `[[command_arg]]` | Argument 357 affiché (ex. `OUTPUT_LOG.message` de BattleLogOutput) |
| `coerces_values` | PluginCommonBase convertit `true`/nombres/JSON : la cible doit rester une chaîne non convertible (`plain_text_value`) |
| `note_tag_families` | Grammaires de note-tags implémentées (`keke_command`) |

Ajouter un profil : lire le source du plugin **entier**, suivre chaque paramètre jusqu'à son usage
(`drawText`/`drawTextEx`/`addCommand` ⇒ display ; `===`, `meta[...]`, `Input.keyMapper` ⇒ blocked),
épingler la version, l'ajouter à `EMBEDDED` dans `profiles.rs`, puis lancer le pilote réel.

16 profils livrés : NUUN_EnemyBook 2.23.4, Keke_VariableActorCommand, BattleVoice, TinyGetInfoWnd,
ChangeEquipOnBattleMZ, SmoothBattleLog, SkillCost, BattleLogOutput, DamageSeEffective,
CharacterPictureManager, DynamicActorGraphic, TriggerOnEquipAndState, LL_StandingPictureBattle,
NRP_SeamlessEncounter, MessageWindowHidden, MPP_ChoiceEX.

## Keke : libellés de commandes et types de compétence liés

Grammaire (miroir de `makeCommandListFree`/`metaAll`) : `<コマンド|cmd|command:TYPE,ordre,libellé,icône,aide>`,
tout espace JS `\s` supprimé (U+3000 et U+00A0 compris), découpe sur `,`. `TYPE = [-]スキル/NOM` :
un NOM contenant un chiffre **n'importe où** est un id de compétence, sinon
`skillTypes.indexOf(NOM) || 1` — un nom inconnu fait disparaître la commande sans erreur.

- Libellés (`notetag.keke_command.labels`) : clé `notetag:v1:keke-cmd:<occ>:2:<pointer>`, contraintes
  `keke_tag_field` (critique : pas de `,` `>` `/` `:`) et `keke_no_spaces` (cosmétique : les espaces
  disparaissent en jeu).
- Types de compétence (`system.skill_types`, **Linked**) : l'export renomme le type dans System.json
  **et** dans chaque tag de toutes les notes (fichiers sans autre traduction exportés aussi).
  Contraintes : `keke_skill_type` (critique : un mot, sans chiffre ni séparateur) et
  `keke_skill_type_unique` (critique, au niveau du rapport : deux types ne peuvent pas porter le même nom).

## Originaux vierges et export idempotent

À l'extraction, chaque fichier modifiable est copié dans `<app-data>/originals/<project_id>/`
(+ `snapshot.json` : SHA-256, provenance). Un projet restauré est snapshotté à la demande (export,
dialogue de couverture). Si le dossier du jeu contient déjà un export, les originaux sont
**reconstruits** depuis la base (valeur ∈ {source, cible, `\FS[n]`+cible} ⇒ source ; sinon
fichier marqué *drifted*). Tous les exports MV/MZ et `debug_inject_file` lisent le snapshot :
exporter deux fois donne deux archives identiques, même après dézippage par-dessus `data/`.

Le `project_id` vient de `.hoshi2star.json` (dossier du jeu, non fiable) et devient un nom de
dossier. `snapshot_root` refuse donc tout ce qui n'est pas un UUID, et `open_project` ignore un id
de manifeste malformé.

**Mise à jour du jeu.** Chaque entrée garde aussi `knownDisk`, les SHA-256 des versions du dossier
du jeu connues pour lui correspondre : le disque à la capture et chaque fichier exporté (zip ou
`debug_inject_file`), au plus 16. Export, preflight et dialogue de couverture appellent
`ensure_fresh_mv_mz_snapshot`. Un fichier du jeu dont le hash est inconnu est reconstruit comme à
la première capture, puis comparé au snapshot (JSON comparé après parsing ; un `locale` égal à la
cible d'export est remis à sa valeur d'origine). S'il est identique, c'était un export plus ancien
et son hash est retenu. Sinon le jeu a changé : le snapshot prend la nouvelle version. Un export ne
ramène donc jamais une ancienne version du jeu.

`plugins.js` est édité par chirurgie d'intervalles (JSON dans des chaînes compris) : seuls les
octets des valeurs traduites changent.

## Gate d'intégrité (avant toute écriture)

Pour chaque fichier produit : il se parse ; seules les valeurs planifiées changent ; chaque valeur
planifiée vaut la source avant édition (ou `""` pour un fill-in). Pour `plugins.js` : même liste de
plugins, mêmes noms/statuts, lignes non touchées identiques à l'octet. Keke : **simulation
différentielle** — chaque tag doit résoudre vers le même id de type/de compétence avant et après.
Un échec bloque l'export (« Export blocked: the integrity check found… ») et
`ExportIntegrityDialog` liste les problèmes ; aucun fichier n'est écrit.

## Option « langue du jeu » (`locale`)

Facultative, **décochée à chaque export**. `preflight_export` lit `locale` dans le System.json
vierge (absent sur MV < 1.6 ⇒ option non proposée), cherche dans le cœur (`js/rmmz_*.js`,
`js/rpg_*.js`) et les plugins activés chaque ligne qui lit le réglage (`isJapanese()`,
`isCJK()`… ; définitions du cœur et commentaires exclus) et en déduit les conséquences **qui changent
réellement** entre les deux locales : lettres d'ennemis pleine/demi-chasse, clavier de saisie de nom
(avec le nombre d'événements 303 du jeu), police (zh/ko), plugins concernés. `export_project(…,
setLocale)` planifie alors l'édition `/locale` comme n'importe quelle traduction (vérifiée par le gate),
même si System.json n'a rien d'autre à exporter.

## Tokenizer et QA

`TokEngine::for_segment` choisit la syntaxe par segment : `MzOnly` (protège `%1`) pour
`system_term` et les contextes `{"placeholderSyntax":"format"}` (templates de plugins). Utilisé par
le pipeline, la QA, le rapport et le dédoublonnage de batch (`DedupKey.placeholder_syntax`).
Un `%n` perdu dans une cible `system_term` ou un template donne `MissingFormatSpecifier`, un
avertissement (−15) qui ne bloque pas l'export : le jeu affiche le message sans la valeur. Les codes
d'échappement perdus restent `MissingPlaceholder` (critique).

Règles `ConstraintViolation` : `plain_text_value` (critique), `no_escape_codes` (cosmétique),
`keke_skill_type` / `keke_skill_type_unique` / `keke_tag_field` (critiques), `keke_no_spaces` (cosmétique).

## Tests

- `tests/text_coverage.rs` — fixture synthétique `tests/fixtures/mv_mz/mz_coverage/` (dont un faux
  Keke épinglé `h2s-test-1` et un extrait du cœur MZ) : classification, blocage, parking, export,
  réécritures liées, gate Keke, option locale.
- `tests/real_game_coverage.rs` — `#[ignore]`, jeu réel via `H2S_MZ_GAME_DIR` (copie `data/` + `js/`
  dans un dossier temporaire) : scan → tout activer → fausse traduction → deux exports identiques →
  option locale ne change que `locale`.

## Limites connues

- Le source JS des plugins n'est jamais modifié ; les résumés 657 restent en japonais (éditeur seulement).
- CharacterPictureManager : si une entrée `FileList` utilise `Speaker`, le plugin compare le nom du
  locuteur au nom de l'acteur — les deux traductions doivent rester identiques (non vérifié
  automatiquement ; sans objet dans le jeu pilote, `Speaker: false`).
