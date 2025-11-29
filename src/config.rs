enum OnConflict {
    Override,
    Error,
}

enum Resolution {
    CurrentFolder,
    Recursive,
    GitRoot,
}

pub struct GlobalConfig {
    pub on_conflict: OnConflict,
    pub resolution: Resolution,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig {
            on_conflict: OnConflict::Override,
            resolution: Resolution::Recursive,
        }
    }
}
