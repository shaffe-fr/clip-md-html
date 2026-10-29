# clip-md-html

[![CI](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml/badge.svg)](https://github.com/shaffe-fr/clip-md-html/actions/workflows/ci.yml)

Petit utilitaire de barre d'état (Windows et macOS) qui transforme le Markdown du presse-papiers en HTML riche, puis le colle. Idéal pour copier du Markdown généré par Kiro (ou un autre outil) et le coller mis en forme dans Thunderbird, sans le style de l'éditeur d'origine.

Le HTML produit est sémantique et sans CSS : Thunderbird applique son style par défaut. Tableaux, texte barré et listes de tâches sont gérés.

## Installation

Depuis les sources (Rust stable requis) :

```sh
cargo install --path .
```

Ou téléchargez un binaire depuis la page [Releases](https://github.com/shaffe-fr/clip-md-html/releases) et placez-le où vous voulez.

## Utilisation

1. Copiez le Markdown (par exemple depuis Kiro).
2. Placez le curseur dans la fenêtre de destination (corps d'un mail Thunderbird).
3. Appuyez sur le raccourci : le presse-papiers est converti puis collé automatiquement.

| Système | Raccourci par défaut |
|---------|----------------------|
| Windows | `Win+Alt+V` |
| macOS   | `Cmd+Option+V` |

Le Markdown d'origine reste disponible en texte brut dans le presse-papiers (collage « sans format » possible).

Menu de l'icône : « Convertir maintenant » (convertit sans coller), « Démarrer avec Windows » (démarrage à l'ouverture de session sur macOS) et « Quitter ». Une seule instance tourne à la fois.

### Mode ponctuel

```sh
clip-md-html --once
```

Convertit le presse-papiers en HTML puis quitte, sans simuler de collage. Code de sortie 1 si le presse-papiers est vide ou non textuel.

## Configuration

Fichier créé au premier lancement :

- Windows : `%APPDATA%\clip-md-html\config.toml`
- macOS : `~/Library/Application Support/clip-md-html/config.toml`

```toml
hotkey = "Win+Alt+V"   # macOS : "Cmd+Alt+V"
```

Modificateurs : `Win`/`Cmd`, `Alt`/`Option`, `Ctrl`, `Shift`. Redémarrez l'application après modification. Éviter `Ctrl+Alt` (AltGr sur clavier AZERTY, refusé sous Windows) et `Ctrl+Maj+V` (collage sans format de Thunderbird, refusé). Si le raccourci est déjà pris, une notification l'indique. Un journal `clip-md-html.log` se trouve dans le même dossier.

## Démarrage automatique

- Windows : case « Démarrer avec Windows » du menu (clé `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- macOS : même case, qui crée `~/Library/LaunchAgents/fr.shaffe.clip-md-html.plist` (actif à la prochaine session).

## macOS : autorisation Accessibilité

Pour simuler `Cmd+V`, macOS exige l'autorisation Accessibilité. Ouvrez Réglages Système > Confidentialité et sécurité > Accessibilité, ajoutez et activez `clip-md-html` (ou le terminal qui le lance), puis relancez l'application. Sans elle, le contenu est converti mais vous devez coller vous-même avec `Cmd+V` ; une notification vous le rappelle. L'application n'a pas d'icône dans le Dock, seulement dans la barre de menus.


## Antivirus

Combiné à l'absence de signature sur macOS (voir ci-dessous), l'exécutable combine un raccourci global, un collage simulé (`SendInput` / `CGEvent`), le presse-papiers et un démarrage automatique optionnel : certains antivirus (heuristiques) peuvent donc le signaler à tort. Ce que fait réellement le programme : il enregistre un seul raccourci via `RegisterHotKey`, sans hook clavier ni lecture de l'état des touches, n'écrit rien en dehors de sa configuration, ne télécharge rien et ne se copie nulle part. Le démarrage automatique n'est activé que si vous cochez la case dans le menu. Le code source est ouvert et l'exécutable embarque ses métadonnées (nom, description, version) et un manifeste sans élévation.

Pendant le développement, vous pouvez ajouter une exclusion pour le dossier `target/` du dépôt.
## Signature du code
- Windows : la signature par SignPath Foundation (service gratuit pour les projets open source) est demandée et en attente d'approbation. Tant qu'elle n'est pas active, les binaires Windows ne sont pas signés et SmartScreen peut afficher un avertissement.
- macOS : les binaires ne sont pas signés pour l'instant (l'Apple Developer Program est payant). Après téléchargement, retirez la quarantaine avec `xattr -d com.apple.quarantine ./clip-md-html`, ou faites un clic droit sur le fichier puis « Ouvrir ».

## Développement

```sh
cargo build --release
cargo clippy -- -D warnings
cargo test
```

La CI GitHub Actions construit et teste sous Windows et macOS. Une release est publiée à chaque tag `v*`.

## Contribuer
Les issues et pull requests sont les bienvenues. Lancez `cargo clippy -- -D warnings` et `cargo test` avant de proposer une modification. Vulnérabilité : voir [SECURITY.md](SECURITY.md).

## Licence

MIT
