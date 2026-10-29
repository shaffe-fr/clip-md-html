use tray_icon::Icon;

/// Icône de la barre d'état (32x32 générée, aucun fichier externe).
pub fn tray_icon() -> Icon {
    Icon::from_rgba(crate::icon_pixels::rgba(), 32, 32).expect("icône valide")
}
