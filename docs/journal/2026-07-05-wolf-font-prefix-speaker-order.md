# Journal — 2026-07-05 — Fix ordre @N/taille de police (3e bug Wolf de la journée)

**Phase** : F5 (Wolf RPG) — bugfix, 3e correctif suite directe de v0.4.7/v0.4.8
**Statut** : ✅ Complété, vérifié end-to-end (export réel)

---

## Contexte

Après les fixes LZ4 (v0.4.7) et encodage (v0.4.8), l'utilisateur signale un
nouveau problème visuel pendant sa traduction : dans certains dialogues, le
nom du personnage (「地賀」dans une boîte jaune) et son portrait disparaissent
complètement à l'écran, remplacés par le texte littéral `@2` affiché en haut
de la boîte de dialogue. Captures avant/après fournies.

## Diagnostic

Requête SQLite directe sur le segment concerné
(`CommonEvents/∟オープニング/735/17`) : `target_text` en base =
`@2\n"Haa... I couldn't figure anything out today either..."` — **propre**,
marqueur `@2\n` en tête, aucune trace de corruption. La base de données n'a
jamais été touchée.

Dump binaire du `CommonEvent.dat` **réellement copié dans le jeu** (via un
test diagnostique temporaire décompressant + parsant l'event 735) :
`string_args = ["\\f[23]@2\n\"Haa...\""]` — le code de taille de police
`\f[23]` est collé **avant** le marqueur `@2\n`, alors que la BDD ne contient
pas ce code du tout.

Cause : `commands/export.rs::apply_font_prefix` fait toujours
`format!("{code}{text}")` — préfixe **inconditionnel** en position 0, sans
jamais regarder si `text` commence déjà par le marqueur de personnage Wolf
`@N\n` (voir la note tokenizer du 2026-07-04 : ce marqueur doit rester le
tout premier caractère pour que le moteur Wolf déclenche l'affichage
nom+portrait). En poussant `\f[23]` devant, le message ne commence plus par
`@` mais par `\`, et le moteur ne reconnaît plus le marqueur — il affiche
tout le texte tel quel, `@2` compris, sans nom ni portrait.

Ce bug est apparu lors d'un **export antérieur** (avant cette session) où
l'utilisateur avait appliqué une taille de police 23 ; le fichier corrompu a
ensuite été copié dans le dossier de jeu et y est resté.

## Correctif

- `RE_WOLF_SPEAKER_PREFIX` (`^@\d+\n`) + `split_wolf_speaker_prefix(text)` —
  sépare un éventuel marqueur `@N\n` en tête du reste du texte.
- `apply_font_prefix` : pour l'engine `"wolf"`, sépare le marqueur d'abord,
  applique/remplace le code de police sur le **reste**, puis recolle
  `marqueur + code + reste` — jamais `code + marqueur + reste`.
- `scan_font_status` (compteur « X ont déjà un \f[N] ») et `strip_font_prefix`
  (nettoyage) mis à jour de la même façon pour rester cohérents une fois les
  messages dans le bon ordre.
- 8 nouveaux tests : insertion après marqueur, absence de marqueur
  (inchangé), non-affecté pour mv_mz, remplacement/conservation avec
  marqueur, strip avec/sans marqueur, helper de split.

## Vérification

- Gate complet : clippy 0 ✅ · **402 tests Rust (+8)** ✅.
- **Live end-to-end** (`pnpm tauri:linux` + MCP Tauri, projet réel) : export
  avec réapplication de la taille de police 23 (mêmes conditions que
  l'export original fautif) → `hoshi2star.zip` généré sans erreur →
  décompression + re-parse du `CommonEvent.dat` frais : `cmd[17]` contient
  désormais `"@2\n\\f[23]\"Haa...\""` — marqueur en tête, code de police
  après. Ordre correct confirmé.

## Note

La base de données de traduction n'a jamais été corrompue par ce bug (le
préfixe est appliqué uniquement en mémoire au moment de l'export, jamais
persisté). Un simple nouvel export avec ce correctif suffit — aucune
restauration de fichier externe n'est nécessaire, contrairement au bug LZ4
de v0.4.7.

## Fichiers modifiés

- `src-tauri/src/commands/export.rs` — `RE_WOLF_SPEAKER_PREFIX`,
  `split_wolf_speaker_prefix`, `apply_font_prefix`, `scan_font_status`,
  `strip_font_prefix`, 8 tests (nouveau module `tests` dans ce fichier)

---
*Généré par Claude Code — Hoshi2Star*
