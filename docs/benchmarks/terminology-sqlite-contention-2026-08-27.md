# Mesure de contention SQLite du scanner terminologique

Date: 2026-08-27

## Contrat mesuré

- 260 segments MV/MZ, soit plus d'un chunk de 250.
- Analyseur artificiellement ralenti à 1 ms par segment pour maintenir le scan actif.
- Mise à jour d'un segment via une seconde connexion pendant le travail CPU.
- Seuil d'acceptation: écriture terminée en moins de 250 ms, scan complet sans perte.

Le test automatisé `regular_segment_edits_remain_responsive_during_cpu_heavy_scan`
valide ce contrat. Le travail linguistique est effectué hors du runtime asynchrone et
aucune transaction SQLite n'est gardée ouverte pendant l'analyse CPU. Les transactions
d'écriture contiennent au maximum 250 segments.

## Décision

Conserver `max_connections=5` et les réglages SQLite actuels. Les mesures ne justifient
pas encore d'activer WAL, `synchronous=NORMAL` ou un `busy_timeout` global. Cette décision
évite une configuration plus complexe avant qu'un pilote réel ne démontre un problème
de contention.
