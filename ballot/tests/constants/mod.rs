pub const ALICE_NAME: &str = "Alice";
pub const BOB_NAME: &str = "Bob";

pub const TRISS: &str = "Triss";
pub const YENNEFER: &str = "Yennefer";
pub const CIRI: &str = "Ciri";
pub const SHANI: &str = "Shani";
pub const GERALT: &str = "Geralt";

macro_rules! get_subjects {
    () => {
        vec![TRISS, YENNEFER, CIRI, SHANI, GERALT]
    };
}
