use tray_icon::Icon;

pub fn tray_icon() -> Icon {
    Icon::from_rgba(crate::icon_pixels::rgba(), 32, 32).expect("valid icon")
}
