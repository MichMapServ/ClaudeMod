# HorizonUtilities Native 0.5 — installation et limites

Mod natif **source-only** au format `.hnmod`, injecté avec Horizon Mixins lors de la compilation du client par le lanceur. **Ne pas déposer ce paquet dans le dossier des `.hmod` WebAssembly.**

## Versions verrouillées

- Horizon `8474ac55c9f6f581e2bb984af9efcb18666bdbb2` (Minecraft 26.3)
- Mixins `3f99a5c60dfa311cc365524c44ae2fd799bfa1de`
- Native Hooks API **2** ; le hook sur `add_pickup` est lié à cette révision exacte
- Code : `native/HorizonUtilities/mixins/horizon_utilities`
- Test et paquet : `python3 scripts/build_hnmod.py pack && python3 scripts/build_hnmod.py verify`
- CI : `.github/workflows/hnmod.yml` ; artifact `hnmod-HorizonUtilitiesNative`

## Modules et limites

| Module | Fonction | Limite honnête |
|---|---|---|
| Nutrition | Faim, saturation et épuisement local, projection aliment tenu | Client local, pas forcément état serveur |
| Armure | 4 slots, pourcentage durabilité par définition d'item | Overrides composants non testés |
| Pickup | Evénement natif de collecte | Quantité exacte de ramassage partiel inconnue |
| Inventory | Tri déterministe par clics d'inventaire du jeu | Pas de transaction ni rollback serveur ; prudence en multijoueur |
| Performance | FPS, 1% low et CPS | Mesures du client, pas un benchmark |
| Navigation | 8 points/profil, 12 profils, cap directionnel, distance et Y | Profil manuel pour isoler les mondes |
| Minimap | Mini-carte moteur activable | Affichée seulement si non occupée par un autre mod |

Réglages dans **Options > Mods** : HUD, nutrition, alertes, armure, pickups, FPS/CPS, inventaire, navigation, mini-carte, compact. Touche N : activation globale, reconfigurable par le jeu.

## Commandes

    /utilities help
    /utilities status
    /utilities toggle
    /utilities sort main
    /utilities sort chest
    /utilities profile use NOM
    /utilities profile off
    /utilities profile list
    /utilities wp add NOM
    /utilities wp select NOM
    /utilities wp remove NOM
    /utilities wp list
    /utilities wp off

Utiliser le tri d'abord sur une copie de monde. Un rejet du serveur ou une fermeture de menu peut interrompre la séquence. Contrôler les slots si le tri signale un échec.

## Installer et vérifier

Dans l'artifact GitHub Actions, depuis le dossier qui contient les deux fichiers :

    sha256sum --check HorizonUtilitiesNative-0.5.0.hnmod.sha256

Installer via le **lanceur Horizon** compatible avec les mods natifs, activer l'option expérimentale, lire le code et accepter explicitement la compilation. Le lanceur reconstruit une instance de Horizon. Une compilation réussie ne valide pas les interactions en jeu, particulièrement les tris et le multijoueur.

## Deux éditions indépendantes

L'édition `mods/HorizonUtilities` reste un `.hmod` WebAssembly limité par son API publique. `experimental/horizon-utilities-ext` est un prototype de SDK étendu **non requis** par le code natif. Ne pas installer les deux HUD simultanément sans vérifier leurs interactions.
