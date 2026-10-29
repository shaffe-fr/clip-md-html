# clip-md-html

Petit utilitaire de barre d'état (Windows et macOS) qui transforme le Markdown du presse-papiers en HTML riche, puis le colle. Idéal pour copier du Markdown généré par Kiro (ou un autre outil) et le coller mis en forme dans Thunderbird, sans le style de l'éditeur d'origine.

Le HTML produit est sémantique et sans CSS : Thunderbird applique son style par défaut. Tableaux, texte barré et listes de tâches sont gérés.

## Installation

Depuis les sources (Rust stable requis) :

```sh
cargo install --path .
```

Ou récupérez un binaire de release et placez-le où vous voulez.

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

## Développement

```sh
cargo build --release
cargo clippy -- -D warnings
cargo test
```

La CI GitHub Actions construit et teste sous Windows et macOS.

## Licence

MIT
