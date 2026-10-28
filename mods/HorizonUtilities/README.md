# Horizon Utilities 0.2 — Mod Horizon autonome

Le mod se compile en **un seul fichier .hmod**. Il utilise exclusivement les fonctions officielles du SDK Horizon, sans compagnon, API externe ni injection.

## Fonctionnalités disponibles

- **Nutrition HUD** : faim réelle 0–20, segments, aperçu de restauration avec les aliments vanilla connus, pertes de points potentielles, notes d'effets.
- **Survie** : avertissements si la faim ou la vie sont au plus à 6/20.
- **Performances** : FPS, 1% low, clics par seconde (attaque/utilisation).
- **Navigation** : jusqu'à 8 waypoints sauvegardés, avec coordonnées, dimension, distance, cap relatif et dénivelé.
- **Mini-carte** : mini-carte officielle du moteur, optionnelle, orientation configurable ; un seul mod à la fois peut l'afficher.
- **Variations de la main** : différences de quantité du même stack, dans le même emplacement, affichées brièvement. **Ce ne sont pas des notifications fiables de ramassage**.
- **Réglages** : Options > Mods > Horizon Utilities, raccourci N pour activer/désactiver le HUD nutrition, sauvegarde locale via l'API storage.

## Commandes

    /utilities status
    /utilities nutrition on|off|toggle
    /utilities extras on|off|toggle
    /utilities wp add NOM
    /utilities wp select NOM
    /utilities wp remove NOM
    /utilities wp list
    /utilities wp off
    /utilities wp clear
    /utilities reset

La commande wp add enregistre les coordonnées et la dimension courantes. Le nom doit contenir 1 à 24 caractères ASCII alphanumériques, tiret ou soulignement. Un waypoint dans une autre dimension ne prétend pas avoir un cap valable.

## Limites impossibles à contourner dans le fichier .hmod actuel

Le SDK horizon:mods@0.1 ne donne PAS accès à :
- **Inventory Manager automatique** : emplacements, conteneurs, déplacement et tri des objets.
- **Armor HUD réel** : équipement porté, durabilité et composants des pièces d'armure.
- **Pickup Notifier fiable** : événement de ramassage et provenance exacte des objets.
- **Saturation exacte** : propriété de l'état de nutrition du joueur.

Les informations existent dans le moteur et certaines dans sa commande de contrôle locale, mais elles ne sont pas exportées à la sandbox WebAssembly. Ces modules requièrent une évolution du moteur Horizon. Ils ne sont pas déclarés fonctionnels par ce mod.

## CI et compilation

Depuis mods/HorizonUtilities :

    cargo fmt --check
    cargo test --lib
    cargo horizon-mod build
    cargo horizon-mod check CHEMIN_DU_HMOD

GitHub Actions (.github/workflows/hmod.yml) vérifie les tests, compile le paquet et fournit l'artifact hmod-HorizonUtilities, contenant HorizonUtilities.hmod.

Les tests de logique et la compilation ne remplacent pas un test visuel dans le jeu. Le SDK ui, render et input est expérimental et peut changer d'une version de Horizon à l'autre.
