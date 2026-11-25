use crate::engine::rex::XpFile;

const EMBEDDED: [(&str, &[u8]); 3] = [
    (
        "../resources/SmallDungeon_80x50.xp",
        include_bytes!("../resources/SmallDungeon_80x50.xp"),
    ),
    (
        "../resources/wfc-demo1.xp",
        include_bytes!("../resources/wfc-demo1.xp"),
    ),
    (
        "../resources/wfc-populated.xp",
        include_bytes!("../resources/wfc-populated.xp"),
    ),
];

pub fn load_xp(path: &str) -> Result<XpFile, String> {
    let (_, bytes) = EMBEDDED
        .iter()
        .find(|(p, _)| *p == path)
        .ok_or_else(|| format!("no embedded .xp file {path}"))?;
    XpFile::from_bytes(bytes)
}

pub struct RexAssets {
    pub menu: XpFile,
}

impl RexAssets {
    #[allow(clippy::new_without_default)]
    pub fn new() -> RexAssets {
        RexAssets {
            menu: load_xp("../resources/SmallDungeon_80x50.xp").unwrap(),
        }
    }
}
