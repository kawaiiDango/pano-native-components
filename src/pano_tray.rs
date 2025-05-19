#[derive(Debug)]
pub struct PanoTray {
    pub tooltip: String,
    pub icon_argb: Vec<u8>,
    pub icon_dim: u32,
    pub menu_items: Vec<(String, String)>,
}
