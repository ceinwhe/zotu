pub enum Table {
    Library,
    Favorite,
    History,
}

impl Table {
    pub fn as_str(&self) -> &str {
        match self {
            Table::Library => "library",
            Table::Favorite => "favorite",
            Table::History => "history",
        }
    }
}