# RPG Maker MZ — Mise en page du texte (text fitting)

> Branche `feat/mv-mz-text-fitting` (2026-10-14). Jeu pilote : 魔術師「恵」と禁断の魔術 (MZ).
> Code : `src-tauri/src/engines/mv_mz/layout/` (`font`, `geometry`, `text`, `wrap`, `simulate`,
> `fit`, `verify`, `message`, `plan`), `core/text_fit.rs`, `commands/text_fit.rs`, export dans
> `commands/export.rs`. UI : `TextFitDialog`, onglet « Aperçu » (`TextFitPreviewPanel`),
> `ExportFitSummary`.

## Principe

À l'export uniquement, les dialogues (Afficher un message, codes 101/401 des cartes, événements
communs et combats) et les descriptions (objets, compétences, armes, armures, profils d'acteurs)
sont mis en page dans les **vraies** fenêtres du jeu, mesurés avec **la police du jeu**. Les
traductions en base restent ligne à ligne (TM, contrôle de cohérence et retraduction intacts).
Désactivé par défaut ; aucun changement de l'export tant que l'utilisateur ne l'active pas.

## Faits moteur (vérifiés dans `rmmz_windows.js` / `rmmz_scenes.js` du jeu)

| Fait | Valeur / règle |
|---|---|
| Largeur de boîte | `Graphics.boxWidth = uiAreaWidth − 8` (`Scene_Boot.adjustBoxSize`) → 808 |
| Fenêtre de message | intérieur 784 × 152 (`fittingHeight(4) + 8`, padding 12), 4 lignes |
| Début de ligne | x = 4, ou 144 + 20 = 164 avec un portrait (portrait = par message) |
| Largeur utile | 784 − x − 4 (marge visuelle) → 776 / 616 px |
| Fenêtre d'aide | texte à x = 8, largeur 768, 2 lignes (72 px), pas de pagination (3e ligne coupée) |
| Hauteur de ligne | `max(taille au début de ligne, \FS/\{/\} de la ligne) + (36 − mainFontSize)` |
| Changement de page | automatique si la ligne suivante dépasse 152 px ; U+000C force une page et avale le `\n` suivant |
| Remise à zéro | `newPage` remet police et couleur : la 1re ligne d'une page fait toujours ≥ 36 px |
| Dessin | caractère par caractère en jeu normal → largeur = Σ avances (la police du jeu suffit) |
| `\}` / `\{` | actifs seulement à ≥ 24 px / ≤ 96 px |

Conséquence : réduire la police donne surtout **plus de mots par ligne** ; une 5e ligne n'apparaît
qu'à ≤ 19 px. Un « \f » écrit avec un antislash serait lu comme un code inconnu (`\fThe` → `FTHE`) :
seul le caractère de contrôle U+000C est utilisé.

## Mesure

`layout::font` lit TTF/OTF/WOFF1 (tables WOFF décompressées avec `flate2`, `ttf-parser` pour cmap
+ hmtx) et somme les avances par caractère. Les tailles d'un en-tête WOFF viennent du dossier du
jeu : au-delà de 64 Mo la police est refusée, et une table n'est jamais décompressée au-delà de sa
taille déclarée. Caractère absent de la police (le navigateur prend une police de secours,
ex. Verdana) → estimation East Asian Width et signalement. Vérifié contre `measureText` de Chromium
sur 419 phrases réelles : 0,00 px d'écart pour tout glyphe présent ; seul `♪` (absent de M+ 1m) est
surestimé de 2,6 px.

## Algorithme (`layout::fit`)

1. Un message est la suite des 401 après un 101. Les lignes avec segment traduit sont
   **modifiables** ; les autres (`……`, codes seuls) restent **telles quelles** (jamais écrites).
2. S'il tient tel quel → inchangé (seule la taille de police des dialogues, si choisie, est ajoutée
   en début de page).
3. Sinon : **recomposition** (lignes jointes, coupures UAX #14 via `unicode-linebreak`, découpe
   équilibrée par programmation dynamique) ou **garder les retours** (seules les lignes trop
   larges sont coupées). Lignes vides et étiquette de nom (【…】) en tête restent à part. La
   découpe compare `(nombre de lignes, écart)` par préfixe : O(n·k) en temps et O(n) en mémoire
   pour n morceaux et au plus k morceaux par ligne. Un test la compare à l'ancien programme (toutes
   les tailles de ligne, O(n²) en mémoire).
4. Si trop haut : **nouvelle page** (pages qui commencent par une phrase, U+000C en fin de page,
   couleur/police réémises), **réduction** (plus grand `\FS[n]` ≥ minimum gardant le nombre de
   pages, réémis à chaque page) ou **laisser** (signalé).
5. Messages avec `\^` (fermeture auto) ou `\|` (attente) : jamais de nouvelle page (réduction ou
   signalement), une page ajoutant un appui de touche.
6. Chaque bloc de lignes modifiables est réécrit **uniquement dans ses propres commandes 401**
   (`\n` intégré) : aucune commande n'est ajoutée ni retirée, la structure de l'événement est
   intacte. Un bloc qui a besoin de moins de lignes que de commandes est réparti exactement sur ses
   commandes.

Descriptions : garder les retours → recomposer en ≤ 2 lignes → réduire → signaler. Avec
TinyGetInfoWndMZ (popup d'objet qui affiche la 1re ligne en texte brut), la 1re ligne d'un objet,
d'une arme ou d'une armure ne reçoit jamais de code (elle est remplie au maximum à la taille normale,
la suite réduite).

## Garde-fous

- `layout::verify` vérifie chaque valeur finale avant export : même texte visible (espaces mis à
  part) et mêmes codes **dans le même ordre**. Les seuls codes ajoutés acceptés sont `\FS[n]` et
  `\C[n]` en début de ligne, là où le fitting réémet la police et la couleur (début de page,
  2ᵉ ligne d'une description). Aucune ligne trop large, aucune page vide, seuls `\n` et U+000C
  comme caractères de contrôle. Un échec → texte exporté tel que traduit (jamais d'export bloqué).
- Un `\FS[n]` en début de ligne traduite est un code du traducteur, gardé et mis en page. Il n'est
  retiré que si l'export remplace les codes de police existants (taille choisie + « remplacer »),
  comme pour les lignes non mises en page (préfixe d'un ancien export).
- Le fitting ne peut pas être activé là où l'export ne peut pas mettre en page (pas MZ, pas de
  System.json) : `set_text_fit` refuse. Sinon la QA masquerait « ligne trop longue » pour un texte
  exporté tel quel.
- Le gate d'intégrité existant vérifie que seules les valeurs planifiées changent.
- Plugins : les redéfinitions de fonctions de mise en page par un plugin activé déclenchent un
  avertissement, sauf profil vérifié (`[layout]` : `neutral`, `inserted_codes`,
  `plain_description_first_line`, `notes`). Profils de mise en page : PluginCommonBase,
  TemplateEvent (`\S`, `\SS`, `\SV`), NUUN_Base, MessageSkip (icône dans le coin bas-droit),
  MPP_ChoiceEX, TinyGetInfoWndMZ.

## Réglages et choix par texte

`project_text_fit` (migration 0011) : dialogues / descriptions activés, retours (recomposer /
garder), dépassement (nouvelle page / réduire / laisser), taille minimale, taille des dialogues.
`text_fit_overrides` : choix par message ou description (pointeur du 101 ou du champ + empreinte du
texte original ; ignoré si le jeu a changé). La taille choisie dans la fenêtre de police à l'export
devient la taille des dialogues (une fois par page) ; les autres lignes de carte gardent le préfixe
par ligne. Un message que le fitting laisse tel que traduit (désactivé pour ce message, codes
`\PX`/`\PY`, échec de vérification…) s'exporte comme toute autre ligne, préfixe par ligne compris.
QA : `LineTooLong` est retiré des textes mis en page (repérés par type ou par structure).

## Vérification

- Tests unitaires de chaque module (polices synthétiques construites en code, pas de fichier binaire).
- `tests/text_fit.rs` (fixture `mz_fit`) : réglages, scan, aperçu, choix par texte, QA, export.
- `tests/real_game_coverage.rs` (`#[ignore]`) : `real_game_font_widths`, `real_game_fit_dump`,
  `real_game_fit_export`. Un harnais Node (hors dépôt) rejoue les sorties dans le code
  `Window_Base`/`Window_Message` du jeu : 0 différence de pages, lignes et largeurs sur 2 480
  messages dans quatre modes ; les 2 471 messages exportés sont identiques au dump vérifié.

## Limites connues

- MZ uniquement (MV : géométrie, dessin et `\FS` différents).
- Choix, texte défilant, journal de combat et fenêtre de nom ne sont pas ajustés.
- Les plugins inconnus qui changent la fenêtre sont signalés mais pas modélisés.
- La 1re ligne de description affichée par TinyGetInfoWndMZ reste compressée dans 424 px : une
  balise `<info:…>` (non extraite) donnerait un texte de popup dédié.
