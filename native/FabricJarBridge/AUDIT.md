# Audit de compatibilité et de robustesse — Fabric Jar Bridge

Date : 2026-10-30. Cible : Horizon 26.3, commit épinglé
\`8474ac55c9f6f581e2bb984af9efcb18666bdbb2\`.

## Verdict

**L'objectif « n'importe quel mod Fabric peut être lu » n'est réalisable
que pour l'inspection des archives et de leur métadonnées, PAS pour
l'exécution de leurs fonctionnalités.** Une grande partie des JAR Fabric
contiennent du bytecode qui appelle les classes Minecraft Java ou modifie
le bytecode de Minecraft avec des Mixins. Ces classes et ce bytecode
n'existent pas en tant que tel dans Horizon, réécrit en Rust.

Le bridge propose :
- un import limité de données et ressources compatibles ;
- un véritable processus JVM opt-in, capable de démarrer une petite classe
  \`ModInitializer\` ou \`ClientModInitializer\` qui utilise les shims fournis ;
- une API partielle de commandes, positions du joueur et événements de tick ;
- une inspection **hors ligne** d'archives Fabric, sans exécuter Java, avec
  résultats JSON, listes de dépendances et références Java manquantes.

Il ne propose pas : Fabric Loader complet, Minecraft Java complet, Fabric
API complète, remapping universel, moteur Mixin SpongePowered, ni
compatibilité garantie avec des mods Fabric du catalogue.

## Constat d'audit

| Gravité | Problème | Remédiation |
| --- | --- | --- |
| P0 | Version 0.3.0 : erreur Rust \`E0277\` (\`Default\` sur \`Child\`/\`SyncSender\`) empêchait le build natif | Suppression du derive incorrect; compilation CI requise |
| P0 | Lecture de stdout JVM via \`read_until\` allouait une ligne de taille illimitée | Lecture par chunks avec budget maximal de 2048 octets par ligne, reste consommé sans allocation |
| P1 | Lancement d'un JAR déclarant Mixins, \`accessWidener\`, libs imbriquées ou dépendances Java/Fabric externes non disponibles | Refus explicite et diagnostic du blocage avant exécution Java |
| P1 | Aucune validation structurelle globale des archives avant lancement de Java | Plafonds 128 MiB / 4096 entrées, rejet doublons, liens symboliques et chemins dangereux |
| P1 | stdout Java pouvait prétendre enregistrer des commandes réservées | Noms réservés rejetés ; la JVM reste non sandboxée après autorisation SHA-256 |
| P1 | Le fait de détecter un JAR semblait constituer une promesse de compatibilité | Rapport hors ligne présentant \`blocked\`, \`candidate_only\`, \`no_entrypoint\`, ressources candidates, dépendances/Mixins |
| P1 | Aucune analyse des références \`net.minecraft\` / \`net.fabricmc\` dans les .class | Scan borné de constant pools classfile ; inventaire des références absentes du shim |
| P2 | Tests trop centrés sur le mod Java spécialement conçu pour le bridge | Tests ZIP adversariaux, dépendances, Mixin, access widener, protocole, références Java |
| P2 | Compatibilité des IDs Fabric avec tirets non implémentée pour l'overlay Horizon | Détection et alerte dans l'audit ; pas de renaming silencieux des namespaces |
| P2 | Version Java nécessaire et mappings non vérifiés | Versions de bytecode signalées, sans garantie de compatibilité avec le JDK installé |

## Sécurité et hypothèses

Le mode Java est désactivé par défaut, et une autorisation manuelle
\`/fabricbridge trust <id>\` accepte une archive par empreinte SHA-256.
**Un JAR approuvé peut lire/effacer des fichiers ou utiliser le réseau
avec les droits Windows du joueur.** Une limite temporelle ou de taille
ne constitue pas une sandbox.

Ne lancer que les JARs provenant de sources dignes de confiance. Faire
une sauvegarde du monde avant tout essai. Les mods et leurs dépendances
peuvent comporter du code malveillant.

La JVM est lancée en processus fils. Aucune séparation de privilèges,
isolation système ou protection contre les threads Java malveillants
n'existe dans ce prototype.

## Utiliser l'audit hors ligne

Depuis la racine du dépôt :

\`\`\`powershell
python scripts/audit_fabric_jars.py "C:\chemin\mod-fabric.jar"
python scripts/audit_fabric_jars.py "C:\chemin\mod-fabric.jar" --json
\`\`\`

Cette commande **n'exécute jamais le JAR**. Les conclusions du scanner
sont conservatrices : une référence de classe Java manquante peut
apparaître sur un chemin de code jamais utilisé, et une classe manquante
peut aussi n'être découverte qu'au lancement. La catégorie
\`candidate_only\` n'est pas une validation en jeu.

Le scanner limite le nombre de JARs via les entrées explicites, la
taille ZIP à 128 MiB, les entrées à 4096, les métadonnées à 64 Kio,
les ressources individuelles à 8 Mio et la lecture du constant pool à
256 classes / 8 Mio. Aucun chargement de classe n'a lieu.

## Tests de validation requis

1. \`python3 -m unittest discover -s scripts/tests -p 'test_fabric_jar_audit.py' -v\`
2. \`python3 -m unittest discover -s scripts/tests -p 'test_fabric_jar_bridge.py' -v\`
3. \`python3 scripts/build_fabric_jar_bridge.py pack\` et \`verify\`
4. \`python3 scripts/build_fabric_bridge_demo.py\` avec JDK local
5. \`horizon-mixins apply\` avec sources épinglées
6. \`cargo check --locked --no-default-features -p horizon-client --bin horizon-client\`
7. \`cargo test --locked --no-default-features -p horizon-client --bin horizon-client fabric_java_bridge::tests\`
8. Essai réel sous Windows : \`/fabricbridge list\`, \`/fabricbridge trust fabricbridge_demo\`, puis \`/javaproof\` et \`/javapos\` en monde.

**Un CI Linux vert ne remplace pas l'étape 8.**

## Lacunes fondamentales — non corrigées par cet audit

- **Fabric Loader** : résolution transitive des dépendances, classloaders,
  language adapters et points d'entrée personnalisés.
- **Minecraft Java** : des milliers de classes/méthodes devant être
  reliées au monde, réseau, rendu et simulation Rust d'Horizon ;
  distribuer une copie de Minecraft Java n'unifierait pas les deux mondes.
- **Fabric API** : ses modules sont majoritairement absents ; seules
  les très petites façades identifiées dans \`FABRIC_COMPATIBILITY.md\`
  existent.
- **Mixins** : une transformation de bytecode visant des classes Java
  ne s'applique pas directement à du Rust. Pour chaque transformation,
  il faut une cible Java de compatibilité valide ou un équivalent de
  hook natif explicite.
- **Mappings** : Yarn/intermediary/official et changement des versions ;
  en l'absence de remappage les références de méthodes/classes
  peuvent rester incompatibles.
- **Tests de mods du catalogue** : validation mod-par-mod, gestion des
  comportements partiels et des crashes.

Ces lacunes constituent un programme de développement à long terme,
pas des bogues corrigibles avec quelques lignes. Aucune prétention à
la compatibilité universelle ne doit être publiée.

## Décision de livraison

Ne distribuer le bridge comme version vérifiée qu'après CI vert
et confirmation Windows. Les branches sont en PR draft : audit basé
sur #20, qui dépend de #19 et de #18. Aucun merge automatique.
